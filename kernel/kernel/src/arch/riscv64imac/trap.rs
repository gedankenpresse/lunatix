use riscv::cpu::{InterruptBits, SStatusFlags, StVecData, TrapEvent};

/// A struct to hold relevant data for tasks that are executed on the CPU which are not directly part of the kernel.
/// It is mainly used to hold the tasks register data so that it can be interrupted, resumed and generally support
/// context switching.
///
/// ## ABI
/// The layout of this data structure is important because it is accessed via the assembly code
/// in `./asm/trap.S`.
///
/// - 32 general purpose register stores
/// - 32 floating point register stores
/// - a pointer to the rust trap handler stack
/// - a program counter indicating where to jump to when switching to this task
///
/// Remaining fields afterwards can be arbitrary in size and value.
#[repr(C)]
#[derive(Debug)]
pub struct TrapFrame {
    /// Storage for backing up all 32 general purpose registers
    pub general_purpose_regs: [usize; 32],
    /// Storage for backing up all 32 floating point registers
    pub floating_point_regs: [usize; 32],
    /// A pointer to the kernel stack that handles a trap for this frame.
    /// There is usually only one kernel interrupt handler stack so all task's trap frames will usually have the same
    /// value.
    ///
    /// This value is set by the kernel scheduler when yielding to the task owning this frame.
    pub trap_handler_stack: *mut usize,
    /// Program counter value which will be used when switching to this task
    pub start_pc: usize,
}

impl TrapFrame {
    /// Create a new trap frame with null initialized values
    ///
    /// # Safety
    /// It is safe to construct this instance however it should not be used directly because things like the stack
    /// pointer are definitely invalid.
    pub fn null() -> Self {
        Self {
            general_purpose_regs: Default::default(),
            floating_point_regs: Default::default(),
            trap_handler_stack: 0x0 as *mut usize,
            start_pc: 0,
        }
    }

    /// Set the stack start address to the given value
    pub fn set_stack_start(&mut self, stack_start: usize) {
        self.general_purpose_regs[2] = stack_start;
    }

    pub fn set_entry_point(&mut self, entry_point: usize) {
        self.start_pc = entry_point;
    }

    pub fn get_syscall_number(&mut self) -> usize {
        self.general_purpose_regs[10]
    }

    /// Get the arguments that are used for syscalls and IPC mechanisms.
    ///
    /// These are the registers `x10`-`x17` as they are defined in the RISCV specification to be
    /// used for function arguments.
    pub fn get_syscall_args_mut(&mut self) -> &mut [usize] {
        &mut self.general_purpose_regs[11..=17]
    }

    pub fn get_syscall_args(&self) -> &[usize] {
        &self.general_purpose_regs[11..=17]
    }

    /// Write the return data of a syscall into the frames registers.
    ///
    /// These are the registers `a0` and `a1` as they are defined in the RISCV specification to be
    /// used for function return values.
    pub fn write_syscall_return(&mut self, data: [usize; 8]) {
        // fill the registers a0 to a a7
        self.general_purpose_regs[10..=17].copy_from_slice(&data)
    }
}

/// Context information about the last triggered trap of a [`TrapFrame`]
#[repr(C)]
#[derive(Debug)]
pub struct TrapInfo {
    /// The exception program counter.
    ///
    /// This is the program counter at the point at which the trap was triggered.
    /// Essentially, the program counter of the interrupted code.
    pub epc: usize,

    /// The event that caused the trap to trigger.
    pub cause: TrapEvent,

    /// Supervisor bad address or instruction data.
    ///
    /// If the `cause` field indicates that the cpu encountered a bad instruction or tried to access a bad memory
    /// address, this field holds that bad instruction or bad address.
    /// However, this value is very specific to the instruction cause so care should be taken when interpreting it.
    pub stval: u64,

    /// Information about the execution conditions under which a trap was triggered.
    pub status: SStatusFlags,
}

impl TrapInfo {
    /// Construct an instance by reading the values that are currently stored in the corresponding CPU registers
    pub fn from_current_regs() -> Self {
        Self {
            epc: riscv::cpu::Sepc::read(),
            cause: riscv::cpu::Scause::read(),
            stval: riscv::cpu::StVal::read(),
            status: riscv::cpu::SStatus::read(),
        }
    }
}

/// Return type of the [`rust_trap_handler`] function
///
/// ## ABI
///
/// When this struct is returned by the rust trap handler, the two fields `frame` and `pc` are written
/// to the registers `a0` and `a1` automatically because that's how the compiler implements `return` in this case.
///
/// This is important because `./asm/trap.S` expects `rust_trap_handler` to return a new [`TrapFrame`] pointer as well
/// as a new program pointer in exactly these registers `a0` and `a1`.
pub type TrapReturn<'a> = &'a mut TrapFrame;

extern "C" {
    /// Restore the given trap frames cpu registers and set the program count to the given value.
    ///
    /// This is implemented by `./asm/trap.S`.
    pub fn trap_frame_load(trap_frame: *mut TrapFrame);
}

/// this function is unsafe, because it actually should
/// be careful to not clobber registers and stack, because
/// this function is not called by expected rust calling convention,
/// but just directly jumped to by assembly
unsafe fn kernel_trap_handler() -> ! {
    unsafe { core::arch::asm!(".align 8") };
    let info = TrapInfo::from_current_regs();
    panic!("kernel trap: {:#0x?}", info);
}

pub unsafe fn set_kernel_trap_handler() {
    let handler: usize = kernel_trap_handler as usize;
    log::trace!("kernel trap handler address: {handler:0x}");
    unsafe {
        riscv::cpu::StVec::write(&StVecData {
            mode: 0,
            base: handler as u64,
        });
    }
}

pub unsafe fn set_user_trap_handler() {
    extern "C" {
        /// the asm_trap_handler function is hand written assembly that
        /// calls the [`rust_trap_handler`] with appropriate arguments
        fn asm_trap_handler();
    }
    let handler = asm_trap_handler as usize;
    log::trace!("trap handler address: {handler:0x}");
    unsafe {
        // set trap handler to our asm_trap_handler function
        riscv::cpu::StVec::write(&StVecData {
            mode: 0,
            base: handler as u64,
        });
    }
}

/// Instruct the CPU to call our trap handler for interrupts and enable triggering of traps.
pub fn enable_interrupts() {
    log::debug!("enabling supervisor interrupts triggering asm_trap_handler");

    unsafe {
        set_user_trap_handler();
        // configure certain interrupt sources to actually trigger an interrupt
        riscv::cpu::Sie::write(
            InterruptBits::SupervisorExternalInterrupt
                | InterruptBits::SupervisorSoftwareInterrupt
                | InterruptBits::SupervisorTimerInterrupt,
        );
        //  globally enable interrupts for the previous configuration now
        riscv::cpu::SStatus::write(SStatusFlags::SPIE);
    }
}
