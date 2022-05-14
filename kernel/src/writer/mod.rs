#[derive(Clone, Copy)]
pub enum PixelFormat {
    Rgb,
    Bgr,
    Bitmask,
    BltOnly,
}

#[derive(Clone, Copy)]
pub struct FrameBufferConfig {
    pub frame_buffer: *mut u8,
    pub pixels_per_scan_line: usize,
    pub horizontal_resolution: usize,
    pub vertical_resolution: usize,
    pub pixel_format: PixelFormat,
}

pub struct PixelColor {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

#[inline(always)]
unsafe fn pixel_at(config: &FrameBufferConfig, x: usize, y: usize) -> *mut u8 {
    config.frame_buffer.add(4 * (config.pixels_per_scan_line * y + x))
}

unsafe fn write_rgb(pos: *mut u8, c: &PixelColor) {
    *pos = c.r;
    *(pos.add(1)) = c.g;
    *(pos.add(2)) = c.b;
}

unsafe fn write_bgr(pos: *mut u8, c: &PixelColor) {
    *pos = c.b;
    *(pos.add(1)) = c.g;
    *(pos.add(2)) = c.r;
}

pub struct PixelWriter {
    pub config: FrameBufferConfig,
    write_: unsafe fn(pos: *mut u8, c: &PixelColor),
}

impl PixelWriter {
    pub fn new(config: FrameBufferConfig) -> Self {
        let f = match config.pixel_format {
            PixelFormat::Rgb => write_rgb,
            PixelFormat::Bgr => write_bgr,
            _ => panic!("can't use this writer")
        };
        PixelWriter {
            config,
            write_: f
        }
    }

    pub fn write(&self, x: usize, y: usize, c: &PixelColor) {
        if x >= self.config.horizontal_resolution || y >= self.config.vertical_resolution {
            return;
        }
        unsafe {
            (self.write_)(pixel_at(&self.config, x, y), c);
        }
    }
}
