#[repr(C)]
pub enum PixelFormat {
    Rgb,
    Bgr,
    Bitmask,
    BltOnly,
}

#[repr(C)]
pub struct FrameBufferConfig {
    pub frame_buffer: *mut u8,
    pub pixels_per_scan_line: usize,
    pub horizontal_resolution: usize,
    pub vertical_resolution: usize,
    pub pixel_format: PixelFormat,
}
