#![no_std]
#![no_main]

#![feature(abi_efiapi)]

use core::arch::asm;
use core::panic::PanicInfo;

pub mod writer;
use writer::*;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

const kFontA: [u8; 16] = [
    0b00000000, //
    0b00011000, //    **
    0b00011000, //    **
    0b00011000, //    **
    0b00011000, //    **
    0b00100100, //   *  *
    0b00100100, //   *  *
    0b00100100, //   *  *
    0b00100100, //   *  *
    0b01111110, //  ******
    0b01000010, //  *    *
    0b01000010, //  *    *
    0b01000010, //  *    *
    0b11100111, // ***  ***
    0b00000000, //
    0b00000000, //
];

fn write_ascii(writer: &PixelWriter, x: usize, y: usize, c: char, color: &PixelColor) {
    if c != 'A' {
        return
    }
    for dy in 0..16 {
        for dx in 0..8 {
            if (kFontA[dy] << dx) & 0x80 != 0 {
                writer.write(x + dx, y + dy, color);
            }
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
    write_ascii(&writer, 50, 50, 'A', &color);
    write_ascii(&writer, 58, 50, 'A', &color);

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
