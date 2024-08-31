use super::ahci::ahci_device::AhciDevice;
use alloc::{collections::btree_map::BTreeMap, string::String};

/// Manages storage devices, specifically AHCI devices, by maintaining a registry of devices.
pub struct StorageManager {
    ahci_devices: BTreeMap<String, AhciDevice>, // A map storing AHCI devices by their name.
}

impl StorageManager {
    pub const fn new() -> Self {
        StorageManager {
            ahci_devices: BTreeMap::new(),
        }
    }

    /// Registers a new AHCI device with the storage manager.
    ///
    /// # Parameters
    ///
    /// - `device`: The `AhciDevice` to be registered.
    /// - `name`: A `String` representing the name to associate with the device.
    ///
    /// This method inserts the device into the internal `BTreeMap` using the provided name as the key.
    pub fn register_ahci_device(&mut self, device: AhciDevice, name: String) {
        self.ahci_devices.insert(name, device);
    }

    /// Retrieves an AHCI device by its name.
    ///
    /// # Parameters
    ///
    /// - `name`: A string slice representing the name of the device to retrieve.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the `AhciDevice` if found, or `None` if the device is not registered.
    pub fn get_ahci_device(&self, name: &str) -> Option<&AhciDevice> {
        self.ahci_devices.get(name)
    }
}
