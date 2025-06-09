use core::{arch::naked_asm, cell::SyncUnsafeCell, mem::MaybeUninit};

const STACK_SIZE: usize = 1024 * 1024 * 8;

#[repr(align(16))]
struct Stack{
    #[allow(unused)]
    data: [u8; STACK_SIZE]
}

static STACK: SyncUnsafeCell<MaybeUninit<Stack>> = SyncUnsafeCell::new(MaybeUninit::uninit());

#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "sysv64" fn kernel_main(_config: *const common::writer_config::FrameBufferConfig, _memmap_ptr: *const uefi::mem::memory_map::MemoryMapOwned) {
    naked_asm!(
        "lea rsp, {}[{} + rip]",
        "call {}",
        "2: hlt",
        "jmp 2b",
        sym STACK,
        const STACK_SIZE,
        sym crate::kernel_main_new_stack,
    )
}
