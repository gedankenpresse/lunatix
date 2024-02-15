//! Handling of CPU control and status registers
//!
//! This module implements some dummy structs which each model a certain cpu register as it is defined in
//! Chapter 4 of the [Risc-V Privileged Specification](https://github.com/riscv/riscv-isa-manual/releases/download/Priv-v1.12/riscv-privileged-20211203.pdf)

use bitflags::bitflags;
use core::arch::asm;
use core::fmt::{Debug, Formatter};

/// Generate code to read from a specific register.
///
/// # Example:
/// ```rust
/// let val = read_reg!("sstatus");
/// let val = read_reg!("sstatus", u64);
/// ```
macro_rules! read_reg {
    ($csr:literal,$width:ty) => {{
        let res: $width;
        asm!(concat!("csrr {}, ", $csr), out(reg) res);
        res
    }};
    ($csr:literal) => {read_reg!($csr, u64)};
}

/// Generate code to write to a specific register.
///
/// # Example
/// ```rust
/// write_reg!("sstatus", 42)
/// ```
macro_rules! write_reg {
    ($csr:literal, $value:expr) => {
        asm!(concat!("csrw ", $csr, ", {}"), in(reg) $value)
    };
}

/// Generate code to set specific register bits but leave others untouched
macro_rules! set_reg {
    ($csr:literal, $value:expr) => {
        asm!(concat!("csrs ", $csr, ", {}"), in(reg) $value)
    }
}

/// Generate code to clear specific register bits but leave others untouched
macro_rules! clear_reg {
    ($csr:literal, $value:expr) => {
        asm!(concat!("csrc ", $csr, ", {}"), in(reg) $value)
    }
}

/// Program Counter
///
/// The program counter holds the address of the current instruction..
/// Writing to this register is not implemented; use a jump instruction instead.
pub struct PC {}

impl PC {
    /// Read the current value of PC which yields the address of the current instruction
    ///
    /// Note that the returned value cannot be considered accurate.
    /// Concretely it refers to some instruction that is located inside this functions body and the compiler may or may not generate additional preamble and post-processing instructions.
    pub fn read() -> u64 {
        let res;
        unsafe {
            asm!("jal {}, 4", "nop", out(reg) res);
        }
        res
    }
}

/// Supervisor Status Register.
///
/// It keeps track of the processor's current operating state.
#[allow(dead_code)]
pub struct SStatus {}

bitflags! {
    #[derive(Debug)]
    pub struct SStatusFlags: u64 {
        /// The SPP bit indicates at which mode a hart was executing before entering supervisor mode.
        /// When a trap is taken, SPP is set to `0` if the trap originated from user mode and `1` otherwise.
        /// When an `SRET` instruction is executed to return from the trap handler the privilege level is set to user
        /// mode if the SPP bit is `0`, or supervisor mode if the SPP bit is `1`.
        /// SPP is then set to `0`.
        const SPP = 1 << 8;
        /// The SIE bit enables or disables all interrupts in supervisor mode.
        /// When SIE is clear, interrupts are not taken while in supervisor mode.
        /// When the hart is running in user-mode, the value in SIE is ignored, and supervisor-level interrupts are enabled.
        /// The supervisor can disable individual interrupt sources using the [SIE CSR](Sie).
        const SIE = 1 << 1;
        /// The SPIE bit indicates whether supervisor interrupts were enabled prior to trapping into supervisor mode.
        /// When a trap is taken into supervisor mode, SPIE is set to SIE, and SIE is set to 0.
        /// When an `SRET` instruction is executed, SIE is set to SPIE, then SPIE is set to 1.
        const SPIE = 1 << 5;
        /// The MXR (Make eXecutable Readable) bit modifies the privilege with which loads access virtual memory.
        /// When `MXR=0`, only loads from pages marked readable (`R=1` in Figure 4.18 of the [Privileged Specification](https://github.com/riscv/riscv-isa-manual/releases/download/Priv-v1.12/riscv-privileged-20211203.pdf)) will succeed.
        /// When `MXR=1`, loads from pages marked either readable or executable (R=1 or X=1) will succeed.
        /// MXR has no effect when page-based virtual memory is not in effect.
        const MXR = 1 << 19;
        /// The SUM (permit Supervisor User Memory access) bit modifies the privilege with which S-mode loads and stores access virtual memory.
        /// When `SUM=0`, S-mode memory accesses to pages that are accessible by U-mode (`U=1` in Figure 4.18 of the [Privileged Specification](https://github.com/riscv/riscv-isa-manual/releases/download/Priv-v1.12/riscv-privileged-20211203.pdf)) will fault.
        /// When `SUM=1`, these accesses are permitted.
        /// SUM has no effect when page-based virtual memory is not in effect, nor when executing in U-mode.
        /// Note that S-mode can never execute instructions from user pages, regardless of the state of SUM
        const SUM = 1 << 18;
        /// The UBE bit is a WARL field that controls the endianness of explicit memory accesses made from U-mode, which may differ from the endianness of memory accesses in S-mode.
        /// An implementation may make UBE be a read-only field that always specifies the same endianness as for S-mode.
        /// UBE controls whether explicit load and store memory accesses made from U-mode are little-endian (`UBE=0`) or big-endian (`UBE=1`).
        /// UBE has no effect on instruction fetches, which are implicit memory accesses that are always little-endian.
        /// For implicit accesses to supervisor-level memory management data structures, such as page tables, S-mode endianness always applies and UBE is ignored.
        ///
        /// *Standard RISC-V ABIs are expected to be purely little-endian-only or big-endian-only, with no
        /// accommodation for mixing endianness. Nevertheless, endianness control has been defined so as
        /// to permit an OS of one endianness to execute user-mode programs of the opposite endianness.*
        const UBE = 1 << 6;
    }
}

