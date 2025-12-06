use core::ptr::null_mut;
use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicPtr, Ordering};

use spin::Mutex;

use log::debug;

use crate::memory_manager::BYTES_PER_FRAME;

#[derive(Debug)]
pub struct List {
    pub size: usize,
    pub next: *mut List,
}

pub const LIST_SIZE: usize = core::mem::size_of::<List>();

pub struct LinkedListAllocator {
    pub center: Mutex<*mut List>,
}

impl LinkedListAllocator {
    pub const fn empty() -> Self {
        LinkedListAllocator { center: Mutex::new(null_mut()) }
    }
    pub unsafe fn init(&self, head: usize, end: usize) {
        *self.center.lock() = head as *mut List;
        let li = unsafe { &mut *(head as *mut List) };
        li.next = null_mut();
        li.size = end - head;
    }
}

unsafe impl Sync for LinkedListAllocator {}

unsafe impl GlobalAlloc for LinkedListAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut lock = self.center.lock();
        let align = layout.align();
        let size = if layout.size() < LIST_SIZE { LIST_SIZE } else { layout.size() };
        debug!("allocate {}, {}", align, size);

        let mut list = lock.clone();
        let mut prev = null_mut::<List>();

        if list.is_null() {
            return null_mut();
        }

        unsafe {
            loop {
                debug!("list: {:p}, {:?}", list, *list);
                let head = list as usize;
                let end = (*list).size + head;

                let res = head % align;
                if res == 0 && head + size == end {
                    if prev.is_null() {
                        *lock = (*list).next;
                    } else {
                        (*prev).next = (*list).next;
                    }
                    return head as *mut u8;
                }
                if res == 0 && head + size + LIST_SIZE <= end {
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
                if res != 0 {
                    let start = if align - res >= LIST_SIZE { head + align - res } else { head + align - res + align };
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
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut lock = self.center.lock();
        let size = if layout.size() < LIST_SIZE { LIST_SIZE } else { layout.size() };
        // println!("deallocate: {:p}, {}, {}", ptr, layout.size(), LIST_SIZE);

        let ptr = ptr as *mut List;
        let mut list = lock.clone();
        let mut prev = null_mut::<List>();
        // println!("list: {:p}, {:?}", list, *list);

        unsafe {
            if (ptr as usize) < (list as usize) {
                if (ptr as usize) + size == (list as usize) {
                    *lock = ptr;
                    (*ptr).next = (*list).next;
                    (*ptr).size = (*list).size + size;
                    return;
                } else {
                    *lock = ptr;
                    (*ptr).next = list;
                    (*ptr).size = size;
                    return;
                }
            }

            while !list.is_null() && (ptr as usize) > (list as usize) {
                prev = list;
                list = (*list).next;
            }

            if list.is_null() {
                if (prev as usize) + (*prev).size == (ptr as usize) {
                    (*prev).size += size;
                    return;
                } else {
                    (*ptr).next = (*prev).next;
                    (*ptr).size = size;
                    (*prev).next = ptr;
                    return;
                }
            }

            (*ptr).next = list;
            (*ptr).size = size;
            if (ptr as usize) + size == (list as usize) {
                (*ptr).next = (*list).next;
                (*ptr).size = (*list).size + size;
            }
            if (prev as usize) + (*prev).size == (ptr as usize) {
                (*prev).next = (*ptr).next;
                (*prev).size += (*ptr).size;
            }
        }
    }
}

pub struct SimplestAllocator {
    pub head: AtomicPtr<u8>,
    pub end: AtomicPtr<u8>,
}

impl SimplestAllocator {
    pub const fn empty() -> Self {
        SimplestAllocator {
            head: AtomicPtr::new(null_mut()),
            end: AtomicPtr::new(null_mut())
        }
    }
    pub unsafe fn init(&self, head: *mut u8, end: *mut u8) {
        self.head.store(head, Ordering::Relaxed);
        self.end.store(end, Ordering::Relaxed);
    }
}

unsafe impl GlobalAlloc for SimplestAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        let mask = align - 1;

        let mut p;
        while {
            let h = self.head.load(Ordering::Relaxed);
            p = if h.addr() & mask != 0 {
                h.map_addr(|u| (u & !mask) + align)
            } else {
                h
            };
            self.head.compare_exchange_weak(h, p.map_addr(|u| u + size), Ordering::Relaxed, Ordering::Relaxed).is_err()
        } {};
        if p.addr() + size < self.end.load(Ordering::Relaxed).addr() {
            p
        } else {
            null_mut()
        }
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {

    }
}

#[global_allocator]
static ALLOCATOR: SimplestAllocator = SimplestAllocator::empty();

pub unsafe fn init_allocator() {
    let heap_frame = 64 * 512;
    let heap_start = crate::memory_manager::page_allocate(heap_frame).expect("cannot initialize heap allocate");
    let start = heap_start.frame();
    let end = start + heap_frame * BYTES_PER_FRAME;
    unsafe { ALLOCATOR.init(start as *mut u8, end as *mut u8) };
}
