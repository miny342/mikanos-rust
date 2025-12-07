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

fn sum_bytes<T>(ptr: &T, size: usize) -> u8 {
    if size_of::<T>() < size {
        panic!("sum_bytes invalid size");
    }
    let ptr = (&raw const *ptr) as *const u8;
    let mut sum = 0u8;
    for i in 0..size {
        sum = sum.wrapping_add(unsafe { *ptr.add(i) });
    }
    sum
}

pub unsafe fn acpi_init(ptr: *const c_void) {
    let rsdp = unsafe { &*(ptr as *const RSDP) };
    if !rsdp.is_valid() {
        panic!("rsdp is invalid");
    }
}
