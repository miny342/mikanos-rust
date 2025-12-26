#[macro_export]
macro_rules! entry {
    ($p:path) => {
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        unsafe extern "sysv64" fn kernel_main(_config: *const common::Config) -> ! {
            use core::arch::naked_asm;
            use core::cell::SyncUnsafeCell;
            use core::mem::MaybeUninit;

            const _TYPE_CHECK: common::EntryFn =
                $p as common::EntryFn;

            const STACK_SIZE: usize = 1024 * 1024 * 8;

            #[repr(align(16))]
            struct Stack{
                #[allow(unused)]
                data: [u8; STACK_SIZE]
            }

            static STACK: SyncUnsafeCell<MaybeUninit<Stack>> = SyncUnsafeCell::new(MaybeUninit::uninit());

            naked_asm!(
                "mov rbp, 0",  // rbp == 0ならスタックフレームの終了
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
