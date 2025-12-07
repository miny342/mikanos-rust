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
use kernel::serial_println;
use kernel::timer::Timer;
use kernel::timer::get_tick;
use kernel::timer::timer_manager;
use kernel::usb::controller::init_xhc;
use log::{debug, error};

use common::writer_config::FrameBufferConfig;

use kernel::graphics::*;
use kernel::console::*;
use kernel::interrupt::disable_interrupt;
use kernel::timer::initialize_apic_timer;
use kernel::window::WindowManager;
use kernel::pci;
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

async fn counter(window: alloc::sync::Arc<spin::Mutex<kernel::window::Window>>) {
    let id = {
        let mut lck = window.lock();
        lck.draw_basic_window("Hello Window");
        lck.write_string(format!("Counter: {:05}", get_tick()).as_str(), PixelColor::WHITE, 8, 30);
        lck.id()
    };
    WindowManager::draw_window(id);

    let mut timeout = 200;
    let mut value = 1;
    let mut timer = Timer::new(timeout, value);
    loop {
        let v = timer.await;
        serial_println!("Timer: {} {}", timeout, v);
        let id = {
            let mut lck = window.lock();
            lck.draw_basic_window("Hello Window");
            lck.write_string(format!("Counter: {:05}", get_tick()).as_str(), PixelColor::WHITE, 8, 30);
            lck.id()
        };
        WindowManager::draw_window(id);
        timeout += 100;
        value += 1;
        timer = Timer::new(timeout, value);
    }
}

async fn counter2() {
    Timer::new(600, 100).await;
    log::warn!("Timer: 600 100");
}

kernel::entry!(kernel_main_new_stack);

pub extern "sysv64" fn kernel_main_new_stack(config: *const FrameBufferConfig, memmap_ptr: *const uefi::mem::memory_map::MemoryMapOwned) -> ! {
    // 初期化は割り込みなしにしておく
    unsafe { disable_interrupt() };
    let framebufferconfig = unsafe { *config };

    kernel::logger::init_serial_and_logger();
    log::set_max_level(log::LevelFilter::Warn);
    unsafe {
        kernel::segment::init_segment();
        kernel::paging::setup_identity_page_table();
        kernel::memory_manager::init_memory_manager(memmap_ptr);
        kernel::allocator::init_allocator();
    }
    // initialized memory allocator

    unsafe { WindowManager::new(framebufferconfig) };

    let mouse_id = mouse::MouseCursor::new();
    let console_id = Console::new(PixelColor { r: 255, g: 255, b: 255, a: 255}, PixelColor { r: 0, g: 0, b: 0, a: 255 });

    WindowManager::up_down(console_id, 0);
    WindowManager::up_down(mouse_id, 1);
    WindowManager::draw();

    initialize_apic_timer();

    kernel::println!("hello");

    let res = pci::scan_all_bus();
    let devices = match res {
        Ok(d) => {
            debug!("scan all bus: Success");
            d
        },
        Err((d, e)) => {
            debug!("scan all bus: {}", e);
            d
        }
    };

    unsafe { interrupt::init_interrupt(); }

    let xhc = Box::leak(init_xhc(&devices).unwrap());
    xhc.run();
    xhc.configure_port();
    // xhc ok

    let (main_window_id, main_window) = WindowManager::new_window(160, 52, false, 300, 100, true);
    WindowManager::up_down(main_window_id, 1);

    let mut executor = task::executor::Executor::new();
    executor.spawn(task::Task::new(xhc.process_event()));
    executor.spawn(task::Task::new(counter(main_window)));
    executor.spawn(task::Task::new(timer_manager()));
    executor.spawn(task::Task::new(counter2()));
    executor.run();
}
