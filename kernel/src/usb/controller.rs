use core::alloc::Layout;
use core::mem::MaybeUninit;

use alloc::vec::Vec;
use futures_util::task::AtomicWaker;

use crate::allocator::SimplestAllocator;
use crate::{debug, make_error, error};
use crate::error::*;

use crate::usb::trb::{
    TRB,
    PortStatusChangeEventTRB,
    CommandCompletionEventTRB,
    TransferEventTRB,
};
use crate::usb::device::{
    ClassDriver, DEVICES_MEM, DeviceContext, DeviceContext64, DeviceContextBaseAddressArray, DeviceContextEnum, InputContext, InputContext64, InputContextEnum, MAX_SLOTS_EN, XhciDevice
};
use crate::usb::memory_pool::{
    MemPoolCrTRB, MemPoolERSTE, MemPoolErTRB, MemPoolTrTRB, TRB_BUF_LEN, TRBTable
};
use crate::usb::registers; 

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigPhase {
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

pub extern "x86-interrupt" fn int_handler_xhci(_frame: crate::interrupt::InterruptFrame) {
    ER_WAKER.wake();
    crate::interrupt::notify_end_of_interrupt();
}

pub static ER_WAKER: AtomicWaker = AtomicWaker::new();

pub struct XhcController {
    pub capability: &'static registers::CapabilityRegisters,
    pub port_config_phase: [ConfigPhase; 256],
    pub addressing_port: u8,
    pub keyboard_handler: fn(modifire: u8, pressing: [u8; 6]),
    pub mouse_handler: fn(modifire: u8, move_x: i8, move_y: i8),
    dcbaa: &'static mut DeviceContextBaseAddressArray,
    pub command_ring: MemPoolCrTRB,
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
        let ptr = allocator.alloc_with_boundary_zeroed(Layout::new::<DeviceContextBaseAddressArray>(), pagesize);
        let dcbaap_ptr = &mut *(ptr as *mut DeviceContextBaseAddressArray); // DCBAA.lock().x.as_ptr() as u64;
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
