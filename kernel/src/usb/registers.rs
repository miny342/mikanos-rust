use core::ptr::slice_from_raw_parts_mut;
use core::slice::from_raw_parts_mut;

use volatile_register::{RW, RO, WO};


#[repr(C)]
pub struct UsbStatusRegister {
    pub data: RW<u32>
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
pub struct UsbCommandRegister {
    pub data: RW<u32>
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
pub struct ConfigureRegister {
    data: RW<u32>
}

impl ConfigureRegister {
    pub fn set_max_slots_en(&mut self, value: u8) {
        unsafe { self.data.write((self.data.read() & !0xff) | (value as u32)) }
    }
}

#[repr(C)]
pub struct DeviceContextBaseAddressArrayPointerRegister {
    data: RW<u64>
}

impl DeviceContextBaseAddressArrayPointerRegister {
    pub fn set_dcbaap(&mut self, value: u64) {
        assert!(value & 0x3f == 0);
        unsafe { self.data.write(value) }
    }
}

#[repr(C)]
pub struct CommandRingControlRegister {
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
pub struct HostControllerRuntimeRegister {
    data: RW<u32>
}

impl HostControllerRuntimeRegister {
    pub fn interrupt_set(&self) -> &mut [InterruptRegister] {
        unsafe { from_raw_parts_mut(((self as *const HostControllerRuntimeRegister as u64) + 0x20) as *mut InterruptRegister, 1024) }
    }
}

#[repr(C)]
pub struct PortStatusAndControlRegister {
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
        unsafe { self.data.write(val | 0x00220010) }
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
pub struct DoorbellRegister {
    data: RW<u32>
}

impl DoorbellRegister {
    pub fn ring(&mut self, target: u8, stream_id: u16) {
        unsafe { self.data.write((target as u32) | ((stream_id as u32) << 16)) }
    }
}


#[repr(C)]
pub struct CapabilityRegisters {
    // length: u8,
    // rsvdz1: u8,
    // hci_version: u16,
    data: RW<u32>,
    hcs_params1: RW<u32>,
    pub hcs_params2: RW<u32>,
    hcs_params3: RW<u32>,
    pub hcc_params1: RW<u32>,
    doorbell_offset: RW<u32>,
    runtime_reg_offset: RW<u32>,
    hcc_params2: RW<u32>,
}

impl CapabilityRegisters {
    pub fn length(&self) -> u8 {
        (self.data.read() & 0xff) as u8
    }
    pub fn hci_version(&self) -> u16 {
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
    pub fn max_interrupts(&self) -> u16 {
        ((self.hcs_params1.read() >> 8) & 0x3ff) as u16
    }
    pub fn pagesize(&self) -> usize {
        let bit = unsafe { *((self.op_base() + 0x8) as *const u32) };
        1 << (bit.trailing_zeros() as usize + 12)
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

#[repr(C)]
pub struct InterruptRegister {
    pub management: RW<u32>,
    pub moderation: RW<u32>,
    pub event_ring_segment_table_size: RW<u32>,
    rsvdz1: RW<u32>,
    pub event_ring_segment_table_base_addr: RW<u64>,
    pub event_ring_dequeue_pointer: RW<u64>,
}
