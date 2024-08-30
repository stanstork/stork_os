use super::{
    fat::fat_driver::FatDriver,
    vfs_dir_entry::{EntryType, VfsDirectoryEntry},
};
use crate::{println, storage::get_ahci_device, sync::mutex::SpinMutex};
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

pub struct VirtualFileSystem {
    mount_points: BTreeMap<String, FatDriver>,
    mount_info: BTreeMap<String, MountInfo>,
}

impl VirtualFileSystem {
    pub const fn new() -> Self {
        VirtualFileSystem {
            mount_points: BTreeMap::new(),
            mount_info: BTreeMap::new(),
        }
    }

    pub fn mount(&mut self, device_name: &str, path: &str, driver_name: &str) {
        let device = get_ahci_device(device_name);
        if let Some(device) = device {
            if self.mount_points.contains_key(path) {
                println!("Path {} already mounted", path);
                return;
            }

            let driver = FatDriver::mount(device);
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

    pub fn create_dir(&self, path: &str) {
        self.create_entry(path, EntryType::Directory);
    }

    pub fn read_dir(&self, path: &str) -> Option<Vec<VfsDirectoryEntry>> {
        // Get driver and resolve the target cluster
        let (driver, current_cluster) = self.resolve_target_cluster(path)?;
        unsafe { Some(driver.get_dir_entries(current_cluster)) }
    }

    pub fn remove_dir(&self, path: &str) {
        self.delete_entry(path, EntryType::Directory);
    }

    pub fn create_file(&self, path: &str) {
        self.create_entry(path, EntryType::File);
    }

    pub fn read_file(&self, path: &str, buffer: *mut u8) {
        if let Some((driver, entry)) = self.get_entry_for_path(path) {
            let cluster = entry.get_cluster();
            driver.read_file(cluster, buffer);
        } else {
            println!("File not found: {}", path);
        }
    }

    pub fn write_file(&self, path: &str, data: *mut u8, size: usize) {
        if let Some((driver, mut entry)) = self.get_entry_for_path(path) {
            if entry.is_dir() {
                println!("Error: {} is a directory", path);
            } else {
                driver.write_file(&mut entry, data, size);
            }
        } else {
            println!("File not found: {}", path);
        }
    }

    pub fn remove_file(&self, path: &str) {
        self.delete_entry(path, EntryType::File);
    }

    pub fn exists(&self, path: &str) -> bool {
        self.get_entry_for_path(path).is_some()
    }

    fn create_entry(&self, path: &str, entry_type: EntryType) {
        if self.exists(path) {
            println!(
                "{} already exists: {}",
                match entry_type {
                    EntryType::Directory => "Directory",
                    EntryType::File => "File",
                },
                path
            );
            return;
        }

        // Get driver and path components
        let (driver, path_components) = match self.get_driver(path) {
            Some(driver_info) => driver_info,
            None => {
                println!("Error retrieving driver for path: {}", path);
                return;
            }
        };

        let entry_name = match path_components.last() {
            Some(name) => name,
            None => {
                println!("Error: Invalid path, no name found.");
                return;
            }
        };

        let parent_cluster = match self.resolve_parent_cluster(driver, &path_components) {
            Some(cluster) => cluster,
            None => {
                println!("Error: Parent directory not found for path: {}", path);
                return;
            }
        };

        // Call the appropriate driver method based on entry type
        match entry_type {
            EntryType::Directory => driver.create_dir(parent_cluster, entry_name),
            EntryType::File => driver.create_file(parent_cluster, entry_name),
        }
    }

    fn delete_entry(&self, path: &str, entry_type: EntryType) {
        if let Some((driver, entry)) = self.get_entry_for_path(path) {
            match entry_type {
                EntryType::File => {
                    if entry.is_dir() {
                        println!("Error: {} is a directory, not a file", path);
                    } else {
                        driver.delete_entry(&entry);
                    }
                }
                EntryType::Directory => {
                    if entry.is_dir() {
                        driver.delete_entry(&entry);
                    } else {
                        println!("Error: {} is not a directory", path);
                    }
                }
            }
        } else {
            println!(
                "{} not found: {}",
                match entry_type {
                    EntryType::File => "File",
                    EntryType::Directory => "Directory",
                },
                path
            );
        }
    }

    fn resolve_target_cluster<'a>(&'a self, path: &'a str) -> Option<(&'a FatDriver, u32)> {
        let (driver, path_components) = self.get_driver(path)?;

        let mut current_cluster = driver.fs.root_dir_cluster;

        if !Self::is_root_path(&path_components) {
            for component in path_components {
                let entry = driver.get_dir_entry(component)?;

                if !entry.is_dir() {
                    println!("Path component '{}' is not a directory", component);
                    return None;
                }

                current_cluster = entry.get_cluster();
            }
        }

        Some((driver, current_cluster))
    }

    fn resolve_parent_cluster(&self, driver: &FatDriver, path_components: &[&str]) -> Option<u32> {
        let parent_path = path_components
            .iter()
            .take(path_components.len().saturating_sub(1))
            .cloned()
            .collect::<Vec<_>>()
            .join("/");

        if parent_path.is_empty() || Self::is_root_path(&path_components) {
            Some(driver.fs.root_dir_cluster)
        } else {
            driver
                .get_dir_entry(&parent_path)
                .map(|entry| entry.get_cluster())
        }
    }

    fn get_entry_for_path<'a>(
        &'a self,
        path: &'a str,
    ) -> Option<(&'a FatDriver, VfsDirectoryEntry)> {
        let (driver, _) = self.get_driver(path)?;
        let entry = driver.get_dir_entry(path)?;

        Some((driver, entry))
    }

    fn is_root_path(path_components: &[&str]) -> bool {
        path_components.is_empty() || (path_components.len() == 1 && path_components[0].is_empty())
    }
}

pub static mut FS: SpinMutex<VirtualFileSystem> = SpinMutex::new(VirtualFileSystem::new());
