#![no_std]
#![no_main]

use elf_rs::ElfFile;
use log::info;
use uefi::boot::PAGE_SIZE;
use uefi::mem::memory_map::MemoryMap;
use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::media::file::Directory;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::file::FileInfo;
use uefi::proto::media::file::FileType::Regular;

use uefi::boot;

use elf_rs::Elf;
use elf_rs::ProgramType;
use uefi::table::cfg::ACPI2_GUID;

use core::arch::asm;
use core::ffi::c_void;
use core::ops::Deref;
use core::ptr::null_mut;
use common::{Config, EntryFn, writer_config::*};

#[macro_use]
extern crate alloc;

#[repr(C, packed)]
struct Rela {
    r_offset: u64,
    r_info: u64,
    r_addend: i64,
}

fn write_memmap(dir: &mut Directory) -> uefi::Result {
    let mut str_buf = [0; 100];
    let name = uefi::CStr16::from_str_with_buf("\\memmap", &mut str_buf).unwrap();
    let memmap_file_type = dir.open(name, FileMode::CreateReadWrite, FileAttribute::empty())?.into_type()?;
    if let Regular(mut memmap_file) = memmap_file_type {
        let descriptor = boot::memory_map(boot::MemoryType::LOADER_DATA)?;
        let header = "Index, Type, Type(name), PhysicalStart, NumberOfPages, Attribute\n".as_bytes();
        memmap_file.write(header).unwrap();

        for (i, d) in descriptor.entries().enumerate() {
            let tmp = format!("{}, {:x}, {:?}, {:>08x}, {:x}, {:x}\n", i, d.ty.0, d.ty, d.phys_start, d.page_count, d.att.bits());
            let buf = tmp.as_bytes();
            memmap_file.write(buf).unwrap();
        }
    }
    Ok(())
}

