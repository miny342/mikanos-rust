#![no_std]
#![no_main]
#![feature(sync_unsafe_cell)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

use kernel::{QemuExitCode, entry, exit_qemu, logger::init_serial_and_logger, panic::{default_panic_print, init_default_panic_print}, serial_println};

entry!(before_test);

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    serial_println!("{}", info);
    unsafe { default_panic_print(info); }
    // すぐ落ちるので見えるように
    for i in 0..1 << 25 { core::hint::black_box(i); }
    exit_qemu(QemuExitCode::Success);
}

extern "sysv64" fn before_test(config: *const common::Config) -> ! {
    unsafe { init_default_panic_print(&raw const (*config).frame_buffer_config); }
    init_serial_and_logger();
    assert_eq!(1, 2); // panic
    exit_qemu(QemuExitCode::Failed);
}
