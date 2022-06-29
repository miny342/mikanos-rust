use core::{alloc::GlobalAlloc, ptr::null_mut};

use crate::{allocator::{LinkedListAllocator, LIST_SIZE, List}, debug};

impl LinkedListAllocator {
    pub fn alloc_with_boundary_zeroed(&self, size: usize, align: usize, boundary: usize) -> *mut u8 {
        unsafe {
            let ptr = self.alloc_with_boundary(size, align, boundary);
            ptr.write_bytes(0, size);
            ptr
        }
    }
    pub unsafe fn alloc_with_boundary(&self, size: usize, align: usize, boundary: usize) -> *mut u8 {
        assert!(align.is_power_of_two());
        assert!(boundary == 0 || (boundary.is_power_of_two() && size <= boundary && align <= boundary));
        let ptr = self.alloc_with_boundary_unchecked(size, align, boundary);
        if ptr.is_null() {
            panic!("oom");
        }
        let us = ptr as usize;
        assert!(boundary == 0 || us / boundary == (us + size) / boundary);
        assert!(us % align == 0);
        ptr
    }
    // safety: align must be 2 ^ n and boundary must be 0 or 2 ^ n and size <= boundary and align <= boundary
    pub unsafe fn alloc_with_boundary_unchecked(&self, size: usize, align: usize, boundary: usize) -> *mut u8 {
        let size = if size < LIST_SIZE { LIST_SIZE } else { size };
        if boundary == 0 {
            return self.alloc(core::alloc::Layout::from_size_align_unchecked(size, align))
        }
        debug!("usballocate {}, {}, {}", align, size, boundary);

        let mut lock = self.center.lock();

        if lock.is_null() {
            return null_mut()
        }

        let mut list = lock.clone();
        let mut prev = null_mut::<List>();

        loop {
            debug!("usblist: {:p}, {:?}", list, *list);
            let head = list as usize;
            let end = (*list).size + head;

            let res = head % align;

            let next_boundary = ((head + boundary - 1) / boundary) * boundary;

            if res == 0 && head + size <= next_boundary {
                if head + size == end {
                    if prev.is_null() {
                        *lock = (*list).next;
                    } else {
                        (*prev).next = (*list).next;
                    }
                    return head as *mut u8;
                }
                if head + size + LIST_SIZE <= end {
                    let new = (head + size) as *mut List;
                    (*new).next = (*list).next;
                    (*new).size = (*list).size - size;
                    if prev.is_null() {
                        *lock = new;
                    } else {
                        (*prev).next = new;
                    }
                    return head as *mut u8;
                }
            } else {
                let start = {
                    if head + size <= next_boundary {
                        let mut start = if align - res >= LIST_SIZE { head + align - res } else { head + align - res + align };
                        if start + size > next_boundary {
                            start = next_boundary;
                        }
                        start
                    } else {
                        next_boundary
                    }
                };
                if start + size == end {
                    (*list).size -= size;
                    return start as *mut u8;
                }
                if start + size + LIST_SIZE <= end {
                    let new = (start + size) as *mut List;
                    (*new).next = (*list).next;
                    (*new).size = end - (start + size);
                    (*list).next = new;
                    (*list).size = start - head;
                    return start as *mut u8;
                }
            }


            prev = list;
            list = (*list).next;
            if list.is_null() {
                return null_mut();
            }
        }
    }
}

