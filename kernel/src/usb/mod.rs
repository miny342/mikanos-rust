use core::intrinsics::transmute;
use core::ptr::{slice_from_raw_parts_mut, write_volatile};
use core::slice::from_raw_parts_mut;

use volatile_register::{RW, RO, WO};
use heapless::FnvIndexMap;

use crate::{println, debug, make_error, error};
use crate::pci::Device;
use crate::error::*;

type HandleError<T> = Result<T, Error>;

#[repr(C)]
struct UsbStatusRegister {
    data: RW<u32>
}

impl UsbStatusRegister {
    pub fn hchalted(&self) -> bool {
        self.data.read() & 0x1 != 0
    }
    pub fn controller_not_ready(&self) -> bool {
        (self.data.read() >> 11) & 0x1 != 0
    }
}

#[repr(C)]
struct UsbCommandRegister {
    data: RW<u32>
}

impl UsbCommandRegister {
    pub fn run(&mut self) {
        unsafe { self.data.write(self.data.read() | 0x1) }
    }
    pub fn stop(&mut self) {
        unsafe { self.data.write(self.data.read() & !0x1) }
    }
    pub fn host_controller_reset(&self) -> bool {
        (self.data.read() >> 1) & 0x1 != 0
    }
    pub fn set_host_controller_reset(&mut self, value: bool) {
        unsafe { self.data.write((self.data.read() & !0x2) | ((value as u32) << 1)) }
    }
    pub fn set_interrupt_enable(&mut self, value: bool) {
        unsafe { self.data.write((self.data.read() & !0x4) | ((value as u32) << 2)) }
    }
}

#[repr(C)]
struct ConfigureRegister {
    data: RW<u32>
}

impl ConfigureRegister {
    pub fn set_max_slots_en(&mut self, value: u8) {
        unsafe { self.data.write((self.data.read() & !0xff) | (value as u32)) }
    }
}

#[repr(C)]
struct DeviceContextBaseAddressArrayPointerRegister {
    data: RW<u64>
}

impl DeviceContextBaseAddressArrayPointerRegister {
    pub fn set_dcbaap(&mut self, value: u64) {
        assert!(value & 0x3f == 0);
        unsafe { self.data.write(value) }
    }
}

#[repr(C)]
struct CommandRingControlRegister {
    data: RW<u64>
}

impl CommandRingControlRegister {
    // pub fn set_ring_cycle_state(&mut self, value: bool) {
    //     unsafe { self.data.write((self.data.read() & !0x1) | (value as u64)) }
    // }
    // pub fn set_pointer(&mut self, value: u64) {
    //     assert!(value & 0x3f == 0);
    //     unsafe { self.data.write((self.data.read() & 0x3f) | value) }
    // }
    pub unsafe fn set_value(&mut self, value: u64) {
        self.data.write(value);
    }
}

#[repr(C)]
struct HostControllerRuntimeRegister {
    data: RW<u32>
}

impl HostControllerRuntimeRegister {
    pub unsafe fn interrupt_set(&self) -> &mut [InterruptRegister] {
        from_raw_parts_mut(((self as *const HostControllerRuntimeRegister as u64) + 0x20) as *mut InterruptRegister, 1024)
    }
}

#[repr(C)]
struct PortStatusAndControlRegister {
    data: RW<u32>
}

impl PortStatusAndControlRegister {
    pub fn is_connected(&self) -> bool {
        self.data.read() & 0x1 != 0
    }
    pub fn is_enabled(&self) -> bool {
        self.data.read() & 0x2 != 0
    }
    pub fn reset(&mut self) {
        let val = self.data.read() & 0x0e00c3e0;
        unsafe { self.data.write(val | 0x00020010) }
        while self.is_port_reset() {}
    }
    pub fn is_port_reset(&self) -> bool {
        self.data.read() & 0x10 !=0
    }
    pub fn is_port_reset_changed(&self) -> bool {
        self.data.read() & 0x200000 != 0
    }
    pub fn clear_is_port_reset_changed(&mut self) {
        let v = self.data.read() & 0x0e01c3e0;
        unsafe {self.data.write(v | 0x200000) };
    }
    pub fn port_speed(&self) -> u8 {
        ((self.data.read() >> 10) & 0xf) as u8
    }
}

#[repr(C)]
struct DoorbellRegister {
    data: RW<u32>
}

impl DoorbellRegister {
    pub fn ring(&mut self, target: u8, stream_id: u16) {
        unsafe { self.data.write((target as u32) | ((stream_id as u32) << 16)) }
    }
}


