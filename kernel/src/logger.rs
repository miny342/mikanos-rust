use log::{Record, Metadata};

macro_rules! __log {
    ($($arg:tt)*) => {
        $crate::serial_println!($($arg)*);
        // 常に画面に出したほうが楽しいし、バグも見つかる(画面に表示する際に無限再帰するとか)
        $crate::println!($($arg)*)
    }
}

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

static LOGGER: Logger = Logger;

pub fn init_serial_and_logger() {
    crate::serial::init_serial();
    log::set_logger(&LOGGER).unwrap_or_else(|_| {
        crate::serial_println!("Failed to set logger");
        panic!();
    });
}

