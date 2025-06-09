use core::{arch::asm, cell::SyncUnsafeCell, ptr::write_volatile};

use bitfield_struct::bitfield;

type GDTType = [SegmentDescriptor; 3];

static GDT: SyncUnsafeCell<GDTType> = SyncUnsafeCell::new([SegmentDescriptor(0); 3]);

#[bitfield(u64)]
struct SegmentDescriptor {
    #[bits(16)]
    limit_low: u64,
    #[bits(16)]
    base_low: u64,
    #[bits(8)]
    base_middle: u64,
    #[bits(4)]
    ty: DescriptorType,
    system_segment: bool,
    #[bits(2)]
    descriptor_privilege_level: u64,
    present: bool,
    #[bits(4)]
    limit_high: u64,
    available: bool,
    long_mode: bool,
    default_operation_size: bool,
    granularity: bool,
    #[bits(8)]
    base_high: u64,
}

#[derive(Debug)]
#[repr(u8)]
enum DescriptorType {
    RO = 0,
    ROA = 1,
    ReadWrite = 2,
    RWA = 3,
    ROE = 4,
    ROEs = 5,
    RWED = 6,
    RWE = 7,
    EO = 8,
    EOA = 9,
    ExecuteRead = 10,
    ERA = 11,
    EOED = 12,
    EOEA = 13,
    ER = 14,
    EREDA = 15,
}

impl DescriptorType {
    const fn into_bits(self) -> u8 {
        self as _
    }
    const fn from_bits(value: u8) -> Self {
        match value {
            0 => Self::RO,
            1 => Self::ROA,
            2 => Self::ReadWrite,
            3 => Self::RWA,
            4 => Self::ROE,
            5 => Self::ROEs,
            6 => Self::RWED,
            7 => Self::RWE,
            8 => Self::EO,
            9 => Self::EOA,
            10 => Self::ExecuteRead,
            11 => Self::ERA,
            12 => Self::EOED,
            13 => Self::EOEA,
            14 => Self::ER,
            15 => Self::EREDA,
            _ => unreachable!()
        }
    }
}

fn set_code_segment(b: &mut SegmentDescriptor, ty: DescriptorType, descriptor_privilege_level: u64, base: u64, limit: u64) {
    b.set_base_low(base & 0xffff);
    b.set_base_middle((base >> 16) & 0xff);
    b.set_base_high((base >> 24) & 0xff);

    b.set_limit_low(limit & 0xffff);
    b.set_limit_high((limit >> 16) & 0xf);

    b.set_ty(ty);
    b.set_system_segment(true);
    b.set_descriptor_privilege_level(descriptor_privilege_level);
    b.set_present(true);
    b.set_long_mode(true);
    b.set_granularity(true);
}

fn set_data_segment(b: &mut SegmentDescriptor, ty: DescriptorType, descriptor_privilege_level: u64, base: u64, limit: u64) {
    set_code_segment(b, ty, descriptor_privilege_level, base, limit);
    b.set_long_mode(false);
    b.set_default_operation_size(true);
}

unsafe fn load_gdt(offset: *mut SegmentDescriptor, limit: u16) {
    unsafe {
        asm!(
            "sub rsp, 10",
            "mov [rsp], {limit:x}",
            "mov [rsp + 2], {offset}",
            "lgdt [rsp]",
            "add rsp, 10",
            limit = in(reg) limit,
            offset = in(reg) offset as u64,
        )
    }
}

pub unsafe fn setup_segments() {
    let mut segments = [SegmentDescriptor(0); 3];
    segments[0].0 = 0;
    set_code_segment(&mut segments[1], DescriptorType::ExecuteRead, 0, 0, 0xfffff);
    set_data_segment(&mut segments[2], DescriptorType::ReadWrite, 0, 0, 0xfffff);
    unsafe {
        let ptr = GDT.get();
        write_volatile(ptr, segments);
        load_gdt(ptr as *mut SegmentDescriptor, core::mem::size_of::<GDTType>() as u16);
    }
}

pub unsafe fn set_ds_all(v: u16) {
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

pub unsafe fn set_csss(cs: u16, ss: u16) {
    let csu64 = cs as u64;
    unsafe {
        asm!(
            "mov ss, {ss:x}",
            "lea {tmp}, [2f+rip]",
            "push {cs}",
            "push {tmp}",
            "retfq",
            "2:",
            ss = in(reg) ss,
            tmp = out(reg) _,
            cs = in(reg) csu64,
        )
    }
}

// unsafe extern "C" {
//     pub unsafe fn set_csss(cs: u16, ss: u16);
// }

