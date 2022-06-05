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
mod segment;
mod paging;

use core::arch::{asm, global_asm};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicPtr, Ordering};
use common::memory_map::MemoryMap;
use heapless::mpmc::Q32;

use common::writer_config::FrameBufferConfig;

use crate::graphics::*;
use crate::console::*;
use crate::logger::*;

global_asm!(include_str!("asm.s"));

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        asm!("cli");
        match PixelWriter::get() {
            Ok(s) => s.force_unlock(),
            Err(_) => {}
        };
        match Console::get() {
            Ok(s) => s.force_unlock(),
            Err(_) => {}
        };
    }
    println!("{}", info);
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

enum Message {
    InterruptXHCI,
}

static MAIN_Q: Q32<Message> = Q32::new();

fn keyboard_handler(modifire: u8, pressing: [u8; 6]) {

}

fn mouse_handler(modifire: u8, move_x: i8, move_y: i8) {
    mouse::CURSOR.lock().move_relative(move_x, move_y)
}

extern "x86-interrupt" fn int_handler_xhci(frame: interrupt::InterruptFrame) {
    // let xhc = unsafe { &mut *XHCI_PTR.load(Ordering::Relaxed) };
    // while xhc.process_event() {}
    MAIN_Q.enqueue(Message::InterruptXHCI).ok();
    interrupt::notify_end_of_interrupt();
}

#[no_mangle]
extern "efiapi" fn kernel_main_new_stack(config: *const FrameBufferConfig, memmap_ptr: *const MemoryMap) -> ! {
    segment::setup_segments();
    segment::set_ds_all(0);
    unsafe {
        segment::set_csss(1 << 3, 2 << 3);
    }
    unsafe{
        paging::setup_identity_page_table();
    }


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
    }

    let memmap = unsafe { &*core::ptr::slice_from_raw_parts((*memmap_ptr).ptr, (*memmap_ptr).size) };
    for i in memmap.iter() {
        println!("type = {}, phys = {:x} - {:x}, pages = {:x}, attr = {:x}",
            i.ty,
            i.phys_start,
            i.phys_start + i.page_count * 4096 - 1,
            i.page_count,
            i.attr,
        )
    }

    loop {
        unsafe {
            asm!("hlt")
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
    xhc.run();
    xhc.configure_port();

    loop {
        unsafe { asm!("cli") };
        if let Some(msg) = MAIN_Q.dequeue() {
            unsafe { asm!("sti") }
            match msg {
                Message::InterruptXHCI => {
                    while xhc.process_event() {}
                }
            }
        } else {
            unsafe {
                asm!(
                    "sti",
                    "hlt"
                )
            }
        }
    }


    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
