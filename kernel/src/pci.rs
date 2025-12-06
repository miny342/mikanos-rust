use core::arch::asm;
use alloc::vec::Vec;
use spin::Mutex;
use crate::error::*;
use crate::make_error;
use crate::io_port::{ind, outd};

const CONFIG_ADDR: u16 = 0x0cf8;
const CONFIG_DATA: u16 = 0x0cfc;

#[derive(Debug)]
pub struct Device {
    bus: u8,
    device: u8,
    func: u8,
    header_type: u8,
    class_code: ClassCode,
}

impl Device {
    unsafe fn new(bus: u8, device: u8, func: u8, header_type: u8, class_code: ClassCode) -> Self {
        Device { bus, device, func, header_type, class_code }
    }
    pub fn read_vendor_id(&self) -> u16 {
        unsafe { read_vendor_id(self.bus, self.device, self.func) }
    }
    pub fn bus(&self) -> u8 {
        self.bus
    }
    pub fn device(&self) -> u8 {
        self.device
    }
    pub fn func(&self) -> u8 {
        self.func
    }
    pub fn header_type(&self) -> u8 {
        self.header_type
    }
    pub fn class_code(&self) -> ClassCode {
        self.class_code
    }
    pub unsafe fn read_config_reg(&self, reg_addr: u8) -> u32 {
        unsafe {
            read_config_reg(self, reg_addr)
        }
    }
    pub unsafe fn write_config_reg(&self, reg_addr: u8, value: u32) {
        unsafe {
            write_config_reg(self, reg_addr, value);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ClassCode {
    base: u8,
    sub: u8,
    interface: u8,
}

impl ClassCode {
    pub fn match1(&self, b: u8) -> bool {
        b == self.base
    }
    pub fn match2(&self, b: u8, s: u8) -> bool {
        self.match1(b) && s == self.sub
    }
    pub fn match3(&self, b: u8, s: u8, i: u8) -> bool {
        self.match2(b, s) && i == self.interface
    }
}

fn make_address(bus: u8, device: u8, func: u8, reg_addr: u8) -> u32 {
    let shl = |x: u32, bits: u32| {
        x << bits
    };
    return shl(1, 31)
        | shl(bus as u32, 16)
        | shl(device as u32, 11)
        | shl(func as u32, 8)
        | (reg_addr as u32 & 0xfc);
}

unsafe fn add_device(v: &mut Vec<Device>, bus: u8, device: u8, func: u8, header_type: u8, class_code: ClassCode) -> Result<(), Error> {
    match v.try_reserve(1) {
        Ok(_) => {
            v.push(unsafe { Device::new(bus, device, func, header_type, class_code) });
            Ok(())
        },
        Err(_) => Err(make_error!(Code::Full)),
    }
}

unsafe fn scan_func(v: &mut Vec<Device>, bus: u8, device: u8, func: u8) -> Result<(), Error> {
    unsafe {
        let class_code = read_class_code(bus, device, func);
        let header_type = read_header_type(bus, device, func);
        add_device(v, bus, device, func, header_type, class_code)?;

        if class_code.match2(0x06, 0x04) {
            let bus_numbers = read_bus_numbers(bus, device, func);
            let secondary_bus = ((bus_numbers >> 8) & 0xff) as u8;
            return scan_bus(v, secondary_bus);
        }
    }
    Ok(())
}

unsafe fn scan_device(v: &mut Vec<Device>, bus: u8, device: u8) -> Result<(), Error> {
    unsafe {
        scan_func(v, bus, device, 0)?;

        if is_single_function_device(read_header_type(bus, device, 0)) {
            return Ok(())
        }

        for func in 1u8..8 {
            if read_vendor_id(bus, device, func) == 0xffff {
                continue;
            }
            scan_func(v, bus, device, func)?;
        }
    }
    Ok(())
}

unsafe fn scan_bus(v: &mut Vec<Device>, bus: u8) -> Result<(), Error> {
    unsafe {
        for device in 0u8..32 {
            if read_vendor_id(bus, device, 0) == 0xffff {
                continue;
            }
            scan_device(v, bus, device)?;
        }
    }
    Ok(())
}


pub unsafe fn write_address(addr: u32) {
    unsafe { outd(CONFIG_ADDR, addr) };
}

pub unsafe fn write_data(value: u32) {
    unsafe { outd(CONFIG_DATA, value) };
}

pub unsafe fn read_data() -> u32 {
    unsafe { ind(CONFIG_DATA) }
}

pub unsafe fn read_vendor_id(bus: u8, device: u8, func: u8) -> u16 {
    unsafe {
        write_address(make_address(bus, device, func, 0x00));
        (read_data() & 0xffff) as u16
    }
}

pub unsafe fn read_device_id(bus: u8, device: u8, func: u8) -> u16 {
    unsafe {
        write_address(make_address(bus, device, func, 0x00));
        (read_data() >> 16) as u16
    }
}

pub unsafe fn read_header_type(bus: u8, device: u8, func: u8) -> u8 {
    unsafe {
        write_address(make_address(bus, device, func, 0x0c));
        ((read_data() >> 16) & 0xff) as u8
    }
}

pub unsafe fn read_class_code(bus: u8, device: u8, func: u8) -> ClassCode {
    unsafe {
        write_address(make_address(bus, device, func, 0x08));
    }
    let reg = unsafe { read_data() };
    ClassCode {
        base: ((reg >> 24) & 0xff) as u8,
        sub: ((reg >> 16) & 0xff) as u8,
        interface: ((reg >> 8) & 0xff) as u8
    }
}

pub unsafe fn read_bus_numbers(bus: u8, device: u8, func: u8) -> u32 {
    unsafe {
        write_address(make_address(bus, device, func, 0x18));
        read_data()
    }
}

pub fn is_single_function_device(header_type: u8) -> bool {
    (header_type & 0x80) == 0
}

pub fn scan_all_bus() -> Result<Vec<Device>, (Vec<Device>, Error)> {
    let mut v = Vec::new();
    unsafe {
        let header_type = read_header_type(0, 0, 0);
        if is_single_function_device(header_type) {
            if let Some(e) = scan_bus(&mut v, 0).err() {
                return Err((v, e))
            }
            return Ok(v)
        }

        for func in 0u8..8 {
            if read_vendor_id(0, 0, func) == 0xffff {
                continue;
            }
            if let Some(e) = scan_bus(&mut v, func).err() {
                return Err((v, e))
            }
        }
        Ok(v)
    }
}

pub unsafe fn read_config_reg(dev: &Device, reg_addr: u8) -> u32 {
    unsafe {
        write_address(make_address(dev.bus, dev.device, dev.func, reg_addr));
        read_data()
    }
}

pub unsafe fn write_config_reg(dev: &Device, reg_addr: u8, value: u32) {
    unsafe {
        write_address(make_address(dev.bus, dev.device, dev.func, reg_addr));
        write_data(value);
    }
}

const fn calc_bar_address(bar_index: u32) -> u8 {
    ((0x10 + 4 * bar_index) & 0xff) as u8
}

pub unsafe fn read_bar(dev: &Device, bar_index: u32) -> Result<u64, Error> {
    if bar_index >= 6 {
        return Err(make_error!(Code::IndexOutOfRange));
    }
    let addr = calc_bar_address(bar_index);
    let bar = unsafe { read_config_reg(dev, addr) };

    if bar & 4 == 0 {
        return Ok(bar as u64)
    }

    if bar_index >= 5 {
        return Err(make_error!(Code::IndexOutOfRange))
    }

    let bar_upper = unsafe { read_config_reg(dev, addr + 4) as u64 };
    Ok(bar as u64 | bar_upper << 32)
}

struct CapabilityHeader {
    data: u32
}

impl CapabilityHeader {
    fn cap_id(&self) -> u32 {
        self.data & 0xff
    }
    fn next_ptr(&self) -> u32 {
        (self.data >> 8) & 0xff
    }
    fn _cap(&self) -> u32 {
        (self.data >> 16) & 0xffff
    }
}

const CAPABILITY_MSI: u8 = 0x05;
const CAPABILITY_MSIX: u8 = 0x11;

unsafe fn read_capability_header(dev: &Device, addr: u8) -> CapabilityHeader {
    CapabilityHeader {
        data: unsafe { read_config_reg(dev, addr) }
    }
}

struct MSICapability {
    data: u32,
    msg_addr: u32,
    msg_upper_addr: u32,
    msg_data: u32,
    mask_bits: u32,
    pending_bits: u32,
}

impl MSICapability {
    fn _cap_id(&self) -> u32 {
        self.data & 0xff
    }
    fn _next_ptr(&self) -> u32 {
        (self.data >> 8) & 0xff
    }
    fn _msi_enable(&self) -> u32 {
        (self.data >> 16) & 0x1
    }
    fn set_msi_enable(&mut self, value: u32) {
        self.data = (self.data & !(0x1 << 16)) | value << 16;
    }
    fn multi_msg_capable(&self) -> u32 {
        (self.data >> 17) & 0b111
    }
    fn _multi_msg_enable(&self) -> u32 {
        (self.data >> 20) & 0x111
    }
    fn set_multi_msg_enable(&mut self, value: u32) {
        self.data = (self.data & !(0x111 << 20)) | value << 20;
    }
    fn addr_64_capable(&self) -> u32 {
        (self.data >> 23) & 0x1
    }
    fn per_vector_mask_capable(&self) -> u32 {
        (self.data >> 24) & 0x1
    }
}

unsafe fn read_msi_capability(dev: &Device, cap_addr: u8) -> MSICapability {
    let mut msi_cap = MSICapability {
        data: 0,
        msg_addr: 0,
        msg_upper_addr: 0,
        msg_data: 0,
        mask_bits: 0,
        pending_bits: 0
    };
    unsafe {
        msi_cap.data = read_config_reg(dev, cap_addr);
        msi_cap.msg_addr = read_config_reg(dev, cap_addr + 4);

        let mut msg_data_addr = cap_addr + 8;
        if msi_cap.addr_64_capable() != 0 {
            msi_cap.msg_upper_addr = read_config_reg(dev, cap_addr + 8);
            msg_data_addr = cap_addr + 12;
        }

        msi_cap.msg_data = read_config_reg(dev, msg_data_addr);

        if msi_cap.per_vector_mask_capable() != 0 {
            msi_cap.mask_bits = read_config_reg(dev, msg_data_addr + 4);
            msi_cap.pending_bits = read_config_reg(dev, msg_data_addr + 8);
        }
    }

    msi_cap
}

unsafe fn write_msi_capability(dev: &Device, cap_addr: u8, msi_cap: &MSICapability) {
    unsafe {
        write_config_reg(dev, cap_addr, msi_cap.data);
        write_config_reg(dev, cap_addr + 4, msi_cap.msg_addr);

        let mut msg_data_addr = cap_addr + 8;
        if msi_cap.addr_64_capable() != 0 {
            write_config_reg(dev, cap_addr + 8, msi_cap.msg_upper_addr);
            msg_data_addr = cap_addr + 12;
        }

        write_config_reg(dev, msg_data_addr, msi_cap.msg_data);

        if msi_cap.per_vector_mask_capable() != 0 {
            write_config_reg(dev, msg_data_addr + 4, msi_cap.mask_bits);
            write_config_reg(dev, msg_data_addr + 8, msi_cap.pending_bits);
        }
    }
}

unsafe fn configure_msi_register(dev: &Device, cap_addr: u8, msg_addr: u32, msg_data: u32, num_vector_exponent: u32) {
    let mut msi_cap = unsafe { read_msi_capability(dev, cap_addr) };

    if msi_cap.multi_msg_capable() <= num_vector_exponent {
        msi_cap.set_multi_msg_enable(msi_cap.multi_msg_capable())
    } else {
        msi_cap.set_multi_msg_enable(num_vector_exponent)
    }

    msi_cap.set_msi_enable(1);
    msi_cap.msg_addr = msg_addr;
    msi_cap.msg_data = msg_data;

    unsafe {
        write_msi_capability(dev, cap_addr, &msi_cap);
    }
}

unsafe fn configure_msi(dev: &Device, msg_addr: u32, msg_data: u32, num_vector_exponent: u32) {
    let mut cap_addr = unsafe { (read_config_reg(dev, 0x34) & 0xff) as u8 };
    let mut msi_cap_addr = 0;
    let mut msix_cap_addr = 0;
    while cap_addr != 0 {
        let header = unsafe { read_capability_header(dev, cap_addr) };
        if header.cap_id() as u8 == CAPABILITY_MSI {
            msi_cap_addr = cap_addr;
        } else if header.cap_id() as u8 == CAPABILITY_MSIX {
            msix_cap_addr = cap_addr;
        }
        cap_addr = header.next_ptr() as u8;
    }

    if msi_cap_addr != 0 {
        return unsafe {
            configure_msi_register(dev, msi_cap_addr, msg_addr, msg_data, num_vector_exponent)
        }
    }
    if msix_cap_addr != 0 {
        todo!("msix is not supported");
    }
    todo!("not found msi");
}

#[derive(PartialEq, Eq)]
pub enum MSITriggerMode {
    Edge = 0,
    Level = 1,
}

pub enum MSIDeliveryMode {
    Fixed = 0b000,
    LowestPriority = 0b001,
    SMI = 0b010,
    NMI = 0b100,
    INIT = 0b101,
    ExtINT = 0b111,
}

pub unsafe fn configure_msi_fixed_destination(dev: &Device, apic_id: u8, trigger_mode: MSITriggerMode, delivery_mode: MSIDeliveryMode, vector: u8, num_vector_exponent: u32) {
    let msg_addr = 0xfee00000 | ((apic_id as u32) << 12);
    let mut msg_data = (delivery_mode as u32) << 8 | vector as u32;
    if trigger_mode == MSITriggerMode::Level {
        msg_data |= 0xc000
    }
    unsafe {
        configure_msi(dev, msg_addr, msg_data, num_vector_exponent);
    }
}
