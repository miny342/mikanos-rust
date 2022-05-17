#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Error = 3,
    Warn = 4,
    Info = 6,
    Debug = 7
}

pub static mut LOG_LEVEL: LogLevel = LogLevel::Warn;

pub unsafe fn set_log_level(level: LogLevel) {
    LOG_LEVEL = level
}

#[macro_export]
macro_rules! log {
    ($c:expr, $($arg:tt)*) => {
        if ($c as usize) <= unsafe {crate::logger::LOG_LEVEL} as usize {
            println!($($arg)*)
        }
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => (log!(crate::logger::LogLevel::Debug, $($arg)*))
}
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => (log!(crate::logger::LogLevel::Info, $($arg)*))
}
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => (log!(crate::logger::LogLevel::Warn, $($arg)*))
}
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => (log!(crate::logger::LogLevel::Error, $($arg)*))
}
