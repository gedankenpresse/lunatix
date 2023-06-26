use super::cpu;
use crate::arch::cpu::{InterruptBits, SStatusFlags, StVecData};

/// A struct to save all registers
#[derive(Debug, Default)]
#[repr(C)]
pub struct Regs {
    pub registers: [usize; 32],
}

/// A struct to hold a trapped Frames data so that it can be interrupted and resumed between
/// context switches.
///
/// ## ABI
/// The layout of this data structure is important because it is accessed via the assembly code
/// in `./asm/trap.S`.
///
/// - 32 general purpose register stores
/// - 32 floating point register stores
/// - a pointer to the rust trap handler stack
#[repr(C)]
#[derive(Debug)]
pub struct TrapFrame {
    /// Storage for backing up general purpose registers
    pub general_purpose_regs: Regs,
    /// Storage for backing up floating point registers
    pub floating_point_regs: Regs,
    /// A pointer to the kernel stack that handles a trap for this frame.
    /// There is usually only one kernel interrupt handler stack so all task's trap frames will usually have the same
    /// value.
    ///
    /// This value is set by the kernel scheduler when yielding to the task owning this frame.
    pub trap_stack: *mut usize,
    /// Context information about the last triggered trap
    pub ctx: TrapContext,
}

impl TrapFrame {
    /// Create a new trap frame with null initialized values
    ///
    /// # Safety
    /// It is safe to construct this instance however it should not be used directly because things like the stack
    /// pointer are definitely invalid.
    pub fn null() -> Self {
        Self {
            general_purpose_regs: Regs::default(),
            floating_point_regs: Regs::default(),
            ctx: TrapContext::null(),
            trap_stack: 0x0 as *mut usize,
        }
    }
}

/// Context information about the last triggered trap of a [`TrapFrame`]
#[repr(C)]
#[derive(Debug)]
pub struct TrapContext {
    /// The exception program counter.
    ///
    /// This is the program counter at the point at which the trap was triggered.
    /// Essentially, the program counter of the interrupted code.
    pub epc: usize,

    /// Supervisor bad address or instruction data.
    ///
    /// **TODO: What does this mean?**
    pub tval: usize,

    /// Supervisor trap cause.
    pub cause: usize,

    /// Currently unused
    pub _nohartid: usize,

    /// Supervisor status data.
    ///
    /// **TODO: Improve docs**
    pub status: usize,
}

impl TrapContext {
    /// Instantiate an *invalid* value
    ///
    /// This is useful if for example no trap has been triggered yet so no context data is available.
    pub fn null() -> Self {
        Self {
            epc: 0,
            tval: 0,
            cause: 0,
            _nohartid: -1isize as usize,
            status: 0,
        }
    }

    pub fn get_cause(&self) -> Cause {
        Cause::from(self.cause)
    }
}

/// Return type of the `rust_trap_handler` function
///
/// ## ABI
///
/// The layout of this data structure **must** be placed into registers the registers `a0` and `a1`
/// when being returned from `rust_trap_handler` because `./asm/trap.S` expects that.
#[repr(C)]
struct TrapReturn<'a> {
    /// A pointer to the [`TrapFrame`] of the task which should be switched to
    frame: &'a mut TrapFrame,
    /// The program counter value of the task which should be switched to
    pc: usize,
}

extern "C" {
    /// Restore the given trap frames cpu registers and set the program count to the given value.
    ///
    /// This is implemented by `./asm/trap.S`.
    pub fn trap_frame_restore(trap_frame: *mut TrapFrame, pc: usize) -> !;
}