fn load_kernel(dir: &mut Directory) -> Option<(EntryFn, usize, *const c_void, u64, *const c_void)> {
    let mut str_buf = [0; 100];
    let name = uefi::CStr16::from_str_with_buf("\\kernel", &mut str_buf).unwrap();
    let kernel_file = dir.open(name, FileMode::Read, FileAttribute::READ_ONLY).unwrap().into_type().unwrap();
    if let Regular(mut kernel_file) = kernel_file {
        let buf = &mut [0u8; 2048];
        let kernel_file_info: &mut FileInfo = kernel_file.get_info(buf).unwrap();



        let kernel_file_size: usize = kernel_file_info.file_size() as usize;

        let kernel_buffer = boot::allocate_pool(boot::MemoryType::LOADER_DATA, kernel_file_size).unwrap();
        unsafe { kernel_buffer.write_bytes(0, kernel_file_size); }
        let kernel_buffer_slice = unsafe { core::slice::from_raw_parts_mut(kernel_buffer.as_ptr(), kernel_file_size) };
        kernel_file.read(kernel_buffer_slice).unwrap();
        let elf = Elf::from_bytes(kernel_buffer_slice).unwrap();
        let elf64 = match elf {
            Elf::Elf64(elf) => elf,
            _ => panic!("cannot run elf32 {:?}", elf)
        };
        let last = {
            // let mut first = usize::MAX;
            let mut last = 0 as usize;
            for hw in elf64.program_header_iter() {
                let h = hw.deref();
                match h.ph_type() {
                    ProgramType::LOAD => {
                        // first = usize::min(first, h.vaddr() as usize);
                        last = usize::max(last, (h.vaddr() + h.memsz()) as usize);
                    },
                    _ => {}
                }
            }
            // (first, last)
            last
        };

        let kernel_ptr = boot::allocate_pages(
            boot::AllocateType::AnyPages,
            boot::MemoryType::LOADER_DATA,
            last.div_ceil(PAGE_SIZE) // (last + 0xfff) / 0x1000
        ).unwrap();
        let kernel_ptr_len = last.div_ceil(PAGE_SIZE) * PAGE_SIZE;
        unsafe { kernel_ptr.write_bytes(0, kernel_ptr_len); }
        let kernel_slice = unsafe { core::slice::from_raw_parts_mut(kernel_ptr.as_ptr(), kernel_ptr_len) };

        for hw in elf64.program_header_iter() {
            let h = hw.deref();
            match h.ph_type() {
                ProgramType::LOAD => {
                    kernel_slice[h.vaddr() as usize..(h.vaddr() + h.filesz()) as usize].copy_from_slice(&kernel_buffer_slice[h.offset() as usize.. (h.offset() + h.filesz()) as usize]);
                },
                _ => {}
            }
        }

        let rela_dyn_section = elf64.lookup_section(b".rela.dyn");
        let rela_plt_section = elf64.lookup_section(b".rela.plt");
        // let dynsym_section = elf64.lookup_section(b".dynsym").unwrap();

        // let sym_table = ((kernel_buffer as u64) + dynsym_section.offset()) as *const Sym;

        // https://docs.oracle.com/cd/E23824_01/html/819-0690/chapter6-54839.html#chapter7-2
        let iter = [rela_dyn_section, rela_plt_section].into_iter();
        for sec_opt in iter {
            let sec = match sec_opt {
                Some(x) => x,
                None => continue
            };
            let rela_dyn = (&kernel_buffer_slice[sec.offset() as usize]) as *const u8 as *const Rela;
            unsafe {
                for i in 0..sec.size() / sec.entsize() {
                    let j = i as usize;
                    let r = &*rela_dyn.add(j);
                    let ty = r.r_info & 0xffffffff;
                    match ty {
                        8 => { // R_X86_64_RELATIVE
                            let to = (kernel_slice.as_mut_ptr().add(r.r_offset as usize)) as *mut u64;
                            *to = kernel_slice.as_ptr().add(r.r_addend as usize) as u64;
                        }
                        _ => panic!("unsupported reallocation type: {}", ty)
                    }
                }
            }
        }

        let (symtab_section, symtab_num) = if let Some(s) = elf64.lookup_section(b".symtab") {
            (kernel_buffer_slice.as_ptr().addr() as u64 + s.offset(), s.size() / s.entsize())
        } else {
            (0, 0)
        };
        let strtab_section = if let Some(s) = elf64.lookup_section(b".strtab") {
            kernel_buffer_slice.as_ptr().addr() as u64 + s.offset()
        } else {
            0
        };
        // バックトレースの際に関数名が参照できるようにfreeしない
        // unsafe { boot::free_pool(kernel_buffer).unwrap(); }

        let entry_point_address =  unsafe { *(kernel_slice.as_ptr().add(24) as *const usize) };
        let entry_point = entry_point_address + kernel_slice.as_ptr() as usize;
        info!("entry point: {:x}", entry_point);

        let kernel_entry = unsafe {
            let f: EntryFn = core::mem::transmute(entry_point);
            f
        };
        Some((kernel_entry, kernel_slice.as_ptr().addr(), symtab_section as *const c_void, symtab_num, strtab_section as *const c_void))
    } else {
        None
    }
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    log::set_max_level(log::LevelFilter::Info);

    info!("Hello, World!");
    let mut fs = boot::get_image_file_system(boot::image_handle()).unwrap();
    let mut root_dir = fs.open_volume().unwrap();
    let k = load_kernel(&mut root_dir);

    let d = write_memmap(&mut root_dir);
    info!("write_memmap: {:?}", d);

    drop(root_dir);
    drop(fs);

    let acpi_table_ptr = uefi::system::with_config_table(|configtablearray| {
        for entry in configtablearray {
            if entry.guid == ACPI2_GUID {
                return entry.address;
            }
        }
        null_mut()
    });

    if acpi_table_ptr.is_null() {
        panic!("acpi_table is null");
    }

    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle).unwrap();

    let mode = gop.current_mode_info();

    let config = FrameBufferConfig {
        frame_buffer: gop.frame_buffer().as_mut_ptr(),
        pixels_per_scan_line: mode.stride(),
        horizontal_resolution: mode.resolution().0,
        vertical_resolution: mode.resolution().1,
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

    let config_ptr = boot::allocate_pool(boot::MemoryType::LOADER_DATA, size_of::<Config>()).unwrap().as_ptr() as *mut Config;
    unsafe { core::ptr::write(&raw mut (*config_ptr).frame_buffer_config, config); }

    if let Some((kernel_entry, kernel_addr, symtab_ptr, symtab_num, strtab_ptr)) = k {
        let memmap = unsafe { boot::exit_boot_services(None) };
        unsafe {
            core::ptr::write(&raw mut (*config_ptr).memmap, memmap);
            core::ptr::write(&raw mut (*config_ptr).acpi_table_ptr, acpi_table_ptr);
            core::ptr::write(&raw mut (*config_ptr).base, kernel_addr);
            core::ptr::write(&raw mut (*config_ptr).symtab, symtab_ptr);
            core::ptr::write(&raw mut (*config_ptr).symtab_num, symtab_num as usize);
            core::ptr::write(&raw mut (*config_ptr).strtab, strtab_ptr);
        }
        kernel_entry(config_ptr);
    }
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
