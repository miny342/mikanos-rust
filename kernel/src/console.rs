use core::fmt::{self, Write};
use core::intrinsics::copy;
use core::mem::MaybeUninit;

use core::sync::atomic::{AtomicBool, Ordering};
use alloc::sync::Arc;
use conquer_once::spin::OnceCell;
use spin::{MutexGuard, Mutex};

use crate::graphics::*;
use crate::window::{Window, WindowManager};

const ROW: usize = 45;
const COL: usize = 100;
const MARGIN: usize = 4;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}

pub fn _print(args: fmt::Arguments) {
    if let Ok(c) = Console::get() {
        c.lock().write_fmt(args).unwrap();
        WindowManager::draw();
    }
}

pub struct Console {
    row: usize,
    column: usize,
    cursor_row: usize,
    cursor_col: usize,
    color: PixelColor,
    bg: PixelColor,
    window: Arc<Mutex<Window>>,
    window_id: usize,
}

static CONSOLE: OnceCell<Mutex<Console>> = OnceCell::uninit();

impl Console {
    pub fn new(color: PixelColor, bg: PixelColor, width: usize, height: usize) -> usize {
        let (id, window) = WindowManager::new_window(width, height, false, 0, 0);
        CONSOLE.try_init_once(|| Mutex::new(Console {
            row: ROW,
            column: COL,
            cursor_row: 0,
            cursor_col: 0,
            color,
            bg,
            window,
            window_id: id,
        })).unwrap();
        id
    }

    pub fn get() -> Result<&'static Mutex<Self>, &'static str> {
        CONSOLE.get().ok_or_else(|| "console is not initialized")
    }

    pub fn put_string(&mut self, s: &str) {
        let c = Arc::clone(&self.window);
        let mut window = c.lock();
        for b in s.bytes() {
            let c = b as char;
            if c == '\n' {
                self.new_line(&mut window);
            } else if self.cursor_col < self.column {
                self.write_ascii_with_update(&mut window, c);
            }
            else {
                self.new_line(&mut window);
                self.write_ascii_with_update(&mut window, c);
            }
        }
    }

    fn write_ascii_with_update(&mut self, writer: &mut MutexGuard<Window>, c: char) {
        writer.write_ascii(self.cursor_col * 8 + MARGIN, self.cursor_row * 16 + MARGIN, c, self.color);
        self.cursor_col += 1;
    }

    fn new_line(&mut self, writer: &mut MutexGuard<Window>) {
        self.cursor_col = 0;
        if self.cursor_row < self.row - 1 {
            self.cursor_row += 1;
            return
        }
        writer.move_up_buffer(16, 0);
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.put_string(s);
        Ok(())
    }
}
