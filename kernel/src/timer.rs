use core::sync::atomic::AtomicUsize;

const COUNT_MAX: u32 = 0xffffffff;
const LVT_TIMER: *mut u32 = 0xfee00320 as *mut u32;
const INITIAL_COUNT: *mut u32 = 0xfee00380 as *mut u32;
const CURRENT_COUNT: *mut u32 = 0xfee00390 as *mut u32;
const DIVIDE_CONFIGURATION: *mut u32 = 0xfee003e0 as *mut u32;

static TICK: AtomicUsize = AtomicUsize::new(0);

pub fn initialize_apic_timer() {
    unsafe {
        *DIVIDE_CONFIGURATION = 0b1011;
        // *LVT_TIMER = (0b001 << 16) | 32;
        *LVT_TIMER = (0b010 << 16) | crate::interrupt::InterruptVector::LAPICTimer as u32;
        *INITIAL_COUNT = 0x1000000;
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

pub static TIMER_WAKER: futures_util::task::AtomicWaker = futures_util::task::AtomicWaker::new();

pub fn get_tick() -> usize {
    TICK.load(core::sync::atomic::Ordering::Relaxed)
}

pub extern "x86-interrupt" fn int_handler_lapic_timer(_frame: crate::interrupt::InterruptFrame) {
    TICK.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    TIMER_WAKER.wake();
    crate::interrupt::notify_end_of_interrupt();
}

static mut SUM: usize = 0;
static mut SUM_: usize = 0;

fn check_time<T: Fn() -> ()>(f: T) -> (u32, usize) {
    start_lapic_timer();
    f();
    let elapsed = lapic_timer_elapsed();
    stop_lapic_timer();
    unsafe {
        SUM += elapsed as usize;
        SUM_ += 1;
    }
    (elapsed, unsafe { SUM / SUM_ })
}
