#![no_std]
#![no_main]

#![feature(abi_efiapi)]
#![feature(abi_x86_interrupt)]

mod graphics;
mod font;
mod ascii;
mod console;
mod pci;
mod error;
mod logger;
mod usb;
mod mouse;
mod interrupt;

use core::arch::asm;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicPtr, Ordering};
use crate::graphics::*;
use crate::console::*;
use crate::logger::*;


#[panic_handler]
extern "C" fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

static XHCI_PTR: AtomicPtr<usb::XhcController> = AtomicPtr::new(0 as *mut usb::XhcController);

fn keyboard_handler(modifire: u8, pressing: [u8; 6]) {

}

fn mouse_handler(modifire: u8, move_x: i8, move_y: i8) {
    mouse::CURSOR.lock().move_relative(move_x, move_y)
}

extern "x86-interrupt" fn int_handler_xhci(frame: interrupt::InterruptFrame) {
    let xhc = unsafe { &mut *XHCI_PTR.load(Ordering::Relaxed) };
    while xhc.process_event() {}
    interrupt::notify_end_of_interrupt();
}

#[no_mangle]
extern "efiapi" fn kernel_main(config: *const FrameBufferConfig) -> ! {
    unsafe {
        PixelWriter::init(*config);
        Console::init(PixelColor { r: 255, g: 255, b: 255}, PixelColor { r: 0, g: 0, b: 0 })
    };
    let writer = PixelWriter::get().unwrap();

    {
        let mut writer = writer.lock();
        for x in 0..writer.horizontal_resolution() {
            for y in 0..writer.vertical_resolution() {
                writer.write(x, y, &PixelColor { r: 0, g: 0, b: 0 });
            }
        }
        for x in 0..200 {
            for y in 0..100 {
                writer.write(100 + x, 100 + y, &PixelColor { r: 255, g: 0, b: 255 });
            }
        }
    }


    println!("hello");
    set_log_level(LogLevel::Debug);

    let res = pci::scan_all_bus();
    match res {
        Ok(_) => debug!("scan all bus: Success"),
        Err(e) => debug!("scan all bus: {}", e),
    };

    let devices = pci::DEVICES.lock();
    for dev in devices.iter() {
        let vendor_id = unsafe { pci::read_vendor_id(dev.bus, dev.device, dev.func) };
        let class_code = unsafe { pci::read_class_code(dev.bus, dev.device, dev.func) };
        debug!("{}.{}.{}: vend {:>4x}, class {:?}, head {:>2x}",
            dev.bus, dev.device, dev.func,
            vendor_id, class_code, dev.header_type
        );
    }

    let mut dev: Option<&pci::Device> = None;
    for d in devices.iter().filter(|d| d.class_code.match3(0x0c, 0x03, 0x30)) {
        dev = Some(d);
        if unsafe { pci::read_vendor_id(d.bus, d.device, d.func) } == 0x8086 {
            break;
        }
    }

    let xhc_dev = dev.expect("not found: xHC");

    unsafe {
        let intel_ehc = devices.iter().filter(|d| d.class_code.match3(0x0c, 0x03, 0x20)).any(|d| pci::read_vendor_id(d.bus, d.device, d.func) == 0x8086);
        if intel_ehc {
            let superspeed_ports = pci::read_config_reg(xhc_dev, 0xdc);
            pci::write_config_reg(xhc_dev, 0xd8, superspeed_ports);
            let ehci2xhci_ports = pci::read_config_reg(xhc_dev, 0xd4);
            pci::write_config_reg(xhc_dev, 0xd0, ehci2xhci_ports);
            debug!("switch ehci2xhci: ss = {}, xhci = {}", superspeed_ports, ehci2xhci_ports);
        }
    }

    info!("xHC has been found: {}.{}.{}", xhc_dev.bus, xhc_dev.device, xhc_dev.func);

    let cs = interrupt::get_cs();
    interrupt::set_idt_entry(interrupt::InterruptVector::XHCI as usize, interrupt::InterruptDescriptorAttr::new(interrupt::DescriptorType::InterruptGate, 0, true, 0), int_handler_xhci as *const fn() as u64, cs);
    interrupt::load_idt();

    unsafe {
        let bsp_local_apic_id = (*(0xfee00020 as *const u32) >> 24) as u8;
        pci::configure_msi_fixed_destination(xhc_dev, bsp_local_apic_id, pci::MSITriggerMode::Level, pci::MSIDeliveryMode::Fixed, interrupt::InterruptVector::XHCI as u8, 0);
    }

    let xhc_bar = unsafe {pci::read_bar(xhc_dev, 0)}.unwrap();
    debug!("read bar: Success");
    let xhc_mmio_base = xhc_bar & !0xf;
    debug!("xHC mmio_base = {:0>8x}", xhc_mmio_base);

    let mut xhc = unsafe {
        usb::XhcController::initialize(xhc_mmio_base, keyboard_handler, mouse_handler)
    };
    XHCI_PTR.store(&mut xhc, Ordering::Relaxed);
    xhc.run();
    xhc.configure_port();

    unsafe {
        asm!("sti")
    }


    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