impl SStatus {
    /// Read the raw 64 bit value that are contained in the register
    pub fn read_raw() -> u64 {
        unsafe { read_reg!("sstatus") }
    }

    /// Read the value that is contained in the register
    pub fn read() -> SStatusFlags {
        SStatusFlags::from_bits_truncate(Self::read_raw())
    }

    /// Write a raw 64 bit value to the register.
    ///
    /// **Note**: Even though this function takes a raw value, only some bits are actually written according to the
    /// RISC-V specification.
    ///
    /// # Safety
    /// Because writing to this register can change how the processor operates it is fundamentally unsafe.
    /// Ensure that you write an intended value!
    pub unsafe fn write_raw(val: u64) {
        write_reg!("sstatus", val & SStatusFlags::all().bits())
    }

    /// Write a value to the register.
    ///
    /// # Safety
    /// Because writing to this register can change how the processor operates it is fundamentally unsafe.
    /// Ensure that you write an intended value!
    pub unsafe fn write(val: SStatusFlags) {
        Self::write_raw(val.bits())
    }

    /// Set the bits of this register where `mask` has a 1 but leave all others untouched
    pub unsafe fn set_raw(mask: u64) {
        set_reg!("sstatus", mask);
    }

    /// Set only those bits of the register to `1` where `mask` is set while leaving all other register bits untouched
    pub unsafe fn set(mask: SStatusFlags) {
        Self::set_raw(mask.bits())
    }

    /// Clear all register bits, setting them to `0` where `mask` has a `1`
    pub unsafe fn clear_raw(mask: u64) {
        clear_reg!("sstatus", mask)
    }

    /// Clear all register bits, setting them to `0` where `mask` is set
    pub unsafe fn clear(mask: SStatusFlags) {
        clear_reg!("sstatus", mask.bits())
    }
}

/// Supervisor Trap Vector Base Address Register
///
/// The stvec register is read/write register that holds trap vector configuration, consisting of a vector base address (BASE) and a vector mode (MODE).
#[allow(dead_code)]
pub struct StVec {}

/// The data contained in the [`StVec`] register in an easy-to-handle format.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct StVecData {
    /// A WARL field that can hold any valid virtual or physical address, subject to the following alignment constraints:
    /// - the address must be 4-byte aligned, and MODE settings other than Direct might impose additional alignment constraints on the value in the BASE field.
    pub base: u64,

    /// The encoding of the MODE field is shown below.
    /// When MODE=Direct, all traps into supervisor mode cause the pc to be set to the address in the BASE field.
    /// When MODE=Vectored, all synchronous exceptions into supervisor mode cause the pc to be set to the address in the BASE
    /// field, whereas interrupts cause the pc to be set to the address in the BASE field plus four times the
    /// interrupt cause number.
    /// For example, a supervisor-mode timer interrupt (see below) causes the pc to be set to BASE+0x14.
    /// Setting MODE=Vectored may impose a stricter alignment constraint on BASE.
    ///
    /// | Value | Name | Description |
    /// | :---: | ---- | ----------- |
    /// | `0` | Direct | All exceptions set pc to BASE |
    /// | `1` | Vectored | Asynchronous interrupts set pc to BASE+4×cause |
    /// | `>1` | - | *Reserved* |
    pub mode: u8,
}

