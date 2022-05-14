#![no_std]
#![no_main]

#![feature(abi_efiapi)]

use core::arch::asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}


#[no_mangle]
extern "efiapi" fn kernel_main(frame_ptr: *mut u8, frame_cnt: usize) -> ! {
    unsafe {
        for i in 0..frame_cnt {
            *(frame_ptr.offset(i as isize)) = (i % 255) as u8;
        }
    }
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
