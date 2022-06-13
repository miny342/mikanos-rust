const COUNT_MAX: u32 = 0xffffffff;
const LVT_TIMER: *mut u32 = 0xfee00320 as *mut u32;
const INITIAL_COUNT: *mut u32 = 0xfee00380 as *mut u32;
const CURRENT_COUNT: *mut u32 = 0xfee00390 as *mut u32;
const DIVIDE_CONFIGURATION: *mut u32 = 0xfee003e0 as *mut u32;

pub fn initialize_apic_timer() {
    unsafe {
        *DIVIDE_CONFIGURATION = 0b1011;
        *LVT_TIMER = (0b001 << 16) | 32;
    }
}

pub fn start_lapic_timer() {
    unsafe {
        *INITIAL_COUNT = COUNT_MAX;
    }
}

pub fn lapic_timer_elapsed() -> u32 {
    unsafe {
        COUNT_MAX - *CURRENT_COUNT
    }
}

pub fn stop_lapic_timer() {
    unsafe {
        *INITIAL_COUNT = 0;
    }
}