impl StVec {
    /// Read the raw register value
    pub fn read_raw() -> u64 {
        unsafe { read_reg!("stvec") }
    }

    /// Read the register value
    pub fn read() -> StVecData {
        let raw_val = Self::read_raw();
        StVecData {
            mode: (raw_val & 0b11) as u8,
            base: raw_val & !(0b11 << 62),
        }
    }

    /// Write a raw value into the register
    pub unsafe fn write_raw(val: u64) {
        write_reg!("stvec", val)
    }

    /// Write a Trap Vector configuration to the register
    pub unsafe fn write(val: &StVecData) {
        assert_eq!(
            val.base & !((1 << 2) - 1),
            val.base,
            "StVec value uses an invalid base"
        );
        assert_eq!(
            val.mode & 0b11,
            val.mode,
            "StVec value uses an invalid mode"
        );
        Self::write_raw(val.base | val.mode as u64)
    }
}

impl Debug for StVecData {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StVec")
            .field("base", &format_args!("{:#x}", self.base))
            .field(
                "mode",
                &match self.mode {
                    0 => "direct",
                    1 => "vectored",
                    _ => "reserved",
                },
            )
            .finish()
    }
}

bitflags! {
    /// A bitmap mapping the CSR scause (Section 4.1.8 of the Privileged Specification) entry number to a bit that is
    /// set in the [`Sip`] and [`Sie`] registers for that specific interrupt.
    #[derive(Debug, Eq, PartialEq)]
    pub struct InterruptBits: u64 {
        /// Bits sip.SSIP and sie.SSIE are the interrupt-pending and interrupt-enable bits for supervisor-level software interrupts.
        /// If implemented, SSIP is writable in [sip](Sip) and may also be set to 1 by a platform-specific interrupt controller.
        ///
        /// Interprocessor interrupts are sent to other harts by implementation-specific means, which will ultimately cause the SSIP bit to be set in the recipient hart’s sip register.
        const SupervisorSoftwareInterrupt = 1 << 1;

        /// Bits sip.STIP and sie.STIE are the interrupt-pending and interrupt-enable bits for supervisor-level timer interrupts.
        /// If implemented, STIP is read-only in [sip](Sip), and is set and cleared by the execution environment.
        const SupervisorTimerInterrupt = 1 << 5;

        /// Bits sip.SEIP and sie.SEIE are the interrupt-pending and interrupt-enable bits for supervisor-level external interrupts.
        /// If implemented, SEIP is read-only in [sip](Sip), and is set and cleared by the execution environment, typically through a platform-specific interrupt controller.
        const SupervisorExternalInterrupt = 1 << 9;

        const UserTimerInterrupt = 1 << 4;
    }
}

/// The sip register is a read/write register containing information on pending interrupts.
/// It works in close correlation to the [sie](Sie) register which contains interrupt enable bits.
///
/// An interrupt i will trap to S-mode if **both** of the following are true:
/// - (a) either the current privilege mode is S and the [SIE bit](SStatusFlags::SIE) in the [sstatus](SStatus) register is set, or the current privilege mode has less privilege than S-mode; and
/// - (b) bit i is set in both sip and sie.
///
/// Interrupts to S-mode take priority over any interrupts to lower privilege modes
///
/// Each individual bit in register sip may be writable or may be read-only.
/// When bit i in sip is writable, a pending interrupt i can be cleared by writing 0 to this bit.
/// If interrupt i can become pending but bit i in sip is read-only, the cpu implementation must provide some other mechanism for clearing the pending interrupt (which may involve a call to the execution environment).
#[allow(dead_code)]
pub struct Sip {}

impl Sip {
    /// Read the raw register value
    pub fn read_raw() -> u64 {
        unsafe { read_reg!("sip") }
    }

