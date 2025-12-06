use core::arch::asm;

pub(crate) unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
        )
    }
}

pub(crate) unsafe fn inb(port: u16) -> u8 {
    let res: u8;
    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            out("al") res,
        );
    }
    res
}

pub(crate) unsafe fn outd(port: u16, value: u32) {
    unsafe {
        asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") value,
        )
    }
}

pub(crate) unsafe fn ind(port: u16) -> u32 {
    let res: u32;
    unsafe {
        asm!(
            "in eax, dx",
            in("dx") port,
            out("eax") res,
        );
    }
    res
}
