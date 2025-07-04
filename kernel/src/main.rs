#![no_std]
#![no_main]

#![feature(sync_unsafe_cell)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

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
mod memory_manager;
mod allocator;
mod task;
mod window;
mod timer;
mod serial;
mod entry;

use core::alloc::Layout;
use core::arch::{asm, global_asm};
use core::panic::PanicInfo;
use alloc::boxed::Box;
use heapless::mpmc::Q32;
use log::{debug, info, warn, error};

use common::writer_config::FrameBufferConfig;
use uefi::boot::{MemoryDescriptor, PAGE_SIZE};
use uefi::mem::memory_map::MemoryMap;

use crate::graphics::*;
use crate::console::*;
use crate::logger::*;
use crate::memory_manager::{
    FrameID,
    BYTES_PER_FRAME, MANAGER
};
use crate::serial::init_serial;
use crate::timer::initialize_apic_timer;
use crate::window::WindowManager;

extern crate alloc;

// #[global_allocator]
// static ALLOCATOR: allocator::LinkedListAllocator = allocator::LinkedListAllocator::empty();

#[global_allocator]
static ALLOCATOR: allocator::SimplestAllocator = allocator::SimplestAllocator::empty();

#[alloc_error_handler]
fn on_oom(_layout: Layout) -> ! {
    panic!("oom");
}


#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        asm!("cli");
    }
    error!("{}", info);
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

static LOGGER: logger::Logger = Logger;

