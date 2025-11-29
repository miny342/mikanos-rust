use core::{arch::asm, cell::SyncUnsafeCell};

const PAGE_DIRECTORY_COUNT: usize = 64;

const PAGE_SIZE_4K: u64 = 4096;
const PAGE_SIZE_2M: u64 = 512 * PAGE_SIZE_4K;
const PAGE_SIZE_1G: u64 = 512 * PAGE_SIZE_2M;

#[repr(align(4096))]
struct AlignedArray([u64; 512]);

#[repr(align(4096))]
struct PageDir([[u64; 512]; PAGE_DIRECTORY_COUNT]);

static PML4_TABLE: SyncUnsafeCell<AlignedArray> = SyncUnsafeCell::new(AlignedArray([0; 512]));
static PDP_TABLE: SyncUnsafeCell<AlignedArray> = SyncUnsafeCell::new(AlignedArray([0; 512]));
static PAGE_DIRECTORY:  SyncUnsafeCell<PageDir> = SyncUnsafeCell::new(PageDir([[0; 512]; PAGE_DIRECTORY_COUNT]));

pub unsafe fn setup_identity_page_table() {
    let pml4 = PML4_TABLE.get();
    let pdp = PDP_TABLE.get();
    let page_directory = PAGE_DIRECTORY.get();

    unsafe {
        (*pml4).0[0] = (pdp as u64) | 0x3;
        for i_pdpt in 0..PAGE_DIRECTORY_COUNT {
            (*pdp).0[i_pdpt] = (&raw mut (*page_directory).0[i_pdpt] as u64) | 0x3;
            for i_pd in 0..512 {
                (*page_directory).0[i_pdpt][i_pd] = i_pdpt as u64 * PAGE_SIZE_1G + i_pd as u64 * PAGE_SIZE_2M | 0x83;
            }
        }
        set_cr3(pml4 as u64);
    }
}

unsafe fn set_cr3(value: u64) {
    unsafe {
        asm!(
            "mov cr3, {}",
            in(reg) value,
        )
    }
}
