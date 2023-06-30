use super::cpu;
use super::cpu::{Exception, InterruptBits, SStatusFlags, StVecData, TrapEvent};

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

    // ABI compatibility ends here
    /// Context information about the last triggered trap
    pub last_trap: Option<TrapInfo>,
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
            last_trap: None,
        }
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
            epc: cpu::Sepc::read(),
            cause: cpu::Scause::read(),
            stval: cpu::StVal::read(),
            status: cpu::SStatus::read(),
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
    pub fn trap_frame_restore(trap_frame: *mut TrapFrame) -> !;
}

/// Rust side of the trap handler code.
///
/// This function closely interoperates with `./asm/trap.S` and **must** therefore be ABI compatible.
///
/// ## ABI
/// `trap.S` passes the following function arguments:
/// - A pointer to the [`TrapFrame`]
/// - The program counter of the trapped frame
/// - [`TrapInfo`] fields (see the struct field description for details)
#[inline(never)]
#[no_mangle]
extern "C" fn rust_trap_handler(tf: &mut TrapFrame) -> TrapReturn {
    // save passed arguments into the TrapFrame
    tf.last_trap = Some(TrapInfo::from_current_regs());

    // call actual trap handler (which might decide to switch the execution to a different TrapFrame)
    extern "Rust" {
        fn handle_trap(_: &mut TrapFrame) -> &mut TrapFrame;
    }
    let res = unsafe { handle_trap(tf) };

    // return the new TrapFrame in the format expected by `trap.S`
    res
}

#[no_mangle]
fn handle_trap(tf: &mut TrapFrame) -> &mut TrapFrame {
    let last_trap = tf.last_trap.as_ref().unwrap();

    match last_trap.cause {
        TrapEvent::Exception(Exception::EnvCallFromUMode) => {
            crate::println!(
                "Got call from user: {}",
                tf.general_purpose_regs[10] as u8 as char
            );
            tf.start_pc = last_trap.epc + 4;
            tf
        }
        _ => {
            crate::println!("Interrupt!: Cause: {:#?}", last_trap);
            panic!("no interrupt handler specified");
        }
    }
}

/// Instruct the CPU to call our trap handler for interrupts and enable triggering of traps.
pub fn enable_interrupts() {
    log::debug!("enabling supervisor interrupts triggering asm_trap_handler");

    extern "C" {
        /// the asm_trap_handler function is hand written assembly that
        /// calls the [`rust_trap_handler`] with appropriate arguments
        fn asm_trap_handler();
    }

    let handler = asm_trap_handler as usize;
    unsafe {
        // set trap handler to our asm_trap_handler function
        cpu::StVec::write(&StVecData {
            mode: 0,
            base: handler as u64,
        });
        // configure certain interrupt sources to actually trigger an interrupt
        cpu::Sie::write(
            InterruptBits::SupervisorExternalInterrupt | InterruptBits::SupervisorSoftwareInterrupt,
        );
        //  globally enable interrupts for the previous configuration now
        cpu::SStatus::write(SStatusFlags::SIE);
    }
}

// pub fn enable_timer_interrupts() {
//     use super::asm_utils::*;
//     use InterruptBits::*;
//     unsafe {
//         clear_sip(STIE as usize);
//     }
//
//     unsafe {
//         set_sie(STIE as usize);
//     }
//
//     use super::sbi::time;
//     time::set_timer(1 << 22);
// }
