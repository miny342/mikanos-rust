use core::arch::asm;

const PAGE_DIRECTORY_COUNT: usize = 64;

const PAGE_SIZE_4K: u64 = 4096;
const PAGE_SIZE_2M: u64 = 512 * PAGE_SIZE_4K;
const PAGE_SIZE_1G: u64 = 512 * PAGE_SIZE_2M;

#[repr(align(4096))]
struct AlignedArray([u64; 512]);

#[repr(align(4096))]
struct PageDir([[u64; 512]; PAGE_DIRECTORY_COUNT]);

static mut PML4_TABLE: AlignedArray = AlignedArray([0; 512]);
static mut PDP_TABLE: AlignedArray = AlignedArray([0; 512]);
static mut PAGE_DIRECTORY: PageDir = PageDir([[0; 512]; PAGE_DIRECTORY_COUNT]);

pub unsafe fn setup_identity_page_table() {
    *PML4_TABLE.0.get_unchecked_mut(0) = (PDP_TABLE.0.get_unchecked(0) as *const _ as u64) | 0x3;
    for i_pdpt in 0..PAGE_DIRECTORY.0.len() {
        *PDP_TABLE.0.get_unchecked_mut(i_pdpt) = (PAGE_DIRECTORY.0.get_unchecked(i_pdpt) as *const _ as u64) | 0x3;
        for i_pd in 0..512 {
            *PAGE_DIRECTORY.0.get_unchecked_mut(i_pdpt).get_unchecked_mut(i_pd) = i_pdpt as u64 * PAGE_SIZE_1G + i_pd as u64 * PAGE_SIZE_2M | 0x83;
        }
    }
    set_cr3(PML4_TABLE.0.get_unchecked(0) as *const _ as u64);
}

unsafe fn set_cr3(value: u64) {
    asm!(
        "mov cr3, {}",
        in(reg) value,
    )
}
