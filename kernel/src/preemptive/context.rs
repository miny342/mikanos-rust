use core::arch::naked_asm;
use core::mem::offset_of;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, AtomicUsize};

use alloc::vec::Vec;

use crate::segment::{KERNEL_CS, KERNEL_SS};
use crate::serial_println;
use crate::timer::get_tick;

static mut KERNEL_CONTEXT: Context = Context {
    cr3: 0,
    rip: 0,
    rflags: 0,

    rax: 0,
    rbx: 0,
    rcx: 0,
    rdx: 0,
    rsi: 0,
    rdi: 0,
    rsp: 0,
    rbp: 0,

    r8: 0,
    r9: 0,
    r10: 0,
    r11: 0,
    r12: 0,
    r13: 0,
    r14: 0,
    r15: 0,

    cs: 0,
    ss: 0,
    fs: 0,
    gs: 0,
};
static PREEMPTIVE_CONTEXT_ADDR: AtomicPtr<Context> = AtomicPtr::new(null_mut());
static PREEMPTIVE_TIMER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Default, Clone)]
#[repr(align(16))]
struct Context {
    cr3: u64,
    rip: u64,
    rflags: u64,

    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rsp: u64,
    rbp: u64,

    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,

    cs: u16,
    ss: u16,
    fs: u16,
    gs: u16,
}

macro_rules! ctxoft {
    ($e:ident) => {
        offset_of!(Context, $e)
    };
}


// 現状のレジスタの状態を取得する
#[unsafe(naked)]
unsafe extern "sysv64" fn context_get(now_ctx: &mut Context) {
    naked_asm!(
        "mov [rdi + {rax_addr}], rax",
        "mov [rdi + {rbx_addr}], rbx",
        "mov [rdi + {rcx_addr}], rcx",
        "mov [rdi + {rdx_addr}], rdx",
        "mov [rdi + {rsi_addr}], rsi",
        "mov [rdi + {rdi_addr}], rdi",

        "lea rax, [rsp + 8]", // return addrの分だけ戻す
        "mov [rdi + {rsp_addr}], rax",
        "mov [rdi + {rbp_addr}], rbp",

        "mov [rdi + {r8_addr}], r8",
        "mov [rdi + {r9_addr}], r9",
        "mov [rdi + {r10_addr}], r10",
        "mov [rdi + {r11_addr}], r11",
        "mov [rdi + {r12_addr}], r12",
        "mov [rdi + {r13_addr}], r13",
        "mov [rdi + {r14_addr}], r14",
        "mov [rdi + {r15_addr}], r15",

        "mov rax, [rsp]", // return addr
        "mov rbx, cr3",
        "mov [rdi + {rip_addr}], rax",
        "mov [rdi + {cr3_addr}], rbx",
        "pushfq",
        "pop qword ptr [rdi + {rflags_addr}]",

        "mov ax, cs",
        "mov bx, ss",
        "mov cx, fs",
        "mov dx, gs",
        "mov [rdi + {cs_addr}], ax",
        "mov [rdi + {ss_addr}], bx",
        "mov [rdi + {fs_addr}], cx",
        "mov [rdi + {gs_addr}], dx",

        // save complete
        // kernelと同様にsseはすべて無効にしておくつもりなのでxsaveoptはいらない

        "ret",

        cr3_addr = const ctxoft!(cr3),
        rip_addr = const ctxoft!(rip),
        rflags_addr = const ctxoft!(rflags),

        rax_addr = const ctxoft!(rax),
        rbx_addr = const ctxoft!(rbx),
        rcx_addr = const ctxoft!(rcx),
        rdx_addr = const ctxoft!(rdx),
        rsi_addr = const ctxoft!(rsi),
        rdi_addr = const ctxoft!(rdi),
        rsp_addr = const ctxoft!(rsp),
        rbp_addr = const ctxoft!(rbp),

        r8_addr = const ctxoft!(r8),
        r9_addr = const ctxoft!(r9),
        r10_addr = const ctxoft!(r10),
        r11_addr = const ctxoft!(r11),
        r12_addr = const ctxoft!(r12),
        r13_addr = const ctxoft!(r13),
        r14_addr = const ctxoft!(r14),
        r15_addr = const ctxoft!(r15),

        cs_addr = const ctxoft!(cs),
        ss_addr = const ctxoft!(ss),
        fs_addr = const ctxoft!(fs),
        gs_addr = const ctxoft!(gs),
    )
}


