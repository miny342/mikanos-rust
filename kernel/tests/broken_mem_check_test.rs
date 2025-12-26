#![no_std]
#![no_main]
#![feature(sync_unsafe_cell)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::{alloc::Layout};
use core::alloc::GlobalAlloc;
use kernel::{QemuExitCode, allocator::MemoryCorruptionCheckAllocator, entry, exit_qemu, logger::init_serial_and_logger, panic::{init_default_panic_print}, serial_println};

entry!(before_test);

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    serial_println!("{}", info);
    serial_println!("{:?}", unsafe { MEMORY });
    exit_qemu(QemuExitCode::Success);
}

static mut MEMORY: [u8; 1024] = [0; 1024];

extern "sysv64" fn before_test(config: *const common::Config) -> ! {
    unsafe { init_default_panic_print(&raw const (*config).frame_buffer_config); }
    init_serial_and_logger();
    let al = MemoryCorruptionCheckAllocator::empty();
    unsafe {
        al.init(&raw mut MEMORY as *mut u8, (&raw mut MEMORY).byte_add(1024) as *mut u8);
        let layout = Layout::from_size_align_unchecked(100, 1);
        let ptr = al.alloc(layout);
        *ptr = 3;
        *ptr.add(99) = 4;
        let ptr2 = al.alloc(Layout::from_size_align_unchecked(100, 1));
        *ptr2 = 3;
        *ptr2.add(99) = 4;
        al.dealloc(ptr2, layout);
        let ptr3 = al.alloc(Layout::from_size_align_unchecked(101, 16));
        *ptr3 = 6;
        *ptr3.add(100) = 7;
        MEMORY[1] = 0;
        al.dealloc(ptr3, Layout::from_size_align_unchecked(101, 1));
        al.dealloc(ptr, layout); // panic by MEMORY[1]
    }
    exit_qemu(QemuExitCode::Failed);
}
