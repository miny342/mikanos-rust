#[derive(Debug, Clone, Copy)]
pub enum Error {
    Full,
    Empty,
    LastOfCode
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            // なぜかわからないが、元のコードでLastOfCodeはindex out of rangeなので一応
            &Error::LastOfCode => panic!("print Last of code!"),
            _ => write!(f, "{:?}", self)
        }
    }
}
