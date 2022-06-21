use core::intrinsics::{copy, write_bytes};
use core::{mem::MaybeUninit, ptr::slice_from_raw_parts_mut, intrinsics::copy_nonoverlapping};
use alloc::{vec::Vec, boxed::Box};
use alloc::vec as m_vec;
use spin::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};

use common::writer_config::{
    PixelFormat,
    FrameBufferConfig,
};

use crate::debug;

#[derive(Debug, Clone, Copy)]
pub struct PixelColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

fn write_rgb_(buffer: &mut [u8], ppsl: usize, x: usize, y: usize, c: &PixelColor) {
    let idx = 4 * (ppsl * y + x);
    buffer[idx] = c.r;
    buffer[idx + 1] = c.g;
    buffer[idx + 2] = c.b;
}

fn write_bgr_(buffer: &mut [u8], ppsl: usize, x: usize, y: usize, c: &PixelColor) {
    let idx = 4 * (ppsl * y + x);
    buffer[idx] = c.b;
    buffer[idx + 1] = c.g;
    buffer[idx + 2] = c.r;
}

pub struct FrameBuffer {
    frame_buffer: &'static mut [u8],
    from_vec: bool,
    pixels_per_scan_line: usize,
    horizontal_resolution: usize,
    vertical_resolution: usize,
    pixel_format: PixelFormat,
    write_: fn(buffer: &mut [u8], ppsl: usize, x: usize, y: usize, c: &PixelColor),
}

impl FrameBuffer {
    pub unsafe fn new(config: FrameBufferConfig) -> Self {
        let per_pixel = bits_per_pixel(config.pixel_format).unwrap();
        let len = ((per_pixel + 7) / 8) * config.horizontal_resolution * config.vertical_resolution;
        let (from_vec, buffer) = if config.frame_buffer.is_null() {
            (true, m_vec![0u8; len].leak())
        } else {
            (false, &mut *slice_from_raw_parts_mut(config.frame_buffer, len))
        };
        FrameBuffer {
            frame_buffer: buffer,
            pixels_per_scan_line: config.pixels_per_scan_line,
            horizontal_resolution: config.horizontal_resolution,
            vertical_resolution: config.vertical_resolution,
            pixel_format: config.pixel_format,
            from_vec,
            write_: match config.pixel_format {
                PixelFormat::Rgb => write_rgb_,
                PixelFormat::Bgr => write_bgr_,
                _ => panic!()
            }
        }
    }
    pub fn fmt(&self) -> PixelFormat {
        self.pixel_format
    }
    pub fn copy(&mut self, pos_x: isize, pos_y: isize, src: &FrameBuffer) {
        if self.pixel_format != src.pixel_format {
            panic!("pixel format not equal")
        }
        let per_pixel = bits_per_pixel(self.pixel_format).unwrap();

        let dst_width = self.horizontal_resolution as isize;
        let dst_height = self.vertical_resolution as isize;
        let src_width = self.horizontal_resolution as isize;
        let src_height = self.vertical_resolution as isize;

        let start_dst_x = isize::max(pos_x, 0) as usize;
        let start_dst_y = isize::max(pos_y, 0) as usize;
        let end_dst_x = isize::min(pos_x + src_width, dst_width) as usize;
        let end_dst_y = isize::min(pos_y + src_height, dst_height) as usize;

        let start_src_x = isize::max(-pos_x, 0) as usize;
        let start_src_y = isize::max(-pos_y, 0) as usize;

        let bytes_per_pixel = (per_pixel + 7) / 8;
        let per_copy = bytes_per_pixel * (end_dst_x - start_dst_x);

        let mut dst_addr = &mut self.frame_buffer[bytes_per_pixel * self.pixels_per_scan_line * start_dst_y + start_dst_x] as *mut u8;
        let mut src_addr = &src.frame_buffer[bytes_per_pixel * src.pixels_per_scan_line * start_src_y + start_src_x] as *const u8;

        // debug!("dst from: {:p}, to: {:p}", self.frame_buffer.as_ptr(), self.frame_buffer.last().unwrap() as *const u8);
        // debug!("src from: {:p}, to: {:p}", src.frame_buffer.as_ptr(), src.frame_buffer.last().unwrap() as *const u8);

        for _ in 0..(end_dst_y - start_dst_y) {
            unsafe {
                copy_nonoverlapping(src_addr, dst_addr, per_copy);
                // debug!("dst: {:p}, src: {:p}, len: {}", dst_addr, src_addr, per_copy);
                dst_addr = dst_addr.add(bytes_per_pixel * self.pixels_per_scan_line);
                src_addr = src_addr.add(bytes_per_pixel * src.pixels_per_scan_line);
            }
        }
    }
    pub fn write(&mut self, x: usize, y: usize, c: &PixelColor) {
        if x >= self.horizontal_resolution || y >= self.vertical_resolution {
            return;
        }
        (self.write_)(self.frame_buffer, self.pixels_per_scan_line, x, y, c);
    }
    pub fn move_up(&mut self, value: usize, fill: u8) {
        let per_pixel = bits_per_pixel(self.pixel_format).unwrap();
        let bytes_per_pixel = (per_pixel + 7) / 8;
        unsafe {
            copy(
                &self.frame_buffer[bytes_per_pixel * self.pixels_per_scan_line * value],
                &mut self.frame_buffer[0],
                bytes_per_pixel * self.pixels_per_scan_line * (self.vertical_resolution - value)
            );
            write_bytes(
                &mut self.frame_buffer[bytes_per_pixel * self.pixels_per_scan_line * (self.vertical_resolution - value)],
                fill,
                bytes_per_pixel * self.pixels_per_scan_line * value
            )
        }

    }
}

impl Drop for FrameBuffer {
    fn drop(&mut self) {
        if self.from_vec {
            core::mem::drop(unsafe { Box::from_raw(self.frame_buffer as *mut [u8]) })
        }
    }
}

fn bits_per_pixel(format: PixelFormat) -> Result<usize, ()> {
    match format {
        PixelFormat::Rgb => Ok(32),
        PixelFormat::Bgr => Ok(32),
        _ => Err(())
    }
}
