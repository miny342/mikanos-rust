use core::ptr::{copy, copy_nonoverlapping, slice_from_raw_parts_mut, write_bytes};
use alloc::boxed::Box;
use alloc::vec as m_vec;

use common::writer_config::{
    PixelFormat,
    FrameBufferConfig,
};

use crate::{math::{Rectangle, Vector2D}, serial_println};

#[derive(Debug, Clone, Copy)]
pub struct PixelColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl PixelColor {
    pub const BLACK: PixelColor = PixelColor { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: PixelColor = PixelColor { r: 255, g: 255, b: 255, a: 255 };
    pub const RED: PixelColor = PixelColor { r: 255, g: 0, b: 0, a: 255 };
    pub const GREEN: PixelColor = PixelColor { r: 0, g: 255, b: 0, a: 255 };
    pub const BLUE: PixelColor = PixelColor { r: 0, g: 0, b: 255, a: 255 };
    pub const TRANSPARENT: PixelColor = PixelColor { r: 0, g: 0, b: 0, a: 0 };
    pub fn from_hex(hex: u32) -> PixelColor {
        PixelColor { r: ((hex >> 16) & 0xff) as u8, g: ((hex >> 8) & 0xff) as u8, b: (hex & 0xff) as u8, a: 255 }
    }
}

fn write_rgb_(buffer: &mut [u8], ppsl: usize, x: usize, y: usize, c: PixelColor) {
    let idx = 4 * (ppsl * y + x);
    buffer[idx] = c.r;
    buffer[idx + 1] = c.g;
    buffer[idx + 2] = c.b;
}

fn write_bgr_(buffer: &mut [u8], ppsl: usize, x: usize, y: usize, c: PixelColor) {
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
    write_: fn(buffer: &mut [u8], ppsl: usize, x: usize, y: usize, c: PixelColor),
}

impl FrameBuffer {
    pub fn new(config: FrameBufferConfig) -> Self {
        let per_pixel = bits_per_pixel(config.pixel_format).unwrap();
        let len = ((per_pixel + 7) / 8) * config.pixels_per_scan_line * config.vertical_resolution;
        let (from_vec, buffer) = (true, m_vec![0u8; len].leak());
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
    pub unsafe fn new_in(config: FrameBufferConfig) -> Self {
        let per_pixel = bits_per_pixel(config.pixel_format).unwrap();
        let len = ((per_pixel + 7) / 8) * config.pixels_per_scan_line * config.vertical_resolution;
        let (from_vec, buffer) = if config.frame_buffer.is_null() {
            (true, m_vec![0u8; len].leak())
        } else {
            (false, unsafe { &mut *slice_from_raw_parts_mut(config.frame_buffer, len) })
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
    pub fn horizontal_resolution(&self) -> usize {
        self.horizontal_resolution
    }
    pub fn vertical_resolution(&self) -> usize {
        self.vertical_resolution
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
        let src_width = src.horizontal_resolution as isize;
        let src_height = src.vertical_resolution as isize;

        let start_dst_x = isize::max(pos_x, 0) as usize;
        let start_dst_y = isize::max(pos_y, 0) as usize;
        let end_dst_x = isize::min(pos_x + src_width, dst_width) as usize;
        let end_dst_y = isize::min(pos_y + src_height, dst_height) as usize;

        let start_src_x = isize::max(-pos_x, 0) as usize;
        let start_src_y = isize::max(-pos_y, 0) as usize;

        let bytes_per_pixel = (per_pixel + 7) / 8;
        let per_copy = bytes_per_pixel * (end_dst_x - start_dst_x);

        let mut dst_addr = &mut self.frame_buffer[bytes_per_pixel * self.pixels_per_scan_line * start_dst_y + bytes_per_pixel * start_dst_x] as *mut u8;
        let mut src_addr = &src.frame_buffer[bytes_per_pixel * src.pixels_per_scan_line * start_src_y + bytes_per_pixel * start_src_x] as *const u8;

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
    pub fn copy_area(&mut self, src_area: &Rectangle, src: &FrameBuffer, r: &Rectangle) {
        if self.pixel_format != src.pixel_format {
            panic!("pixel format not equal")
        }
        let per_pixel = bits_per_pixel(self.pixel_format).unwrap();

        let dst_width = self.horizontal_resolution as isize;
        let dst_height = self.vertical_resolution as isize;

        let Some(dst_area) = Rectangle::new(Vector2D::new(0, 0), Vector2D::new(dst_width, dst_height)).intersect(r) else { return };
        let Some(dst_area) = dst_area.intersect(src_area) else { return };

        let bytes_per_pixel = (per_pixel + 7) / 8;
        let per_copy = bytes_per_pixel * (dst_area.size().x as usize);

        let mut dst_addr = &raw mut self.frame_buffer[bytes_per_pixel * self.pixels_per_scan_line * dst_area.pos.y as usize + bytes_per_pixel * dst_area.pos.x as usize];
        let mut src_addr = &raw const src.frame_buffer[bytes_per_pixel * src.pixels_per_scan_line * (dst_area.pos.y - src_area.pos.y) as usize + bytes_per_pixel * (dst_area.pos.x - src_area.pos.x) as usize];

        for _ in 0..dst_area.size().y {
            unsafe {
                copy_nonoverlapping(src_addr, dst_addr, per_copy);
                // serial_println!("dst: {:p}, src: {:p}, len: {}, {}, {}", dst_addr, src_addr, per_copy, self.pixels_per_scan_line, src.pixels_per_scan_line);
                dst_addr = dst_addr.add(bytes_per_pixel * self.pixels_per_scan_line);
                src_addr = src_addr.add(bytes_per_pixel * src.pixels_per_scan_line);
            }
        }
    }
    pub fn write(&mut self, x: usize, y: usize, c: PixelColor) {
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
    pub fn area(&self, pos: Vector2D<isize>) -> Rectangle {
        Rectangle::new(pos, Vector2D::new(self.horizontal_resolution as isize, self.vertical_resolution as isize))
    }
}

impl Drop for FrameBuffer {
    fn drop(&mut self) {
        if self.from_vec {
            core::mem::drop(unsafe { Box::from_raw(self.frame_buffer as *mut [u8]) })
        }
    }
}

pub fn bits_per_pixel(format: PixelFormat) -> Result<usize, ()> {
    match format {
        PixelFormat::Rgb => Ok(32),
        PixelFormat::Bgr => Ok(32),
        _ => Err(())
    }
}
