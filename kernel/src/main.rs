#![no_std]
#![no_main]

#![feature(abi_efiapi)]

use core::{arch::asm, slice};
use core::panic::PanicInfo;

use commons::writer::{FrameBufferConfig, PixelFormat};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

struct PixelColor {
    r: u8,
    g: u8,
    b: u8
}

fn write_pixel(config: &FrameBufferConfig, x: usize, y: usize, c: PixelColor) {
    let pixel_position = config.pixels_per_scan_line * y + x;
    let p = unsafe {
        if !(x <= config.horizontal_resolution && y <= config.vertical_resolution ) { panic!() };
        slice::from_raw_parts_mut(config.frame_buffer.offset(4 * pixel_position as isize), 3)
    };
    match config.pixel_format {
        PixelFormat::Rgb => {
            p[0] = c.r;
            p[1] = c.g;
            p[2] = c.b;
        },
        PixelFormat::Bgr => {
            p[0] = c.b;
            p[1] = c.g;
            p[2] = c.r;
        },
        _ => panic!()
    }
}

#[no_mangle]
extern "efiapi" fn kernel_main(config: *const FrameBufferConfig) -> ! {
    let conf = unsafe { *config };
    for x in 0..conf.horizontal_resolution {
        for y in 0..conf.vertical_resolution {
            write_pixel(&conf, x, y, PixelColor { r: 255, g: 255, b: 255 });
        }
    }
    for x in 0..200 {
        for y in 0..100 {
            write_pixel(&conf, 100 + x, 100 + y, PixelColor { r: 255, g: 255, b: 0 });
        }
    }

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
