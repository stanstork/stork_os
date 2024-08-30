use super::pci_device::PciDevice;
use alloc::vec::Vec;

pub struct DeviceManager {
    pub devices: Vec<PciDevice>,
}

impl DeviceManager {
    pub const fn new() -> Self {
        DeviceManager {
            devices: Vec::new(),
        }
    }

    pub fn add_device(&mut self, device: PciDevice) {
        self.devices.push(device);
    }

    pub fn search_device(&self, class: u8, subclass: u8) -> Option<PciDevice> {
        for device in self.devices.iter() {
            if device.class == class && device.subclass == subclass {
                return Some(*device);
            }
        }

        None
    }
}

pub static mut DEVICE_MANAGER: DeviceManager = DeviceManager::new();

pub fn add_device(device: PciDevice) {
    unsafe {
        DEVICE_MANAGER.add_device(device);
    }
}

pub fn search_device(class: u8, subclass: u8) -> Option<PciDevice> {
    unsafe { DEVICE_MANAGER.search_device(class, subclass) }
}