#[repr(C)]
struct CapabilityRegisters {
    // length: u8,
    // rsvdz1: u8,
    // hci_version: u16,
    data: RW<u32>,
    hcs_params1: RW<u32>,
    hcs_params2: RW<u32>,
    hcs_params3: RW<u32>,
    hcc_params1: RW<u32>,
    doorbell_offset: RW<u32>,
    runtime_reg_offset: RW<u32>,
    hcc_params2: RW<u32>,
}

impl CapabilityRegisters {
    fn length(&self) -> u8 {
        (self.data.read() & 0xff) as u8
    }
    fn hci_version(&self) -> u16 {
        (self.data.read() >> 16) as u16
    }
    fn op_base(&self) -> u64 {
        self as *const CapabilityRegisters as u64 + self.length() as u64
    }
    pub fn usb_status(&self) -> &mut UsbStatusRegister {
        unsafe { &mut *((self.op_base() + 0x04) as *mut UsbStatusRegister) }
    }
    pub fn usb_command(&self) -> &mut UsbCommandRegister {
        unsafe { &mut *(self.op_base() as *mut UsbCommandRegister) }
    }
    pub fn max_slots(&self) -> u8 {
        (self.hcs_params1.read() & 0xff) as u8
    }
    pub fn max_ports(&self) -> u8 {
        (self.hcs_params1.read() >> 24) as u8
    }
    pub fn pagesize(&self) -> u32 {
        unsafe { *((self.op_base() + 0x8) as *const u32) }
    }
    pub fn configure(&self) -> &mut ConfigureRegister {
        unsafe { &mut *((self.op_base() + 0x38) as *mut ConfigureRegister) }
    }
    pub fn dcbaap(&self) -> &mut DeviceContextBaseAddressArrayPointerRegister {
        unsafe { &mut *((self.op_base() + 0x30) as *mut DeviceContextBaseAddressArrayPointerRegister) }
    }
    pub fn crcr(&self) -> &mut CommandRingControlRegister {
        unsafe { &mut *((self.op_base() + 0x18) as *mut CommandRingControlRegister) }
    }
    pub fn runtime(&self) -> &mut HostControllerRuntimeRegister {
        unsafe { &mut *((self as *const CapabilityRegisters as u64 + self.runtime_reg_offset.read() as u64) as *mut HostControllerRuntimeRegister) }
    }
    pub fn port_sc(&self, port: u8) -> &mut PortStatusAndControlRegister {
        unsafe { &mut *((self.op_base() + 0x400 + 0x10 * (port as u64 - 1)) as *mut PortStatusAndControlRegister) }
    }
    pub fn doorbell(&self) -> &mut [DoorbellRegister] {
        unsafe { &mut *(slice_from_raw_parts_mut((self as *const CapabilityRegisters as u64 + self.doorbell_offset.read() as u64) as *mut DoorbellRegister, 256)) }
    }
}

const max_slots_en: u8 = 8;

#[repr(align(64))]
struct MemPool {
    x: [u64; max_slots_en as usize + 1]
}

