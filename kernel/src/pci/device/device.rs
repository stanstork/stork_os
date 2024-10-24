use super::pci_device::PciDevice;
use alloc::vec::Vec;

// Structure to manage a collection of PCI devices.
// This struct holds a vector of `PciDevice` objects, representing all detected PCI devices in the system.
pub struct DeviceManager {
    pub devices: Vec<PciDevice>, // A vector that stores the list of detected PCI devices.
}

impl DeviceManager {
    pub const fn new() -> Self {
        DeviceManager {
            devices: Vec::new(),
        }
    }

    /// Adds a new PCI device to the manager's list of devices.
    pub fn add_device(&mut self, device: PciDevice) {
        self.devices.push(device);
    }

    /// Searches for a PCI device in the manager's list based on its class and subclass.
    /// Returns an `Option<PciDevice>` that contains the first matching device, if found.
    pub fn search_device(&self, class: u8, subclass: u8) -> Option<PciDevice> {
        // Iterate over all devices in the vector to find a matching class and subclass.
        for device in self.devices.iter() {
            if device.class == class && device.subclass == subclass {
                return Some(*device);
            }
        }

        None // Return `None` if no matching device is found.
    }
}

// Global mutable instance of `DeviceManager` to manage all detected PCI devices.
pub static mut DEVICE_MANAGER: DeviceManager = DeviceManager::new();

/// Adds a PCI device to the global `DEVICE_MANAGER` instance.
pub fn add_device(device: PciDevice) {
    unsafe {
        DEVICE_MANAGER.add_device(device);
    }
}

/// Searches for a PCI device in the global `DEVICE_MANAGER` based on its class and subclass.
pub fn search_device(class: u8, subclass: u8) -> Option<PciDevice> {
    unsafe { DEVICE_MANAGER.search_device(class, subclass) }
}
