#![no_std]
#![no_main]
#![feature(abi_efiapi)]

extern crate uefi_services;

use uefi::prelude::*;
use core::fmt::Write;

#[entry]
fn efi_main(_image: Handle, mut st: SystemTable<Boot>) -> Status {
    writeln!(st.stdout(), "Hello, World!").unwrap();
    loop {}
}

