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
    kernel::test_panic_handler(info);
}

extern "sysv64" fn before_test(_config: *const common::writer_config::FrameBufferConfig, _memmap_ptr: *const uefi::mem::memory_map::MemoryMapOwned) -> ! {
    test_main();
    exit_qemu(QemuExitCode::Failed);
}

#[test_case]
fn boot_success() {}
