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

// Constants for accessing the PCI configuration space

// CONFIG_ADDRESS and CONFIG_DATA are the I/O ports used to access the PCI configuration space.
const CONFIG_ADDRESS: u16 = 0xCF8; // Address register to specify which PCI device register to access.
const CONFIG_DATA: u16 = 0xCFC; // Data register to read/write the data from/to the specified PCI device register.

// Bit mask to enable the PCI configuration space.
const PCI_ENABLE_BIT: u32 = 0x80000000; // The 31st bit must be set to 1 to enable access to the PCI configuration space.

// Limits for scanning the PCI bus.
const MAX_PCI_BUS: u8 = 255; // Maximum number of PCI buses (0-255).
const MAX_PCI_DEVICE: u8 = 31; // Maximum number of PCI devices per bus (0-31).
const MAX_FUNCTION: u8 = 7; // Maximum number of functions per device (0-7).

// PCI Header Type constants
const HEADER_TYPE_MULTIFUNCTION: u16 = 0x80; // Bit indicating if a device supports multiple functions.
const HEADER_TYPE_MASK: u16 = 0x7F; // Mask to isolate the header type bits.

// Constant for an invalid or non-existent PCI device.
const INVALID_VENDOR_ID: u16 = 0xFFFF; // Vendor ID value returned if no device is present.

// Offsets within the PCI configuration space for various fields.
const PCI_CLASS_CODE_OFFSET: u8 = 0x0B; // Offset for the class code register.
const PCI_SUBCLASS_CODE_OFFSET: u8 = 0x0A; // Offset for the subclass code register.
const PCI_PROG_IF_OFFSET: u8 = 0x09; // Offset for the programming interface register.
const PCI_HEADER_TYPE_OFFSET: u8 = 0x0E; // Offset for the header type register.

// Shifts for calculating the address in the CONFIG_ADDRESS register.
const PCI_SLOT_SHIFT: u8 = 11; // Number of bits to shift to encode the device (slot) number in CONFIG_ADDRESS.
const PCI_FUNC_SHIFT: u8 = 8; // Number of bits to shift to encode the function number in CONFIG_ADDRESS.

// Mask for offset alignment in the CONFIG_ADDRESS register.
const PCI_OFFSET_MASK: u32 = 0xFC; // Mask to ensure the offset is 4-byte aligned for PCI register access.

// Define the PCI struct with associated methods.
pub struct PCI {}

