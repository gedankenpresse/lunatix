use crate::back_to_enum;

back_to_enum! {
    /// The error type that a syscall can return
    #[derive(Debug, Eq, PartialEq)]
    #[repr(usize)]
    pub enum SyscallError {
        InvalidCAddr = 1,
        NoMem = 2,
        OccupiedSlot = 3,
        InvalidCap = 4,
        InvalidArg = 6,
        AliasingCSlot = 7,
        InvalidReturn = 8,
        Unsupported = 9,
        AlreadyMapped = 10,
        NoAsid = 11,
        NotFound = 12,
        ValueInvalid = usize::MAX - 2,
        UnknownError = usize::MAX - 1,
        UnknownSyscall = usize::MAX,
    }
}
