use super::fat::driver::FatDriver;
use crate::{println, storage::get_ahci_device, sync::mutex::SpinMutex};
use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use entry::{EntryType, VfsDirectoryEntry};

pub(crate) mod entry;

pub(crate) struct MountInfo {
    // The name of the device that is mounted.
    pub device: String,
    // The target path where the device is mounted within the virtual file system.
    pub target: String,
    // The name of the driver used to mount the device (e.g., "FAT").
    pub driver: String,
}

pub struct VirtualFileSystem {
    // A map that associates mount points (paths) with their corresponding file system drivers (`FatDriver`).
    mount_points: BTreeMap<String, FatDriver>,
    // A map that contains information about each mount point, such as device name, target path, and driver.
    mount_info: BTreeMap<String, MountInfo>,
}

impl VirtualFileSystem {
    /// Creates a new, empty `VirtualFileSystem`.
    pub const fn new() -> Self {
        VirtualFileSystem {
            mount_points: BTreeMap::new(),
            mount_info: BTreeMap::new(),
        }
    }

    /// Mounts a device to a specified path in the virtual file system using a specified driver.
    ///
    /// # Arguments
    ///
    /// * `device_name` - The name of the device to be mounted (e.g., "sda1").
    /// * `path` - The target path within the virtual file system where the device will be mounted.
    /// * `driver_name` - The name of the driver used to handle the file system on the device (e.g., "FAT").
    pub fn mount(&mut self, device_name: &str, path: &str, driver_name: &str) {
        // Retrieve the AHCI device corresponding to `device_name`.
        let device = get_ahci_device(device_name);

        if let Some(device) = device {
            // Check if the path is already mounted to prevent double mounting.
            if self.mount_points.contains_key(path) {
                println!("Path {} already mounted", path);
                return;
            }

            // Mount the device using the FAT driver.
            let driver = FatDriver::mount(device);

            // Insert the mounted file system driver into the mount points map.
            self.mount_points.insert(path.to_string(), driver);

            // Insert mount information into the mount info map.
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

    /// Unmounts a device from a specified path in the virtual file system.
    ///
    /// # Arguments
    ///
    /// * `path` - The target path within the virtual file system where the device is mounted.
    ///
    /// The function removes the mount point and mount information from their respective maps.
    /// If the path is not mounted, this operation does nothing.
    pub fn unmount(&mut self, path: &str) {
        // Remove the file system driver associated with the path from the mount points map.
        self.mount_points.remove(path);

        // Remove the mount information associated with the path from the mount info map.
        self.mount_info.remove(path);
    }

    /// Retrieves the driver and path components for a given path within the virtual file system.
    pub fn get_driver<'a>(&'a self, path: &'a str) -> Option<(&'a FatDriver, Vec<&'a str>)> {
        // Find the longest matching mount point path that is a prefix of the requested path.
        // The iterator filters mount points to find those where the mount point path (`mp`) is a prefix of `path`.
        // `max_by_key` is used to find the longest such prefix, ensuring the most specific mount point is chosen.
        let mount_point = self
            .mount_points
            .iter()
            .filter(|(mp, _)| path.starts_with(mp.as_str()))
            .max_by_key(|(mp, _)| mp.len());

        // If a mount point is found, compute the relative path within the mount point.
        if let Some((mount_path, driver)) = mount_point {
            // Compute the relative path within the mount point
            let relative_path = &path[mount_path.len()..].trim_start_matches('/');
            let path_components: Vec<&str> = relative_path.split('/').collect();
            Some((driver, path_components))
        } else {
            None
        }
    }

    /// Creates a new directory at the specified path in the virtual file system.
    ///
    /// # Arguments
    ///
    /// * `path` - A string slice that holds the path where the new directory should be created.
    pub fn create_dir(&self, path: &str) {
        self.create_entry(path, EntryType::Directory);
    }

    /// Reads the contents of a directory at the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - A string slice representing the path of the directory to read.
    ///
    /// # Returns
    ///
    /// An `Option<Vec<VfsDirectoryEntry>>` containing a vector of directory entries if the path is valid,
    /// or `None` if the path does not exist or is invalid.
    pub fn read_dir(&self, path: &str) -> Option<Vec<VfsDirectoryEntry>> {
        // Get driver and resolve the target cluster
        let (driver, current_cluster) = self.resolve_target_cluster(path)?;
        unsafe { Some(driver.get_dir_entries(current_cluster)) }
    }

    /// Removes a directory at the specified path from the virtual file system.
    ///
    /// # Arguments
    ///
    /// * `path` - A string slice that holds the path of the directory to be removed.
    pub fn remove_dir(&self, path: &str) {
        self.delete_entry(path, EntryType::Directory);
    }

    /// Creates a new file at the specified path in the virtual file system.
    ///
    /// # Arguments
    ///
    /// * `path` - A string slice that holds the path where the new file should be created.
    pub fn create_file(&self, path: &str) {
        self.create_entry(path, EntryType::File);
    }

    /// Reads the contents of a file at the specified path into the provided buffer.
    ///
    /// # Arguments
    ///
    /// * `path` - A string slice representing the path of the file to be read.
    /// * `buffer` - A mutable raw pointer to a buffer where the file data will be stored.
    ///
    /// # Safety
    ///
    /// This function is marked as `unsafe` because it uses raw pointers for the buffer
    /// and involves low-level disk operations. The caller must ensure that the buffer
    /// is valid and that the memory access does not cause undefined behavior.
    pub fn read_file(&self, path: &str, buffer: *mut u8) {
        if let Some((driver, entry)) = self.get_entry_for_path(path) {
            let cluster = entry.get_cluster();
            driver.read_file(cluster, buffer);
        } else {
            println!("File not found: {}", path);
        }
    }

    /// Writes data to a file at the specified path from the provided buffer.
    ///
    /// # Arguments
    ///
    /// * `path` - A string slice representing the path of the file to be written to.
    /// * `data` - A mutable raw pointer to the data buffer containing the data to write to the file.
    /// * `size` - The size of the data to be written, in bytes.
    ///
    /// # Safety
    ///
    /// This function is marked as `unsafe` because it uses raw pointers for the data buffer
    /// and involves low-level disk operations. The caller must ensure that the buffer is valid,
    /// the size is correct, and that the memory access does not cause undefined behavior.
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

    /// Removes a file at the specified path from the virtual file system.
    ///
    /// # Arguments
    ///
    /// * `path` - A string slice representing the path of the file to be removed.
    pub fn remove_file(&self, path: &str) {
        self.delete_entry(path, EntryType::File);
    }

    /// Checks if a file or directory exists at the specified path in the virtual file system.
    ///
    /// # Arguments
    ///
    /// * `path` - A string slice representing the path to check for existence.
    ///
    /// # Returns
    ///
    /// A boolean value indicating whether a file or directory exists at the specified path.
    pub fn exists(&self, path: &str) -> bool {
        self.get_entry_for_path(path).is_some()
    }

    fn create_entry(&self, path: &str, entry_type: EntryType) {
        // Check if the entry already exists at the specified path.
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

        // Retrieve the file system driver and path components for the specified path.
        let (driver, path_components) = match self.get_driver(path) {
            Some(driver_info) => driver_info,
            None => {
                println!("Error retrieving driver for path: {}", path);
                return; // Exit early if the driver cannot be retrieved.
            }
        };

        // Determine the name of the new entry (file or directory) from the last component of the path.
        let entry_name = match path_components.last() {
            Some(name) => name,
            None => {
                println!("Error: Invalid path, no name found.");
                return; // Exit early if no valid name is found.
            }
        };

        // Resolve the parent directory's cluster number.
        let parent_cluster = match self.resolve_parent_cluster(driver, &path_components) {
            Some(cluster) => cluster,
            None => {
                println!("Error: Parent directory not found for path: {}", path);
                return; // Exit early if the parent directory is not found.
            }
        };

        // Call the appropriate driver method to create the entry based on its type (directory or file).
        match entry_type {
            EntryType::Directory => driver.create_dir(parent_cluster, entry_name),
            EntryType::File => driver.create_file(parent_cluster, entry_name),
        }
    }

    fn delete_entry(&self, path: &str, entry_type: EntryType) {
        // Retrieve the file system driver and entry for the specified path.
        if let Some((driver, entry)) = self.get_entry_for_path(path) {
            // Check the entry type and perform the appropriate deletion.
            match entry_type {
                EntryType::File => {
                    if entry.is_dir() {
                        println!("Error: {} is a directory, not a file", path);
                    } else {
                        driver.delete_entry(&entry); // Delete the file entry.
                    }
                }
                EntryType::Directory => {
                    if entry.is_dir() {
                        driver.delete_entry(&entry); // Delete the directory entry.
                    } else {
                        println!("Error: {} is not a directory", path);
                    }
                }
            }
        } else {
            // Print an error if the entry is not found.
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
        // Retrieve the file system driver and path components for the specified path.
        let (driver, path_components) = self.get_driver(path)?;

        // Start from the root directory cluster of the file system.
        let mut current_cluster = driver.fs.root_dir_cluster;

        // Check if the path is not the root path.
        if !Self::is_root_path(&path_components) {
            // Iterate through each component of the path to resolve the target cluster.
            for component in path_components {
                // Retrieve the directory entry for the current component.
                let entry = driver.get_dir_entry(component)?;

                // Ensure that the current path component is a directory.
                if !entry.is_dir() {
                    println!("Path component '{}' is not a directory", component);
                    return None;
                }

                // Update the current cluster to the cluster number of the directory.
                current_cluster = entry.get_cluster();
            }
        }

        // Return the file system driver and the resolved cluster number.
        Some((driver, current_cluster))
    }

    fn resolve_parent_cluster(&self, driver: &FatDriver, path_components: &[&str]) -> Option<u32> {
        // Construct the parent path by joining all components except the last one.
        let parent_path = path_components
            .iter()
            .take(path_components.len().saturating_sub(1)) // Take all components except the last one.
            .cloned() // Clone each component to avoid borrowing issues.
            .collect::<Vec<_>>() // Collect into a vector.
            .join("/"); // Join the components to form a path.

        // Check if the parent path is empty or represents the root path.
        if parent_path.is_empty() || Self::is_root_path(&path_components) {
            Some(driver.fs.root_dir_cluster) // Return the root directory cluster if the parent path is the root.
        } else {
            // Retrieve the directory entry for the parent path and return its cluster number.
            driver
                .get_dir_entry(&parent_path)
                .map(|entry| entry.get_cluster())
        }
    }

    fn get_entry_for_path<'a>(
        &'a self,
        path: &'a str,
    ) -> Option<(&'a FatDriver, VfsDirectoryEntry)> {
        // Retrieve the file system driver for the specified path.
        let (driver, _) = self.get_driver(path)?;

        // Attempt to get the directory entry corresponding to the path using the retrieved driver.
        let entry = driver.get_dir_entry(path)?;

        // Return a tuple containing the driver and the found directory entry.
        Some((driver, entry))
    }

    fn is_root_path(path_components: &[&str]) -> bool {
        // Check if the path components indicate the root path.
        // This is true if the path components are empty or if there is exactly one component that is an empty string.
        path_components.is_empty() || (path_components.len() == 1 && path_components[0].is_empty())
    }
}

/// A global, mutable instance of the `VirtualFileSystem` protected by a `SpinMutex`.
pub static mut FS: SpinMutex<VirtualFileSystem> = SpinMutex::new(VirtualFileSystem::new());