    /// Read the pending interrupt bitmask
    pub fn read() -> InterruptBits {
        InterruptBits::from_bits_retain(Self::read_raw())
    }

    /// Write a raw value into the register
    ///
    /// # Safety
    /// Writing to the Sip register may immediately trigger an interrupt and stop further code from executing
    pub unsafe fn write_raw(val: u64) {
        write_reg!("sip", val)
    }

    /// Write a value to the register
    ///
    /// # Safety
    /// Writing to the Sip register may immediately trigger an interrupt and stop further code from executing
    pub unsafe fn write(val: InterruptBits) {
        Self::write_raw(val.bits())
    }

    pub unsafe fn set_raw(mask: u64) {
        set_reg!("sip", mask)
    }

    pub unsafe fn set(mask: InterruptBits) {
        set_reg!("sip", mask.bits())
    }

    pub unsafe fn clear_raw(mask: u64) {
        clear_reg!("sip", mask)
    }

    pub unsafe fn clear(mask: InterruptBits) {
        clear_reg!("sip", mask.bits())
    }
}

/// The sie register is a read/write register containing interrupt enable bits.
/// It works in close correlation to the [`Sip`] register which contains information on pending interrupts.
///
/// A bit in sie must be writable if the corresponding interrupt can ever become pending.
/// Bits of sie that are not writable are read-only zero.
///
/// Multiple simultaneous interrupts destined for supervisor mode are handled in the following decreas-ing priority order: [SEI](InterruptBits::SupervisorExternalInterrupt), [SSI](InterruptBits::SupervisorSoftwareInterrupt), [STI](InterruptBits::SupervisorTimerInterrupt).
#[allow(dead_code)]
pub struct Sie {}

impl Sie {
    /// Read the raw register value
    pub fn read_raw() -> u64 {
        unsafe { read_reg!("sie") }
    }

    /// Read the pending interrupt bitmask
    pub fn read() -> InterruptBits {
        InterruptBits::from_bits_retain(Self::read_raw())
    }

    /// Write a raw value into the register
    ///
    /// # Safety
    /// Writing to the sie register changes the hardware configuration to enable/disable certain interrupts which may
    /// break assumptions about the execution order of other code.
    pub unsafe fn write_raw(val: u64) {
        write_reg!("sie", val)
    }

    /// Write a value to the register
    ///
    /// # Safety
    /// Writing to the sie register changes the hardware configuration to enable/disable certain interrupts which may
    /// break assumptions about the execution order of other code.
    pub unsafe fn write(val: InterruptBits) {
        Self::write_raw(val.bits())
    }
}

bitflags! {
    /// When the `CY`, `TM`, `IR` or `HPMn` bit in the [`scounteren`](SCounterEn) register is clear, attempty to read
    /// the `cycle`, `time` `instrset` or `hpmcountern` register while executing in U-mode will cause an illegal
    /// instruction exception.
    /// When one of these bits is set, access to the corresponding register is permitted.
    #[derive(Debug, Eq, PartialEq)]
    pub struct ScounterBits: u32 {
        const CY = 0x0;
        const TM = 0x1;
        const IR = 0x2;
    }
}

/// The counter-enable register `scounteren` is a 32-bit register that controls the availability of the hardware
/// performance monitoring counters tu U-Mode.
#[allow(unused)]
pub struct SCounterEn {}

impl SCounterEn {
    /// Read the raw value contained in the register
    pub fn read_raw() -> u32 {
        unsafe { read_reg!("scounteren", u32) }
    }

    /// Read the value contained in the register.
    ///
    /// The returned `ScounterBits` retains additional values for the `HPMn` bits that are present in the raw register
    /// value.
    pub fn read() -> ScounterBits {
        ScounterBits::from_bits_retain(Self::read_raw())
    }

    /// Write a raw value into the register.
    ///
    /// # Safety
    /// Changing the register value affects the instructions available to other (U-mode) code and may suddenly break
    /// it.
    pub unsafe fn write_raw(value: u32) {
        unsafe { write_reg!("scounteren", value) }
    }

    /// Write a value into the register.
    ///
    /// # Safety
    /// Changing the register value affects the instructions available to other (U-mode) code and may suddenly break
    /// it.
    pub unsafe fn write(value: ScounterBits) {
        Self::write_raw(value.bits())
    }
}

