
pub struct MemoryMap {
    pub ptr: *const MemoryDescriptor,
    pub size: usize,
}

pub struct MemoryDescriptor {
    pub ty: usize,
    pub phys_start: usize,
    pub virt_start: usize,
    pub page_count: usize,
    pub attr: usize,
}

impl MemoryDescriptor {
    pub fn is_available(&self) -> bool {
        self.ty == 3 || self.ty == 4 || self.ty == 7
    }
}

pub const UEFI_PAGE_SIZE: usize = 4096;


