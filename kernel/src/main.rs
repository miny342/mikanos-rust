#![no_std]
#![no_main]

#![feature(abi_efiapi)]
#![feature(once_cell)]

mod graphics;
mod font;
mod ascii;
mod console;

use core::arch::asm;
use core::panic::PanicInfo;

use crate::graphics::*;
use crate::console::*;


#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

#[no_mangle]
extern "efiapi" fn kernel_main(config: *const FrameBufferConfig) -> ! {
    unsafe {
        PixelWriter::init(*config);
        Console::init(PixelColor { r: 0, g: 0, b: 0}, PixelColor { r: 255, g: 255, b: 255 })
    };
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

    for i in 0..30 {
        println!("println! {}", i);
    }
    for i in 0..30 {
        print!("print! {}", i);
    }
    println!("");
    panic!("test panic");

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
