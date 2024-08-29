use super::ahci::ahci_device::AhciDevice;
use alloc::{collections::btree_map::BTreeMap, string::String};

pub struct StorageManager {
    ahci_devices: BTreeMap<String, AhciDevice>,
}

impl StorageManager {
    pub const fn new() -> Self {
        StorageManager {
            ahci_devices: BTreeMap::new(),
        }
    }

    pub fn register_ahci_device(&mut self, device: AhciDevice, name: String) {
        self.ahci_devices.insert(name, device);
    }

    pub fn get_ahci_device(&self, name: &str) -> Option<&AhciDevice> {
        self.ahci_devices.get(name)
    }
}
