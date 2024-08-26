use crate::storage::ahci_device::AhciDevice;
use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use fat::fat_driver::FatDriver;

pub(crate) mod entry;
pub(crate) mod fat;
pub(crate) mod fs2;
pub(crate) mod vfs_directory_entry;
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
        let driver = FatDriver::mount(device);
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

    pub fn read(&self, path: &str, buffer: *mut u8) {}

    pub fn write(&self, path: &str, data: &[u8]) -> Option<usize> {
        todo!("Implement write");
    }

    pub fn exists(&self, path: &str) -> bool {
        todo!("Implement exists");
    }

    pub fn create(&self, path: &str) -> Option<crate::fs::vfs_directory_entry::VfsDirectoryEntry> {
        todo!("Implement create");
    }

    pub fn mkdir(&self, path: &str) -> Option<crate::fs::vfs_directory_entry::VfsDirectoryEntry> {
        todo!("Implement mkdir");
    }

    pub fn rm(&self, path: &str) -> bool {
        todo!("Implement rm");
    }

    pub fn rmdir(&self, path: &str) -> bool {
        todo!("Implement rmdir");
    }

    pub fn ls(&self, path: &str) -> Option<Vec<crate::fs::vfs_directory_entry::VfsDirectoryEntry>> {
        todo!("Implement ls");
    }

    pub fn size(&self, path: &str) -> Option<usize> {
        todo!("Implement size");
    }
}
