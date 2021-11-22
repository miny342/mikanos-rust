#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(alloc_error_handler)]

//extern crate uefi_services;

use alloc::alloc::Layout;

#[alloc_error_handler]
fn on_oom(_layout: Layout) -> ! {
    loop {}
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[macro_use]
extern crate alloc;

use uefi::alloc::*;
use uefi::prelude::*;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::Directory;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::file::RegularFile;
use uefi::proto::media::fs::SimpleFileSystem;
use core::fmt::Write;

#[entry]
fn efi_main(handle: Handle, st: SystemTable<Boot>) -> Status {
    unsafe {
        init(st.boot_services());
    }

    writeln!(st.stdout(), "Hello, World!").unwrap();

    let mem_desc_buffer: &mut [u8] = &mut [0; 4 * 4096];
    let (_memory_map_key, descriptor_iter) = st.boot_services().memory_map(mem_desc_buffer).unwrap_success();

    let loaded_image = st.boot_services().handle_protocol::<LoadedImage>(handle).unwrap_success().get();
    let device;
    unsafe {
        device = (*loaded_image).device();
    }
    let fs = st.boot_services().handle_protocol::<SimpleFileSystem>(device).unwrap_success().get();
    let mut root_dir: Directory;
    unsafe {
        root_dir = (*fs).open_volume().unwrap_success();
    }

    let memmap_file_handle = root_dir.open("\\memmap", FileMode::CreateReadWrite, FileAttribute::empty()).unwrap_success();
    let mut memmap_file;
    unsafe {
        memmap_file = RegularFile::new(memmap_file_handle);
    }
    let header = "Index, Type, Type(name), PhysicalStart, NumberOfPages, Attribute\n".as_bytes();
    memmap_file.write(header).unwrap_success();

    for (i, d) in descriptor_iter.enumerate() {
        let tmp = format!("{}, {}, {:?}, {}, {}, {}\n", i, d.ty.0, d.ty, d.phys_start, d.page_count, d.att.bits());
        let buf = tmp.as_bytes();
        memmap_file.write(buf).unwrap_success();
    }

    memmap_file.close();

    writeln!(st.stdout(), "All done").unwrap();

    exit_boot_services();

    loop {}
}

