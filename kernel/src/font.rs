use core::fmt::{self, Write};
use crate::graphics::*;
use crate::ascii::FONTS;

#[macro_export]
macro_rules! print {
    ($x:expr, $y:expr, $($arg:tt)*) => ($crate::_print($x, $y, format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    ($x:expr, $y:expr, $fmt:expr) => (print!($x, $y, concat!($fmt, "\n")));
    ($x:expr, $y:expr, $fmt:expr, $($arg:tt)*) => (print!($x, $y, concat!($fmt, "\n"), $($arg)*));
}

pub fn _print(x: usize, y: usize, args: fmt::Arguments) {
    let p = PixelWriter::get().unwrap();
    let mut writer = TextWriter::new(&p, x, y, &PixelColor {r: 0, g: 0, b: 0}, &Some(PixelColor {r: 255, g: 255, b: 255}));
    writer.write_fmt(args).unwrap();
}

#[allow(dead_code)]
struct TextWriter<'a> {
    writer: &'a PixelWriter,
    initial_x: usize,
    initial_y: usize,
    current_x: usize,
    current_y: usize,
    color: &'a PixelColor,
    bg: &'a Option<PixelColor>,
}

impl<'a> TextWriter<'a> {
    pub fn new(writer: &'a PixelWriter, x: usize, y: usize, color: &'a PixelColor, bg: &'a Option<PixelColor>) -> Self {
        TextWriter {
            writer: &writer,
            initial_x: x,
            initial_y: y,
            current_x: x,
            current_y: y,
            color: &color,
            bg: &bg,
        }
    }

    pub fn write_(&mut self, s: &str) {
        for c in s.bytes() {
            if self.write_ascii(self.current_x, self.current_y, c as char, &self.color) {
                self.current_x = self.initial_x;
                self.current_y += 16;
            } else {
                self.current_x += 8;
            }
        }
    }

    pub fn write_ascii(&self, x: usize, y: usize, c: char, color: &PixelColor) -> bool {
        let i = c as u8;
        if c == '\n' {
            return true;
        }
        if (' ' as u8) <= i && i <= ('~' as u8) {
            for dy in 0..16 {
                for dx in 0..8 {
                    if (FONTS[c as usize][dy] << dx) & 0x80 != 0 {
                        self.writer.write(x + dx, y + dy, color);
                    } else {
                        match self.bg {
                            Some(cc) => {
                                self.writer.write(x + dx, y + dy, cc);
                            },
                            None => {}
                        }
                    }
                }
            }
        }
        false
    }
}

impl<'a> fmt::Write for TextWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_(s);
        Ok(())
    }
}

