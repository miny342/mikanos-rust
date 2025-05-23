use core::sync::atomic::AtomicBool;


// #[derive(Debug, Clone, Copy)]
// pub enum LogLevel {
//     Error = 3,
//     Warn = 4,
//     Info = 6,
//     Debug = 7
// }

// impl LogLevel {
//     pub const fn to_num(&self) -> usize {
//         *self as usize
//     }
// }

// pub static LOG_LEVEL: AtomicUsize = AtomicUsize::new(LogLevel::Warn.to_num());
pub static SERIAL_USABLE: AtomicBool = AtomicBool::new(false);

// pub fn set_log_level(level: LogLevel) {
//     LOG_LEVEL.store(level.to_num(), Ordering::Relaxed)
// }

// #[macro_export]
macro_rules! __log {
    ($($arg:tt)*) => {
        if $crate::logger::SERIAL_USABLE.load(core::sync::atomic::Ordering::Relaxed) {
                $crate::serial_println!($($arg)*)
        } else {
                $crate::println!($($arg)*)
        }
    }
}

// #[macro_export]
// macro_rules! debug {
//     ($fmt:expr) => ($crate::log!($crate::logger::LogLevel::Debug, concat!("Debug: ", $fmt)));
//     ($fmt:expr, $($arg:tt)*) => ($crate::log!($crate::logger::LogLevel::Debug, concat!("Debug: ", $fmt), $($arg)*));
// }
// #[macro_export]
// macro_rules! info {
//     ($fmt:expr) => ($crate::log!($crate::logger::LogLevel::Info, concat!("Info: ", $fmt)));
//     ($fmt:expr, $($arg:tt)*) => ($crate::log!($crate::logger::LogLevel::Info, concat!("Info: ", $fmt), $($arg)*));
// }
// #[macro_export]
// macro_rules! warn {
//     ($fmt:expr) => ($crate::log!($crate::logger::LogLevel::Warn, concat!("Warn: ", $fmt)));
//     ($fmt:expr, $($arg:tt)*) => ($crate::log!($crate::logger::LogLevel::Warn, concat!("Warn: ", $fmt), $($arg)*));
// }
// #[macro_export]
// macro_rules! error {
//     ($fmt:expr) => ($crate::log!($crate::logger::LogLevel::Error, concat!("Error: ", $fmt)));
//     ($fmt:expr, $($arg:tt)*) => ($crate::log!($crate::logger::LogLevel::Error, concat!("Error: ", $fmt), $($arg)*));
// }

use log::{Record, Metadata};

pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {
        if let Some(filename) = record.file() {
            if let Some(linenum) = record.line() {
                __log!("[{}]: {}@{}: {}", record.level(), filename, linenum, record.args());
            } else {
                __log!("[{}]: {}: {}", record.level(), filename, record.args());
            }
        } else {
            __log!("[{}]: {}", record.level(), record.args());
        }

    }
    fn flush(&self) {}
}
