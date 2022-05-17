use core::arch::asm;
use heapless::Vec;
use crate::error::*;
use crate::make_error;

const CONFIG_ADDR: u16 = 0x0cf8;
const CONFIG_DATA: u16 = 0x0cfc;

#[repr(C)]
pub struct Device {
    pub bus: u8,
    pub device: u8,
    pub func: u8,
    pub header_type: u8,
}

pub static mut DEVICES: Vec<Device, 32> = Vec::new();

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

unsafe fn add_device(bus: u8, device: u8, func: u8, header_type: u8) -> Result<(), Error> {
    let res = DEVICES.push(Device {bus, device, func, header_type});
    match res {
        Ok(_) => Ok(()),
        Err(_) => Err(make_error!(Code::Full)),
    }
}

unsafe fn scan_func(bus: u8, device: u8, func: u8) -> Result<(), Error> {
    let header_type = read_header_type(bus, device, func);
    add_device(bus, device, func, header_type)?;

    let class_code = read_class_code(bus, device, func);
    let base = ((class_code >> 24) & 0xff) as u8;
    let sub = ((class_code >> 16) & 0xff) as u8;

    if base == 0x06 && sub == 0x04 {
        let bus_numbers = read_bus_numbers(bus, device, func);
        let secondary_bus = ((bus_numbers >> 8) & 0xff) as u8;
        return scan_bus(secondary_bus);
    }

    Ok(())
}

unsafe fn scan_device(bus: u8, device: u8) -> Result<(), Error> {
    scan_func(bus, device, 0)?;

    if is_single_function_device(read_header_type(bus, device, 0)) {
        return Ok(())
    }

    for func in 1u8..8 {
        if read_vendor_id(bus, device, func) == 0xffff {
            continue;
        }
        scan_func(bus, device, func)?;
    }
    Ok(())
}

unsafe fn scan_bus(bus: u8) -> Result<(), Error> {
    for device in 0u8..32 {
        if read_vendor_id(bus, device, 0) == 0xffff {
            continue;
        }
        scan_device(bus, device)?;
    }
    Ok(())
}


pub unsafe fn write_address(addr: u32) {
    asm!(
        "out dx, eax",
        in("dx") CONFIG_ADDR,
        in("eax") addr,
    )
}

pub unsafe fn write_data(value: u32) {
    asm!(
        "out dx, eax",
        in("dx") CONFIG_DATA,
        in("eax") value,
    )
}

pub unsafe fn read_data() -> u32 {
    let ret: u32;
    asm!(
        "in eax, dx",
        in("dx") CONFIG_DATA,
        out("eax") ret,
    );
    ret
}

pub unsafe fn read_vendor_id(bus: u8, device: u8, func: u8) -> u16 {
    write_address(make_address(bus, device, func, 0x00));
    (read_data() & 0xffff) as u16
}

pub unsafe fn read_device_id(bus: u8, device: u8, func: u8) -> u16 {
    write_address(make_address(bus, device, func, 0x00));
    (read_data() >> 16) as u16
}

pub unsafe fn read_header_type(bus: u8, device: u8, func: u8) -> u8 {
    write_address(make_address(bus, device, func, 0x0c));
    ((read_data() >> 16) & 0xff) as u8
}

pub unsafe fn read_class_code(bus: u8, device: u8, func: u8) -> u32 {
    write_address(make_address(bus, device, func, 0x08));
    read_data()
}

pub unsafe fn read_bus_numbers(bus: u8, device: u8, func: u8) -> u32 {
    write_address(make_address(bus, device, func, 0x18));
    read_data()
}

pub fn is_single_function_device(header_type: u8) -> bool {
    (header_type & 0x80) == 0
}

pub fn scan_all_bus() -> Result<(), Error> {
    unsafe {
        let header_type = read_header_type(0, 0, 0);
        if is_single_function_device(header_type) {
            return scan_bus(0);
        }

        for func in 1u8..8 {
            if read_vendor_id(0, 0, func) == 0xffff {
                continue;
            }
            scan_bus(func)?;
        }
        Ok(())
    }
}
