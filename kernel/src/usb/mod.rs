use core::slice::from_raw_parts_mut;

use crate::{println, print};
use crate::pci::{
    Device, read_config_reg
};
use crate::error::*;

#[repr(C, packed)]
struct CapabilityRegisters {
    length: u8,
    reserved: u8,
    hci_version: u16,
    hcs_params1: u32,
    hcs_params2: u32,
    hcs_params3: u32,
    hcc_params1: u32,
    doorbell_offset: u32,
    runtime_reg_offset: u32,
    hcc_params2: u32,
}

const max_slots_en: u8 = 8;

#[repr(align(64))]
struct MemPool {
    x: [u64; max_slots_en as usize + 1]
}

static mut DCBAA: MemPool = MemPool { x: [0; max_slots_en as usize + 1] };

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct TRB {
    data: [u32; 4]
}

#[repr(align(64))]
struct MemPoolTRB {
    x: [TRB; 32]
}

static mut CR_BUF: MemPoolTRB = MemPoolTRB { x: [TRB { data: [0; 4] }; 32] };
static mut CR_CYCLE: bool = true;

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

static mut ER_BUF: MemPoolTRB = MemPoolTRB { x: [TRB { data: [0; 4] }; 32] };
static mut ER_CYCLE: bool = true;

#[repr(align(4096))]  // this is PAGESIZE = 1
struct MemPoolDeviceCtx {

}

#[repr(C, packed)]
struct InterruptRegister {
    management: u32,
    moderation: u32,
    event_ring_segment_table_size: u32,
    rsvdz1: u32,
    event_ring_segment_table_base_addr: u64,
    event_ring_dequeue_pointer: u64,
}

pub unsafe fn driver_handle_test(mmio_base: u64, device: &Device) {
    let cap_reg = &*(mmio_base as *const CapabilityRegisters);
    println!("cap reg: {}", cap_reg.length);

    let op_base = mmio_base + cap_reg.length as u64;

    let usbsts = &*((op_base + 0x04) as *const u32);

    assert!(usbsts & 0x1 == 1);  // assert USBSTS.HCH == 1

    let usbcmd_addr = op_base as *mut u32;
    *usbcmd_addr = *usbcmd_addr | 0x02;  // USBCMD.HCRST = 1

    while *usbcmd_addr & 0x02 != 0 {}  // wait HCRST == 0

    while usbsts & 0x800 != 0 {}  // wait USBSTS.CNR == 0

    println!("USBSTS.CNR is 0! ready!");


    let max_slots = (cap_reg.hcs_params1 & 0xff) as u8;

    println!("max slots: {}", max_slots);
    assert!(max_slots >= max_slots_en);

    let config_reg_addr = (op_base + 0x38) as *mut u32;
    *config_reg_addr = *config_reg_addr | max_slots_en as u32;

    let dcbaap_addr = (op_base + 0x30) as *mut u64;
    let ptr = DCBAA.x.as_ptr() as u64;
    assert!(ptr & 0x3f == 0);
    *dcbaap_addr = ptr;

    let crcr_addr = (op_base + 0x18) as *mut u64;
    let ptr = CR_BUF.x.as_ptr() as u64;
    assert!(ptr & 0x3f == 0);
    *crcr_addr = ptr | 0x1;

    let ptr = ER_BUF.x.as_ptr() as u64;
    assert!(ptr & 0x3f == 0);
    ERSTE_BUF.x[0].addr = ptr;
    ERSTE_BUF.x[0].size = 32;

    let runtime_base = mmio_base + cap_reg.runtime_reg_offset as u64;
    let interrupt_regs = from_raw_parts_mut((runtime_base + 0x20) as *mut InterruptRegister, 1024);
    interrupt_regs[0].event_ring_segment_table_size = 1;
    interrupt_regs[0].event_ring_dequeue_pointer = ptr;
    let ptr = ERSTE_BUF.x.as_ptr() as u64;
    assert!(ptr & 0x3f == 0);
    interrupt_regs[0].event_ring_segment_table_base_addr = ptr;
    interrupt_regs[0].moderation = 4000;
    interrupt_regs[0].management = 0x3;
    *usbcmd_addr = *usbcmd_addr | 0x4;
    // initialized

    // interrupt setting?

    // xhc start
    *usbcmd_addr = *usbcmd_addr | 0x1;
    while usbsts & 0x1 == 1 {}

    println!("xhc started");


    // let local_apic_id = *(0xfee00020 as *const u32) >> 24;
    // let msi_msg_addr = 0xfee00000 | (local_apic_id << 12);
    // let msi_msg_data = 0xc000u32 | 0x40;

    let max_ports = (cap_reg.hcs_params1 >> 24) as u8;
    for n in 0..max_ports {
        let addr = (op_base + 0x400 + (0x10 * n as u64)) as *mut u32;
        let connected = *addr & 0x1 == 0x1;  // ccs == 1

        println!("port {}: {}", n + 1, connected);
        if !connected {
            continue;
        }
        let val = *addr & 0x0e00c3e0;
        *addr = val | 0x20010;   // portsc.pr = 1 && portsc.csc = 1
        while *addr & 0x10 != 0 {}
    }

    println!("device reset");

    let addr = (op_base + 0x8) as *const u32;
    println!("pagesize: {:b}", *addr);


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
