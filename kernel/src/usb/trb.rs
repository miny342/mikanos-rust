use core::mem::transmute;
use core::slice::from_raw_parts_mut;

use spin::Mutex;
use heapless::FnvIndexMap;
use log::{debug, error};

use crate::usb::controller::{ConfigPhase, XhcController};
use crate::usb::device::{DEVICES_MEM, XhciDevice};
use crate::{make_error, error::Code};

pub trait TRBtrait {
    const TY: u32;
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TRB {
    pub data: [u32; 4]
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

pub struct PortStatusChangeEventTRB {
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
pub struct CommandCompletionEventTRB {
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

pub static SETUP_TRB_MAP: Mutex<FnvIndexMap<u64, TRB, 32>> = Mutex::new(FnvIndexMap::new());

#[repr(C)]
pub struct TransferEventTRB {
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