/// Cycle counter register
///
/// This is the number of clock cycles executed by the processor core on which the hart is running from an arbitrary
/// star time in the past.
#[allow(unused)]
pub struct Cycle {}

impl Cycle {
    pub fn read() -> u64 {
        let res: u64;
        unsafe { asm!("rdcycle {}", out(reg) res) };
        res
    }
}

/// The Time register always holds the current cpu time
#[allow(unused)]
pub struct Time {}

impl Time {
    pub fn read() -> u64 {
        let res: u64;
        unsafe { asm!("rdtime {}", out(reg) res) };
        res
    }
}

/// Instruction counter register
///
/// This counts the number of instructions retired by this hart from some arbitrary start point in the past.
#[allow(unused)]
pub struct InstRet {}

impl InstRet {
    pub fn read() -> u64 {
        let res: u64;
        unsafe { asm!("rdinstret {}", out(reg) res) };
        res
    }
}

/// Supervisor Scratch Register
///
/// A read/write register, dedicated for use by the supervisor.
/// Typically, sscratch is used to hold a pointer to the hart-local supervisor context while the hart is executing user code.
/// At the beginning of a trap handler, sscratch is swapped with a user register to provide an initial working register.
#[allow(dead_code)]
pub struct SScratch {}

impl SScratch {
    pub fn read() -> usize {
        let res: usize;
        unsafe { asm!("csrr {}, sscratch", out(reg) res) };
        res
    }

    pub unsafe fn write(val: usize) {
        unsafe { asm!("csrw sscratch, {}", in(reg) val) }
    }
}

/// Supervisor Exception Program Counter
///
/// A 64 bit read/write register.
/// The low bit of sepc (sepc[0]) is always zero.
/// sepc is a WARL register that is able to hold all valid virtual addresses.
/// It need not be capable of holding all possible invalid addresses.
/// Prior to writing sepc, hardware implementations may convert an invalid address into some other invalid address that sepc is capable of holding.
///
/// When a trap is taken into S-mode, sepc is written with the virtual address of the instruction that was interrupted or that encountered the exception.
/// Otherwise, sepc is never written by the hardware implementation, though it may be explicitly written by software.
///
/// On implementations that support only IALIGN=32, the two low bits (sepc[1:0]) are always zero.
/// If an implementation allows IALIGN to be either 16 or 32 (by changing CSR misa, for example),
/// then, whenever IALIGN=32, bit sepc[1] is masked on reads so that it appears to be 0.
/// This masking occurs also for the implicit read by the SRET instruction.
/// Though masked, sepc[1] remains writable when IALIGN=32.
#[allow(dead_code)]
pub struct Sepc {}

impl Sepc {
    pub fn read() -> usize {
        let res: usize;
        unsafe { asm!("csrr {}, sepc", out(reg) res) };
        res
    }

    pub fn write(val: usize) {
        unsafe { asm!("csrw sepc, {}", in(reg) val) }
    }
}

/// An indication of the event that caused a trap to trigger
#[derive(Debug)]
pub enum TrapEvent {
    Interrupt(Interrupt),
    Exception(Exception),
}

impl From<u64> for TrapEvent {
    fn from(value: u64) -> Self {
        let is_interrupt = value >> 63 == 0b1; // highest 1 bit
        let cause = value & !(0b1 << 31); // lowest 31 bits
        if is_interrupt {
            TrapEvent::Interrupt(Interrupt::from(cause as u32))
        } else {
            TrapEvent::Exception(Exception::from(cause as u32))
        }
    }
}

/// An interrupt code indicating the cause of a trap
#[derive(Debug)]
pub enum Interrupt {
    SupervisorSoftwareInterrupt,
    SupervisorTimerInterrupt,
    SupervisorExternalInterrupt,
    Unknown(u32),
}

impl From<u32> for Interrupt {
    fn from(value: u32) -> Self {
        match value {
            1 => Interrupt::SupervisorSoftwareInterrupt,
            5 => Interrupt::SupervisorTimerInterrupt,
            9 => Interrupt::SupervisorExternalInterrupt,
            other => Interrupt::Unknown(other),
        }
    }
}

