use core::intrinsics::transmute;
use core::ptr::slice_from_raw_parts_mut;
use core::slice::from_raw_parts_mut;

use volatile_register::RW;

use crate::{println, print, debug, log, make_error, error};
use crate::pci::{
    Device, read_config_reg
};
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
    pub fn set_ring_cycle_state(&mut self, value: bool) {
        unsafe { self.data.write((self.data.read() & !0x1) | (value as u64)) }
    }
    pub fn set_pointer(&mut self, value: u64) {
        assert!(value & 0x3f == 0);
        unsafe { self.data.write((self.data.read() & 0x3f) | value) }
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
        while !self.is_enabled() {}
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
struct SlotContext {
    data: [u32; 8]
}

#[derive(Debug, Clone, Copy)]
struct EndpointContext {
    data: [u32; 8]
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct DeviceContext {
    slot_ctx: SlotContext,
    ep_ctx: [EndpointContext; 31]
}

#[derive(Debug, Clone, Copy)]
struct InputControlContext {
    data: [u32; 8]
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct InputContext {
    input_control_ctx: InputControlContext,
    slot_ctx: SlotContext,
    ep_ctx: [EndpointContext; 31]
}

#[derive(Debug, Clone, Copy)]
#[repr(align(4096))]
struct XhciDevice {
    device_ctx: DeviceContext,
    input_ctx: InputContext,
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
        }
    }; (max_slots_en + 1) as usize
];

trait TRBtrait {
    const TY: u32;
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
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
    pub fn new_link_trb() -> TRB {
        let ptr = unsafe { CR_BUF.x.as_ptr() as u64 };
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
                error!("{}", make_error!(Code::InvalidPhase));
            }
        }
    }
}

#[repr(C, packed)]
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
        }
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
            let mut link = TRB::new_link_trb();
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
        unsafe { interrupt_reg[0].event_ring_dequeue_pointer.write(p | (&self.x[self.index] as *const TRB as u64)) };
    }
}

static mut ER_BUF: MemPoolErTRB = MemPoolErTRB { x: [TRB { data: [0; 4] }; TRB_BUF_LEN], index: 0, cycle: true };

#[repr(align(4096))]  // this is PAGESIZE = 1
struct MemPoolDeviceCtx {

}

#[repr(C)]
struct InterruptRegister {
    management: RW<u32>,
    moderation: RW<u32>,
    event_ring_segment_table_size: RW<u32>,
    rsvdz1: RW<u32>,
    event_ring_segment_table_base_addr: RW<u64>,
    event_ring_dequeue_pointer: RW<u64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
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

        let usbsts = cap_reg.usb_status(); // &*((op_base + 0x04) as *const u32);

        assert!(usbsts.hchalted());  // assert USBSTS.HCH == 1

        let usbcmd = cap_reg.usb_command();
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
        crcr.set_pointer(ptr);
        crcr.set_ring_cycle_state(true);

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
            _ => {}
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
        }
    }

    pub unsafe fn address_deivce(&mut self, slot_id: u8, port_num: u8) {
        let dev = &mut DEVICES_MEM[slot_id as usize];
        DCBAA.x[slot_id as usize] = &dev.device_ctx as *const DeviceContext as u64;
        for d in dev.input_ctx.input_control_ctx.data.iter_mut() {
            *d = 0;
        }
        dev.input_ctx.input_control_ctx.data[1] |= 3;

        let port = self.capability.port_sc(port_num);
        dev.input_ctx.slot_ctx.data[0] = (dev.input_ctx.slot_ctx.data[0] & 0xff0fffff) | ((port.port_speed() as u32) << 20);
        dev.input_ctx.slot_ctx.data[1] = (dev.input_ctx.slot_ctx.data[1] & 0xff00ffff) | ((port_num as u32) << 16);

        let max_packet = match port.port_speed() {
            1 | 2 => 8u32,
            3 => 64,
            4 => 512,
            _ => panic!()
        };

        dev.input_ctx.ep_ctx[0].data[1] = max_packet << 16 | 4 << 3 | 3 << 1;
        dev.input_ctx.ep_ctx[0].data[2] = 1;

        self.port_config_phase[port_num as usize] = ConfigPhase::AddressingDevice;

        CR_BUF.push(TRB::address_device_command_trb(&dev.input_ctx as *const InputContext, slot_id));
        self.capability.doorbell()[0].ring(0, 0);
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
                casted.on_event(self)
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
