use super::PCI;

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub revision: u8,
    pub prog_if: u8,
    pub class: u8,
    pub subclass: u8,
}

impl PciDevice {
    pub const fn default() -> Self {
        PciDevice {
            bus: 0,
            device: 0,
            function: 0,
            vendor_id: 0,
            device_id: 0,
            revision: 0,
            prog_if: 0,
            class: 0,
            subclass: 0,
        }
    }

    pub fn read_word(&self, reg: u8) -> u16 {
        PCI::read_word(self.bus, self.device, self.function, reg)
    }

    pub fn write_word(&self, reg: u8, data: u16) {
        PCI::write_word(self.bus, self.device, self.function, reg, data);
    }

    pub fn read_dword(&self, reg: u8) -> u32 {
        PCI::read_dword(self.bus, self.device, self.function, reg)
    }

    pub fn address(&self) -> u32 {
        ((self.bus as u32) << 16) | ((self.device as u32) << 11) | ((self.function as u32) << 8)
    }
}