/// Rust side of the trap handler code.
///
/// This function closely interoperates with `./asm/trap.S` and **must** therefore be ABI compatible.
///
/// ## ABI
/// `trap.S` passes the following function arguments:
/// - A pointer to the [`TrapFrame`]
/// - The program counter of the trapped frame
/// - [`TrapContext`] fields (see the struct field description for details)
#[inline(never)]
#[no_mangle]
extern "C" fn rust_trap_handler(
    tf: &mut TrapFrame,
    epc: usize,
    tval: usize,
    cause: usize,
    nohartid: usize,
    status: usize,
) -> TrapReturn {
    // save passed arguments into the TrapFrame
    let ctx = TrapContext {
        epc,
        tval,
        cause,
        _nohartid: nohartid,
        status,
    };
    tf.ctx = ctx;

    // call actual trap handler (which might decide to switch the execution to a different TrapFrame)
    extern "Rust" {
        fn handle_trap(_: &mut TrapFrame) -> &mut TrapFrame;
    }
    let res = unsafe { handle_trap(tf) };

    // return the new TrapFrame in the format expected by `trap.S`
    let epc = res.ctx.epc;
    TrapReturn {
        frame: res,
        pc: epc,
    }
}

/// The specific error which resulted in an exception
#[derive(Debug, Clone)]
pub enum Fault {
    Misaligned,
    AccessFault,
    PageFault,
}

/// In which stage of an instruction an exception was triggered
#[derive(Debug, Clone)]
pub enum Mode {
    Instruction,
    Load,
    Store,
}

/// The privilege level from which a trap was triggered
#[derive(Debug, Clone, PartialEq)]
pub enum Priv {
    User = 0,
    Supervisor = 1,
    Reserved = 2,
    Machine = 3,
}

/// The type of interrupt which was triggered
#[derive(Debug, Clone)]
pub enum InterruptType {
    Software = 0,
    Timer = 4,
    External = 8,
}

/// The cause for a triggered trap
#[derive(Debug, Clone)]
pub enum Cause {
    Interrupt(InterruptType, Priv),
    EcallFrom(Priv),
    IllegalInstruction,
    Breakpoint,
    Exception(Mode, Fault),
    Reserved,
}

impl From<usize> for Cause {
    fn from(num: usize) -> Self {
        const MXLEN_MASK: usize = usize::MAX >> 1; // isize::MAX is currently a private constant so we emulate it -.-
        use Cause::*;
        use Fault::*;
        use InterruptType::*;
        use Mode::*;
        if (num >> 5) != 0 {
            let prive = match num & 0b11 {
                0 => Priv::User,
                1 => Priv::Supervisor,
                2 => Priv::Reserved,
                3 => Priv::Machine,
                _ => unreachable!(),
            };
            let typ = match (num & MXLEN_MASK) >> 2 {
                0 => Software,
                1 => Timer,
                2 => External,
                _ => {
                    return Reserved;
                }
            };
            Interrupt(typ, prive)
        } else {
            match num {
                0 => Exception(Instruction, Misaligned),
                1 => Exception(Instruction, AccessFault),
                2 => IllegalInstruction,
                3 => Breakpoint,
                4 => Exception(Load, Misaligned),
                5 => Exception(Load, AccessFault),
                6 => Exception(Store, Misaligned),
                7 => Exception(Store, AccessFault),
                8 => EcallFrom(Priv::User),
                9 => EcallFrom(Priv::Supervisor),
                10 => EcallFrom(Priv::Reserved),
                11 => EcallFrom(Priv::Machine),
                12 => Exception(Instruction, PageFault),
                13 => Exception(Load, PageFault),
                14 => Reserved,
                15 => Exception(Store, PageFault),
                _ => Reserved,
            }
        }
    }
}

#[no_mangle]
fn handle_trap(tf: &mut TrapFrame) -> &mut TrapFrame {
    match tf.ctx.get_cause() {
        Cause::EcallFrom(Priv::User) => {
            log::debug!(
                "Got ecall from user: {}",
                tf.general_purpose_regs.registers[10] as u8 as char
            );
            tf.ctx.epc += 4;
            tf
        }
        _ => {
            log::debug!("Interrupt!: Cause: {:?}", tf.ctx.get_cause());
            log::debug!("PC: {:p}", tf.ctx.epc as *mut u8);
            log::debug!("{:#x?}", tf.ctx);
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
