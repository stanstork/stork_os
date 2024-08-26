use super::{fat::fat_driver::FatDriver, vfs_directory_entry::VfsDirectoryEntry};
use crate::{println, storage::STORAGE_MANAGER, sync::mutex::SpinMutex};
use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

pub(crate) struct MountInfo {
    pub device: String,
    pub target: String,
    pub driver: String,
}

pub struct FileSystem {
    mount_points: BTreeMap<String, FatDriver>,
    mount_info: BTreeMap<String, MountInfo>,
}

impl FileSystem {
    pub const fn new() -> Self {
        FileSystem {
            mount_points: BTreeMap::new(),
            mount_info: BTreeMap::new(),
        }
    }

    pub fn mount(&mut self, device_name: &str, path: &str, driver_name: &str) {
        let device = unsafe { STORAGE_MANAGER.get_ahci_device(device_name) };
        if let Some(device) = device {
            if self.mount_points.contains_key(path) {
                println!("Path {} already mounted", path);
                return;
            }

            let driver = FatDriver::mount(*device);
            self.mount_points.insert(path.to_string(), driver);
            self.mount_info.insert(
                path.to_string(),
                MountInfo {
                    device: device_name.to_string(),
                    target: path.to_string(),
                    driver: driver_name.to_string(),
                },
            );

            return;
        }

        println!("Device {} not found", device_name);
    }

    pub fn unmount(&mut self, path: &str) {
        self.mount_points.remove(path);
        self.mount_info.remove(path);
    }

    pub fn get_driver<'a>(&'a self, path: &'a str) -> Option<(&'a FatDriver, Vec<&'a str>)> {
        // Find the longest matching mount point path that is a prefix of the requested path
        let mount_point = self
            .mount_points
            .iter()
            .filter(|(mp, _)| path.starts_with(mp.as_str()))
            .max_by_key(|(mp, _)| mp.len());

        if let Some((mount_path, driver)) = mount_point {
            // Compute the relative path within the mount point
            let relative_path = &path[mount_path.len()..].trim_start_matches('/');
            let path_components: Vec<&str> = relative_path.split('/').collect();
            Some((driver, path_components))
        } else {
            None
        }
    }

    pub fn list_directory(&self, path: &str) -> Option<Vec<VfsDirectoryEntry>> {
        // Find the driver associated with the given path
        let (driver, path_components) = self.get_driver(path)?;

        let mut current_cluster = driver.fs.root_dir_cluster;

        if Self::is_root_path(&path_components) {
            // Directly list the entries in the root directory cluster
            return unsafe { Some(driver.get_dir_entries(current_cluster)) };
        }

        // Navigate through each component of the path
        for component in path_components {
            let entry = driver.get_dir_entry(component)?;

            if !entry.is_dir() {
                println!("Path component '{}' is not a directory", component);
                return None;
            }

            current_cluster = entry.get_cluster();
        }

        // Get the directory entries for the target cluster
        unsafe { Some(driver.get_dir_entries(current_cluster)) }
    }

    fn is_root_path(path_components: &[&str]) -> bool {
        path_components.is_empty() || (path_components.len() == 1 && path_components[0].is_empty())
    }
}

pub static mut FS: SpinMutex<FileSystem> = SpinMutex::new(FileSystem::new());
