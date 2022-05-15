#![no_std]
#![no_main]

#![feature(abi_efiapi)]

mod graphics;
mod font;
mod ascii;

use core::arch::asm;
use core::panic::PanicInfo;

use crate::graphics::*;
use crate::font::*;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

#[no_mangle]
extern "efiapi" fn kernel_main(config: *const FrameBufferConfig) -> ! {
    let writer = PixelWriter::new(unsafe {*config});
    for x in 0..writer.config.horizontal_resolution {
        for y in 0..writer.config.vertical_resolution {
            writer.write(x, y, &PixelColor { r: 255, g: 255, b: 255 });
        }
    }
    for x in 0..200 {
        for y in 0..100 {
            writer.write(100 + x, 100 + y, &PixelColor { r: 255, g: 0, b: 255 });
        }
    }

    let color = PixelColor { r: 0, g: 0, b: 0};
    for i in ('!' as u8)..=('~' as u8) {
        write_ascii(&writer, 8 * (i - '!' as u8) as usize, 50, i as char, &color);
    }

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
