#[macro_export]
macro_rules! make_error {
    ($c:expr) => {
        $crate::error::Error {
            code: $c,
            line: line!(),
            file: file!(),
        }
    };
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum Code {
    Full,
    Empty,
    NoEnoughMemory,
    IndexOutOfRange,
    HostControllerNotHalted,
    InvalidSlotID,
    PortNotConnected,
    InvalidEndpointNumber,
    TransferRingNotSet,
    AlreadyAllocated,
    NotImplemented,
    InvalidDescriptor,
    BufferTooSmall,
    UnknownDevice,
    NoCorrespondingSetupStage,
    TransferFailed,
    InvalidPhase,
    UnknownXHCISpeedID,
    NoWaiter,
}

#[derive(Debug)]
pub struct Error {
    pub code: Code,
    pub line: u32,
    pub file: &'static str,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:?} at {} {}", self.code, self.file, self.line)
    }
}
