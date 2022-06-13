use core::{arch::asm, fmt::Write};

use spin::Mutex;


const PORT: u16 = 0x3f8;

unsafe fn outb(port: u16, value: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
    )
}

unsafe fn inb(port: u16) -> u8 {
    let res: u8;
    asm!(
        "in al, dx",
        in("dx") port,
        out("al") res,
    );
    res
}

pub unsafe fn init_serial() {
    outb(PORT + 1, 0x00);
    outb(PORT + 3, 0x80);
    outb(PORT + 0, 0x03);
    outb(PORT + 1, 0x00);
    outb(PORT + 3, 0x03);
    outb(PORT + 2, 0xC7);
    outb(PORT + 4, 0x0B);
    outb(PORT + 4, 0x1E);
    outb(PORT + 0, 0xAE);

    if inb(PORT + 0) != 0xAE {
        loop {}
    }

    outb(PORT + 4, 0x0f);
}

fn is_transmit_empty() -> u8 {
    unsafe { inb(PORT + 5) & 0x20 }
}

pub fn write_serial(value: u8) {
    while is_transmit_empty() == 0 {}

    unsafe { outb(PORT, value) }
}

struct Serial;

impl core::fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.as_bytes() {
            write_serial(*c)
        }
        Ok(())
    }
}

static SERIAL: Mutex<Serial> = Mutex::new(Serial);

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ($crate::serial::_serial_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! serial_println {
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}

pub fn _serial_print(args: core::fmt::Arguments) {
    SERIAL.lock().write_fmt(args).unwrap();
}


