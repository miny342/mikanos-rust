#![no_std]
#![no_main]

#![feature(abi_efiapi)]
#![feature(once_cell)]

mod graphics;
mod font;
mod ascii;

use core::arch::asm;
use core::panic::PanicInfo;

use crate::graphics::*;
use crate::font::*;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!(0, 0, "{}", info);
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

#[no_mangle]
extern "efiapi" fn kernel_main(config: *const FrameBufferConfig) -> ! {
    unsafe { PixelWriter::init(*config) };
    let writer = PixelWriter::get().unwrap();

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

    println!(100, 100, "hello kernel!!!\n    by {}", "println");

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
