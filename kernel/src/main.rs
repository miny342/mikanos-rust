#![no_std]
#![no_main]

#![feature(abi_efiapi)]
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

use core::alloc::Layout;
use core::arch::{asm, global_asm};
use core::panic::PanicInfo;
use alloc::boxed::Box;
use common::memory_map::{MemoryMap, UEFI_PAGE_SIZE};
use heapless::mpmc::Q32;

use common::writer_config::FrameBufferConfig;

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

global_asm!(include_str!("asm.s"));

#[global_allocator]
static ALLOCATOR: allocator::LinkedListAllocator = allocator::LinkedListAllocator::empty();

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

#[no_mangle]
extern "efiapi" fn kernel_main_new_stack(config: *const FrameBufferConfig, memmap_ptr: *const MemoryMap) -> ! {
    SERIAL_USABLE.store(unsafe { init_serial() }, core::sync::atomic::Ordering::Relaxed);
    set_log_level(LogLevel::Error);
    segment::setup_segments();
    segment::set_ds_all(0);
    unsafe {
        segment::set_csss(1 << 3, 2 << 3);
    }
    unsafe{
        paging::setup_identity_page_table();
    }

    let memmap = unsafe { &*core::ptr::slice_from_raw_parts((*memmap_ptr).ptr, (*memmap_ptr).size) };
    let memory_manager = unsafe { &mut MANAGER };
    let mut available_end: usize = 0;
    for desc in memmap.iter() {
        if available_end < desc.phys_start {
            memory_manager.mark_allocated(
                &FrameID(available_end / BYTES_PER_FRAME),
                (desc.phys_start - available_end) / BYTES_PER_FRAME,
            )
        }
        let physical_end = desc.phys_start + desc.page_count * UEFI_PAGE_SIZE;
        if desc.is_available() {
            available_end = physical_end;
        } else {
            memory_manager.mark_allocated(
                &FrameID(desc.phys_start / BYTES_PER_FRAME),
                desc.page_count * UEFI_PAGE_SIZE / BYTES_PER_FRAME
            )
        }
    }
    memory_manager.set_memory_range(&FrameID(1), &FrameID(available_end / BYTES_PER_FRAME));

    let heap_frame = 64 * 512;
    let heap_start = memory_manager.allocate(heap_frame).expect("cannot initialize heap allocate");
    let start = heap_start.0 * BYTES_PER_FRAME;
    let end = start + heap_frame * BYTES_PER_FRAME;
    unsafe {
        ALLOCATOR.init(start, end);
    }
    // initialized memory allocator

    // unsafe {
    //     PixelWriter::init(*config);
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
        ((*config).horizontal_resolution, (*config).vertical_resolution)
    };
    let screen = unsafe { FrameBuffer::new(*config) };
    WindowManager::new(screen);

    let mouse_id = mouse::MouseCursor::new(width, height);
    let console_id = Console::new(PixelColor { r: 255, g: 255, b: 255, a: 255}, PixelColor { r: 0, g: 0, b: 0, a: 255 }, width, height);

    WindowManager::up_down(console_id, 0);
    WindowManager::up_down(mouse_id, 1);
    WindowManager::draw();

    initialize_apic_timer();

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
    interrupt::set_idt_entry(interrupt::InterruptVector::XHCI as usize, interrupt::InterruptDescriptorAttr::new(interrupt::DescriptorType::InterruptGate, 0, true, 0), usb::int_handler_xhci as *const fn() as u64, cs);
    interrupt::load_idt();

    unsafe {
        let bsp_local_apic_id = (*(0xfee00020 as *const u32) >> 24) as u8;
        pci::configure_msi_fixed_destination(xhc_dev, bsp_local_apic_id, pci::MSITriggerMode::Level, pci::MSIDeliveryMode::Fixed, interrupt::InterruptVector::XHCI as u8, 0);
    }

    set_log_level(LogLevel::Debug);

    let xhc_bar = unsafe {pci::read_bar(xhc_dev, 0)}.unwrap();
    debug!("read bar: Success");
    let xhc_mmio_base = xhc_bar & !0xf;
    debug!("xHC mmio_base = {:0>8x}", xhc_mmio_base);

    let xhc = unsafe {
        Box::leak(Box::new(usb::XhcController::initialize(xhc_mmio_base, keyboard_handler, mouse::mouse_handler)))
    };
    xhc.run();
    xhc.configure_port();

    unsafe { asm!("sti") }

    let mut executor = task::executor::Executor::new();
    executor.spawn(task::Task::new(xhc.process_event()));
    executor.run();

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