impl PCI {
    /// Scans all PCI buses and devices for available PCI devices.
    pub fn scan_pci_bus() {
        // Iterate through all possible buses (0 to MAX_PCI_BUS).
        for bus in 0..=MAX_PCI_BUS {
            // Iterate through all possible devices on each bus (0 to MAX_PCI_DEVICE).
            for device in 0..MAX_PCI_DEVICE {
                // Inspect each device to check if it exists and to read its configuration.
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

    // Inspects a specific PCI device on a given bus for the presence and functionality.
    fn inspect_device(bus: u8, device: u8) {
        // Retrieves the vendor ID to determine if a device is present at this bus and device address.
        let vendor_id = Self::get_vendor_id(bus, device);

        // If the vendor ID is INVALID_VENDOR_ID, it means no device is present, so we exit early.
        if vendor_id == INVALID_VENDOR_ID {
            return;
        }

        // If a device is present, we check its primary function (function 0).
        Self::check_function(bus, device, 0);

        // Read the header type of the PCI device to determine if it supports multiple functions.
        let header_type =
            Self::read_word(bus, device, 0, PCI_HEADER_TYPE_OFFSET) & HEADER_TYPE_MASK;

        // If the device supports multiple functions, indicated by the multifunction bit (bit 7) being set:
        if header_type & HEADER_TYPE_MULTIFUNCTION != 0 {
            // Iterate over possible functions (1 to 7, since function 0 is already checked).
            for function in 1..MAX_FUNCTION {
                // Get the vendor ID for each function to verify if it is valid.
                let vendor_id = Self::get_vendor_id(bus, device);

                // If the vendor ID is valid, check this function.
                if vendor_id != INVALID_VENDOR_ID {
                    Self::check_function(bus, device, function);
                }
            }
        }
    }

    /// Checks a specific function of a PCI device to identify and gather information about it.
    fn check_function(bus: u8, device: u8, function: u8) {
        // Read the Vendor ID and Device ID to identify the specific PCI device.
        let vendor_id = Self::get_vendor_id(bus, device);
        let device_id = Self::read_word(bus, device, function, 2);

        // Fetch device type information including class code, subclass code, and programming interface.
        // These codes help identify the general type of device (e.g., network controller, storage controller).
        let (class_code, sub_class_code, prog_if) =
            Self::get_device_class_info(bus, device, function);

        // Retrieve the human-readable vendor name using the vendor ID.
        let vendor_name = pci_vendor::get_vendor_name(vendor_id);
        // Retrieve human-readable information about the device's class type.
        let class_info = pci_class::get_class_code_info(class_code, sub_class_code, prog_if);

        // Print the device information such as vendor name, device ID, and class information.
        Self::print_device_info(vendor_name.unwrap(), device_id, &class_info.unwrap());

        // Create a new PCI device structure to represent this device.
        // The structure is populated with all the relevant information about the PCI device.
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

        // Add the newly identified PCI device to a global list of devices.
        // This list is used later for further processing and management of PCI devices.
        add_device(pci_device);

        // Check if the device is a PCI-to-PCI bridge (class code 0x06 and subclass code 0x04).
        // Bridges are used to connect different PCI buses together.
        if class_code == 0x06 && sub_class_code == 0x04 {
            // If the device is a PCI-to-PCI bridge, read the secondary bus number.
            let secondary_bus = Self::read_word(bus, device, function, 0x18) as u8;
            // Recursively scan devices on the secondary bus connected through this bridge.
            Self::scan_secondary_bus(secondary_bus);
        }
    }

    // Retrieves the Vendor ID for a specific PCI device at a given bus and device number.
    // The Vendor ID is located at offset 0 in the PCI configuration space and is 16 bits wide.
    fn get_vendor_id(bus: u8, device: u8) -> u16 {
        // The `read_word` function reads a 16-bit word from the PCI configuration space.
        // Parameters:
        // - `bus`: The PCI bus number (0-255).
        // - `device`: The device number on the specified bus (0-31).
        // - `function`: The function number within the device (0 in this case for the main function).
        // - `offset`: The offset within the PCI configuration space (0 for Vendor ID).
        PCI::read_word(bus, device, 0, 0)
    }

    // Helper function to construct a 32-bit address for accessing the PCI configuration space.
    // This address is written to the CONFIG_ADDRESS I/O port to specify which PCI device register to access.
    fn construct_address(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        // The PCI configuration address format is as follows:
        // Bit 31: Enable bit (must be 1 to access PCI configuration space)
        // Bits 30-24: Reserved
        // Bits 23-16: Bus number
        // Bits 15-11: Device number (slot)
        // Bits 10-8: Function number
        // Bits 7-2: Register offset (must be aligned to 4 bytes)
        // Bits 1-0: Must be 0 (because the offset is aligned to 4 bytes)

        // The PCI_ENABLE_BIT is a constant (0x80000000) that sets the 31st bit to 1, enabling PCI configuration space access.
        // The bus, slot, func, and offset are shifted and masked to fit into their respective bits in the 32-bit address.
        PCI_ENABLE_BIT
        | ((bus as u32) << 16)                 // Shift the bus number to bits 23-16.
        | ((slot as u32) << PCI_SLOT_SHIFT)    // Shift the slot number (device) to bits 15-11.
        | ((func as u32) << PCI_FUNC_SHIFT)    // Shift the function number to bits 10-8.
        | ((offset as u32) & PCI_OFFSET_MASK) // Mask the offset to ensure it is 4-byte aligned and fits in bits 7-2.
    }

    // Reads a 16-bit word from the PCI configuration space for a specific bus, device (slot), function, and register offset.
    fn read_word(bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
        // Construct the 32-bit address for accessing the specific PCI register.
        let address = Self::construct_address(bus, slot, func, offset);
        // Write the constructed address to the CONFIG_ADDRESS I/O port to select the desired PCI register.
        outl(CONFIG_ADDRESS, address);

        // Read a 32-bit value from the CONFIG_DATA I/O port.
        // We then need to extract the specific 16-bit word from the 32-bit data.
        // The offset's least significant bit (offset & 2) determines if we need the lower or upper 16 bits.
        let data = ((inl(CONFIG_DATA) >> ((offset & 2) * 8)) & 0xFFFF) as u16; // Extract the 16-bit word.
        data // Return the 16-bit word.
    }

    // Reads a 32-bit double word from the PCI configuration space for a specific bus, device (slot), function, and register offset.
    fn read_dword(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        // Construct the 32-bit address for accessing the specific PCI register.
        let address = Self::construct_address(bus, slot, func, offset);
        // Write the constructed address to the CONFIG_ADDRESS I/O port to select the desired PCI register.
        outl(CONFIG_ADDRESS, address);

        // Read and return a 32-bit value directly from the CONFIG_DATA I/O port.
        inl(CONFIG_DATA)
    }

    // Writes a 16-bit word to the PCI configuration space for a specific bus, device (slot), function, and register offset.
    fn write_word(bus: u8, slot: u8, func: u8, offset: u8, data: u16) {
        // Construct the 32-bit address for accessing the specific PCI register.
        let address = Self::construct_address(bus, slot, func, offset);
        // Write the constructed address to the CONFIG_ADDRESS I/O port to select the desired PCI register.
        outl(CONFIG_ADDRESS, address);

        // Read the current 32-bit data from the CONFIG_DATA I/O port.
        // This is necessary because we want to modify only a 16-bit portion of the 32-bit register.
        let current = inl(CONFIG_DATA);

        // Create a mask to isolate the 16-bit portion that we want to write to.
        // The offset's least significant bit (offset & 2) determines if we are modifying the lower or upper 16 bits.
        let mask = 0xFFFF << ((offset & 2) * 8); // Create a mask to isolate the 16-bit portion.

        // Compute the new 32-bit value by combining the unchanged parts with the new 16-bit data.
        // `(current & !mask)` clears the 16-bit portion to be updated.
        // `((data as u32) << ((offset & 2) * 8))` shifts the new 16-bit data to the correct position.
        let new = (current & !mask) | ((data as u32) << ((offset & 2) * 8));

        // Write the new 32-bit value to the CONFIG_DATA I/O port.
        // This updates only the desired 16-bit portion while preserving the other parts.
        outl(CONFIG_DATA, new);
    }

    // Retrieves the class code, subclass code, and programming interface of a PCI device.
    // These values are used to identify the type of device and its functionality.
    fn get_device_class_info(bus: u8, device: u8, function: u8) -> (u8, u8, u8) {
        // Read the 32-bit register from the PCI configuration space starting at the PCI_CLASS_CODE_OFFSET.
        // The class code, subclass code, and programming interface are stored in the upper three bytes of this register.
        let class_info = Self::read_dword(bus, device, function, PCI_CLASS_CODE_OFFSET);

        // Extract the class code from the upper byte of the 32-bit register (bits 31-24).
        let class_code = ((class_info >> 24) & 0xFF) as u8; // Upper byte (8 bits) represents the class code.

        // Extract the subclass code from the next byte (bits 23-16).
        let sub_class_code = ((class_info >> 16) & 0xFF) as u8; // Second byte (8 bits) represents the subclass code.

        // Extract the programming interface from the next byte (bits 15-8).
        let prog_if = ((class_info >> 8) & 0xFF) as u8; // Third byte (8 bits) represents the programming interface.

        // Return the extracted class code, subclass code, and programming interface as a tuple.
        (class_code, sub_class_code, prog_if)
    }

    // Scans all PCI devices on a specified secondary bus.
    // This function is typically called when a PCI-to-PCI bridge is detected, which connects to a secondary bus.
    fn scan_secondary_bus(bus: u8) {
        // Iterate over all possible device numbers (0 to MAX_PCI_DEVICE-1) on the specified bus.
        // MAX_PCI_DEVICE is typically 31, representing the maximum number of devices on a single PCI bus.
        for device in 0..MAX_PCI_DEVICE {
            // Call `inspect_device` to check if there is a valid PCI device at the given bus and device number.
            // If a device is present, `inspect_device` will further identify it and gather its configuration information.
            PCI::inspect_device(bus, device);
        }
    }

    // Helper function to modify the command register of a PCI device with a specific bit mask.
    // The command register controls the basic functionality of the PCI device, such as enabling memory or I/O space.
    fn modify_command_register(bus: u8, device: u8, function: u8, bit_mask: u16) {
        let command_offset = 0x04; // Offset for the command register within the PCI configuration space.

        // Read the current value of the command register.
        let command = Self::read_word(bus, device, function, command_offset);

        // Modify the command register value by setting the specified bit(s) using a bitwise OR operation.
        // This enables or configures specific functionalities as defined by the `bit_mask`.
        // The modified value is then written back to the command register.
        Self::write_word(bus, device, function, command_offset, command | bit_mask);
    }

    fn print_device_info(vendor_name: &str, device_id: u16, class_info: &PciClassCodeInfo) {
        println!(
            "Vendor: {} | Device ID: {:X} | Class: {} | Subclass: {} | Prog IF: {}",
            vendor_name, device_id, class_info.base_desc, class_info.sub_desc, class_info.prog_desc
        );
    }
}
