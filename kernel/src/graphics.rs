use core::{mem::MaybeUninit, ptr::slice_from_raw_parts_mut};
use spin::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};

use common::writer_config::{
    PixelFormat,
    FrameBufferConfig,
};

// #[repr(C)]
// #[allow(dead_code)]
// #[derive(Debug, Clone, Copy)]
// pub enum PixelFormat {
//     Rgb,
//     Bgr,
//     Bitmask,
//     BltOnly,
// }

// #[repr(C)]
// #[derive(Debug, Clone, Copy)]
// pub struct FrameBufferConfig {
//     pub frame_buffer: *mut u8,
//     pub pixels_per_scan_line: usize,
//     pub horizontal_resolution: usize,
//     pub vertical_resolution: usize,
//     pub pixel_format: PixelFormat,
// }

#[derive(Debug, Clone, Copy)]
pub struct PixelColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

fn write_rgb(pos: *mut u8, c: &PixelColor) {
    // pos[0] = c.r;
    // pos[1] = c.g;
    // pos[2] = c.b;
    unsafe {
        *(pos.add(0)) = c.r;
        *(pos.add(1)) = c.g;
        *(pos.add(2)) = c.b;
    }
}

fn write_bgr(pos: *mut u8, c: &PixelColor) {
    // pos[0] = c.b;
    // pos[1] = c.g;
    // pos[2] = c.r;
    unsafe {
        *(pos.add(0)) = c.b;
        *(pos.add(1)) = c.g;
        *(pos.add(2)) = c.r;
    }
}

pub struct PixelWriter {
    // pub config: FrameBufferConfig,
    frame_buffer: *mut u8,
    pixels_per_scan_line: usize,
    horizontal_resolution: usize,
    vertical_resolution: usize,
    pixel_format: PixelFormat,
    write_: fn(pos: *mut u8, c: &PixelColor),
}

static mut WRITER: MaybeUninit<Mutex<PixelWriter>> = MaybeUninit::<Mutex<PixelWriter>>::uninit();
static INITIALIZED: AtomicBool = AtomicBool::new(false);

impl PixelWriter {
    unsafe fn new(config: FrameBufferConfig) -> Self {
        let f = match config.pixel_format {
            PixelFormat::Rgb => write_rgb,
            PixelFormat::Bgr => write_bgr,
            _ => panic!("can't use this writer")
        };
        PixelWriter {
            frame_buffer: config.frame_buffer, // &mut *slice_from_raw_parts_mut(config.frame_buffer, config.vertical_resolution * config.horizontal_resolution),
            pixels_per_scan_line: config.pixels_per_scan_line,
            horizontal_resolution: config.horizontal_resolution,
            vertical_resolution: config.vertical_resolution,
            pixel_format: config.pixel_format,
            write_: f
        }
    }

    pub fn get() -> Result<&'static Mutex<Self>, &'static str> {
        unsafe {
            if INITIALIZED.load(Ordering::Relaxed) {
                Ok(&WRITER.assume_init_ref())
            } else {
                Err("pixel writer is not initialized")
            }
        }
    }

    pub unsafe fn init(config: FrameBufferConfig) {
        WRITER.write(Mutex::new(PixelWriter::new(config)));
        INITIALIZED.store(true, Ordering::Relaxed);
    }

    pub fn horizontal_resolution(&self) -> usize {
        self.horizontal_resolution
    }

    pub fn vertical_resolution(&self) -> usize {
        self.vertical_resolution
    }

    pub fn write(&mut self, x: usize, y: usize, c: &PixelColor) {
        if x >= self.horizontal_resolution || y >= self.vertical_resolution {
            return;
        }
        unsafe {
            (self.write_)(self.frame_buffer.add(4* (self.pixels_per_scan_line * y + x)), c);
        }

    }
}
