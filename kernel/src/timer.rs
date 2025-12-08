use core::{cmp::{Ordering, Reverse}, future::Pending, sync::atomic::{AtomicBool, AtomicUsize}};
use alloc::{collections::BinaryHeap, sync::Arc};
use futures_util::task::AtomicWaker;
use spin::Mutex;

const COUNT_MAX: u32 = 0xffffffff;
const LVT_TIMER: *mut u32 = 0xfee00320 as *mut u32;
const INITIAL_COUNT: *mut u32 = 0xfee00380 as *mut u32;
const CURRENT_COUNT: *mut u32 = 0xfee00390 as *mut u32;
const DIVIDE_CONFIGURATION: *mut u32 = 0xfee003e0 as *mut u32;

const TIMER_FREQ: u32 = 100;

static TICK: AtomicUsize = AtomicUsize::new(0);
static PRIORITY_QUEUE: Mutex<BinaryHeap<Timer>> = Mutex::new(BinaryHeap::new());
static TIMER_MANAGER_WAKER: AtomicWaker = AtomicWaker::new();

struct TimerInner {
    timeout: Reverse<usize>,
    value: usize,
    waker: AtomicWaker,
}

impl TimerInner {
    fn new(timeout: usize, value: usize) -> TimerInner {
        TimerInner { timeout: Reverse(timeout), value, waker: AtomicWaker::new() }
    }
    fn timeout(&self) -> usize {
        self.timeout.0
    }
    fn value(&self) -> usize {
        self.value
    }
    fn waker(&self) -> &AtomicWaker {
        &self.waker
    }
}

impl PartialEq for TimerInner {
    fn eq(&self, other: &Self) -> bool {
        self.timeout == other.timeout
    }
}

impl Eq for TimerInner {}

impl PartialOrd for TimerInner {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.timeout.cmp(&other.timeout))
    }
}

impl Ord for TimerInner {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timeout.cmp(&other.timeout)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Timer {
    inner: Arc<TimerInner>,
}

impl Timer {
    pub fn new(timeout: usize, value: usize) -> Timer {
        Timer { inner: Arc::new(TimerInner::new(timeout, value)) }
    }
    fn add(&self) {
        let mut lck = PRIORITY_QUEUE.lock();
        lck.push(Timer { inner: Arc::clone(&self.inner) });
    }
    // pub fn timeout(&self) -> usize {
    //     self.timeout.0
    // }
    // pub fn value(&self) -> usize {
    //     self.value
    // }
    // pub fn waker(&self) -> &AtomicWaker {
    //     &self.waker
    // }
}


impl Future for Timer {
    type Output = usize;
    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> core::task::Poll<Self::Output> {
        let tick = get_tick();
        if tick > self.inner.timeout() {
            core::task::Poll::Ready(self.inner.value())
        } else {
            self.inner.waker().register(cx.waker());
            self.add();

            if get_tick() > self.inner.timeout() {
                self.inner.waker().take();
                core::task::Poll::Ready(self.inner.value())
            } else {
                core::task::Poll::Pending
            }
        }
    }
}

#[derive(Clone, Copy)]
struct TimerManager;

impl Future for TimerManager {
    type Output = ();
    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> core::task::Poll<Self::Output> {
        let tick = get_tick();
        // ロックはとれるなら取る
        if let Some(mut lck) = PRIORITY_QUEUE.try_lock() {
            while let Some(t) = lck.peek() {
                if t.inner.timeout() > tick {
                    break;
                }
                t.inner.waker().wake();
                lck.pop();
            }
        }
        TIMER_MANAGER_WAKER.register(cx.waker());
        core::task::Poll::Pending
    }
}

pub async fn timer_manager() {
    TimerManager.await
}

pub fn initialize_apic_timer(fadt: Option<&'static crate::acpi::FADT>) {
    unsafe {
        if let Some(fadt) = fadt {
            *DIVIDE_CONFIGURATION = 0b1011;
            *LVT_TIMER = 0b001 << 16;

            start_lapic_timer();
            fadt.wait_milliseconds(100);
            let elapsed = lapic_timer_elapsed();
            stop_lapic_timer();

            let lapic_timer_freq = elapsed * 10; // 1s

            *DIVIDE_CONFIGURATION = 0b1011;
            *LVT_TIMER = (0b010 << 16) | crate::interrupt::InterruptVector::LAPICTimer as u32;
            *INITIAL_COUNT = lapic_timer_freq / TIMER_FREQ;
        } else {
            *DIVIDE_CONFIGURATION = 0b1011;
            *LVT_TIMER = (0b010 << 16) | crate::interrupt::InterruptVector::LAPICTimer as u32;
            *INITIAL_COUNT = 0x1000000;
        }
    }
}

fn start_lapic_timer() {
    unsafe {
        *INITIAL_COUNT = COUNT_MAX;
    }
}

fn lapic_timer_elapsed() -> u32 {
    unsafe {
        COUNT_MAX - *CURRENT_COUNT
    }
}

fn stop_lapic_timer() {
    unsafe {
        *INITIAL_COUNT = 0;
    }
}

pub fn get_tick() -> usize {
    TICK.load(core::sync::atomic::Ordering::Relaxed)
}

pub extern "x86-interrupt" fn int_handler_lapic_timer(_frame: crate::interrupt::InterruptFrame) {
    TICK.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    TIMER_MANAGER_WAKER.wake();
    crate::interrupt::notify_end_of_interrupt();
}

mod test {
    #[test_case]
    fn timer_ord_test() {
        use super::Timer;
        let t1 = Timer::new(1, 2);
        let t2 = Timer::new(2, 3);
        let t3 = Timer::new(1, 3);
        assert!(t1 == t3);
        assert!(t2 < t1);
        assert!(t1 > t2);
        assert!(t1 == t1);
        assert!(t3 > t2);
    }
    #[test_case]
    fn timer_binary_heap() {
        use super::Timer;
        use alloc::collections::BinaryHeap;
        let mut heap = BinaryHeap::new();
        heap.push(Timer::new(1, 2));
        heap.push(Timer::new(2, 3));
        heap.push(Timer::new(1, 3));
        let v = heap.pop().unwrap();
        assert!(v.inner.timeout() == 1);
        let v2 = heap.pop().unwrap();
        assert!(v2.inner.timeout() == 1);
        let v3 = heap.pop().unwrap();
        assert!(v3.inner.timeout() == 2);
    }
}
