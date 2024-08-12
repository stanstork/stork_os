use alloc::vec::Vec;
use device::PciDevice;

use crate::{
    cpu::io::{inl, outl},
    println,
};

pub mod class;
pub mod device;
pub mod vendor;

// Constants for PCI configuration space access
pub const CONFIG_ADDRESS: u16 = 0xCF8;
pub const CONFIG_DATA: u16 = 0xCFC;
pub const PCI_ENABLE_BIT: u32 = 0x80000000;
pub const MAX_BUS: u8 = 255;
pub const MAX_DEVICE: u8 = 31;
pub const MAX_FUNCTION: u8 = 7;
pub const HEADER_TYPE_MULTIFUNCTION: u16 = 0x80;
pub const HEADER_TYPE_MASK: u16 = 0x7F;
pub const INVALID_VENDOR_ID: u16 = 0xFFFF;
pub const PCI_CLASS_CODE_OFFSET: u8 = 0x0B;
pub const PCI_SUBCLASS_CODE_OFFSET: u8 = 0x0A;
pub const PCI_PROG_IF_OFFSET: u8 = 0x09;
pub const PCI_HEADER_TYPE_OFFSET: u8 = 0x0E;
pub const PCI_SLOT_SHIFT: u8 = 11;
pub const PCI_FUNC_SHIFT: u8 = 8;
pub const PCI_OFFSET_MASK: u32 = 0xFC;

pub static mut DEVICES: Vec<PciDevice> = Vec::new();

pub struct PCI {}

impl PCI {
    fn read_word(bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
        let address = PCI_ENABLE_BIT
            | ((bus as u32) << 16)
            | ((slot as u32) << PCI_SLOT_SHIFT)
            | ((func as u32) << PCI_FUNC_SHIFT)
            | ((offset as u32) & PCI_OFFSET_MASK);

        outl(CONFIG_ADDRESS, address);
        // Read the data from the configuration data port.
        // The data is read as a 32-bit value, so we need to shift the data to the right
        // to get the correct value.
        // // (offset & 2) * 8) = 0 will choose the first word of the 32-bit register
        let data = ((inl(CONFIG_DATA) >> ((offset & 2) * 8)) & 0xFFFF) as u16;
        data
    }

    fn read_dword(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        let address = PCI_ENABLE_BIT
            | ((bus as u32) << 16)
            | ((slot as u32) << PCI_SLOT_SHIFT)
            | ((func as u32) << PCI_FUNC_SHIFT)
            | ((offset as u32) & PCI_OFFSET_MASK);

        outl(CONFIG_ADDRESS, address);
        inl(CONFIG_DATA)
    }

    fn write_word(bus: u8, slot: u8, func: u8, offset: u8, data: u16) {
        let address = PCI_ENABLE_BIT
            | ((bus as u32) << 16)
            | ((slot as u32) << PCI_SLOT_SHIFT)
            | ((func as u32) << PCI_FUNC_SHIFT)
            | ((offset as u32) & PCI_OFFSET_MASK);

        outl(CONFIG_ADDRESS, address);
        let current = inl(CONFIG_DATA);
        let mask = 0xFFFF << ((offset & 2) * 8);
        let new = (current & !mask) | ((data as u32) << ((offset & 2) * 8));
        outl(CONFIG_DATA, new);
    }

    pub fn scan() {
        for bus in 0..=MAX_BUS {
            for device in 0..MAX_DEVICE {
                PCI::check_device(bus, device);
            }
        }
    }

    fn check_device(bus: u8, device: u8) {
        let vendor_id = PCI::get_vendor_id(bus, device);
        if vendor_id == INVALID_VENDOR_ID {
            return;
        }

        PCI::check_function(bus, device, 0);
        let header_type = PCI::read_word(bus, device, 0, PCI_HEADER_TYPE_OFFSET) & HEADER_TYPE_MASK;
        if header_type & HEADER_TYPE_MULTIFUNCTION != 0 {
            for function in 1..MAX_FUNCTION {
                let vendor_id = PCI::get_vendor_id(bus, device);
                if vendor_id != INVALID_VENDOR_ID {
                    PCI::check_function(bus, device, function);
                }
            }
        }
    }

    fn get_vendor_id(bus: u8, device: u8) -> u16 {
        PCI::read_word(bus, device, 0, 0)
    }

    fn check_function(bus: u8, device: u8, function: u8) {
        let vendor_id = PCI::get_vendor_id(bus, device);
        let device_id = PCI::read_word(bus, device, function, 2);
        let (class_code, sub_class_code, prog_if) = PCI::get_device_type(bus, device, function);

        let vendor_name = vendor::get_vendor_name(vendor_id);
        let class_info = class::get_class_code_info(class_code, sub_class_code, prog_if);

        if let Some(class_info) = class_info {
            println!(
                "PCI: {:?} {:?} {:?} {:?} device_id: {:X}",
                vendor_name.unwrap(),
                class_info.base_desc,
                class_info.sub_desc,
                class_info.prog_desc,
                device_id
            );
            unsafe {
                DEVICES.push(PciDevice {
                    bus,
                    device,
                    function,
                    vendor_id,
                    device_id,
                    revision: 0,
                    prog_if,
                    class: class_code,
                    subclass: sub_class_code,
                });
            }
        }

        // Additional processing depending on device type
        if class_code == 0x06 && sub_class_code == 0x04 {
            let secondary_bus = PCI::read_word(bus, device, function, 0x18) as u8;
            PCI::check_bus(secondary_bus);
        }
    }

    fn get_device_type(bus: u8, device: u8, function: u8) -> (u8, u8, u8) {
        let class_code = (PCI::read_word(bus, device, function, PCI_CLASS_CODE_OFFSET) >> 8) as u8;
        let sub_class_code =
            (PCI::read_word(bus, device, function, PCI_SUBCLASS_CODE_OFFSET) & 0xFF) as u8;
        let prog_if = (PCI::read_word(bus, device, function, PCI_PROG_IF_OFFSET) & 0xFF) as u8;

        (class_code, sub_class_code, prog_if)
    }

    fn check_bus(bus: u8) {
        for device in 0..MAX_DEVICE {
            PCI::check_device(bus, device);
        }
    }

    pub fn search_device(class: u8, subclass: u8) -> Option<PciDevice> {
        for device in unsafe { DEVICES.iter() } {
            if device.class == class && device.subclass == subclass {
                return Some(*device);
            }
        }
        None
    }

    pub fn enable_interrupt_line(bus: u8, device: u8, function: u8) {
        let command_offset = 0x04;
        let interrupt_enable_bit = 0x400;

        let command = PCI::read_word(bus, device, function, command_offset);

        PCI::write_word(
            bus,
            device,
            function,
            command_offset,
            command | interrupt_enable_bit,
        );
    }

    pub fn enable_bus_mastering(bus: u8, device: u8, function: u8) {
        let command_offset = 0x04;
        let bus_master_bit = 0x4;

        let command = PCI::read_word(bus, device, function, command_offset);

        PCI::write_word(
            bus,
            device,
            function,
            command_offset,
            command | bus_master_bit,
        );
    }
}