/// An exception indicating the cause of a trap
#[derive(Debug)]
pub enum Exception {
    InstructionAddressMisaligned,
    InstructionAccessFault,
    IllegalInstruction,
    Breakpoint,
    LoadAddressMisaligned,
    LoadAccessFault,
    StoreAddressMisaligned,
    StoreAccessFault,
    EnvCallFromUMode,
    EnvCallFromSMode,
    InstructionPageFault,
    LoadPageFault,
    StorePageFault,
    Unknown(u32),
}

impl From<u32> for Exception {
    fn from(value: u32) -> Self {
        match value {
            0 => Exception::InstructionAddressMisaligned,
            1 => Exception::InstructionAccessFault,
            2 => Exception::IllegalInstruction,
            3 => Exception::Breakpoint,
            4 => Exception::LoadAddressMisaligned,
            5 => Exception::LoadAccessFault,
            6 => Exception::StoreAddressMisaligned,
            7 => Exception::StoreAccessFault,
            8 => Exception::EnvCallFromUMode,
            9 => Exception::EnvCallFromSMode,
            12 => Exception::InstructionPageFault,
            13 => Exception::LoadPageFault,
            15 => Exception::StorePageFault,
            other => Exception::Unknown(other),
        }
    }
}

/// The `scause` register is a read-write register.
/// When a trap is taken into S-mode, `scause` is written with a code indicating the event that caused the trap.
/// Otherwise, `scause` is never written by the hardware implementation, though it may be explicitly written by software.
#[allow(dead_code)]
pub struct Scause {}

impl Scause {
    /// Read the raw 32 bits value from the register
    pub fn read_raw() -> u64 {
        unsafe { read_reg!("scause", u64) }
    }

    /// Read the cause of the triggered trap from the register
    pub fn read() -> TrapEvent {
        TrapEvent::from(Self::read_raw())
    }

    /// Clear all bits in the register, setting them to 0
    pub fn clear() {
        unsafe { write_reg!("scause", 0) }
    }
}

/// The `stval` register is a read-write register.
/// When a trap is taken into S-mode, `stval` is written with exception-specific information to assist software in handling the trap.
/// Otherwise, `stval` is never written by the hardware implementation, though it may be explicitly written by software.
/// The hardware platform will specify which exceptions must set `stval` informatively and which may unconditionally set it to zero.
///
/// If `stval` is written with a nonzero value when a **breakpoint, address-misaligned, access-fault**, or **page-fault** exception occurs **on an instruction fetch, load, or store**, then `stval` will contain the faulting virtual address.
///
/// If `stval` is written with a nonzero value when a **misaligned load*or store** causes an **access-fault or page-fault exception**, then `stval` will contain the virtual address of the portion of the access that caused the fault.
///
/// If `stval` is written with a nonzero value when an **instruction access-fault or page-fault exception** occurs **on a system with variable-length instructions**, then `stval` will contain the virtual address of the portion of the instruction that caused the fault, while [`sepc`](Sepc) will point to the beginning of the instruction.
///
/// The `stval` register can optionally also be used to return the faulting instruction bits on an illegal instruction exception ([`sepc`](Sepc) points to the faulting instruction in memory).
/// If stval is written with a nonzero value when an **illegal-instruction exception** occurs, then `stval` will contain the shortest
/// of:
/// - the actual faulting instruction
/// - the first ILEN bits of the faulting instruction
/// - the first SXLEN bits of the faulting instruction
#[allow(dead_code)]
pub struct StVal {}

impl StVal {
    pub fn read() -> u64 {
        unsafe { read_reg!("stval") }
    }
}

bitflags! {
    #[derive(Debug)]
    pub struct SEnvFlags: u64 {
        /// If bit FIOM (Fence of I/O implies Memory) is set to one in `senvcfg`, FENCE instructions executed in U-mode are modified so the requirement to order accesses to device I/O implies also the re-quirement to order main memory accesses.
        /// The below table details the modified interpretation of FENCE instruction bits PI, PO, SI, and SO in U-mode when `FIOM=1`.
        ///
        /// Similarly, for U-mode when FIOM=1, if an atomic instruction that accesses a region ordered as device I/O has its aq and/or rl bit set, then that instruction is ordered as though it accesses both device I/O and memory.
        ///
        /// If [satp.MODE](SatpData) is read-only zero (always Bare), the hardware implementation may make FIOM read-only zero.
        ///
        /// # Modified Interpretation of `FENCE` instruction
        ///
        /// | Instruction Bit | Meaning when set |
        /// | --- | --- |
        /// | PI<br>PO | Predecessor device input and memory reads (PR implied)<br>Predecessor device output and memory write (PW implied)
        /// | SI<br>SO | Successor device input and memory reads (SR implied)<br>Successor device output and memory writes (SW implied) |
        const FIOM = 0;
    }
}

