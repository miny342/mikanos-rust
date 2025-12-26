#![no_std]
#![no_main]
#![feature(sync_unsafe_cell)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

use kernel::{QemuExitCode, entry, exit_qemu};

entry!(before_test);

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    kernel::panic::test_panic_handler(info);
}

extern "sysv64" fn before_test(_config: *const common::Config) -> ! {
    kernel::logger::init_serial_and_logger();
    test_main();
    exit_qemu(QemuExitCode::Failed);
}

#[test_case]
fn test_logger() {
    log::set_max_level(log::LevelFilter::Warn);
    log::error!("This should appear.");
    log::warn!("This should appear. info and below should not.");
    log::info!("This should not appear.");
    log::debug!("This should not appear.");
    log::trace!("This should not appear.");
}