// safety: コンテキストが正当であること
#[unsafe(naked)]
unsafe extern "sysv64" fn context_switch(next_ctx: &Context, now_ctx: &mut Context) {
    naked_asm!(
        "mov [rsi + {rax_addr}], rax",
        "mov [rsi + {rbx_addr}], rbx",
        "mov [rsi + {rcx_addr}], rcx",
        "mov [rsi + {rdx_addr}], rdx",
        "mov [rsi + {rsi_addr}], rsi",
        "mov [rsi + {rdi_addr}], rdi",

        "lea rax, [rsp + 8]", // return addrの分だけ戻す
        "mov [rsi + {rsp_addr}], rax",
        "mov [rsi + {rbp_addr}], rbp",

        "mov [rsi + {r8_addr}], r8",
        "mov [rsi + {r9_addr}], r9",
        "mov [rsi + {r10_addr}], r10",
        "mov [rsi + {r11_addr}], r11",
        "mov [rsi + {r12_addr}], r12",
        "mov [rsi + {r13_addr}], r13",
        "mov [rsi + {r14_addr}], r14",
        "mov [rsi + {r15_addr}], r15",

        "mov rax, [rsp]", // return addr
        "mov rbx, cr3",
        "mov [rsi + {rip_addr}], rax",
        "mov [rsi + {cr3_addr}], rbx",
        "pushfq",
        "pop qword ptr [rsi + {rflags_addr}]",

        "mov ax, cs",
        "mov bx, ss",
        "mov cx, fs",
        "mov dx, gs",
        "mov [rsi + {cs_addr}], ax",
        "mov [rsi + {ss_addr}], bx",
        "mov [rsi + {fs_addr}], cx",
        "mov [rsi + {gs_addr}], dx",

        // save complete
        // kernelと同様にsseはすべて無効にしておくつもりなのでxsaveoptはいらない

        // iret用のスタックを作成
        "movzx rax, word ptr [rdi + {ss_addr}]",
        "movzx rbx, word ptr [rdi + {cs_addr}]",
        "push rax",
        "push qword ptr [rdi + {rsp_addr}]",
        "push qword ptr [rdi + {rflags_addr}]",
        "push rbx",
        "push qword ptr [rdi + {rip_addr}]",

        "mov rax, [rdi + {cr3_addr}]",
        "mov bx, [rdi + {fs_addr}]",
        "mov cx, [rdi + {gs_addr}]",
        "mov cr3, rax",
        "mov fs, bx",
        "mov gs, cx",

        "mov rax, [rdi + {rax_addr}]",
        "mov rbx, [rdi + {rbx_addr}]",
        "mov rcx, [rdi + {rcx_addr}]",
        "mov rdx, [rdi + {rdx_addr}]",
        "mov rsi, [rdi + {rsi_addr}]",
        "mov rbp, [rdi + {rbp_addr}]",
        "mov r8, [rdi + {r8_addr}]",
        "mov r9, [rdi + {r9_addr}]",
        "mov r10, [rdi + {r10_addr}]",
        "mov r11, [rdi + {r11_addr}]",
        "mov r12, [rdi + {r12_addr}]",
        "mov r13, [rdi + {r13_addr}]",
        "mov r14, [rdi + {r14_addr}]",
        "mov r15, [rdi + {r15_addr}]",

        "mov rdi, [rdi + {rdi_addr}]",

        "iretq",

        cr3_addr = const ctxoft!(cr3),
        rip_addr = const ctxoft!(rip),
        rflags_addr = const ctxoft!(rflags),

        rax_addr = const ctxoft!(rax),
        rbx_addr = const ctxoft!(rbx),
        rcx_addr = const ctxoft!(rcx),
        rdx_addr = const ctxoft!(rdx),
        rsi_addr = const ctxoft!(rsi),
        rdi_addr = const ctxoft!(rdi),
        rsp_addr = const ctxoft!(rsp),
        rbp_addr = const ctxoft!(rbp),

        r8_addr = const ctxoft!(r8),
        r9_addr = const ctxoft!(r9),
        r10_addr = const ctxoft!(r10),
        r11_addr = const ctxoft!(r11),
        r12_addr = const ctxoft!(r12),
        r13_addr = const ctxoft!(r13),
        r14_addr = const ctxoft!(r14),
        r15_addr = const ctxoft!(r15),

        cs_addr = const ctxoft!(cs),
        ss_addr = const ctxoft!(ss),
        fs_addr = const ctxoft!(fs),
        gs_addr = const ctxoft!(gs),
    )
}

