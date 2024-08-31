use super::PCI;

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,        // The PCI bus number where the device is located (0-255).
    pub device: u8,     // The device number on the specified bus (0-31).
    pub function: u8,   // The function number within the PCI device (0-7).
    pub vendor_id: u16, // The vendor ID of the PCI device, identifying the manufacturer.
    pub device_id: u16, // The device ID of the PCI device, identifying the specific device model.
    pub revision: u8,   // The revision ID of the PCI device, indicating the device revision.
    pub prog_if: u8, // The programming interface of the PCI device, providing additional interface information.
    pub class: u8, // The base class code of the PCI device, identifying the general type of device.
    pub subclass: u8, // The subclass code of the PCI device, providing a more specific classification.
}

impl PciDevice {
    // Reads a 16-bit word from a specified register within the PCI device's configuration space.
    pub fn read_word(&self, reg: u8) -> u16 {
        PCI::read_word(self.bus, self.device, self.function, reg)
    }

    // Writes a 16-bit word to a specified register within the PCI device's configuration space.
    pub fn write_word(&self, reg: u8, data: u16) {
        PCI::write_word(self.bus, self.device, self.function, reg, data);
    }

    // Reads a 32-bit double word from a specified register within the PCI device's configuration space.
    pub fn read_dword(&self, reg: u8) -> u32 {
        PCI::read_dword(self.bus, self.device, self.function, reg)
    }

    // Returns the computed PCI address of the device in a format suitable for configuration space access.
    pub fn address(&self) -> u32 {
        // Compute the address using the bus, device, and function numbers.
        // The address format is as follows:
        // Bits 31-24: Reserved
        // Bits 23-16: Bus number
        // Bits 15-11: Device number
        // Bits 10-8: Function number
        // Bits 7-0: Register offset (not included here)
        ((self.bus as u32) << 16)           // Shift the bus number to bits 23-16.
            | ((self.device as u32) << 11)  // Shift the device number to bits 15-11.
            | ((self.function as u32) << 8) // Shift the function number to bits 10-8.
    }
}
