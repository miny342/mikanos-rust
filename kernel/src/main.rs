#![no_std]
#![no_main]

#![feature(sync_unsafe_cell)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::arch::asm;
use core::panic::PanicInfo;
use alloc::boxed::Box;
use alloc::format;
use futures_util::StreamExt;
use futures_util::task::AtomicWaker;
use kernel::serial_println;
use log::{debug, info, error};

use common::writer_config::FrameBufferConfig;
use uefi::boot::PAGE_SIZE;
use uefi::mem::memory_map::MemoryMap;

use kernel::graphics::*;
use kernel::console::*;
use kernel::interrupt::disable_interrupt;
use kernel::logger::*;
use kernel::memory_manager::{
    FrameID,
    BYTES_PER_FRAME, MANAGER
};
use kernel::serial::init_serial;
use kernel::timer::initialize_apic_timer;
use kernel::window::WindowManager;
use kernel::pci;
use kernel::usb;
use kernel::mouse;
use kernel::interrupt;
use kernel::task;

extern crate alloc;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe { disable_interrupt() };
    error!("{}", info);
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

#[test_case]
fn test_works_test() {
    kernel::serial_println!("if this printed, test works!");
}

fn keyboard_handler(_modifire: u8, _pressing: [u8; 6]) {

}

static TMP_WAKER: AtomicWaker = AtomicWaker::new();
static TMP2_WAKER: AtomicWaker = AtomicWaker::new();

struct TmpTask {state: bool}

impl futures_util::Stream for TmpTask {
    type Item = ();
    fn poll_next(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> core::task::Poll<Option<Self::Item>> {
        if self.state {
            TMP2_WAKER.wake();
            self.get_mut().state = false;
            core::task::Poll::Ready(Some(()))
        } else {
            TMP_WAKER.register(cx.waker());
            self.get_mut().state = true;
            core::task::Poll::Pending
        }
    }
}

struct Tmp2Task {state: bool}

impl futures_util::Stream for Tmp2Task {
    type Item = ();
    fn poll_next(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> core::task::Poll<Option<Self::Item>> {
        if self.state {
            TMP_WAKER.wake();
            self.get_mut().state = false;
            core::task::Poll::Ready(Some(()))
        } else {
            TMP2_WAKER.register(cx.waker());
            self.get_mut().state = true;
            core::task::Poll::Pending
        }
    }
}

async fn tmp_task() {
    let mut task = Box::new(Tmp2Task { state: true});
    while task.next().await.is_some() {
        debug!("tmp task running");
    }
}

async fn counter(window: alloc::sync::Arc<spin::Mutex<kernel::window::Window>>) {
    let mut cnt = 0;
    let mut task = Box::new(TmpTask { state: true});
    while task.next().await.is_some() {
        {
            let mut lck = window.lock();
            lck.draw_basic_window("Hello Window");
            lck.write_string(format!("Counter: {:05}", cnt).as_str(), PixelColor::WHITE, 8, 30);
            cnt += 1;
        }
    }
}

kernel::entry!(kernel_main_new_stack);

pub extern "sysv64" fn kernel_main_new_stack(config: *const FrameBufferConfig, memmap_ptr: *const uefi::mem::memory_map::MemoryMapOwned) -> ! {
    // 初期化は割り込みなしにしておく
    unsafe { disable_interrupt() };
    let framebufferconfig = unsafe { *config };

    kernel::logger::init_serial_and_logger();
    log::set_max_level(log::LevelFilter::Trace);
    unsafe {
        kernel::segment::setup_segments();
        kernel::segment::set_ds_all(0);
        kernel::segment::set_csss(1 << 3, 2 << 3);
        kernel::paging::setup_identity_page_table();
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
        kernel::ALLOCATOR.init(start as *mut u8, end as *mut u8);
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


    let (width, height) = ((framebufferconfig).horizontal_resolution, (framebufferconfig).vertical_resolution);
    let screen = unsafe { FrameBuffer::new(framebufferconfig) };
    WindowManager::new(screen);

    let mouse_id = mouse::MouseCursor::new(width, height);
    let console_id = Console::new(PixelColor { r: 255, g: 255, b: 255, a: 255}, PixelColor { r: 0, g: 0, b: 0, a: 255 }, width, height);

    WindowManager::up_down(console_id, 0);
    WindowManager::up_down(mouse_id, 1);
    WindowManager::draw();

    initialize_apic_timer();

    kernel::println!("hello");

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
    interrupt::set_idt_entry(interrupt::InterruptVector::XHCI as usize, interrupt::InterruptDescriptorAttr::new(interrupt::DescriptorType::InterruptGate, 0, true, 0), usb::controller::int_handler_xhci as *const fn() as u64, cs);
    interrupt::load_idt();

    unsafe {
        let bsp_local_apic_id = (*(0xfee00020 as *const u32) >> 24) as u8;
        pci::configure_msi_fixed_destination(xhc_dev, bsp_local_apic_id, pci::MSITriggerMode::Level, pci::MSIDeliveryMode::Fixed, interrupt::InterruptVector::XHCI as u8, 0);
    }

    let xhc_bar = unsafe {pci::read_bar(xhc_dev, 0)}.unwrap();
    debug!("read bar: Success");
    let xhc_mmio_base = xhc_bar & !0xf;
    debug!("xHC mmio_base = {:0>8x}", xhc_mmio_base);

    let xhc = unsafe {
        Box::new(usb::controller::XhcController::initialize(xhc_mmio_base, keyboard_handler, mouse::mouse_handler))
    };
    let xhc = Box::leak(xhc);
    xhc.run();
    xhc.configure_port();

    // xhc ok

    let (main_window_id, main_window) = WindowManager::new_window(160, 52, false, 300, 100);
    WindowManager::up_down(main_window_id, 1);

    #[cfg(test)]
    test_main();

    let mut executor = task::executor::Executor::new();
    executor.spawn(task::Task::new(xhc.process_event()));
    executor.spawn(task::Task::new(counter(main_window)));
    executor.spawn(task::Task::new(tmp_task()));
    executor.run();
}
