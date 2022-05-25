use core::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Error = 3,
    Warn = 4,
    Info = 6,
    Debug = 7
}

impl LogLevel {
    pub const fn to_num(&self) -> usize {
        *self as usize
    }
}

pub static LOG_LEVEL: AtomicUsize = AtomicUsize::new(LogLevel::Warn.to_num());

pub fn set_log_level(level: LogLevel) {
    LOG_LEVEL.store(level.to_num(), Ordering::Relaxed)
}

#[macro_export]
macro_rules! log {
    ($c:expr, $($arg:tt)*) => {
        if $c.to_num() <= $crate::logger::LOG_LEVEL.load(core::sync::atomic::Ordering::Relaxed) {
            println!($($arg)*)
        }
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::log!($crate::logger::LogLevel::Debug, $($arg)*))
}
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::log!($crate::logger::LogLevel::Info, $($arg)*))
}
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::log!($crate::logger::LogLevel::Warn, $($arg)*))
}
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::log!($crate::logger::LogLevel::Error, $($arg)*))
}
