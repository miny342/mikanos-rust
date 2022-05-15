use core::fmt::{self, Write};
use core::mem::MaybeUninit;

use crate::graphics::*;
use crate::font::*;

const ROW: usize = 25;
const COL: usize = 80;
const MARGIN: usize = 4;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

pub fn _print(args: fmt::Arguments) {
    let c = Console::get().unwrap();
    c.write_fmt(args).unwrap();
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

static mut CONSOLE: MaybeUninit<Console> = MaybeUninit::<Console>::uninit();
static mut INITIALIZED: bool = false;

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
        CONSOLE.write(Console::new(color, bg));
        INITIALIZED = true;
    }

    pub fn get() -> Result<&'static mut Self, &'static str> {
        unsafe {
            if INITIALIZED {
                Ok(&mut *CONSOLE.as_mut_ptr())
            } else {
                Err("this is not uninitialized")
            }
        }
    }

    pub fn put_string(&mut self, s: &str) {
        let writer = PixelWriter::get().unwrap();
        for b in s.bytes() {
            let c = b as char;
            if c == '\n' {
                self.new_line(&writer);
            } else if self.cursor_col < self.column {
                self.write_ascii_with_update(&writer, c);
            }
            else {
                self.new_line(&writer);
                self.write_ascii_with_update(&writer, c);
            }
        }
    }

    fn write_ascii_with_update(&mut self, writer: &PixelWriter, c: char) {
        write_ascii(&writer, self.cursor_col * 8 + MARGIN, self.cursor_row * 16 + MARGIN, c, &self.color);
        self.buf[self.cursor_row][self.cursor_col] = c;
        self.cursor_col += 1;
    }

    fn new_line(&mut self, writer: &PixelWriter) {
        self.cursor_col = 0;
        if self.cursor_row < self.row - 1 {
            self.cursor_row += 1;
            return
        }
        for y in 0..self.row * 16 {
            for x in 0..self.column * 8 {
                writer.write(x + MARGIN, y + MARGIN, &self.bg)
            }
        }
        for i in 0..self.row - 1 {
            for j in 0..self.column {
                let c = self.buf[i + 1][j];
                write_ascii(&writer, j * 8 + MARGIN, i * 16 + MARGIN, c, &self.color);
                self.buf[i][j] = c;
            }
        }
        for i in 0..self.column {
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