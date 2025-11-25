use spin::Mutex;

use crate::usb::memory_pool::MemPoolTrTRB;
use crate::usb::registers;
use crate::usb::trb::{SETUP_TRB_MAP, TRB};


pub const MAX_SLOTS_EN: u8 = 8;

#[repr(align(64))]
pub struct DeviceContextBaseAddressArray(pub [u64; MAX_SLOTS_EN as usize + 1]);

#[derive(Debug, Clone, Copy)]
pub struct ClassDriver {
    pub class: u16,
    pub sub_class: u16,
    pub protocol: u16,
    pub interface: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct SlotContext {
    data: [u32; 8]
}

impl SlotContext {
    fn root_hub_port_num(&self) -> u8 {
        (self.data[1] >> 16 & 0xff) as u8
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EndpointContext {
    data: [u32; 8]
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct DeviceContext {
    slot_ctx: SlotContext,
    ep_ctx: [EndpointContext; 31]
}

#[derive(Debug, Clone, Copy)]
pub struct InputControlContext {
    data: [u32; 8]
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct InputContext {
    input_control_ctx: InputControlContext,
    slot_ctx: SlotContext,
    ep_ctx: [EndpointContext; 31]
}

#[derive(Debug, Clone, Copy)]
pub struct SlotContext64 {
    data: [u32; 16]
}

impl SlotContext64 {
    fn root_hub_port_num(&self) -> u8 {
        (self.data[1] >> 16 & 0xff) as u8
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EndpointContext64 {
    data: [u32; 16]
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct DeviceContext64 {
    slot_ctx: SlotContext64,
    ep_ctx: [EndpointContext64; 31]
}

#[derive(Debug, Clone, Copy)]
pub struct InputControlContext64 {
    data: [u32; 16]
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct InputContext64 {
    input_control_ctx: InputControlContext64,
    slot_ctx: SlotContext64,
    ep_ctx: [EndpointContext64; 31]
}

pub enum DeviceContextEnum {
    V1(&'static DeviceContext),
    V2(&'static DeviceContext64)
}

impl DeviceContextEnum {
    pub fn root_hub_port_num(&self) -> u8 {
        match self {
            DeviceContextEnum::V1(x) => x.slot_ctx.root_hub_port_num(),
            DeviceContextEnum::V2(x) => x.slot_ctx.root_hub_port_num(),
        }
    }
    pub fn slot_ctx(&self) -> &[u32] {
        match self {
            DeviceContextEnum::V1(x) => &x.slot_ctx.data,
            DeviceContextEnum::V2(x) => &x.slot_ctx.data,
        }
    }
    pub fn as_inner_ptr(&self) -> u64 {
        match self {
            DeviceContextEnum::V1(x) => *x as *const DeviceContext as u64,
            DeviceContextEnum::V2(x) => *x as *const DeviceContext64 as u64,
        }
    }
}

pub enum InputContextEnum {
    V1(&'static mut InputContext),
    V2(&'static mut InputContext64),
}

impl InputContextEnum {
    pub fn input_control_ctx(&mut self) -> &mut [u32] {
        match self {
            InputContextEnum::V1(x) => &mut x.input_control_ctx.data,
            InputContextEnum::V2(x) => &mut x.input_control_ctx.data,
        }
    }
    pub fn slot_ctx(&mut self) -> &mut [u32] {
        match self {
            InputContextEnum::V1(x) => &mut x.slot_ctx.data,
            InputContextEnum::V2(x) => &mut x.slot_ctx.data,
        }
    }
    pub fn ep_ctx(&mut self, idx: usize) -> &mut [u32] {
        match self {
            InputContextEnum::V1(x) => &mut x.ep_ctx[idx].data,
            InputContextEnum::V2(x) => &mut x.ep_ctx[idx].data,
        }
    }
    pub fn as_inner_ptr(&self) -> u64 {
        match self {
            InputContextEnum::V1(x) => *x as *const InputContext as u64,
            InputContextEnum::V2(x) => *x as *const InputContext64 as u64,
        }
    }
}

pub struct XhciDevice {
    pub device_ctx: DeviceContextEnum,
    pub input_ctx: InputContextEnum,
    pub slot_id: u8,
    pub buf: [u8; 512],
    pub doorbell: u64,
    pub num_configuration: u8,
    pub max_packet_size: u16,
    pub classes: [ClassDriver; 15],
    pub default: usize,  // default class driver (boot protocol)
    pub transfer_rings: [MemPoolTrTRB; 31],
}

impl XhciDevice {
    pub fn doorbell(&self) -> &'static mut registers::DoorbellRegister {
        if self.doorbell == 0 {
            panic!("doorbell is not initialized")
        }
        unsafe { &mut *(self.doorbell as *mut registers::DoorbellRegister) }
    }
    pub fn start_init(&mut self) {
        self.get_descriptor(1, 0);
    }
    pub fn get_descriptor(&mut self, ty: u8, num: u8) {
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
    pub fn set_protocol_boot(&mut self) {
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

pub static DEVICES_MEM: DeviceMemType = unsafe { device_mem_init() } ;
