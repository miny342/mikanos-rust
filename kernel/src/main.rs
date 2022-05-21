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
mod logger;
mod usb;

use core::arch::asm;
use core::panic::PanicInfo;

use crate::graphics::*;
use crate::console::*;
use crate::logger::*;
use crate::usb::driver_handle_test;


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
    println!("hello");
    unsafe {set_log_level(LogLevel::Debug)}

    let res = pci::scan_all_bus();
    match res {
        Ok(_) => debug!("scan all bus: Success"),
        Err(e) => debug!("scan all bus: {}", e),
    };


    for dev in unsafe { pci::DEVICES.iter() } {
        let vendor_id = unsafe { pci::read_vendor_id(dev.bus, dev.device, dev.func) };
        let class_code = unsafe { pci::read_class_code(dev.bus, dev.device, dev.func) };
        debug!("{}.{}.{}: vend {:>4x}, class {:?}, head {:>2x}",
            dev.bus, dev.device, dev.func,
            vendor_id, class_code, dev.header_type
        );
    }

    let mut dev: Option<&pci::Device> = None;
    unsafe {
        for d in pci::DEVICES.iter().filter(|d| d.class_code.match3(0x0c, 0x03, 0x30)) {
            dev = Some(d);
            if (pci::read_vendor_id(d.bus, d.device, d.func)) == 0x8086 {
                break;
            }
        }
    };

    let xhc_dev = dev.expect("not found: xHC");

    info!("xHC has been found: {}.{}.{}", xhc_dev.bus, xhc_dev.device, xhc_dev.func);

    let xhc_bar = unsafe {pci::read_bar(xhc_dev, 0)}.unwrap();
    debug!("read bar: Success");
    let xhc_mmio_base = xhc_bar & !0xf;
    debug!("xHC mmio_base = {:0>8x}", xhc_mmio_base);

    unsafe { driver_handle_test(xhc_mmio_base, xhc_dev); }

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
