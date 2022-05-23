#![no_std]
#![no_main]
#![feature(abi_efiapi)]

use uefi::CStr16;
use uefi::Identify;
use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::table::boot::AllocateType;
use uefi::table::boot::MemoryType;
use uefi::proto::media::file::Directory;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::file::FileInfo;
use uefi::proto::media::file::FileType::Regular;
use uefi::table::boot::{
    SearchType, OpenProtocolParams, OpenProtocolAttributes
};

use elf_rs::Elf;
use elf_rs::ProgramType;

use core::arch::asm;
use core::fmt::Write;
use core::ops::Deref;

pub mod writer_config;

use writer_config::*;

#[macro_use]
extern crate alloc;

#[entry]
fn efi_main(handle: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    writeln!(st.stdout(), "Hello, World!").unwrap();

    let mem_desc_buffer: &mut [u8] = &mut [0; 4 * 4096];
    let (_memory_map_key, descriptor_iter) = st.boot_services().memory_map(mem_desc_buffer).unwrap();

    let fs = st.boot_services().get_image_file_system(handle).unwrap().interface.get();
    let mut root_dir: Directory;
    unsafe {
        root_dir = (*fs).open_volume().unwrap();
    }

    let mut str_buf = [0; 100];
    let name = CStr16::from_str_with_buf("\\memmap", &mut str_buf).unwrap();
    let memmap_file_type = root_dir.open(name, FileMode::CreateReadWrite, FileAttribute::empty()).unwrap().into_type().unwrap();
    if let Regular(mut memmap_file) = memmap_file_type {
        let header = "Index, Type, Type(name), PhysicalStart, NumberOfPages, Attribute\n".as_bytes();
        memmap_file.write(header).unwrap();

        for (i, d) in descriptor_iter.enumerate() {
            let tmp = format!("{}, {:x}, {:?}, {:>08x}, {:x}, {:x}\n", i, d.ty.0, d.ty, d.phys_start, d.page_count, d.att.bits());
            let buf = tmp.as_bytes();
            memmap_file.write(buf).unwrap();
        }

        memmap_file.close();
    }

    // https://stackoverflow.com/questions/57487924/what-is-the-correct-way-to-load-a-uefi-protocol

    let gop = {
        // st immutable borrowed...
        let framehandlebuffer = st.boot_services().locate_handle_buffer(SearchType::ByProtocol(&GraphicsOutput::GUID)).unwrap();
        let gophandle = framehandlebuffer.handles()[0];

        let gop_ptr = st.boot_services().open_protocol::<GraphicsOutput>(
            OpenProtocolParams {
                handle: gophandle,
                agent: handle,
                controller: None
            },
            OpenProtocolAttributes::GetProtocol
        ).unwrap().interface.get();
        unsafe { &mut *gop_ptr }
    };

    let mode = gop.current_mode_info();

    let config = FrameBufferConfig {
        frame_buffer: gop.frame_buffer().as_mut_ptr(),
        pixels_per_scan_line: mode.stride(),
        horizontal_resolution: mode.resolution().0,
        vertical_resolution: mode.resolution().1,
        size: gop.frame_buffer().size(),
        pixel_format: {
            let n = mode.pixel_format() as usize;
            match n {
                0 => Some(PixelFormat::Rgb),
                1 => Some(PixelFormat::Bgr),
                // 2 => Some(PixelFormat::Bitmask),
                // 3 => Some(PixelFormat::BltOnly),
                _ => None
            }.unwrap()
        },
    };

    let name = CStr16::from_str_with_buf("\\kernel", &mut str_buf).unwrap();
    let kernel_file = root_dir.open(name, FileMode::Read, FileAttribute::READ_ONLY).unwrap().into_type().unwrap();
    if let Regular(mut kernel_file) = kernel_file {
        let buf = &mut [0u8; 2048];
        let kernel_file_info: &mut FileInfo = kernel_file.get_info(buf).unwrap();

        let kernel_file_size: usize = kernel_file_info.file_size() as usize;

        let kernel_buffer = st.boot_services().allocate_pool(MemoryType::LOADER_DATA, kernel_file_size).unwrap();
        let kernel_buffer_slice = unsafe { core::slice::from_raw_parts_mut(kernel_buffer, kernel_file_size) };
        kernel_file.read(kernel_buffer_slice).unwrap();
        let elf = Elf::from_bytes(kernel_buffer_slice).unwrap();
        let elf64 = match elf {
            Elf::Elf64(elf) => elf,
            _ => panic!()
        };
        let (first, last) = {
            let mut first = usize::MAX;
            let mut last = 0 as usize;
            for hw in elf64.program_header_iter() {
                let h = hw.deref();
                match h.ph_type() {
                    ProgramType::LOAD => {
                        first = usize::min(first, h.vaddr() as usize);
                        last = usize::max(last, (h.vaddr() + h.memsz()) as usize);
                    },
                    _ => {}
                }
            }
            (first, last)
        };

        let kernel_ptr = st.boot_services().allocate_pages(
            AllocateType::Address(first),
            MemoryType::LOADER_DATA,
            (last - first + 0xfff) / 0x1000
        ).unwrap();

        for hw in elf64.program_header_iter() {
            let h = hw.deref();
            match h.ph_type() {
                ProgramType::LOAD => unsafe {
                    let segm_in_file = kernel_buffer.add(h.offset() as usize) as *mut u8;
                    st.boot_services().memmove(h.vaddr() as *mut u8, segm_in_file, h.filesz() as usize);
                    let remain_bytes = (h.memsz() - h.filesz()) as usize;
                    st.boot_services().set_mem((h.vaddr() + h.filesz()) as *mut u8, remain_bytes, 0);
                },
                _ => {}
            }
        }

        st.boot_services().free_pool(kernel_buffer).unwrap();

        let entry_point_address = kernel_ptr + 24;
        let entry_point = unsafe { *(entry_point_address as *const u64) };
        writeln!(st.stdout(), "entry point: {:x}", entry_point).unwrap();

        let kernel_entry = unsafe {
            let f: extern "efiapi" fn(*const FrameBufferConfig) -> ! = core::mem::transmute(entry_point);
            f
        };

        let mut b = vec![0u8; st.boot_services().memory_map_size().map_size + 2048].into_boxed_slice();
        st.exit_boot_services(handle, &mut b[..]).unwrap();

        kernel_entry(&config);
    }
    writeln!(st.stdout(), "kernel load error").unwrap();
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

