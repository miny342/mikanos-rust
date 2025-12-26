#![no_std]
#![cfg_attr(test, no_main)]

#![feature(sync_unsafe_cell)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

pub mod graphics;
pub mod ascii;
pub mod console;
pub mod pci;
pub mod error;
pub mod logger;
pub mod usb;
pub mod mouse;
pub mod interrupt;
pub mod segment;
pub mod paging;
pub mod memory_manager;
pub mod allocator;
pub mod task;
pub mod window;
pub mod timer;
pub mod serial;
pub mod entry;
pub mod math;
pub mod io_port;
pub mod acpi;
pub mod panic;
pub mod keyboard;
pub mod preemptive;
pub mod backtrace;

extern crate alloc;

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T where T: Fn() {
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<Self>());
        self();
        serial_println!("[ok]");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    unsafe {
        crate::io_port::outd(0xf4, exit_code as u32);
    }
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

#[cfg(test)]
entry!(crate::kernel_test);

#[cfg(test)]
pub extern "sysv64" fn kernel_test(_config: *const common::Config) -> ! {
    // ロガーとアロケータのみ初期化
    logger::init_serial_and_logger();
    log::set_max_level(log::LevelFilter::Trace);
    unsafe {
        segment::init_segment();
        paging::setup_identity_page_table();
        memory_manager::init_memory_manager(memmap_ptr);
        allocator::init_allocator();
    }
    test_main();
    exit_qemu(QemuExitCode::Failed);
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    crate::panic::test_panic_handler(info);
}
