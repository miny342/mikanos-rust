use spin::Mutex;
use core::arch::asm;

#[repr(C)]
pub struct InterruptFrame {
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

static IDT: Mutex<[InterruptDescriptor; 256]> = Mutex::new([
    InterruptDescriptor {
        offset_low: 0,
        segment_selector: 0,
        attr: 0,
        offset_middle: 0,
        offset_high: 0,
        revdz: 0
    }; 256
]);

pub enum InterruptVector {
    XHCI = 0x40
}

pub enum DescriptorType {
    Upper8Bytes = 0,
    LDT = 2,
    TSSAvailable = 9,
    TSSBusy = 11,
    CallGate = 12,
    InterruptGate = 14,
    TrapGate = 15,
}

pub struct InterruptDescriptorAttr {
    ist: u16,
    ty: u16,
    dpl: u16,
    p: u16
}

impl InterruptDescriptorAttr {
    pub fn new(ty: DescriptorType, descriptor_privilege_level: u8, present: bool, interrupt_stack_table: u8) -> InterruptDescriptorAttr {
        InterruptDescriptorAttr {
            ist: interrupt_stack_table as u16,
            ty: ty as u16,
            dpl: descriptor_privilege_level as u16,
            p: present as u16,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct InterruptDescriptor {
    offset_low: u16,
    segment_selector: u16,
    attr: u16,
    offset_middle: u16,
    offset_high: u32,
    revdz: u32
}

impl InterruptDescriptor {
    fn set_attr(&mut self, attr: InterruptDescriptorAttr) {
        self.attr = attr.ist | attr.ty << 8 | attr.dpl << 13 | attr.p << 15
    }
}

pub fn set_idt_entry(idx: usize, attr: InterruptDescriptorAttr, offset: u64, segment_selector: u16) {
    let idt = &mut IDT.lock()[idx];
    idt.set_attr(attr);
    idt.offset_low = (offset & 0xffff) as u16;
    idt.offset_middle = ((offset >> 16) & 0xffff) as u16;
    idt.offset_high = (offset >> 32) as u32;
    idt.segment_selector = segment_selector;
}

pub fn get_cs() -> u16 {
    let ret: u16;
    unsafe {
        asm!(
            "mov {:x}, cs",
            out(reg) ret,
        )
    }
    ret
}

pub fn load_idt() {
    const LIMIT: u16 = core::mem::size_of::<[InterruptDescriptor; 256]>() as u16 - 1;
    let offset = {
        &IDT.lock()[0] as *const InterruptDescriptor as u64
    };
    unsafe {
        asm!(
            "sub rsp, 10",
            "mov [rsp], {limit:x}",
            "mov [rsp + 2], {offset}",
            "lidt [rsp]",
            "add rsp, 10",
            limit = in(reg) LIMIT,
            offset = in(reg) offset,
        )
    }
}

pub fn notify_end_of_interrupt() {
    unsafe {
        *(0xfee000b0 as *mut u32) = 0;
    }
}
