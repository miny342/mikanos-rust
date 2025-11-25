use core::alloc::Layout;
use core::intrinsics::transmute;
use core::mem::{MaybeUninit, size_of};
use core::ptr::{slice_from_raw_parts_mut, write_volatile};
use core::slice::from_raw_parts_mut;
use core::sync::atomic::{AtomicU8, Ordering};
use core::task::Poll;

use alloc::boxed::Box;
use alloc::vec::Vec;
use conquer_once::spin::OnceCell;
use futures_util::{Stream, StreamExt};
use futures_util::task::AtomicWaker;
use spin::Mutex;
use volatile_register::{RW, RO, WO};
use heapless::FnvIndexMap;

use crate::allocator::{LinkedListAllocator, SimplestAllocator};
use crate::{println, debug, make_error, error, print};
use crate::pci::Device;
use crate::error::*;

// use self::usb_allocator::SimpleAllocator;

mod usb_allocator;
mod registers;

// static ALLOCATOR: LinkedListAllocator = LinkedListAllocator::empty();

type HandleError<T> = Result<T, Error>;


const MAX_SLOTS_EN: u8 = 8;

type DcbaaTy = [u64; MAX_SLOTS_EN as usize + 1];

#[repr(align(64))]
struct DCBAA([u64; MAX_SLOTS_EN as usize + 1]);
// struct MemPool([u64; max_slots_en as usize + 1]);

// static DCBAA: Mutex<MemPool> = Mutex::new(MemPool { x: [0; max_slots_en as usize + 1] });

#[derive(Debug, Clone, Copy)]
struct ClassDriver {
    class: u16,
    sub_class: u16,
    protocol: u16,
    interface: u16,
}

#[derive(Debug, Clone, Copy)]
struct SlotContext {
    data: [u32; 8]
}

impl SlotContext {
    fn root_hub_port_num(&self) -> u8 {
        (self.data[1] >> 16 & 0xff) as u8
    }
}

