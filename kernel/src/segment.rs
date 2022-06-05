use core::arch::asm;

static mut GDT: [SegmentDescriptor; 3] = [SegmentDescriptor {0: 0}; 3];

#[derive(Debug, Clone, Copy)]
struct SegmentDescriptor(u64);

impl SegmentDescriptor {
    fn or(&mut self, v: u64, shift: u64) {
        self.0 |= v << shift
    }
}

enum DescriptorType {
    ReadWrite = 2,
    ExecuteRead = 10
}

unsafe fn set_code_segment(idx: usize, ty: DescriptorType, descriptor_privilege_level: u64, base: u64, limit: u64) {
    let b = GDT.get_unchecked_mut(idx);
    b.0 = 0;
    let ty = ty as u64;
    b.or(base & 0xffff, 16);
    b.or((base >> 16) & 0xff, 32);
    b.or((base >> 24) & 0xff, 56);
    b.or(limit & 0xffff, 0);
    b.or((limit >> 16) & 0xf, 48);
    b.or(ty, 40);
    b.or(1, 44);
    b.or(descriptor_privilege_level, 45);
    b.or(1, 47);
    // b.or(0, 52);
    b.or(1, 53);
    // b.or(0, 54);
    b.or(1, 55);
}

unsafe fn set_data_segment(idx: usize, ty: DescriptorType, descriptor_privilege_level: u64, base: u64, limit: u64) {
    set_code_segment(idx, ty, descriptor_privilege_level, base, limit);
    let b = GDT.get_unchecked_mut(idx);
    b.0 = b.0 | (1 << 54) & !(1 << 53);
}

unsafe fn load_gdt() {
    const LIMIT: u16 = core::mem::size_of::<[SegmentDescriptor; 3]>() as u16;
    let offset = &GDT[0] as *const _ as u64;
    asm!(
        "sub rsp, 10",
        "mov [rsp], {limit:x}",
        "mov [rsp + 2], {offset}",
        "lgdt [rsp]",
        "add rsp, 10",
        limit = in(reg) LIMIT,
        offset = in(reg) offset,
    )
}

pub fn setup_segments() {
    unsafe {
        GDT.get_unchecked_mut(0).0 = 0;
        set_code_segment(1, DescriptorType::ExecuteRead, 0, 0, 0xfffff);
        set_data_segment(2, DescriptorType::ReadWrite, 0, 0, 0xfffff);
        load_gdt();
    }
}

pub fn set_ds_all(v: u16) {
    unsafe {
        asm!(
            "mov ds, {0:x}",
            "mov es, {0:x}",
            "mov fs, {0:x}",
            "mov gs, {0:x}",
            in(reg) v,
        )
    }
}

extern "C" {
    pub fn set_csss(cs: u16, ss: u16);
}

