use core::fmt::{self, Write};
use core::mem::MaybeUninit;

use core::sync::atomic::{AtomicBool, Ordering};
use spin::{MutexGuard, Mutex};

use crate::graphics::*;
use crate::font::*;

const ROW: usize = 45;
const COL: usize = 100;
const MARGIN: usize = 4;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}

pub fn _print(args: fmt::Arguments) {
    let c = Console::get().unwrap();
    c.lock().write_fmt(args).unwrap();
}

#[derive(Debug, Clone, Copy)]
pub struct Console {
    row: usize,
    column: usize,
    buf: [[char; COL]; ROW],
    cursor_row: usize,
    cursor_col: usize,
    color: PixelColor,
    bg: PixelColor,
}

static mut CONSOLE: MaybeUninit<Mutex<Console>> = MaybeUninit::<Mutex<Console>>::uninit();
static INITIALIZED: AtomicBool = AtomicBool::new(false);

impl Console {
    pub fn new(color: PixelColor, bg: PixelColor) -> Self {
        Console {
            row: ROW,
            column: COL,
            buf: [[0 as char; COL]; ROW],
            cursor_row: 0,
            cursor_col: 0,
            color,
            bg
        }
    }

    pub unsafe fn init(color: PixelColor, bg: PixelColor ) {
        CONSOLE.write(Mutex::new(Console::new(color, bg)));
        INITIALIZED.store(true, Ordering::Relaxed);
    }

    pub fn get() -> Result<&'static Mutex<Self>, &'static str> {
        if INITIALIZED.load(Ordering::Relaxed) {
            Ok(unsafe { CONSOLE.assume_init_mut() })
        } else {
            Err("this is not uninitialized")
        }
    }

    pub fn put_string(&mut self, s: &str) {
        let writer_ = PixelWriter::get().unwrap();
        let mut writer = writer_.lock();
        for b in s.bytes() {
            let c = b as char;
            if c == '\n' {
                self.new_line(&mut writer);
            } else if self.cursor_col < self.column {
                self.write_ascii_with_update(&mut writer, c);
            }
            else {
                self.new_line(&mut writer);
                self.write_ascii_with_update(&mut writer, c);
            }
        }
    }

    fn write_ascii_with_update(&mut self, writer: &mut MutexGuard<PixelWriter>, c: char) {
        write_ascii(writer, self.cursor_col * 8 + MARGIN, self.cursor_row * 16 + MARGIN, c, &self.color);
        self.buf[self.cursor_row][self.cursor_col] = c;
        self.cursor_col += 1;
    }

    fn new_line(&mut self, writer: &mut MutexGuard<PixelWriter>) {
        self.cursor_col = 0;
        if self.cursor_row < self.row - 1 {
            self.cursor_row += 1;
            return
        }
        for i in 0..self.row - 1 {
            for j in 0..self.column {
                for y in 0..16 {
                    for x in 0..8 {
                        writer.write(j * 8 + x + MARGIN, i * 16 + y + MARGIN, &self.bg)
                    }
                }
                // let c = self.buf[i + 1][j];
                let c = unsafe { *self.buf.get_unchecked(i + 1).get_unchecked(j) };
                write_ascii(writer, j * 8 + MARGIN, i * 16 + MARGIN, c, &self.color);
                // self.buf[i][j] = c;
                unsafe { *self.buf.get_unchecked_mut(i).get_unchecked_mut(j) = c; }
            }
        }
        for i in 0..self.column {
            for y in 0..16 {
                for x in 0..8 {
                    writer.write(i * 8 + x + MARGIN, 16 * (self.row - 1) + y + MARGIN, &self.bg)
                }
            }
            self.buf[self.row - 1][i] = 0 as char;
        }
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.put_string(s);
        Ok(())
    }
}