/// Supervisor Environment Configuration Register
///
/// The `senvcfg` CSR is a read/write register, that controls certain characteristics of the U-mode execution environment.
#[allow(dead_code)]
pub struct SEnvCfg {}

impl SEnvCfg {
    pub fn read_raw() -> u64 {
        unsafe { read_reg!("senvcfg") }
    }

    pub fn read() -> SEnvFlags {
        SEnvFlags::from_bits_retain(Self::read_raw())
    }

    pub unsafe fn write_raw(val: u64) {
        write_reg!("senvcfg", val)
    }

    pub unsafe fn write(val: SEnvFlags) {
        Self::write_raw(val.bits())
    }
}

/// The data that is held by the [`Satp`] register.
///
/// Generally this register holds the physical page number (PPN) of the root page table, i.e., its supervisor physical address divided by 4 KiB;
/// an address space identifier (ASID), which facilitates address-translation fences on a per-address-space basis; and the MODE field, which selects the current address-translation scheme.
///
/// **Warning**: Read the mode variant descriptions carefully as they impose restrictions on valid values for the other fields.
#[derive(Debug, Eq, PartialEq)]
pub struct SatpData {
    pub mode: SatpMode,
    pub asid: u64,
    pub ppn: u64,
}

impl From<u64> for SatpData {
    fn from(value: u64) -> Self {
        SatpData {
            mode: SatpMode::from(value >> 60),   // bits 60-63
            asid: value >> 44 & ((1 << 16) - 1), // bits 44-59
            ppn: value & ((1 << 44) - 1),        // bits 0-43
        }
    }
}

impl From<SatpData> for u64 {
    fn from(value: SatpData) -> Self {
        u64::from(value.mode) << 60
            | (value.asid & ((1 << 16) - 1)) << 44
            | value.ppn & ((1 << 44) - 1)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum SatpMode {
    /// No translation or protection
    ///
    /// When MODE=Bare, supervisor virtual addresses are equal to supervisor physical addresses, and there is no additional memory protection beyond the physical memory protection scheme.
    /// **To select MODE=Bare, software must write zero to the remaining fields of satp.**
    /// Attempting to select MODE=Bare with a nonzero pattern in the remaining fields has an unspecified effect on the value that the remaining fields assume and an unspecified effect on address translation and protection behavior.
    Bare,
    /// Page-based 39-bit virtual addressing
    Sv39,
    /// Page-based 48-bit virtual addressing
    Sv48,
    /// Page-based 57-bit virtual addressing
    Sv57,
}

impl From<u64> for SatpMode {
    fn from(value: u64) -> Self {
        match value {
            0 => SatpMode::Bare,
            8 => SatpMode::Sv39,
            9 => SatpMode::Sv48,
            10 => SatpMode::Sv57,
            other => unimplemented!("unknown satp mode {}", other),
        }
    }
}

impl From<SatpMode> for u64 {
    fn from(value: SatpMode) -> Self {
        match value {
            SatpMode::Bare => 0,
            SatpMode::Sv39 => 8,
            SatpMode::Sv48 => 9,
            SatpMode::Sv57 => 10,
        }
    }
}

/// Supervisor Address Translation and Protection Register
///
/// The satp register is a  read/write register, which controls supervisor-mode address translation and protection.
#[allow(unused)]
pub struct Satp {}

impl Satp {
    pub fn read_raw() -> u64 {
        unsafe { read_reg!("satp") }
    }

    pub fn read() -> SatpData {
        SatpData::from(Self::read_raw())
    }

    pub unsafe fn write_raw(val: u64) {
        asm!("
            sfence.vma
            csrw satp, {}
        ", in(reg) val);
    }

    pub unsafe fn write(val: SatpData) {
        Self::write_raw(val.into())
    }
}
