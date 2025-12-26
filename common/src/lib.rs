#![no_std]
pub mod writer_config;

use core::ffi::c_void;
use uefi::mem::memory_map::MemoryMapOwned;

#[derive(Debug)]
#[repr(C)]
pub struct Config {
    pub frame_buffer_config: writer_config::FrameBufferConfig,
    pub memmap: MemoryMapOwned,
    pub acpi_table_ptr: *const c_void,
    pub base: usize,
    pub symtab: *const c_void,
    pub symtab_num: usize,
    pub strtab: *const c_void,
}

pub type EntryFn = extern "sysv64" fn(*const Config) -> !;
