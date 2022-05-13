#![no_std]
#![no_main]

#![feature(abi_efiapi)]

use core::arch::asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {

    }
}


#[no_mangle]
extern "efiapi" fn kernel_main() -> ! {
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
