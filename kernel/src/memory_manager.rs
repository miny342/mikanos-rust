use spin::Mutex;
use uefi::{boot::PAGE_SIZE, mem::memory_map::MemoryMap};

use crate::make_error;

const KIB: usize = 1024;
const MIB: usize = 1024 * KIB;
const GIB: usize = 1024 * MIB;

pub const BYTES_PER_FRAME: usize = 4 * KIB;

#[derive(Debug, Clone, Copy)]
pub struct FrameID(usize);

impl FrameID {
    pub const fn frame(&self) -> usize {
        self.0 * BYTES_PER_FRAME
    }
    fn add(&self, idx: usize) -> Self {
        FrameID(self.0 + idx)
    }
}

pub const MAX_PHYSICAL_MEMORY_BYTES: usize = 32 * GIB;  // if 128, cannot allocate kernel entry because alloc_map is huge
pub const FRAME_COUNT: usize = MAX_PHYSICAL_MEMORY_BYTES / BYTES_PER_FRAME;

type MapLineType = usize;
pub const BITS_PER_MAPLINE: usize = 8 * core::mem::size_of::<MapLineType>();

pub static MANAGER: Mutex<BitmapMemoryManager> = Mutex::new(BitmapMemoryManager::new()); // todo fix

pub struct BitmapMemoryManager {
    alloc_map: [MapLineType; FRAME_COUNT / BITS_PER_MAPLINE],
    range_begin: FrameID,
    range_end: FrameID,
}

impl BitmapMemoryManager {
    pub const fn new() -> Self {
        BitmapMemoryManager {
            alloc_map: [0; FRAME_COUNT / BITS_PER_MAPLINE],
            range_begin: FrameID(0),
            range_end: FrameID(FRAME_COUNT)
        }
    }
    pub fn set_bit(&mut self, frame: &FrameID, allocated: bool) {
        let line_idx = frame.0 / BITS_PER_MAPLINE;
        let bit_idx = frame.0 % BITS_PER_MAPLINE;

        if allocated {
            self.alloc_map[line_idx] |= 1 << bit_idx;
        } else {
            self.alloc_map[line_idx] &= !(1 << bit_idx);
        }
    }
    pub const fn get_bit(&self, frame: &FrameID) -> bool {
        let line_idx = frame.0 / BITS_PER_MAPLINE;
        let bit_idx = frame.0 % BITS_PER_MAPLINE;

        self.alloc_map[line_idx] & (1 << bit_idx) != 0
    }
    pub fn set_memory_range(&mut self, begin: &FrameID, end: &FrameID) {
        self.range_begin = *begin;
        self.range_end = *end;
    }
    pub fn mark_allocated(&mut self, start: &FrameID, num_frame: usize) {
        for i in 0..num_frame {
            let v = start.add(i);
            self.set_bit(&v, true);
        }
    }
    pub fn allocate(&mut self, num_frame: usize) -> Result<FrameID, crate::error::Error> {
        let mut start = self.range_begin.0;
        loop {
            let mut i = 0;
            while i < num_frame {
                if start + i >= self.range_end.0 {
                    return Err(make_error!(crate::error::Code::NoEnoughMemory))
                }
                if self.get_bit(&FrameID(start + i)) {
                    break;
                }
                i += 1;
            }
            if i == num_frame {
                self.mark_allocated(&FrameID(start), num_frame);
                return Ok(FrameID(start))
            }
            start += i + 1;
        }
    }
    pub fn free(&mut self, start_frame: &FrameID, num_frame: usize) -> Result<(), crate::error::Error> {
        for i in 0..num_frame {
            self.set_bit(&start_frame.add(i), false);
        }
        return Ok(())
    }
}

pub unsafe fn init_memory_manager(memmap_ptr: *const uefi::mem::memory_map::MemoryMapOwned) {
    let memmap = unsafe { &*memmap_ptr };
    let mut memory_manager = MANAGER.lock();
    let mut available_end: usize = 0;
    for desc in memmap.entries() {
        let phys_start = desc.phys_start as usize;
        let page_count = desc.page_count as usize;
        if available_end < phys_start {
            memory_manager.mark_allocated(
                &FrameID(available_end / BYTES_PER_FRAME),
                (phys_start - available_end) / BYTES_PER_FRAME,
            )
        }
        let physical_end = phys_start + page_count * PAGE_SIZE;
        if desc.ty == uefi::mem::memory_map::MemoryType::CONVENTIONAL ||
           desc.ty == uefi::mem::memory_map::MemoryType::BOOT_SERVICES_CODE ||
           desc.ty == uefi::mem::memory_map::MemoryType::BOOT_SERVICES_DATA {
            available_end = physical_end;
        } else {
            memory_manager.mark_allocated(
                &FrameID(phys_start / BYTES_PER_FRAME),
                page_count * PAGE_SIZE / BYTES_PER_FRAME
            )
        }
    }
    memory_manager.set_memory_range(&FrameID(1), &FrameID(available_end / BYTES_PER_FRAME));
}

pub fn page_allocate(num_frame: usize) -> Result<FrameID, crate::error::Error> {
    MANAGER.lock().allocate(num_frame)
}
