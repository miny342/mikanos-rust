use core::arch::asm;
use core::fmt::Display;
use core::fmt::Write;
use core::panic::PanicInfo;
use core::ptr::null;

use common::writer_config::FrameBufferConfig;

use crate::graphics::bits_per_pixel;
use crate::serial_println;
use crate::exit_qemu;
use crate::QemuExitCode;
use crate::interrupt::disable_interrupt;

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
}

// Panic発生時に画面出力もできない場合、デバッグが困難なため
// Panicの出力はほぼ初期化せずとも出力されるようにする
struct PanicWriter {
    ptr: *const FrameBufferConfig,
    x: usize,
    y: usize,
}

static mut PANIC_WRITER: PanicWriter = PanicWriter { ptr: null(), x: 0, y: 0 };

impl Write for PanicWriter {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        let Ok(_) = (unsafe { bits_per_pixel((*self.ptr).pixel_format) }) else {
            return Ok(())
        };
        // 現状bits_per_pixelは32しか返ってこないのでOK
        let buffer = unsafe { (*self.ptr).frame_buffer as *mut u32 };
        for c in s.bytes() {
            if c == b'\n' {
                self.x = 2;
                self.y += 16;
                continue;
            } else if unsafe { (*self.ptr).horizontal_resolution < self.x + 8 } {
                self.x = 2;
                self.y += 16;
            }
            if unsafe { (*self.ptr).vertical_resolution < self.y + 16 } {
                break;
            }
            let f = unsafe { crate::ascii::FONTS.get_unchecked(c as usize) };
            if (' ' as u8) <= c && c <= ('~' as u8) {
                for dy in 0..16 {
                    let val = unsafe { f.get_unchecked(dy) };
                    for dx in 0..8 {
                        if (val << dx) & 0x80 != 0 {
                            unsafe { *buffer.add((*self.ptr).pixels_per_scan_line * (self.y + dy) + self.x + dx) = 0xffffffff }
                        } else {
                            unsafe { *buffer.add((*self.ptr).pixels_per_scan_line * (self.y + dy) + self.x + dx) = 0 }
                        }
                    }
                }
            }
            self.x += 8;
        }
        Ok(())
    }
}

pub unsafe fn init_default_panic_print(config: *const FrameBufferConfig) {
    unsafe {
        PANIC_WRITER.ptr = config;
    }
}

pub unsafe fn default_panic_print<T: Display>(val: T) {
    unsafe {
        if !PANIC_WRITER.ptr.is_null() {
            write!(*&raw mut PANIC_WRITER, "{}", val).unwrap();
        }
    }
}

pub unsafe fn default_panic_handler(info: &PanicInfo) -> ! {
    unsafe {
        disable_interrupt();
        crate::backtrace::print_backtrace();
        serial_println!("{}", info);
        default_panic_print(info);
        loop {
            asm!("hlt");
        }
    }
}
