use core::{ffi::{CStr, c_void}, ptr::null, sync::atomic::{AtomicBool, AtomicPtr}};

use crate::serial_println;

static LOCK: AtomicBool = AtomicBool::new(false);
static INITALIZED: AtomicBool = AtomicBool::new(false);
static mut SYMTAB_PTR: *const Elf64Sym = null();
static mut SYMTAB_NUM: usize = 0;
static mut STRTAB_PTR: *const i8 = null();
static mut BASE: usize = 0;

#[derive(Debug)]
#[repr(C)]
struct Elf64Sym {
    name: u32,
    info: u8,
    other: u8,
    shndx: u16,
    value: u64,
    size: u64,
}

// パラメータがすべて正当であること
pub unsafe fn init_backtrace(base: usize, symtab_ptr: *const c_void, symtab_num: usize, strtab_ptr: *const c_void) {
    // 最低限のassertをしておく
    if base == 0 || symtab_ptr.is_null() || symtab_num == 0 || strtab_ptr.is_null() {
        return;
    }

    // 初期化は一度きり
    if LOCK.compare_exchange_weak(false, true, core::sync::atomic::Ordering::SeqCst, core::sync::atomic::Ordering::SeqCst).is_ok() {
        unsafe {
            BASE = base;
            SYMTAB_PTR = symtab_ptr as *const Elf64Sym;
            SYMTAB_NUM = symtab_num;
            STRTAB_PTR = strtab_ptr as *const i8;
        }
        INITALIZED.store(true, core::sync::atomic::Ordering::Release);
    }
}

fn print_fn_name(rip: u64) {
    unsafe {
        if !INITALIZED.load(core::sync::atomic::Ordering::Acquire) {
            return;
        }

        // 別にパフォーマンスは気にしないのでループを回す
        for i in 0..SYMTAB_NUM {
            let sym = &*SYMTAB_PTR.add(i);
            let value = sym.value as usize;
            let size = sym.size as usize;
            let rip = rip as usize;
            if BASE + value <= rip && rip < BASE + value + size {
                serial_println!("Function address: {:x}", rip - BASE);
                let str = CStr::from_ptr(STRTAB_PTR.add(sym.name as usize));
                // マングリングはあきらめる
                serial_println!("         name   : {:?}", str);
                break;
            }
        }
    }
}

pub fn print_backtrace() {
    unsafe {
        let mut rbp: *const u64;
        core::arch::asm!("mov {}, rbp", out(reg) rbp);

        while *rbp != 0 {
            let ret_addr = *rbp.offset(1);
            print_fn_name(ret_addr);
            rbp = *rbp as *const u64;
        }
    }
}
