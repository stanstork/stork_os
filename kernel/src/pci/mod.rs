use crate::{
    cpu::io::{inl, outl},
    println,
};
use device_manager::add_device;
use pci_class::PciClassCodeInfo;
use pci_device::PciDevice;

pub mod device_manager;
pub mod pci_class;
pub mod pci_device;
pub mod pci_vendor;

// Constants for PCI configuration space access
const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;
const PCI_ENABLE_BIT: u32 = 0x80000000;
const MAX_PCI_BUS: u8 = 255;
const MAX_PCI_DEVICE: u8 = 31;
const MAX_FUNCTION: u8 = 7;
const HEADER_TYPE_MULTIFUNCTION: u16 = 0x80;
const HEADER_TYPE_MASK: u16 = 0x7F;
const INVALID_VENDOR_ID: u16 = 0xFFFF;
const PCI_CLASS_CODE_OFFSET: u8 = 0x0B;
const PCI_SUBCLASS_CODE_OFFSET: u8 = 0x0A;
const PCI_PROG_IF_OFFSET: u8 = 0x09;
const PCI_HEADER_TYPE_OFFSET: u8 = 0x0E;
const PCI_SLOT_SHIFT: u8 = 11;
const PCI_FUNC_SHIFT: u8 = 8;
const PCI_OFFSET_MASK: u32 = 0xFC;

pub struct PCI {}

impl PCI {
    pub fn scan_pci_bus() {
        for bus in 0..=MAX_PCI_BUS {
            for device in 0..MAX_PCI_DEVICE {
                PCI::inspect_device(bus, device);
            }
        }
    }

    // Enables the interrupt line by setting the appropriate bit in the command register
    pub fn enable_interrupt_line(bus: u8, device: u8, function: u8) {
        let interrupt_enable_bit = 0x400; // Interrupt enable bit mask
        Self::modify_command_register(bus, device, function, interrupt_enable_bit);
    }

    // Enables bus mastering by setting the appropriate bit in the command register
    pub fn enable_bus_mastering(bus: u8, device: u8, function: u8) {
        let bus_master_bit = 0x4; // Bus mastering enable bit mask
        Self::modify_command_register(bus, device, function, bus_master_bit);
    }

    fn inspect_device(bus: u8, device: u8) {
        let vendor_id = Self::get_vendor_id(bus, device);

        // Check if device is present by verifying the vendor ID
        if vendor_id == INVALID_VENDOR_ID {
            return;
        }

        // Check the primary function (function 0)
        Self::check_function(bus, device, 0);

        // Read header type to check for multifunction devices
        let header_type =
            Self::read_word(bus, device, 0, PCI_HEADER_TYPE_OFFSET) & HEADER_TYPE_MASK;

        if header_type & HEADER_TYPE_MULTIFUNCTION != 0 {
            // If the device is multifunction, iterate over possible functions
            for function in 1..MAX_FUNCTION {
                let vendor_id = Self::get_vendor_id(bus, device);

                // If function is valid, check it
                if vendor_id != INVALID_VENDOR_ID {
                    Self::check_function(bus, device, function);
                }
            }
        }
    }

    fn check_function(bus: u8, device: u8, function: u8) {
        // Read the vendor ID and device ID
        let vendor_id = Self::get_vendor_id(bus, device);
        let device_id = Self::read_word(bus, device, function, 2);

        // Fetch the device type information (class, subclass, programming interface)
        let (class_code, sub_class_code, prog_if) =
            Self::get_device_class_info(bus, device, function);

        let vendor_name = pci_vendor::get_vendor_name(vendor_id);
        let class_info = pci_class::get_class_code_info(class_code, sub_class_code, prog_if);

        // Print the device information
        Self::print_device_info(vendor_name.unwrap(), device_id, &class_info.unwrap());

        // Create a new PCI device and add it to the list of devices
        let pci_device = PciDevice {
            bus,
            device,
            function,
            vendor_id,
            device_id,
            class: class_code,
            subclass: sub_class_code,
            prog_if,
            revision: 0,
        };
        add_device(pci_device);

        // Check for bridges (PCI-to-PCI) and scan the secondary bus if found
        if class_code == 0x06 && sub_class_code == 0x04 {
            let secondary_bus = Self::read_word(bus, device, function, 0x18) as u8;
            Self::scan_secondary_bus(secondary_bus);
        }
    }

    fn get_vendor_id(bus: u8, device: u8) -> u16 {
        PCI::read_word(bus, device, 0, 0)
    }

    // Helper function to construct the address
    fn construct_address(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        PCI_ENABLE_BIT
            | ((bus as u32) << 16)
            | ((slot as u32) << PCI_SLOT_SHIFT)
            | ((func as u32) << PCI_FUNC_SHIFT)
            | ((offset as u32) & PCI_OFFSET_MASK)
    }

    fn read_word(bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
        let address = Self::construct_address(bus, slot, func, offset);
        outl(CONFIG_ADDRESS, address);

        // Read and shift the data to get the correct word
        let data = ((inl(CONFIG_DATA) >> ((offset & 2) * 8)) & 0xFFFF) as u16;
        data
    }

    fn read_dword(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        let address = Self::construct_address(bus, slot, func, offset);
        outl(CONFIG_ADDRESS, address);

        // Read the data as a 32-bit value
        inl(CONFIG_DATA)
    }

    fn write_word(bus: u8, slot: u8, func: u8, offset: u8, data: u16) {
        let address = Self::construct_address(bus, slot, func, offset);
        outl(CONFIG_ADDRESS, address);

        // Read the current data at CONFIG_DATA
        let current = inl(CONFIG_DATA);

        // Create a mask and compute the new value to write
        let mask = 0xFFFF << ((offset & 2) * 8);
        let new = (current & !mask) | ((data as u32) << ((offset & 2) * 8));

        // Write the new value to CONFIG_DATA
        outl(CONFIG_DATA, new);
    }

    fn get_device_class_info(bus: u8, device: u8, function: u8) -> (u8, u8, u8) {
        // Read the 32-bit register starting at PCI_CLASS_CODE_OFFSET
        let class_info = Self::read_dword(bus, device, function, PCI_CLASS_CODE_OFFSET);

        // Extract class, subclass, and programming interface from the 32-bit register
        let class_code = ((class_info >> 24) & 0xFF) as u8; // Upper byte
        let sub_class_code = ((class_info >> 16) & 0xFF) as u8; // Next byte
        let prog_if = ((class_info >> 8) & 0xFF) as u8; // Next byte

        (class_code, sub_class_code, prog_if)
    }

    fn scan_secondary_bus(bus: u8) {
        for device in 0..MAX_PCI_DEVICE {
            PCI::inspect_device(bus, device);
        }
    }

    // Helper function to modify a command register with a specific bit mask
    fn modify_command_register(bus: u8, device: u8, function: u8, bit_mask: u16) {
        let command_offset = 0x04; // Command register offset

        // Read the current command register value
        let command = Self::read_word(bus, device, function, command_offset);

        // Write back the modified command register value with the specified bit set
        Self::write_word(bus, device, function, command_offset, command | bit_mask);
    }

    fn print_device_info(vendor_name: &str, device_id: u16, class_info: &PciClassCodeInfo) {
        println!(
            "Vendor: {} | Device ID: {:X} | Class: {} | Subclass: {} | Prog IF: {}",
            vendor_name, device_id, class_info.base_desc, class_info.sub_desc, class_info.prog_desc
        );
    }
}