static mut DCBAA: MemPool = MemPool { x: [0; max_slots_en as usize + 1] };

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct SetupData {
    request: u16,
    value: u16,
    index: u16,
    length: u16
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

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct DeviceContext {
    slot_ctx: SlotContext,
    ep_ctx: [EndpointContext; 31]
}

#[derive(Debug, Clone, Copy)]
struct InputControlContext {
    data: [u32; 8]
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct InputContext {
    input_control_ctx: InputControlContext,
    slot_ctx: SlotContext,
    ep_ctx: [EndpointContext; 31]
}

#[derive(Debug, Clone, Copy)]
#[repr(C, align(4096))]
struct XhciDevice {
    device_ctx: DeviceContext,
    input_ctx: InputContext,
    slot_id: u8,
    buf: [u8; 512],
    doorbell: *mut DoorbellRegister,
    num_configuration: u8,
    max_packet_size: u16,
}

impl XhciDevice {
    fn doorbell(&self) -> &'static mut DoorbellRegister {
        if self.doorbell as u64 == 0 {
            panic!("doorbell is not initialized")
        }
        unsafe { &mut *(self.doorbell) }
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
        unsafe {
            let ptr = &mut TR_BUF[self.slot_id as usize][0];
            ptr.push(setup_trb);
            ptr.push(data_trb);
            SETUP_TRB_MAP.insert(ptr.center() as *const TRB as u64, setup_trb).unwrap();
            ptr.push(status_trb);
        }
        self.doorbell().ring(1, 0);
    }
}

static mut DEVICES_MEM: [XhciDevice; (max_slots_en + 1) as usize] = [
    XhciDevice {
        device_ctx: DeviceContext {
            slot_ctx: SlotContext {
                data: [0; 8]
            },
            ep_ctx: [EndpointContext {
                data: [0; 8]
            }; 31]
        },
        input_ctx: InputContext {
            input_control_ctx: InputControlContext {
                data: [0; 8]
            },
            slot_ctx: SlotContext {
                data: [0; 8]
            },
            ep_ctx: [EndpointContext {
                data: [0; 8]
            }; 31]
        },
        slot_id: 0,
        buf: [0; 512],
        doorbell: 0 as *mut DoorbellRegister,
        num_configuration: 0,
        max_packet_size: 0,
    }; (max_slots_en + 1) as usize
];

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
                (ptr & 0xfffffff0) as u32, (ptr >> 32) as u32, 0, 6 << 10
            ]
        }
    }
    pub fn address_device_command_trb(input_context_ptr: *const InputContext, slot_id: u8) -> TRB {
        let ptr = input_context_ptr as u64;
        assert!(ptr & 0x3f == 0);
        TRB {
            data: [
                (ptr & 0xfffffff0) as u32, (ptr >> 32) as u32, 0, ((slot_id as u32) << 24) | 11 << 10
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
                    unsafe {xhc.reset_port(id);}
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
        println!("cce-ty:{}", ty);
        if ty == 9 { // enable slot command
            if xhc.port_config_phase[xhc.addressing_port as usize] != ConfigPhase::EnablingSlot {
                panic!()
            }
            unsafe { xhc.address_deivce(self.slot_id(), xhc.addressing_port); }
        } else if ty == 11 { // address device command
            unsafe {
                let dev = &DEVICES_MEM[self.slot_id() as usize];
                let port_id = dev.device_ctx.slot_ctx.root_hub_port_num();
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

                xhc.initialize_device(self.slot_id(), port_id);
            }
        } else if ty == 12 { // configure endpoint command

        } else {
            error!("{}", make_error!(Code::NotImplemented))
        }
    }
}

static mut SETUP_TRB_MAP: FnvIndexMap<u64, TRB, 32> = FnvIndexMap::new();

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
    pub unsafe fn on_event(&self, xhc: &mut XhcController) {
        let req = (self.ptr().data[0] >> 8) & 0xff;
        let dev = &mut DEVICES_MEM[self.slot_id() as usize];
        // debug!("transfer: {:?}", DEVICES_MEM[self.slot_id() as usize].buf);
        debug!("data: {:?}", self.ptr().data);
        let trb = match SETUP_TRB_MAP.remove(&self.trb_ptr) {
            Some(x) => x,
            None => return
        };
        debug!("trb: {:?}", trb);
        if (trb.data[0] >> 8) & 0xff == 6 && (trb.data[0] >> 16) == 0x0100 { // get_descriptor device
            dev.num_configuration = dev.buf[17];
            dev.max_packet_size = match dev.buf[7] {
                9 => 512,
                x => x as u16,
            };
            if dev.buf[4] != 0 {
                error!("{}", make_error!(Code::NotImplemented));
                return;
            }
            dev.get_descriptor(2, 0);
        } else if (trb.data[0] >> 8) & 0xff == 6 && (trb.data[0] >> 24)  == 0x02 { // get_descriptor configuration 0
            debug!("get configuration");
            for i in dev.input_ctx.input_control_ctx.data.iter_mut() {
                *i = 0;
            }
            for i in 0..8 {
                dev.input_ctx.slot_ctx.data[i] = dev.device_ctx.slot_ctx.data[i];
            }
            dev.input_ctx.input_control_ctx.data[1] = 1;

            let mut base = 0;
            let max = dev.buf[2] as usize;
            while base < max {
                let ty = dev.buf[base + 1];
                let buf = from_raw_parts_mut(&mut dev.buf[base] as *mut u8, dev.buf[base] as usize);
                debug!("scaning buf: {:?}", buf);
                match ty {
                    2 => { // CONFIGURATION

                    },
                    3 => { // STRING

                    },
                    4 => { // INTERFACE

                    },
                    5 => { // ENDPOINT
                        let dci = (buf[2] & 0b111) * 2 + (buf[2] >> 7);
                        debug!("buf[2] & 0b111: {}, buf[2] >> 7: {}", buf[2] & 0b111, buf[2] >> 7);
                        dev.input_ctx.input_control_ctx.data[1] |= 1 << (dci as u32);
                        let ptr = TR_BUF[self.slot_id() as usize][dci as usize - 1].x.as_ptr() as u64;
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
                        dev.input_ctx.ep_ctx[dci as usize - 1].data = [
                            b_interval << 16,
                            ep_type << 3 | w_max_packet_size << 16 | 3 << 1,
                            (ptr & 0xffffffff) as u32 | 1,
                            (ptr >> 16) as u32,
                            0,
                            0,
                            0,
                            0
                        ];
                    },
                    33 => { // HID

                    },
                    _ => {
                        todo!()
                    }
                }
                base += dev.buf[base] as usize;
            }
            debug!("input context control: {:?}", dev.input_ctx.input_control_ctx.data);
            let ptr = dev.input_ctx.input_control_ctx.data.as_ptr() as u64;
            assert!(ptr & 0x3f == 0);
            CR_BUF.push(TRB {
                data: [
                    (ptr & 0xffffffff) as u32,
                    (ptr >> 32) as u32,
                    0,
                    (self.slot_id() as u32) << 24 | 12 << 10 | 1,
                ]
            });
            xhc.capability.doorbell()[0].ring(0, 0);
        }


        // let setup_trb = TRB {
        //     data: [
        //         0b10000000 | 6 << 8 | 0x0100 << 16,
        //         (self.buf.len() as u32) << 16,
        //         8,
        //         2 << 10 | 3 << 16 | 1 << 6
        //     ]
        // };
        // let ptr = self.buf.as_ptr() as u64;
        // let data_trb = TRB {
        //     data: [
        //         (ptr & 0xffffffff) as u32,
        //         (ptr >> 32) as u32,
        //         (self.buf.len() as u32),
        //         1 << 16 | 1 << 5 | 3 << 10,
        //     ]
        // };
        // let status_trb = TRB {
        //     data: [
        //         0, 0, 0, 4 << 10
        //     ]
        // };
        // unsafe {
        //     TR_BUF[self.slot_id as usize].push(setup_trb);
        //     TR_BUF[self.slot_id as usize].push(data_trb);
        //     TR_BUF[self.slot_id as usize].push(status_trb);
        // }
        // xhc.capability.doorbell()[self.slot_id as usize].ring(1, 0);
    }
}

