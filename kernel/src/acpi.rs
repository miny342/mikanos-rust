use core::ffi::c_void;

#[repr(C, packed)]
struct RSDP {
    signature: [u8; 8],
    check_sum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    _reserved: [u8; 3]
}

impl RSDP {
    fn is_valid(&self) -> bool {
        if self.signature != *b"RSD PTR " {
            false
        } else if self.revision != 2 {
            false
        } else if sum_bytes(self, 20) != 0 {
            false
        } else if sum_bytes(self, 36) != 0 {
            false
        } else {
            true
        }
    }
}

#[repr(C, packed)]
struct DescriptionHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

impl DescriptionHeader {
    unsafe fn is_valid(&self, signature: [u8; 4]) -> bool {
        if self.signature != signature {
            false
        } else if unsafe { sum_bytes_unchecked(&raw const *self as *mut u8, self.length as usize) } != 0 {
            false
        } else {
            true
        }
    }
}

#[repr(C, packed)]
struct XSDT {
    header: DescriptionHeader,
}

impl XSDT {
    unsafe fn from_raw(ptr: *mut u8) -> Option<&'static XSDT> {
        let header = unsafe { &*(ptr as *const DescriptionHeader) };
        if unsafe { header.is_valid(*b"XSDT") } {
            Some(unsafe { &*(ptr as *const XSDT) })
        } else {
            None
        }
    }
    fn entry(&self, idx: usize) -> &'static DescriptionHeader {
        if idx >= self.header.length as usize {
            panic!("idx is large");
        }
        unsafe {
            (self as *const XSDT as *const &'static DescriptionHeader)
                .byte_add(size_of::<DescriptionHeader>())
                .add(idx)
                .read_unaligned()
        }
    }
}

#[repr(C, packed)]
pub struct FADT {
    header: DescriptionHeader,
    _rsvd1: [u8; 76 - size_of::<DescriptionHeader>()],
    pub pm_tmr_blk: u32,
    _rsvd2: [u8; 112 - 80],
    pub flags: u32,
    _rsvd3: [u8; 276 - 116]
}

impl FADT {
    unsafe fn from_raw(header: &'static DescriptionHeader) -> Option<&'static FADT> {
        if unsafe { header.is_valid(*b"FACP") } {
            Some(unsafe { &*(header as *const DescriptionHeader as *const FADT) })
        } else {
            None
        }
    }
}

unsafe fn sum_bytes_unchecked(ptr: *const u8, size: usize) -> u8 {
    let mut sum = 0u8;
    for i in 0..size {
        sum = sum.wrapping_add(unsafe { *ptr.add(i) });
    }
    sum
}

fn sum_bytes<T>(ptr: &T, size: usize) -> u8 {
    if size_of::<T>() < size {
        panic!("sum_bytes invalid size");
    }
    let ptr = (&raw const *ptr) as *const u8;
    unsafe { sum_bytes_unchecked(ptr, size) }
}

// safety: ptr is valid rsdp
pub unsafe fn get_fadt(ptr: *const c_void) -> Option<&'static FADT> {
    let rsdp = unsafe { &*(ptr as *const RSDP) };
    if !rsdp.is_valid() {
        return None
    }

    let Some(xsdt) = (unsafe { XSDT::from_raw(rsdp.xsdt_address as *mut u8) }) else {
        return None
    };

    for i in 0..xsdt.header.length {
        let entry = xsdt.entry(i as usize);
        if let Some(fadt) = unsafe { FADT::from_raw(entry) } {
            return Some(fadt)
        }
    }
    None
}
