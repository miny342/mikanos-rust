#[macro_export]
macro_rules! entry {
    ($p:path) => {
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        unsafe extern "sysv64" fn kernel_main(_config: *const common::writer_config::FrameBufferConfig, _memmap_ptr: *const uefi::mem::memory_map::MemoryMapOwned) {
            use core::arch::naked_asm;
            use core::cell::SyncUnsafeCell;
            use core::mem::MaybeUninit;

            const _TYPE_CHECK: extern "sysv64" fn(*const common::writer_config::FrameBufferConfig, *const uefi::mem::memory_map::MemoryMapOwned) -> ! =
                $p as extern "sysv64" fn(*const common::writer_config::FrameBufferConfig, *const uefi::mem::memory_map::MemoryMapOwned) -> !;

            const STACK_SIZE: usize = 1024 * 1024 * 8;

            #[repr(align(16))]
            struct Stack{
                #[allow(unused)]
                data: [u8; STACK_SIZE]
            }

            static STACK: SyncUnsafeCell<MaybeUninit<Stack>> = SyncUnsafeCell::new(MaybeUninit::uninit());

            naked_asm!(
                "lea rsp, {}[{} + rip]",
                "call {}",
                "2: cli",
                "hlt",
                "jmp 2b",
                sym STACK,
                const STACK_SIZE,
                sym $p,
            )
        }
    };
}
