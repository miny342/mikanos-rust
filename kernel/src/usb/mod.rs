
#[repr(C, packed)]
struct CapabilityRegisters {
    length: u8,
    reserved: u8,
    hci_version: u16,
    hcs_param1: u32,
    hcs_param2: u32,
    hcs_param3: u32,
    hcc_param1: u32,
    doorbell_offset: u32,
    runtime_reg_offset: u32,
    hcc_params2: u32,
}


use crate::{println, print};

pub unsafe fn driver_handle_test(mmio_base: u64) {
    let cap_reg = &*(mmio_base as *const CapabilityRegisters);
    println!("cap reg: {}", cap_reg.length);

    let op_base = mmio_base + cap_reg.length as u64;

    let usbsts = &*((op_base + 0x04) as *const u32);

    if usbsts & 0x1 == 0 { // assert USBSTS.HCH == 1
        panic!();
    }

    let usbcmd_addr = op_base as *mut u32;
    *(usbcmd_addr) = *(usbcmd_addr) | 0x02;  // USBCMD.HCRST = 1

    while *(usbcmd_addr) & 0x02 != 0 {}  // wait HCRST == 0

    while usbsts & 0x800 != 0 {}  // wait USBSTS.CNR == 0

    println!("USBSTS.CNR is 0! ready!");

}