#[derive(Debug, Clone, Copy)]
struct EndpointContext {
    data: [u32; 8]
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
struct DeviceContext {
    slot_ctx: SlotContext,
    ep_ctx: [EndpointContext; 31]
}

#[derive(Debug, Clone, Copy)]
struct InputControlContext {
    data: [u32; 8]
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
struct InputContext {
    input_control_ctx: InputControlContext,
    slot_ctx: SlotContext,
    ep_ctx: [EndpointContext; 31]
}

#[derive(Debug, Clone, Copy)]
struct SlotContext64 {
    data: [u32; 16]
}

impl SlotContext64 {
    fn root_hub_port_num(&self) -> u8 {
        (self.data[1] >> 16 & 0xff) as u8
    }
}

#[derive(Debug, Clone, Copy)]
struct EndpointContext64 {
    data: [u32; 16]
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
struct DeviceContext64 {
    slot_ctx: SlotContext64,
    ep_ctx: [EndpointContext64; 31]
}

#[derive(Debug, Clone, Copy)]
struct InputControlContext64 {
    data: [u32; 16]
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
struct InputContext64 {
    input_control_ctx: InputControlContext64,
    slot_ctx: SlotContext64,
    ep_ctx: [EndpointContext64; 31]
}

enum DeviceContextEnum {
    V1(&'static DeviceContext),
    V2(&'static DeviceContext64)
}

impl DeviceContextEnum {
    fn root_hub_port_num(&self) -> u8 {
        match self {
            DeviceContextEnum::V1(x) => x.slot_ctx.root_hub_port_num(),
            DeviceContextEnum::V2(x) => x.slot_ctx.root_hub_port_num(),
        }
    }
    fn slot_ctx(&self) -> &[u32] {
        match self {
            DeviceContextEnum::V1(x) => &x.slot_ctx.data,
            DeviceContextEnum::V2(x) => &x.slot_ctx.data,
        }
    }
    fn as_inner_ptr(&self) -> u64 {
        match self {
            DeviceContextEnum::V1(x) => *x as *const DeviceContext as u64,
            DeviceContextEnum::V2(x) => *x as *const DeviceContext64 as u64,
        }
    }
}

enum InputContextEnum {
    V1(&'static mut InputContext),
    V2(&'static mut InputContext64),
}

impl InputContextEnum {
    fn input_control_ctx(&mut self) -> &mut [u32] {
        match self {
            InputContextEnum::V1(x) => &mut x.input_control_ctx.data,
            InputContextEnum::V2(x) => &mut x.input_control_ctx.data,
        }
    }
    fn slot_ctx(&mut self) -> &mut [u32] {
        match self {
            InputContextEnum::V1(x) => &mut x.slot_ctx.data,
            InputContextEnum::V2(x) => &mut x.slot_ctx.data,
        }
    }
    fn ep_ctx(&mut self, idx: usize) -> &mut [u32] {
        match self {
            InputContextEnum::V1(x) => &mut x.ep_ctx[idx].data,
            InputContextEnum::V2(x) => &mut x.ep_ctx[idx].data,
        }
    }
    fn as_inner_ptr(&self) -> u64 {
        match self {
            InputContextEnum::V1(x) => *x as *const InputContext as u64,
            InputContextEnum::V2(x) => *x as *const InputContext64 as u64,
        }
    }
}

struct XhciDevice {
    device_ctx: DeviceContextEnum,
    input_ctx: InputContextEnum,
    slot_id: u8,
    buf: [u8; 512],
    doorbell: u64,
    num_configuration: u8,
    max_packet_size: u16,
    classes: [ClassDriver; 15],
    default: usize,  // default class driver (boot protocol)
    transfer_rings: [MemPoolTrTRB; 31],
}

impl XhciDevice {
    fn doorbell(&self) -> &'static mut registers::DoorbellRegister {
        if self.doorbell == 0 {
            panic!("doorbell is not initialized")
        }
        unsafe { &mut *(self.doorbell as *mut registers::DoorbellRegister) }
    }
    fn start_init(&mut self) {
        self.get_descriptor(1, 0);
    }
    fn get_descriptor(&mut self, ty: u8, num: u8) {
        let setup_trb = TRB {
            data: [
                0b10000000 | 6 << 8 | (ty as u32) << 24 | (num as u32) << 16,
                (self.buf.len() as u32) << 16,
                8,
                2 << 10 | 3 << 16 | 1 << 6
            ]
        };
        let ptr = self.buf.as_ptr() as u64;
        let data_trb = TRB {
            data: [
                (ptr & 0xffffffff) as u32,
                (ptr >> 32) as u32,
                (self.buf.len() as u32),
                1 << 16 | 3 << 10,
            ]
        };
        let status_trb = TRB {
            data: [
                0, 0, 0, 4 << 10 | 1 << 5
            ]
        };

        let ptr = &mut self.transfer_rings[0];
        ptr.push(setup_trb);
        ptr.push(data_trb);
        SETUP_TRB_MAP.lock().insert(ptr.center() as *const TRB as u64, setup_trb).unwrap();
        ptr.push(status_trb);

        self.doorbell().ring(1, 0);
    }
    fn set_protocol_boot(&mut self) {
        let driver = match self.classes.iter().enumerate().filter(|(_x, y)| y.class == 3 && y.sub_class == 1).next() {
            Some(x) => x,
            None => return,
        };
        self.default = driver.0;
        let setup_trb = TRB {
            data: [
                0b00100001 | 11 << 8,
                driver.1.interface as u32,
                8,
                2 << 10 | 1 << 6
            ]
        };
        let ptr = self.buf.as_ptr() as u64;
        let data_trb = TRB {
            data: [
                (ptr & 0xffffffff) as u32,
                (ptr >> 32) as u32,
                (self.buf.len() as u32),
                3 << 10,
            ]
        };
        let status_trb = TRB {
            data: [
                0, 0, 0, 4 << 10 | 1 << 5 | 1 << 16
            ]
        };

        let ptr = &mut self.transfer_rings[0];
        ptr.push(setup_trb);
        ptr.push(data_trb);
        SETUP_TRB_MAP.lock().insert(ptr.center() as *const TRB as u64, setup_trb).unwrap();
        ptr.push(status_trb);

        self.doorbell().ring(1, 0);
    }
}

type DeviceMemType = [Mutex<Option<XhciDevice>>; (MAX_SLOTS_EN + 1) as usize];

const unsafe fn device_mem_init() -> DeviceMemType {
    let mut arr = core::mem::MaybeUninit::<DeviceMemType>::uninit().assume_init();
    let mut outer = 0;
    while outer < (MAX_SLOTS_EN + 1) as usize {
        arr[outer] = Mutex::new(None);
        outer += 1;
    }
    arr
}

static DEVICES_MEM: DeviceMemType = unsafe { device_mem_init() } ;

trait TRBtrait {
    const TY: u32;
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct TRB {
    data: [u32; 4]
}

impl TRBtrait for TRB {
    const TY: u32 = 0;
}

impl TRB {
    pub fn ty(&self) -> u32 {
        (self.data[3] >> 10) & 0x3f
    }
    pub fn cast<T: TRBtrait>(&self) -> Option<&T> {
        if self.ty() == T::TY {
            unsafe { Some(transmute::<&Self, &T>(self)) }
        } else {
            None
        }
    }
    pub fn cycle(&self) -> bool {
        self.data[3] & 0x1 != 0
    }
    pub fn new_enable_slot_trb() -> TRB {
        TRB {
            data: [
                0, 0, 0, 9 << 10
            ]
        }
    }
    pub fn new_link_trb(ptr: u64) -> TRB {
        TRB {
            data: [
                (ptr & 0xfffffff0) as u32, (ptr >> 32) as u32, 0, 6 << 10 | 1 << 1
            ]
        }
    }
    pub fn address_device_command_trb(input_context_ptr: u64, slot_id: u8) -> TRB {
        let ptr = input_context_ptr as u64;
        assert!(ptr & 0x3f == 0);
        TRB {
            data: [
                (ptr & 0xfffffff0) as u32, (ptr >> 32) as u32, 0, ((slot_id as u32) << 24) | 11 << 10
            ]
        }
    }
    pub fn no_op_command_trb() -> TRB {
        TRB {
            data: [
                0, 0, 0, 23 << 10
            ]
        }
    }
}

struct PortStatusChangeEventTRB {
    data: [u32; 4]
}

impl TRBtrait for PortStatusChangeEventTRB {
    const TY: u32 = 34;
}

impl PortStatusChangeEventTRB {
    fn port_id(&self) -> u8 {
        (self.data[0] >> 24) as u8
    }
    pub fn on_event(&self, xhc: &mut XhcController) {
        let id = self.port_id();
        let port = xhc.capability.port_sc(id);
        match xhc.port_config_phase[id as usize] {
            ConfigPhase::NotConnected => {
                if port.is_connected() {
                    xhc.reset_port(id);
                }
            },
            ConfigPhase::ResettingPort => {
                xhc.enable_slot(id);
            },
            _ => {
                error!("{:?} {}", xhc.port_config_phase[id as usize], make_error!(Code::InvalidPhase));
            }
        }
    }
}

#[repr(C)]
struct CommandCompletionEventTRB {
    trb_ptr: u64,
    data: [u32; 2]
}

impl TRBtrait for CommandCompletionEventTRB {
    const TY: u32 = 33;
}

impl CommandCompletionEventTRB {
    fn ptr(&self) -> &TRB {
        unsafe { &*((self.trb_ptr & !0xf) as *const TRB) }
    }
    fn slot_id(&self) -> u8 {
        (self.data[1] >> 24) as u8
    }
    pub fn on_event(&self, xhc: &mut XhcController) {
        let ty = self.ptr().ty();
        debug!("cce-ty:{}", ty);
        if ty == 9 { // enable slot command
            if xhc.port_config_phase[xhc.addressing_port as usize] != ConfigPhase::EnablingSlot {
                panic!()
            }
            xhc.address_deivce(self.slot_id(), xhc.addressing_port);
        } else if ty == 11 { // address device command
            let mut lock = DEVICES_MEM[self.slot_id() as usize].lock();
            let dev = lock.as_mut().unwrap();
            let port_id = dev.device_ctx.root_hub_port_num();
            if port_id != xhc.addressing_port || xhc.port_config_phase[port_id as usize] != ConfigPhase::AddressingDevice {
                panic!()
            }

            xhc.addressing_port = 0;
            if let Some(next_port_id) = xhc.port_config_phase.iter().enumerate().filter(|x| *x.1 == ConfigPhase::WaitingAddressed).next() {
                let port = xhc.capability.port_sc(next_port_id.0 as u8);
                if port.is_connected() {
                    xhc.reset_port(next_port_id.0 as u8)
                }
            }

            xhc.port_config_phase[port_id as usize] = ConfigPhase::InitializingDevice;
            dev.start_init();
        } else if ty == 12 { // configure endpoint command
            let dev = &mut DEVICES_MEM[self.slot_id() as usize].lock();
            dev.as_mut().unwrap().set_protocol_boot();
        } else {
            error!("{}", make_error!(Code::NotImplemented))
        }
    }
}

static SETUP_TRB_MAP: Mutex<FnvIndexMap<u64, TRB, 32>> = Mutex::new(FnvIndexMap::new());

#[repr(C)]
struct TransferEventTRB {
    trb_ptr: u64,
    data: [u32; 2]
}

impl TRBtrait for TransferEventTRB {
    const TY: u32 = 32;
}

impl TransferEventTRB {
    fn ptr(&self) -> &TRB {
        unsafe { &*(self.trb_ptr as *const TRB) }
    }
    fn slot_id(&self) -> u8 {
        (self.data[1] >> 24) as u8
    }
    fn set_normal_trb(&self, dev: &mut XhciDevice) {
        let dci = (dev.default + 1) * 2 + 1;  // default driver interrupt in
        let ptr = dev.buf.as_ptr() as u64;
        dev.transfer_rings[dci - 1].push(TRB {
            data: [
                (ptr & 0xffffffff) as u32,
                (ptr >> 32) as u32,
                (dev.buf.len() as u32) | 0 << 22,
                1 << 10 | 1 << 5
            ]
        });
        dev.doorbell().ring(dci as u8, 0);
    }
    pub fn on_event(&self, xhc: &mut XhcController) {
        let mut lock = DEVICES_MEM[self.slot_id() as usize].lock();
        let dev = lock.as_mut().unwrap();
        // debug!("transfer: {:?}", DEVICES_MEM[self.slot_id() as usize].buf);
        // debug!("data: {:?}", self.ptr().data);
        let trb = match SETUP_TRB_MAP.lock().remove(&self.trb_ptr) {
            Some(x) => x,
            None => {
                if self.ptr().data[3] >> 10 & 0x3f == 1 {
                    if dev.classes[dev.default].protocol == 2 { // mouse
                        // print!("transfer: ");
                        // for i in 0..10 {
                        //     print!("{:0>2x},", dev.buf[i]);
                        // }
                        // println!("");
                        (xhc.mouse_handler)(dev.buf[0], dev.buf[1] as i8, dev.buf[2] as i8)
                    } else if dev.classes[dev.default].protocol == 1 {  // keyboard
                        // static PREV: Mutex<[u8; 8]> = Mutex::new([0; 8]);
                        // static PRESSING: AtomicU8 = AtomicU8::new(0);
                        // let mut prev = PREV.lock();
                        // let pressed = {
                        //     let mut v = 0u8;
                        //     for k in &dev.buf[2..8] {
                        //         if *k == 0 || prev[2..].iter().any(|p| *p == *k) {
                        //             continue;
                        //         }
                        //         v = *k;
                        //     };
                        //     v
                        // };
                        // let released = {
                        //     let mut v = 0u8;
                        //     for k in &prev[2..] {
                        //         if *k == 0 || dev.buf[2..8].iter().any(|p| *p == *k) {
                        //             continue;
                        //         }
                        //         v = *k;
                        //     };
                        //     v
                        // };
                        // if pressed != 0 {
                        //     PRESSING.store(pressed, Ordering::Relaxed)
                        // }
                        // if released == PRESSING.load(Ordering::Relaxed) {
                        //     PRESSING.store(0, Ordering::Relaxed)
                        // }
                        // for (a, b) in prev.iter_mut().zip(dev.buf.iter()) {
                        //     *a = *b;
                        // }
                        let mut arr = [0; 6];
                        arr.clone_from_slice(&dev.buf[2..8]);
                        (xhc.keyboard_handler)(dev.buf[0], arr);
                    }
                    self.set_normal_trb(dev);
                } else {
                    error!("{}", make_error!(Code::NotImplemented))
                }
                return;
            }
        };
        debug!("trb: {:?}", trb);
        if (trb.data[0] >> 8) & 0xff == 6 && (trb.data[0] >> 16) == 0x0100 { // get_descriptor device
            dev.num_configuration = dev.buf[17];
            dev.max_packet_size = match dev.buf[7] {
                9 => 512,
                x => x as u16,
            };
            if dev.buf[4] != 0 {
                error!("buf4: {}, {}", dev.buf[4], make_error!(Code::NotImplemented));
                return;
            }
            dev.get_descriptor(2, 0);
        } else if (trb.data[0] >> 8) & 0xff == 6 && (trb.data[0] >> 24)  == 0x02 { // get_descriptor configuration 0
            debug!("get configuration");
            for i in dev.input_ctx.input_control_ctx().iter_mut() {
                *i = 0;
            }
            for i in 0..8 {
                dev.input_ctx.slot_ctx()[i] = dev.device_ctx.slot_ctx()[i];
            }
            dev.input_ctx.input_control_ctx()[1] = 1;

            let mut base = 0;
            let max = dev.buf[2] as usize;
            while base < max {
                let ty = dev.buf[base + 1];
                let buf = unsafe { from_raw_parts_mut(&mut dev.buf[base] as *mut u8, dev.buf[base] as usize) };
                debug!("scaning buf: {:?}", buf);
                match ty {
                    2 => { // CONFIGURATION
                        debug!("configuration found");
                    },
                    4 => { // INTERFACE
                        debug!("interface found");
                        let idx = buf[2] as usize;
                        dev.classes[idx].class = buf[5] as u16;
                        dev.classes[idx].sub_class = buf[6] as u16;
                        dev.classes[idx].protocol = buf[7] as u16;
                        dev.classes[idx].interface = buf[8] as u16;
                    },
                    5 => { // ENDPOINT
                        debug!("endpoint found");
                        let dci = (buf[2] & 0b111) * 2 + (buf[2] >> 7);
                        debug!("buf[2] & 0b111: {}, buf[2] >> 7: {}", buf[2] & 0b111, buf[2] >> 7);
                        dev.input_ctx.input_control_ctx()[1] |= 1 << (dci as u32);
                        let ptr = dev.transfer_rings[dci as usize - 1].x.0.as_ptr() as u64;
                        let ep_type = match (buf[2] >> 7, buf[3] & 0b11) {
                            (0, 1) => 1u32,
                            (0, 2) => 2,
                            (0, 3) => 3,
                            (_, 0) => 4,
                            (1, 1) => 5,
                            (1, 2) => 6,
                            (1, 3) => 7,
                            _ => unreachable!(),
                        };
                        let w_max_packet_size = (buf[4] as u32) | (buf[5] as u32) << 2;
                        let b_interval = buf[6] as u32;
                        // dev.input_ctx.ep_ctx[dci as usize - 1].data = [
                        //     b_interval << 16,
                        //     ep_type << 3 | w_max_packet_size << 16 | 3 << 1,
                        //     (ptr & 0xffffffff) as u32 | 1,
                        //     (ptr >> 32) as u32,
                        //     0,
                        //     0,
                        //     0,
                        //     0
                        // ];
                        let ep_ctx = dev.input_ctx.ep_ctx(dci as usize - 1);
                        for i in ep_ctx.iter_mut() {
                            *i = 0;
                        }
                        ep_ctx[0] = b_interval << 16;
                        ep_ctx[1] = ep_type << 3 | w_max_packet_size << 16 | 3 << 1;
                        ep_ctx[2] = (ptr & 0xffffffff) as u32 | 1;
                        ep_ctx[3] = (ptr >> 32) as u32;
                    },
                    33 => { // HID
                        debug!("hid found");
                    },
                    x => {
                        debug!("unknown field: {} found", x);
                    }
                }
                base += dev.buf[base] as usize;
            }
            debug!("input context control: {:?}", dev.input_ctx.input_control_ctx());
            let ptr = dev.input_ctx.input_control_ctx().as_ptr() as u64;
            assert!(ptr & 0x3f == 0);
            xhc.command_ring.push(TRB {
                data: [
                    (ptr & 0xffffffff) as u32,
                    (ptr >> 32) as u32,
                    0,
                    (self.slot_id() as u32) << 24 | 12 << 10 | 1,
                ]
            });
            xhc.capability.doorbell()[0].ring(0, 0);
        } else if (trb.data[0] >> 8) & 0xff == 11 {
            self.set_normal_trb(dev);
        } else {
            error!("{}", make_error!(Code::NotImplemented))
        }
    }
}

const TRB_BUF_LEN: usize = 32;

type TRBbufTy = [TRB; TRB_BUF_LEN];

#[repr(align(64))]
struct TRBTable([TRB; TRB_BUF_LEN]);

struct MemPoolCrTRB {
    x: &'static mut TRBTable, // [TRB; TRB_BUF_LEN],
    index: usize,
    cycle: bool
}

impl MemPoolCrTRB {
    pub fn push(&mut self, mut trb: TRB) {
        trb.data[3] = (trb.data[3] & !0x1) | (self.cycle as u32);
        for i in 0..4 {
            self.x.0[self.index].data[i] = trb.data[i]
        }
        self.index += 1;
        if self.index == TRB_BUF_LEN - 1 {
            let mut link = TRB::new_link_trb(self.x.0.as_ptr() as u64);
            link.data[3] = link.data[3] | (self.cycle as u32);
            for i in 0..4 {
                self.x.0[self.index].data[i] = link.data[i];
            }
            self.index = 0;
            self.cycle = !self.cycle;
        }
    }
}

// static CR_BUF: Mutex<MemPoolCrTRB> = Mutex::new(MemPoolCrTRB { x: [TRB { data: [0; 4] }; TRB_BUF_LEN], index: 0, cycle: true });

#[repr(C, packed)]
struct EventRingSegmentTableEntry {
    addr: u64,
    size: u16,
    rsvdz1: u16,
    rsvdz2: u32,
}

#[repr(align(64))]
struct MemPoolERSTE {
    x: [EventRingSegmentTableEntry; 1]
}

// static ERSTE_BUF: Mutex<MemPoolERSTE> = Mutex::new(MemPoolERSTE { x: [EventRingSegmentTableEntry { addr: 0, size: 0, rsvdz1: 0, rsvdz2: 0 }; 1] });

struct MemPoolErTRB {
    x: &'static TRBTable, // [TRB; TRB_BUF_LEN],
    index: usize,
    cycle: bool,
}

impl MemPoolErTRB {
    fn next_(&mut self) -> Option<TRB> {
        let v = self.x.0[self.index];
        if v.cycle() == self.cycle {
            if self.index == TRB_BUF_LEN - 1 {
                self.index = 0;
                self.cycle = !self.cycle;
            } else {
                self.index += 1;
            }
            Some(v)
        } else {
            None
        }
    }
    fn clean(&self, xhc: &XhcController) {
        unsafe {
            let interrupt_reg = xhc.capability.runtime().interrupt_set();
            let p = interrupt_reg[0].event_ring_dequeue_pointer.read() & 0xf;
            interrupt_reg[0].event_ring_dequeue_pointer.write(p | (&self.x.0[self.index] as *const TRB as u64));
        }
    }
}

impl Stream for MemPoolErTRB {
    type Item = TRB;

    fn poll_next(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> core::task::Poll<Option<Self::Item>> {
        let v = self.get_mut();
        if let Some(trb) = v.next_() {
            return Poll::Ready(Some(trb))
        }

        ER_WAKER.register(&cx.waker());
        match v.next_() {
            Some(trb) => {
                ER_WAKER.take();
                Poll::Ready(Some(trb))
            }
            None => Poll::Pending
        }
    }
}

pub extern "x86-interrupt" fn int_handler_xhci(frame: crate::interrupt::InterruptFrame) {
    ER_WAKER.wake();
    crate::interrupt::notify_end_of_interrupt();
}

// static ER_BUF: Mutex<MemPoolErTRB> = Mutex::new(MemPoolErTRB { x: [TRB { data: [0; 4] }; TRB_BUF_LEN], index: 0, cycle: true });
static ER_WAKER: AtomicWaker = AtomicWaker::new();

#[repr(C, align(64))]
struct MemPoolTrTRB {
    x: &'static mut TRBTable, // [TRB; TRB_BUF_LEN],
    index: usize,
    cycle: bool,
}

impl MemPoolTrTRB {
    pub fn center(&self) -> &TRB {
        &self.x.0[self.index]
    }
    pub fn push(&mut self, mut trb: TRB) {
        trb.data[3] = (trb.data[3] & !0x1) | (self.cycle as u32);
        for i in 0..4 {
            self.x.0[self.index].data[i] = trb.data[i]
        }
        self.index += 1;
        if self.index == TRB_BUF_LEN - 1 {
            let mut link = TRB::new_link_trb(self.x.0.as_ptr() as u64);
            link.data[3] = (link.data[3] & !0x1) | (self.cycle as u32);
            for i in 0..4 {
                self.x.0[self.index].data[i] = link.data[i];
            }
            self.index = 0;
            self.cycle = !self.cycle;
        }
    }
}

// type TrBufType = [[Mutex<MemPoolTrTRB>; 32]; (max_slots_en + 1) as usize];

// const unsafe fn init_tr_buf() -> TrBufType {
//     let mut arr = core::mem::MaybeUninit::<TrBufType>::uninit().assume_init();
//     let mut outer = 0;
//     let mut inner;
//     while outer < (max_slots_en + 1) as usize {
//         inner = 0;
//         while inner < 32 {
//             arr[outer][inner] = Mutex::new(MemPoolTrTRB {
//                 x: [TRB { data: [0; 4] }; TRB_BUF_LEN], index: 0, cycle: true
//             });
//             inner += 1;
//         }
//         outer += 1;
//     }
//     arr
// }

// static TR_BUF: TrBufType = unsafe { init_tr_buf() };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigPhase {
    Broken,
    NotConnected,
    WaitingAddressed,
    ResettingPort,
    EnablingSlot,
    AddressingDevice,
    InitializingDevice,
    ConfiguringEndpoints,
    Configured,
}

pub struct XhcController {
    pub capability: &'static registers::CapabilityRegisters,
    port_config_phase: [ConfigPhase; 256],
    addressing_port: u8,
    keyboard_handler: fn(modifire: u8, pressing: [u8; 6]),
    mouse_handler: fn(modifire: u8, move_x: i8, move_y: i8),
    dcbaa: &'static mut DCBAA,
    command_ring: MemPoolCrTRB,
    event_ring: MemPoolErTRB,
    erste: &'static mut MemPoolERSTE,
    allocator: SimplestAllocator,
    center: usize,
}

impl XhcController {
    pub unsafe fn initialize(mmio_base: u64, keyboard_handler: fn(u8, [u8; 6]), mouse_handler: fn(u8, i8, i8)) -> XhcController {
        let mem = Vec::<u8>::with_capacity(1024 * 1024 * 4).leak();
        let head = mem as *mut [u8] as *mut u8 as usize;
        let end = head + 1024 * 1024 * 4;
        let allocator = SimplestAllocator::empty();
        allocator.init(head as *mut u8, end as *mut u8);



        let cap_reg = &*(mmio_base as *const registers::CapabilityRegisters);
        debug!("cap reg: {}", cap_reg.length());

        // if cap_reg.hcc_params1.read() & 0b100 != 0 {
        //     panic!("not support 64bit context now")
        // }

        let ptr = mmio_base + (cap_reg.hcc_params1.read() >> 16 << 2) as u64;
        let ptr = ptr as *mut u32;
        let mut val = ptr;
        loop {
            if *val & 0xff == 1 {
                debug!("bios to os: {:x}", *val);
                if *val >> 16 & 1 != 0 {
                    // let v = (val as u64 + 3) as *mut u8;
                    // debug!("bios to os: {:x}, {:x}", *val, *v);
                    *val |= 1 << 24;
                    while *val >> 16 & 1 == 1 {}
                    debug!("success")
                }
                val = val.add(1);
                let mut v = *val;
                v &= (0x7 << 1) + (0xff << 5) + (0x7 << 17);
                v |= 0x7 << 29;
                *val = v;
                break;
            }
            let next = (*val >> 8) & 0xff;
            if next == 0 {
                break
            } else {
                val = ((val as usize) + ((next as usize) << 2)) as *mut u32
            }
        }

        let usbsts = cap_reg.usb_status(); // &*((op_base + 0x04) as *const u32);
        let usbcmd = cap_reg.usb_command();

        // assert!(usbsts.hchalted());  // assert USBSTS.HCH == 1
        if !usbsts.hchalted() {
            usbcmd.stop();
            while !usbsts.hchalted() {}
        }

        usbcmd.set_host_controller_reset(true);  // USBCMD.HCRST = 1

        while usbcmd.host_controller_reset() {}  // wait HCRST == 0

        while usbsts.controller_not_ready() {}  // wait USBSTS.CNR == 0

        debug!("USBSTS.CNR is 0! ready!");


        let max_slots = cap_reg.max_slots();

        debug!("max slots: {}", max_slots);
        assert!(max_slots >= MAX_SLOTS_EN);

        let config_reg = cap_reg.configure();
        config_reg.set_max_slots_en(MAX_SLOTS_EN);

        let pagesize = cap_reg.pagesize() as usize;

        let hcs_params2 = cap_reg.hcs_params2.read();
        let max_scratchpad_buffers = (((hcs_params2 >> 16) & (0x1f << 5)) | (hcs_params2 >> 27)) as usize;

        debug!("max_scratchpad_buffers: {}, pagesize: {}", max_scratchpad_buffers, pagesize);

        let dcbaap = cap_reg.dcbaap();
        let ptr = allocator.alloc_with_boundary_zeroed(Layout::new::<DCBAA>(), pagesize);
        let dcbaap_ptr = &mut *(ptr as *mut DCBAA); // DCBAA.lock().x.as_ptr() as u64;
        dcbaap.set_dcbaap(ptr as u64);

        if max_scratchpad_buffers > 0 {
            let arr = allocator.alloc_with_boundary_zeroed(Layout::from_size_align(max_scratchpad_buffers * 8, 64).unwrap(), pagesize) as *mut u64;
            for i in 0..max_scratchpad_buffers {
                arr.add(i).write(
                    allocator.alloc_with_boundary_zeroed(Layout::from_size_align(pagesize, pagesize).unwrap(), 0) as usize as u64
                );
            }
            dcbaap_ptr.0[0] = arr as usize as u64;
        }

        let crcr = cap_reg.crcr();
        let ptr = allocator.alloc_with_boundary_zeroed(Layout::new::<TRBTable>(), 65536); // CR_BUF.lock().x.as_ptr() as u64;
        let cr_ptr = &mut *(ptr as *mut TRBTable);
        assert!(ptr as u64 & 0x3f == 0);
        crcr.set_value(ptr as u64 | 1);
        // crcr.set_pointer(ptr);
        // crcr.set_ring_cycle_state(true);

        let ptr = allocator.alloc_with_boundary_zeroed(Layout::new::<TRBTable>(), 65536); // ER_BUF.lock().x.as_ptr() as u64;
        let er_ptr = &mut *(ptr as *mut TRBTable);
        assert!(ptr as u64 & 0x3f == 0);
        let erste_ptr = allocator.alloc_with_boundary_zeroed(Layout::new::<MemPoolERSTE>(), 0);
        let mut erste_lock = &mut *(erste_ptr as *mut MemPoolERSTE); // ERSTE_BUF.lock();
        erste_lock.x[0].addr = ptr as u64;
        erste_lock.x[0].size = TRB_BUF_LEN as u16;

        let runtime = cap_reg.runtime();
        let interrupt_regs = runtime.interrupt_set();
        interrupt_regs[0].event_ring_segment_table_size.write(1);
        interrupt_regs[0].event_ring_dequeue_pointer.write(ptr as u64);
        let ptr = erste_lock.x.as_ptr() as u64;
        assert!(ptr & 0x3f == 0);
        interrupt_regs[0].event_ring_segment_table_base_addr.write(ptr);
        interrupt_regs[0].moderation.write(4000);
        interrupt_regs[0].management.write(0x3);
        usbcmd.set_interrupt_enable(true);
        let mut port_config_phase = [ConfigPhase::NotConnected; 256];
        // port_config_phase[16] = ConfigPhase::Broken;

        XhcController {
            capability: cap_reg,
            port_config_phase,
            addressing_port: 0,
            keyboard_handler,
            mouse_handler,
            dcbaa: dcbaap_ptr,
            command_ring: MemPoolCrTRB { x: cr_ptr, index: 0, cycle: true },
            event_ring: MemPoolErTRB { x: er_ptr, index: 0, cycle: true },
            erste: erste_lock,
            allocator,
            center: head,
        }
    }
    pub fn run(&self) {
        let usbcmd = self.capability.usb_command();
        usbcmd.run();
        let usbsts = self.capability.usb_status();
        while usbsts.hchalted() {}
    }
    pub fn configure_port(&mut self) {
        let max_ports = self.capability.max_ports();
        for n in 1..=max_ports {
            let port = self.capability.port_sc(n);

            if port.is_connected() {
                self.reset_port(n);
            }
        }
    }

    // safety: port must be connected
    pub fn reset_port(&mut self, port_num: u8) {
        // if port_num != 3 && port_num != 4 {
        //     debug!("pass {}", port_num);
        //     return;
        // }
        if self.addressing_port != 0 {
            self.port_config_phase[port_num as usize] = ConfigPhase::WaitingAddressed;
            return;
        }
        let port = self.capability.port_sc(port_num);
        match self.port_config_phase[port_num as usize] {
            ConfigPhase::NotConnected | ConfigPhase::WaitingAddressed => {
                self.addressing_port = port_num;
                self.port_config_phase[port_num as usize] = ConfigPhase::ResettingPort;
                port.reset();
            },
            _ => {
                error!("{}", make_error!(Code::InvalidPhase))
            }
        }
    }

    pub fn enable_slot(&mut self, port_num: u8) {
        let port = self.capability.port_sc(port_num);
        if port.is_enabled() && port.is_port_reset_changed() {
            port.clear_is_port_reset_changed();
            self.port_config_phase[port_num as usize] = ConfigPhase::EnablingSlot;
            self.command_ring.push(TRB::no_op_command_trb());
            self.command_ring.push(TRB::new_enable_slot_trb());
            self.capability.doorbell()[0].ring(0, 0);
            debug!("enable port: {}", port_num);
        } else {
            error!("{}, {}", port.is_enabled(), port.is_port_reset_changed());
            error!("{}", make_error!(Code::InvalidPhase));
        }
    }

    pub fn address_deivce(&mut self, slot_id: u8, port_num: u8) {
        let mut lock = DEVICES_MEM[slot_id as usize].lock();
        let pagesize = self.capability.pagesize() as usize;
        let (device_ctx, input_ctx) = unsafe {
            if self.capability.hcc_params1.read() & 0b100 != 0 {
                (
                    DeviceContextEnum::V2(&*(self.allocator.alloc_with_boundary_zeroed(Layout::new::<DeviceContext64>(), pagesize) as *const DeviceContext64)),
                    InputContextEnum::V2(&mut *(self.allocator.alloc_with_boundary_zeroed(Layout::new::<InputContext64>(), pagesize) as *mut InputContext64))
                )
            } else {
                (
                    DeviceContextEnum::V1(&*(self.allocator.alloc_with_boundary_zeroed(Layout::new::<DeviceContext>(), pagesize) as *const DeviceContext)),
                    InputContextEnum::V1(&mut *(self.allocator.alloc_with_boundary_zeroed(Layout::new::<InputContext>(), pagesize) as *mut InputContext))
                )
            }
        };

        *lock = Some(XhciDevice {
            device_ctx,
            input_ctx,
            slot_id,
            buf: [0; 512],
            doorbell: &mut self.capability.doorbell()[slot_id as usize] as *mut registers::DoorbellRegister as u64,
            num_configuration: 0,
            max_packet_size: 0,
            classes: [ClassDriver { class: 0, sub_class: 0, protocol: 0, interface: 0 }; 15],
            default: 0,
            transfer_rings: unsafe {
                let mut arr: [MaybeUninit<MemPoolTrTRB>; 31] = MaybeUninit::uninit().assume_init();
                for elem in arr.iter_mut() {
                    elem.write(MemPoolTrTRB {
                        x: &mut *(self.allocator.alloc_with_boundary_zeroed(Layout::new::<TRBTable>(), 65536) as *mut TRBTable),
                        index: 0,
                        cycle: true
                    });
                }
                core::mem::transmute(arr)
            }
        });
        let dev = lock.as_mut().unwrap();
        // dev.slot_id = slot_id;
        // dev.doorbell = &mut self.capability.doorbell()[slot_id as usize] as *mut DoorbellRegister as u64;
        self.dcbaa.0[slot_id as usize] = dev.device_ctx.as_inner_ptr();
        for d in dev.input_ctx.input_control_ctx().iter_mut() {
            *d = 0;
        }
        dev.input_ctx.input_control_ctx()[1] |= 0b11;

        let port = self.capability.port_sc(port_num);
        dev.input_ctx.slot_ctx()[0] = ((port.port_speed() as u32) << 20) | 1 << 27;
        dev.input_ctx.slot_ctx()[1] = (port_num as u32) << 16;

        let max_packet = match port.port_speed() {
            1 | 2 => 8u32,
            3 => 64,
            4 => 512,
            _ => {
                panic!("{}", make_error!(Code::UnknownXHCISpeedID))
            }
        };

        let ptr = dev.transfer_rings[0].x.0.as_ptr() as *const TRB as u64;
        assert!(ptr & 0x3f == 0);
        dev.input_ctx.ep_ctx(0)[1] = max_packet << 16 | 4 << 3 | 3 << 1;
        dev.input_ctx.ep_ctx(0)[2] = (ptr & 0xffffffc0) as u32 | 1;
        dev.input_ctx.ep_ctx(0)[3] = (ptr >> 32) as u32;

        self.port_config_phase[port_num as usize] = ConfigPhase::AddressingDevice;

        self.command_ring.push(TRB::address_device_command_trb(dev.input_ctx.as_inner_ptr(), slot_id));
        self.capability.doorbell()[0].ring(0, 0);
    }

    pub fn process_event(&mut self) -> bool {
        // let mut er_lock = ER_BUF.lock();
        if let Some(trb) = self.event_ring.next_() {
            let v1 = trb.data[0];
            let v2 = trb.data[1];
            let v3 = trb.data[2];
            let v4 = trb.data[3];
            debug!("trb: {:x} {:x} {:x} {:x}", v1, v2, v3, v4);
            if let Some(casted) = trb.cast::<PortStatusChangeEventTRB>() {
                // debug!("portstatuschangeevent");
                casted.on_event(self)
            } else if let Some(casted) = trb.cast::<CommandCompletionEventTRB>() {
                // debug!("commandcompletionevent");
                if v3 >> 24 != 1 {
                    error!("command completion error: {}", v3 >> 24);
                }
                casted.on_event(self)
            } else if let Some(casted) = trb.cast::<TransferEventTRB>() {
                // debug!("transferevent");
                casted.on_event(self)
            } else {
                error!("TRB type: {} {}", trb.ty(), make_error!(Code::NotImplemented))
            }
            self.event_ring.clean(self);


        }
            if self.capability.usb_status().hchalted() {
                error!("usb halted");
                for trb in self.event_ring.x.0 {
                    let v1 = trb.data[0];
                    let v2 = trb.data[1];
                    let v3 = trb.data[2];
                    let v4 = trb.data[3];
                    debug!("{:x} {:x} {:x} {:x}", v1, v2, v3, v4);
                }
                // let max_ports = self.capability.max_ports();
                // for i in 1..max_ports {
                //     debug!("port {} {}", i, self.capability.port_sc(i).data.read());
                // }
                debug!("{} {}", self.capability.usb_command().data.read(), self.capability.usb_status().data.read());
                // for trb in self.command_ring.x.0 {
                //     let v1 = trb.data[0];
                //     let v2 = trb.data[1];
                //     let v3 = trb.data[2];
                //     let v4 = trb.data[3];
                //     debug!("{:x} {:x} {:x} {:x}", v1, v2, v3, v4);
                // }
                debug!("{} {}", self.command_ring.index, self.command_ring.cycle);
                return false;
            }
        true
    }
}

impl Drop for XhcController {
    fn drop(&mut self) {
        unsafe {
            let v = Vec::from_raw_parts(self.center as *mut u8, 1024 * 1024 * 4, 1024 * 1024 * 4);
            drop(v);
        }
    }
}


// pub unsafe fn driver_handle_test(mmio_base: u64, device: &Device) {
//     let mut xhci = XhcController::initialize(mmio_base);
//     xhci.run();
//     xhci.configure_port();

//     loop {
//         xhci.process_event();
//         // xhci.capability.doorbell()[1]
//     }


    // initialized

    // interrupt setting?
    // let local_apic_id = *(0xfee00020 as *const u32) >> 24;
    // let msi_msg_addr = 0xfee00000 | (local_apic_id << 12);
    // let msi_msg_data = 0xc000u32 | 0x40;

    // let i = 0;
    // loop {
    //     while ER_BUF.x[i].data[3] & 0x1 == ER_CYCLE as u32 {
    //         let trb_type = (ER_BUF.x[i].data[3] >> 10) & 0b111111;
    //         if trb_type == 34 {
    //             let port_id = ER_BUF.x[i].data[0] >> 24;

    //             // enable slot only
    //             let addr = (op_base + 0x400 + (0x10 * n as u64)) as *mut u32;
    //             let val = *addr & 0x0e01c3e0;
    //             *addr = val | 0x200000;
    //             CR_BUF.x[0].
    //         }
    //     }
    // }
// }

// unsafe fn configure_msi(device: &Device, msg_addr: u32, msg_data: u32, num_vector_exponent: u32) {
//     let mut msi_cap_addr = 0u8;
//     let mut msix_cap_addr = 0u8;
//     let cap_addr = read_config_reg(device, 0x34) & 0xff;
//     while cap_addr != 0 {
//         let cap = read_config_reg(device, cap_addr as u8);
//         let cap_id = (cap & 0xff) as u8;
//         if cap_id == 0x05 {
//             msi_cap_addr = cap_addr;
//         } else if cap_id == 0x11 {
//             msix_cap_addr = cap_addr;
//         }
//         cap_addr = ((cap >> 8) & 0xff) as u8;
//     }


// }