#[repr(align(16))]
struct Mem(
    #[allow(dead_code)]
    [u64; 2]
);

fn get_cr3() -> u64 {
    let v;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) v);
    }
    v
}

pub struct PreemptiveTask {
    context: Context,
}

impl PreemptiveTask {
    pub fn new(f: fn() -> !) -> Self {
        let mut ctx = Context::default();
        ctx.rip = f as *const fn() as u64;

        let stack: Vec<Mem> = Vec::with_capacity(1024 * 512);
        ctx.rsp = stack.as_ptr().addr() as u64 + size_of::<Mem>() as u64 * 1024 * 512 - 8;

        ctx.cr3 = get_cr3();
        ctx.rflags = 0x202;
        ctx.cs = KERNEL_CS;
        ctx.ss = KERNEL_SS;

        PreemptiveTask { context: ctx }
    }
}

impl Future for PreemptiveTask {
    type Output = ();
    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> core::task::Poll<Self::Output> {
        let s = self.get_mut();
        if PREEMPTIVE_CONTEXT_ADDR.compare_exchange_weak(null_mut(), &raw mut s.context, core::sync::atomic::Ordering::Relaxed, core::sync::atomic::Ordering::Relaxed).is_err() {
            panic!("PreemptiveTask is already running");
        }
        PREEMPTIVE_TIMER.store(get_tick() + 2, core::sync::atomic::Ordering::Relaxed);
        unsafe { context_switch(&mut s.context, &mut *(&raw mut KERNEL_CONTEXT)); }
        cx.waker().wake_by_ref(); // 常にタスクに入れておく
        core::task::Poll::Pending
    }
}

pub unsafe fn check_and_stop_preemptive() {
    if PREEMPTIVE_TIMER.load(core::sync::atomic::Ordering::Relaxed) > get_tick() {
        return;
    }

    let ptr = PREEMPTIVE_CONTEXT_ADDR.swap(null_mut(), core::sync::atomic::Ordering::Relaxed);
    if ptr.is_null() {
        return;
    }

    unsafe { context_switch(&mut *(&raw mut KERNEL_CONTEXT), &mut (*ptr)); }
}

// 後でマクロで対応
pub fn test_func() {
    let mut c = Context::default();
    unsafe { context_get(&mut c); }

    serial_println!("{:#?}", c);

    // とりあえずコンテキストをそのままパクる
    let mut d = Context::default();

    // 実行場所と引数を差し替える
    d.rip = test_func_internal as *const fn() as u64;
    d.rdi = &raw mut d as u64;
    d.rsi = &raw mut c as u64;

    // スタックは差し替える
    let stack: Vec<Mem> = Vec::with_capacity(512);
    d.rsp = stack.as_ptr() as u64 + size_of::<Mem>() as u64 * 512 - 8;

    // その他初期設定
    d.cr3 = get_cr3();
    d.rflags = 0x202; // 0x200 (interrupt enable) | 0x2 (always 1)
    d.cs = KERNEL_CS;
    d.ss = KERNEL_SS;

    unsafe { context_switch(&d, &mut c); }
    serial_println!("no_problem");
}

extern "sysv64" fn test_func_internal(ptr: *mut Context, prev: *const Context) {
    let context = unsafe { &mut *ptr };
    let prev = unsafe { &*prev };
    serial_println!("test_func_hello");
    unsafe { context_switch(prev, context); }
}