const TRB_BUF_LEN: usize = 32;

#[repr(C, align(64))]
struct MemPoolCrTRB {
    x: [TRB; TRB_BUF_LEN],
    index: usize,
    cycle: bool
}

impl MemPoolCrTRB {
    pub fn push(&mut self, mut trb: TRB) {
        trb.data[3] = (trb.data[3] & !0x1) | (self.cycle as u32);
        for i in 0..4 {
            self.x[self.index].data[i] = trb.data[i]
        }
        self.index += 1;
        if self.index == TRB_BUF_LEN - 1 {
            let mut link = TRB::new_link_trb(self.x.as_ptr() as u64);
            link.data[3] = link.data[3] | (self.cycle as u32);
            for i in 0..4 {
                self.x[self.index].data[i] = link.data[i];
            }
            self.index = 0;
            self.cycle = !self.cycle;
        }
    }
}

static mut CR_BUF: MemPoolCrTRB = MemPoolCrTRB { x: [TRB { data: [0; 4] }; TRB_BUF_LEN], index: 0, cycle: true };

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

static mut ERSTE_BUF: MemPoolERSTE = MemPoolERSTE { x: [EventRingSegmentTableEntry { addr: 0, size: 0, rsvdz1: 0, rsvdz2: 0 }; 1] };

#[repr(C, align(64))]
struct MemPoolErTRB {
    x: [TRB; TRB_BUF_LEN],
    index: usize,
    cycle: bool,
}

