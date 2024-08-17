use crate::storage::ahci_device::AhciDevice;
use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
};
use fat32::fat32_driver::FatDriver;

pub(crate) mod entry;
pub(crate) mod fat32;
pub(crate) mod node;
pub(crate) mod vsf_manager;

pub struct MountInfo {
    pub device: String,
    pub target: String,
    pub driver: String,
}

pub struct VirtualFileSystem {
    pub mount_points: BTreeMap<String, FatDriver>,
    pub mount_info: BTreeMap<String, MountInfo>,
}

impl VirtualFileSystem {
    pub fn new() -> Self {
        VirtualFileSystem {
            mount_points: BTreeMap::new(),
            mount_info: BTreeMap::new(),
        }
    }

    pub fn mount(&mut self, device: AhciDevice, path: String, driver_name: String) {
        let driver = FatDriver::mount(device, 0, 0);
        self.mount_points.insert(path.clone(), driver);
        self.mount_info.insert(
            path.clone(),
            MountInfo {
                device: "AHCI".to_string(),
                target: path,
                driver: driver_name,
            },
        );
    }

    pub fn unmount(&mut self, path: String) {
        self.mount_points.remove(&path);
        self.mount_info.remove(&path);
    }

    pub fn get_driver(&self, path: &str) -> Option<&FatDriver> {
        self.mount_points.get(path)
    }
}