extern "sysv64" fn kernel_main_new_stack(config: *const FrameBufferConfig, memmap_ptr: *const uefi::mem::memory_map::MemoryMapOwned) -> ! {
    let framebufferconfig = unsafe { *config };

    SERIAL_USABLE.store(unsafe { init_serial() }, core::sync::atomic::Ordering::Relaxed);
    log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Debug)).unwrap();
    unsafe {
        segment::setup_segments();
        segment::set_ds_all(0);
        segment::set_csss(1 << 3, 2 << 3);
        paging::setup_identity_page_table();
    }

    let memmap = unsafe { &*memmap_ptr };
    let mut memory_manager = MANAGER.lock();
    let mut available_end: usize = 0;
    for desc in memmap.entries() {
        let phys_start = desc.phys_start as usize;
        let page_count = desc.page_count as usize;
        if available_end < phys_start {
            memory_manager.mark_allocated(
                &FrameID(available_end / BYTES_PER_FRAME),
                (phys_start - available_end) / BYTES_PER_FRAME,
            )
        }
        let physical_end = phys_start + page_count * PAGE_SIZE;
        if desc.ty == uefi::mem::memory_map::MemoryType::CONVENTIONAL ||
           desc.ty == uefi::mem::memory_map::MemoryType::BOOT_SERVICES_CODE ||
           desc.ty == uefi::mem::memory_map::MemoryType::BOOT_SERVICES_DATA {
            available_end = physical_end;
        } else {
            memory_manager.mark_allocated(
                &FrameID(phys_start / BYTES_PER_FRAME),
                page_count * PAGE_SIZE / BYTES_PER_FRAME
            )
        }
    }
    memory_manager.set_memory_range(&FrameID(1), &FrameID(available_end / BYTES_PER_FRAME));

    let heap_frame = 64 * 512;
    let heap_start = memory_manager.allocate(heap_frame).expect("cannot initialize heap allocate");
    let start = heap_start.0 * BYTES_PER_FRAME;
    let end = start + heap_frame * BYTES_PER_FRAME;
    unsafe {
        ALLOCATOR.init(start as *mut u8, end as *mut u8);
    }
    // initialized memory allocator

    // unsafe {
    //     PixelWriter::init(framebufferconfig);
    //     // Console::init(PixelColor { r: 255, g: 255, b: 255, a: 255}, PixelColor { r: 0, g: 0, b: 0, a: 255 })
    // };
    // let writer = PixelWriter::get().unwrap();

    // {
    //     let mut writer = writer.lock();
    //     for x in 0..writer.horizontal_resolution() {
    //         for y in 0..writer.vertical_resolution() {
    //             writer.write(x, y, &PixelColor { r: 0, g: 0, b: 0, a: 255 });
    //         }
    //     }
    // }


    let (width, height) = unsafe {
        ((framebufferconfig).horizontal_resolution, (framebufferconfig).vertical_resolution)
    };
    let screen = unsafe { FrameBuffer::new(framebufferconfig) };
    WindowManager::new(screen);

    let mouse_id = mouse::MouseCursor::new(width, height);
    let console_id = Console::new(PixelColor { r: 255, g: 255, b: 255, a: 255}, PixelColor { r: 0, g: 0, b: 0, a: 255 }, width, height);

    WindowManager::up_down(console_id, 0);
    WindowManager::up_down(mouse_id, 1);
    WindowManager::draw();

    initialize_apic_timer();

    println!("hello");

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

            let mut ports_available = pci::read_config_reg(xhc_dev, 0xdc);
            debug!("configurable ports to enable superspeed: {:x}", ports_available);
            pci::write_config_reg(xhc_dev, 0xd8, ports_available);
            ports_available = pci::read_config_reg(xhc_dev, 0xdc);
            debug!("usb 3.0 ports that are now enabled under xhci: {:x}", ports_available);
            ports_available = pci::read_config_reg(xhc_dev, 0xd4);
            debug!("configurable usb 2.0 ports to hand over to xhci: {:x}", ports_available);
            pci::write_config_reg(xhc_dev, 0xd0, ports_available);
            ports_available = pci::read_config_reg(xhc_dev, 0xd0);
            debug!("usb 2.0 ports that are now switched over to xhci: {:x}", ports_available);
        }
    }

    info!("xHC has been found: {}.{}.{}", xhc_dev.bus, xhc_dev.device, xhc_dev.func);

    let cs = interrupt::get_cs();
    interrupt::set_idt_entry(interrupt::InterruptVector::XHCI as usize, interrupt::InterruptDescriptorAttr::new(interrupt::DescriptorType::InterruptGate, 0, true, 0), usb::int_handler_xhci as *const fn() as u64, cs);
    interrupt::load_idt();

    unsafe {
        let bsp_local_apic_id = (*(0xfee00020 as *const u32) >> 24) as u8;
        pci::configure_msi_fixed_destination(xhc_dev, bsp_local_apic_id, pci::MSITriggerMode::Level, pci::MSIDeliveryMode::Fixed, interrupt::InterruptVector::XHCI as u8, 0);
    }

    let xhc_bar = unsafe {pci::read_bar(xhc_dev, 0)}.unwrap();
    debug!("read bar: Success");
    let xhc_mmio_base = xhc_bar & !0xf;
    debug!("xHC mmio_base = {:0>8x}", xhc_mmio_base);

    for _ in 0..1 {
        let mut xhc = unsafe {
            Box::new(usb::XhcController::initialize(xhc_mmio_base, keyboard_handler, mouse::mouse_handler))
        };
        xhc.run();
        xhc.configure_port();
        // unsafe { asm!("sti") };
        while xhc.process_event() {}
        // unsafe { asm!("cli") };
        debug!("xhc error! restarting...");
    }



    // unsafe { asm!("sti") }

    // let mut executor = task::executor::Executor::new();
    // executor.spawn(task::Task::new(xhc.process_event()));
    // executor.run();

    // loop {
    //     unsafe { asm!("cli") };
    //     if let Some(msg) = MAIN_Q.dequeue() {
    //         unsafe { asm!("sti") }
    //         match msg {
    //             Message::InterruptXHCI => {
    //                 while xhc.process_event_() {}
    //             }
    //         }
    //     } else {
    //         unsafe {
    //             asm!(
    //                 "sti",
    //                 "hlt"
    //             )
    //         }
    //     }
    // }


    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