impl MemPoolErTRB {
    pub unsafe fn next(&mut self) -> Option<&TRB> {
        let v = &self.x[self.index];
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
    pub unsafe fn clean(&self, xhc: &XhcController) {
        let interrupt_reg = xhc.capability.runtime().interrupt_set();
        let p = interrupt_reg[0].event_ring_dequeue_pointer.read() & 0xf;
        interrupt_reg[0].event_ring_dequeue_pointer.write(p | (&self.x[self.index] as *const TRB as u64));
    }
}

static mut ER_BUF: MemPoolErTRB = MemPoolErTRB { x: [TRB { data: [0; 4] }; TRB_BUF_LEN], index: 0, cycle: true };

#[derive(Clone, Copy)]
#[repr(C, align(64))]
struct MemPoolTrTRB {
    x: [TRB; TRB_BUF_LEN],
    index: usize,
    cycle: bool,
}

impl MemPoolTrTRB {
    pub fn center(&self) -> &TRB {
        &self.x[self.index]
    }
    pub fn push(&mut self, mut trb: TRB) {
        trb.data[3] = (trb.data[3] & !0x1) | (self.cycle as u32);
        for i in 0..4 {
            self.x[self.index].data[i] = trb.data[i]
        }
        self.index += 1;
        if self.index == TRB_BUF_LEN - 1 {
            let mut link = TRB::new_link_trb(self.x.as_ptr() as u64);
            link.data[3] = link.data[3] | (self.cycle as u32);
            for i in 0..4 {
                self.x[self.index].data[i] = link.data[i];
            }
            self.index = 0;
            self.cycle = !self.cycle;
        }
    }
}

static mut TR_BUF: [[MemPoolTrTRB; 32]; (max_slots_en + 1) as usize] = [
    [
        MemPoolTrTRB {
            x: [TRB { data: [0; 4] }; TRB_BUF_LEN], index: 0, cycle: true
        }; 32
    ]; (max_slots_en + 1) as usize
];


#[repr(C)]
struct InterruptRegister {
    management: RW<u32>,
    moderation: RW<u32>,
    event_ring_segment_table_size: RW<u32>,
    rsvdz1: RW<u32>,
    event_ring_segment_table_base_addr: RW<u64>,
    event_ring_dequeue_pointer: RW<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigPhase {
    NotConnected,
    WaitingAddressed,
    ResettingPort,
    EnablingSlot,
    AddressingDevice,
    InitializingDevice,
    ConfiguringEndpoints,
    Configured,
}

struct XhcController {
    capability: &'static CapabilityRegisters,
    port_config_phase: [ConfigPhase; 256],
    addressing_port: u8,
}

impl XhcController {
    pub unsafe fn initialize(mmio_base: u64) -> XhcController {
        let cap_reg = &*(mmio_base as *const CapabilityRegisters);
        debug!("cap reg: {}", cap_reg.length());

        if cap_reg.hcc_params1.read() & 0b100 != 0 {
            panic!("not surpport 64bit context")
        }

        let ptr = mmio_base + (cap_reg.hcc_params1.read() >> 16 << 2) as u64;
        let ptr = ptr as *mut u32;
        let mut val = ptr;
        loop {
            if *val & 0xff == 1 {
                debug!("bios to os: {:x}", *val);
                if *val >> 24 & 1 == 0 {
                    let v = (val as u64 + 3) as *mut u8;
                    debug!("bios to os: {:x}, {:x}", *val, *v);
                    while *val >> 24 & 1 == 0 || *val >> 16 & 1 == 1 {
                        *v = 1;
                    }
                    debug!("success")
                }
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
        assert!(max_slots >= max_slots_en);

        let config_reg = cap_reg.configure();
        config_reg.set_max_slots_en(max_slots_en);

        let dcbaap = cap_reg.dcbaap();
        let ptr = DCBAA.x.as_ptr() as u64;
        dcbaap.set_dcbaap(ptr);

        let crcr = cap_reg.crcr();
        let ptr = CR_BUF.x.as_ptr() as u64;
        assert!(ptr & 0x3f == 0);
        crcr.set_value(ptr | 1);
        // crcr.set_pointer(ptr);
        // crcr.set_ring_cycle_state(true);

        let ptr = ER_BUF.x.as_ptr() as u64;
        assert!(ptr & 0x3f == 0);
        ERSTE_BUF.x[0].addr = ptr;
        ERSTE_BUF.x[0].size = TRB_BUF_LEN as u16;

        let runtime = cap_reg.runtime();
        let interrupt_regs = runtime.interrupt_set();
        interrupt_regs[0].event_ring_segment_table_size.write(1);
        interrupt_regs[0].event_ring_dequeue_pointer.write(ptr);
        let ptr = ERSTE_BUF.x.as_ptr() as u64;
        assert!(ptr & 0x3f == 0);
        interrupt_regs[0].event_ring_segment_table_base_addr.write(ptr);
        interrupt_regs[0].moderation.write(4000);
        interrupt_regs[0].management.write(0x3);
        usbcmd.set_interrupt_enable(true);
        XhcController {
            capability: cap_reg,
            port_config_phase: [ConfigPhase::NotConnected; 256],
            addressing_port: 0,
        }
    }
    pub fn run(&self) {
        let usbcmd = self.capability.usb_command();
        usbcmd.run();
        let usbsts = self.capability.usb_status();
        while usbsts.hchalted() {}
    }
    pub fn configure_port(&mut self) {
        unsafe {
            let max_ports = self.capability.max_ports();
            for n in 1..=max_ports {
                let port = self.capability.port_sc(n);

                if port.is_connected() {
                    self.reset_port(n);
                }
            }
        }
    }
    // safety: port must be connected
    pub unsafe fn reset_port(&mut self, port_num: u8) {
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
            unsafe {
                CR_BUF.push(TRB::new_enable_slot_trb());
            }
            self.capability.doorbell()[0].ring(0, 0);
        } else {
            error!("{}, {}", port.is_enabled(), port.is_port_reset_changed());
            error!("{}", make_error!(Code::InvalidPhase));
        }
    }

    pub unsafe fn address_deivce(&mut self, slot_id: u8, port_num: u8) {
        let dev = &mut DEVICES_MEM[slot_id as usize];
        dev.slot_id = slot_id;
        dev.doorbell = &mut self.capability.doorbell()[slot_id as usize] as *mut DoorbellRegister;
        DCBAA.x[slot_id as usize] = &dev.device_ctx as *const DeviceContext as u64;
        for d in dev.input_ctx.input_control_ctx.data.iter_mut() {
            *d = 0;
        }
        dev.input_ctx.input_control_ctx.data[1] |= 0b11;

        let port = self.capability.port_sc(port_num);
        dev.input_ctx.slot_ctx.data[0] = ((port.port_speed() as u32) << 20) | 1 << 27;
        dev.input_ctx.slot_ctx.data[1] = (port_num as u32) << 16;

        let max_packet = match port.port_speed() {
            1 | 2 => 8u32,
            3 => 64,
            4 => 512,
            _ => {
                panic!("{}", make_error!(Code::UnknownXHCISpeedID))
            }
        };

        let ptr = &mut TR_BUF[slot_id as usize] as *mut MemPoolTrTRB as u64;
        assert!(ptr & 0x3f == 0);
        dev.input_ctx.ep_ctx[0].data[1] = max_packet << 16 | 4 << 3 | 3 << 1;
        dev.input_ctx.ep_ctx[0].data[2] = (ptr & 0xffffffc0) as u32 | 1;
        dev.input_ctx.ep_ctx[0].data[3] = (ptr >> 32) as u32;

        self.port_config_phase[port_num as usize] = ConfigPhase::AddressingDevice;

        CR_BUF.push(TRB::address_device_command_trb(&dev.input_ctx, slot_id));
        self.capability.doorbell()[0].ring(0, 0);
    }

    pub fn initialize_device(&mut self, slot_id: u8, port_num: u8) {
        self.port_config_phase[port_num as usize] = ConfigPhase::InitializingDevice;
        unsafe { DEVICES_MEM[slot_id as usize].start_init() };
    }

    pub fn process_event(&mut self) {
        while let Some(trb) = unsafe { ER_BUF.next() } {
            let v1 = trb.data[0];
            let v2 = trb.data[1];
            let v3 = trb.data[2];
            let v4 = trb.data[3];
            debug!("{:x} {:x} {:x} {:x}", v1, v2, v3, v4);
            if let Some(casted) = trb.cast::<PortStatusChangeEventTRB>() {
                casted.on_event(self)
            } else if let Some(casted) = trb.cast::<CommandCompletionEventTRB>() {
                if v3 >> 24 != 1 {
                    error!("command completion error: {}", v3 >> 24);
                }
                casted.on_event(self)
            } else if let Some(casted) = trb.cast::<TransferEventTRB>() {
                unsafe {casted.on_event(self)}
            } else {
                error!("{}", make_error!(Code::NotImplemented))
            }
        }
        unsafe { ER_BUF.clean(self) }
    }
}


pub unsafe fn driver_handle_test(mmio_base: u64, device: &Device) {
    let mut xhci = XhcController::initialize(mmio_base);
    xhci.run();
    xhci.configure_port();

    loop {
        xhci.process_event();
        // xhci.capability.doorbell()[1]
    }


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
}

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
