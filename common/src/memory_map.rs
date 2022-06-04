
pub struct MemoryMap {
    pub ptr: *const MemoryDescriptor,
    pub size: usize,
}

pub struct MemoryDescriptor {
    pub ty: u32,
    pub phys_start: u64,
    pub virt_start: u64,
    pub page_count: u64,
    pub attr: u64,
}

