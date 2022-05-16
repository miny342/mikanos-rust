#![no_std]
#![no_main]

#![feature(abi_efiapi)]
#![feature(once_cell)]

mod graphics;
mod font;
mod ascii;
mod console;
mod pci;
mod error;

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

    let res = pci::scan_all_bus();
    let s = match res {
        Ok(_) => "Success",
        Err(e) => e.name(),
    };
    println!("scan all bus: {}", s);

    unsafe {
        for dev in pci::DEVICES.iter() {
            let vendor_id = pci::read_vendor_id(dev.bus, dev.device, dev.func);
            let class_code = pci::read_class_code(dev.bus, dev.device, dev.func);
            println!("{}.{}.{}: vend {:>4x}, class {:>8x}, head {:>2x}",
                dev.bus, dev.device, dev.func,
                vendor_id, class_code, dev.header_type
            );
        }
    }

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
