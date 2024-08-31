use super::{boot_sector::Fat32BootSector, calculate_checksum, create_short_filename};
use crate::{
    fs::{
        fat::{
            dir_entry::DirectoryEntry,
            long_dir_entry::{create_lfn_entries, LongDirectoryEntry},
        },
        vfs_dir_entry::VfsDirectoryEntry,
    },
    memory, print, println,
    storage::ahci::ahci_device::AhciDevice,
};
use alloc::vec::Vec;
use core::{intrinsics::size_of, slice::from_raw_parts};

pub struct FileSystemInfo {
    // The starting sector of the file system on the device.
    pub start_sector: u64,
    // Total number of sectors in the file system.
    pub sectors_count: u64,
    // Number of bytes per sector, a typical value is 512 bytes.
    pub bytes_per_sector: u16,
    // Number of sectors occupied by the root directory.
    pub root_sectors: u32,
    // Number of sectors per cluster, the smallest unit of allocation.
    pub sectors_per_cluster: u8,
    // Size of each cluster in bytes, calculated as `bytes_per_sector * sectors_per_cluster`.
    pub cluster_size: u32,
    // The first data sector where the cluster chain for files and directories begins.
    pub first_data_sector: u32,
    // The first sector of the File Allocation Table (FAT).
    pub first_fat_sector: u32,
    // The cluster number where the root directory starts, typically 2 for FAT32.
    pub root_dir_cluster: u32,
    // Total number of clusters in the file system.
    pub total_clusters: u32,
    // Maximum number of entries in the root directory (for FAT12/FAT16); typically 0 for FAT32.
    pub root_dir_entries: u16,
}

pub struct FatDriver {
    // File system information, including sector and cluster details.
    pub(crate) fs: FileSystemInfo,
    // The underlying device on which the file system is mounted.
    pub(crate) device: AhciDevice,
}

// Constants representing different cluster statuses in the FAT.
pub const CLUSTER_FREE: u32 = 0x00000000; // Free cluster
pub const CLUSTER_RESERVED: u32 = 0x0FFFFFF0; // Reserved cluster range
pub const CLUSTER_BAD: u32 = 0x0FFFFFF7; // Bad cluster, unusable
pub const CLUSTER_LAST: u32 = 0x0FFFFFF8; // Last cluster in a file (EOF marker)

// Constants representing different directory entry statuses and attributes.
pub const ENTRY_END: u8 = 0x00; // Indicates the end of the directory entries
pub const ENTRY_FREE: u8 = 0xE5; // Indicates a free directory entry
pub const ENTRY_DELETED: u8 = 0x05; // Indicates a deleted directory entry (with first byte set to 0x05)
pub const ENTRY_LONG: u8 = 0x0F; // Attribute for long file name entry

pub const ATTR_DIRECTORY: u8 = 0x10; // Attribute flag indicating an entry is a directory

impl FatDriver {
    /// Mounts the FAT file system on the given device.
    ///
    /// This function reads the boot sector from the device and calculates various parameters
    /// necessary for managing the file system, such as cluster size, data sector offsets,
    /// and total number of clusters. It initializes a `FileSystemInfo` struct with these parameters.
    ///
    /// # Arguments
    ///
    /// * `device` - The AHCI device on which the FAT file system resides.
    ///
    /// # Returns
    ///
    /// Returns a new instance of `FatDriver` initialized with the file system information and device.
    pub fn mount(device: AhciDevice) -> Self {
        // Read the boot sector from the device to obtain fundamental file system parameters.
        let boot_sector = Self::read_boot_sector(&device);

        // Compute various file system values based on the information retrieved from the boot sector.
        let bytes_per_sector = boot_sector.bytes_per_sector as u32; // Number of bytes per sector.
        let root_sectors = ((boot_sector.root_dir_entries as u32 * 32) + (bytes_per_sector - 1))
            / bytes_per_sector; // Calculate the number of sectors needed for the root directory.
        let first_fat_sector = boot_sector.reserved_sectors as u32; // The first sector where FAT starts.

        // Calculate the first sector where the data area (file clusters) begins.
        let first_data_sector = first_fat_sector
            + (boot_sector.fat_count as u32 * boot_sector.sectors_per_fat_large as u32);

        // Compute the size of each cluster in bytes.
        let cluster_size = bytes_per_sector * boot_sector.sectors_per_cluster as u32;

        // Calculate the total number of clusters in the data region.
        let total_clusters = (boot_sector.total_sectors_large - first_data_sector)
            / boot_sector.sectors_per_cluster as u32;

        // Initialize the FileSystemInfo struct with computed values from the boot sector.
        let fs_info = FileSystemInfo {
            start_sector: 0, // Typically the starting sector of the partition.
            sectors_count: boot_sector.total_sectors_large as u64, // Total number of sectors in the volume.
            bytes_per_sector: boot_sector.bytes_per_sector,        // Bytes per sector.
            root_sectors, // Number of sectors for the root directory.
            sectors_per_cluster: boot_sector.sectors_per_cluster, // Sectors per cluster.
            cluster_size, // Size of a cluster in bytes.
            first_data_sector, // The first sector number of the data area.
            first_fat_sector, // The first sector number of the FAT area.
            root_dir_cluster: boot_sector.root_dir_start, // Cluster number where the root directory starts.
            total_clusters,                               // Total number of clusters.
            root_dir_entries: boot_sector.root_dir_entries, // Number of root directory entries (FAT12/16).
        };

        FatDriver {
            fs: fs_info,
            device,
        }
    }

    /// Retrieves all directory entries within a given cluster chain in the FAT file system.
    ///
    /// This function navigates through a chain of clusters starting from the specified cluster,
    /// reading directory entries from each cluster and collecting them into a vector of `VfsDirectoryEntry`.
    ///
    /// # Safety
    ///
    /// This function is marked as `unsafe` because it directly interacts with low-level disk sectors
    /// and relies on the validity of cluster and sector calculations, which can potentially cause
    /// undefined behavior if used incorrectly.
    ///
    /// # Arguments
    ///
    /// * `cluster` - The starting cluster of the directory whose entries are to be read.
    ///
    /// # Returns
    ///
    /// A vector of `VfsDirectoryEntry` containing all the entries found in the directory across
    /// all clusters in the cluster chain.
    pub unsafe fn get_dir_entries(&self, cluster: u32) -> Vec<VfsDirectoryEntry> {
        let mut entries = Vec::new(); // Initialize an empty vector to store directory entries.
        let mut current_cluster = cluster; // Start with the specified cluster.

        // Loop to read all clusters in the cluster chain.
        while self.is_valid_cluster(current_cluster) {
            // Determine the sector number corresponding to the current cluster.
            let sector = self.get_sector(current_cluster);

            // Read all directory entries from the current cluster sector.
            let cluster_entries = self.read_cluster_entries(sector);

            // Add the entries read from the current cluster to the vector.
            entries.extend(cluster_entries);

            // Get the next cluster in the chain using the FAT.
            current_cluster = self.get_next_cluster(current_cluster);
        }

        // Return the collected directory entries.
        entries
    }

    /// Retrieves a directory entry corresponding to a given path in the FAT file system.
    ///
    /// This function navigates through the directory structure of the FAT file system,
    /// starting from the root directory and moving through subdirectories to locate
    /// the specified file or directory. It returns the corresponding `VfsDirectoryEntry`
    /// if found, or `None` if the entry does not exist.
    ///
    /// # Arguments
    ///
    /// * `path` - A string slice representing the path to the directory or file, e.g., "/folder/file.txt".
    ///
    /// # Returns
    ///
    /// An `Option<VfsDirectoryEntry>` containing the directory entry if found,
    /// or `None` if the entry does not exist.
    pub fn get_dir_entry(&self, path: &str) -> Option<VfsDirectoryEntry> {
        // Split the path into components by '/' and filter out any empty components.
        let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        // If the path is the root "/", search the root directory directly.
        if path_parts.is_empty() {
            return self.search_in_dir(self.fs.root_dir_cluster, "/");
        }

        // Initialize the search starting at the root directory cluster.
        let mut current_cluster = self.fs.root_dir_cluster;
        let mut last_entry = None;

        // Iterate through each part of the path to navigate through directories.
        for (_i, part) in path_parts.iter().enumerate() {
            // Search for the current part in the current directory cluster.
            let entry = self.search_in_dir(current_cluster, part)?;

            // If the entry is a directory, update the current cluster to continue the search.
            if entry.is_dir() {
                current_cluster = entry.get_cluster();
                last_entry = Some(entry); // Keep track of the last directory entry found.
            } else {
                // If the entry is not a directory, return it as the final result.
                return Some(entry);
            }
        }

        // Return the last directory entry found, or `None` if not found.
        last_entry
    }

    /// Reads the contents of a file starting from a given cluster and writes it into a buffer.
    ///
    /// This function navigates through the cluster chain starting from the specified cluster,
    /// reads the file data from each cluster's sectors, and copies it into the provided buffer.
    ///
    /// # Arguments
    ///
    /// * `cluster` - The starting cluster of the file to read.
    /// * `buffer` - A mutable raw pointer to a buffer where the file data will be stored.
    ///
    /// # Safety
    ///
    /// This function is marked as `unsafe` because it dereferences raw pointers to write data
    /// into the provided buffer. The caller must ensure that the buffer is valid, correctly sized,
    /// and that the pointer arithmetic does not cause any memory violations.
    pub fn read_file(&self, cluster: u32, buffer: *mut u8) {
        let mut cluster = cluster; // Initialize the current cluster to the starting cluster.
        let mut buffer_offset = 0; // Initialize the offset within the buffer where data will be written.

        // Loop through the cluster chain until reaching a cluster marked as the last.
        while cluster < CLUSTER_LAST {
            // Determine the sector number corresponding to the current cluster.
            let sector = self.get_sector(cluster);

            // Loop through all sectors within the current cluster.
            for i in 0..self.fs.sectors_per_cluster {
                // Read the sector data from the device into the buffer.
                // `buffer.add(buffer_offset)` calculates the address in the buffer to write to.
                // `sector as u64 + i as u64` calculates the actual sector number to read from.
                // `1` indicates that one sector should be read.
                self.device.read_sectors(
                    unsafe { buffer.add(buffer_offset) },
                    sector as u64 + i as u64,
                    1,
                );

                // Update the buffer offset by the number of bytes in a sector.
                buffer_offset += self.fs.bytes_per_sector as usize;
            }

            // Get the next cluster in the chain from the FAT and continue reading.
            cluster = self.get_next_cluster(cluster);
        }
    }

    /// Creates a new file in the specified directory within the FAT file system.
    ///
    /// This function attempts to allocate a new cluster for the file, creates a file entry
    /// in the parent directory, and optionally initializes the newly allocated cluster to zero.
    ///
    /// # Arguments
    ///
    /// * `parent_cluster` - The cluster number of the parent directory where the file will be created.
    /// * `name` - The name of the new file to be created.
    ///
    /// # Safety
    ///
    /// This function uses `unsafe` blocks to perform operations that involve direct memory manipulation
    /// and low-level disk operations. The caller must ensure that the provided arguments are valid and
    /// that the disk operations do not cause unintended side effects.
    pub fn create_file(&self, parent_cluster: u32, name: &str) {
        // Attempt to allocate a new cluster for the file.
        let new_file_cluster = match unsafe { self.alloc_cluster() } {
            Some(cluster) => cluster, // Successfully allocated a new cluster.
            None => {
                // If allocation fails, print an error message and return early.
                println!("Failed to allocate cluster for new file");
                return;
            }
        };

        unsafe {
            // Safely create the file entry in the parent directory.
            self.create_entry(parent_cluster, name, 0, new_file_cluster);

            // Clear the newly allocated cluster to zero to initialize it.
            // This ensures that the file's data area starts empty.
            self.clear_cluster(new_file_cluster);
        }
    }

    /// Writes data to a file starting from the specified directory entry in the FAT file system.
    ///
    /// This function writes the contents of the provided buffer into the file represented by the given
    /// directory entry. It manages the allocation of clusters, updates the File Allocation Table (FAT),
    /// and ensures that the file data is written correctly to the disk.
    ///
    /// # Arguments
    ///
    /// * `entry` - A mutable reference to a `VfsDirectoryEntry` representing the file to write to.
    /// * `buffer` - A raw pointer to the data buffer that contains the data to write to the file.
    /// * `size` - The size of the data to be written, in bytes.
    ///
    /// # Safety
    ///
    /// This function uses `unsafe` blocks for direct memory manipulation and low-level disk operations.
    /// The caller must ensure that the buffer is valid and correctly sized, and that the disk operations
    /// do not cause unintended side effects.
    pub fn write_file(&self, entry: &mut VfsDirectoryEntry, buffer: *const u8, size: usize) {
        // Calculate the number of clusters needed to store the data.
        let clusters_needed = (size as u32 + self.fs.cluster_size - 1) / self.fs.cluster_size;
        let mut written_bytes = 0; // Track the number of bytes written to the file.
        let mut current_cluster = entry.get_cluster(); // Get the starting cluster of the file.
        let mut last_cluster = current_cluster; // Track the last cluster used for writing.

        // Iterate over the number of clusters needed to write the entire data.
        for _ in 0..clusters_needed {
            // Determine the sector number corresponding to the current cluster.
            let sector = self.get_sector(current_cluster);

            // Iterate through all sectors within the current cluster.
            for sector_offset in 0..self.fs.sectors_per_cluster {
                let bytes_left = size - written_bytes; // Calculate remaining bytes to be written.
                let bytes_to_write = bytes_left.min(self.fs.bytes_per_sector as usize); // Determine how many bytes to write in this iteration.

                // Allocate a DMA buffer for the sector write operation.
                let write_buffer =
                    memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;

                // Perform the actual write operation to the sector.
                self.write_to_sector(
                    buffer,         // Original data buffer.
                    write_buffer,   // DMA buffer to use for writing.
                    sector,         // Current sector to write to.
                    sector_offset,  // Offset within the cluster.
                    written_bytes,  // Offset within the buffer.
                    bytes_to_write, // Number of bytes to write in this operation.
                );

                // Update the count of written bytes.
                written_bytes += bytes_to_write;

                // Break the loop if all data has been written.
                if written_bytes >= size {
                    break;
                }
            }

            // Check if more clusters are needed to continue writing.
            if written_bytes < size {
                // Get the next cluster in the chain from the FAT.
                let next_cluster = self.get_next_cluster(current_cluster);

                if next_cluster == 0x0FFFFFFF {
                    // If no next cluster is allocated, allocate a new one and update the FAT.
                    current_cluster = unsafe { self.next_cluster(current_cluster).unwrap() };
                } else {
                    // Use the existing next cluster.
                    current_cluster = next_cluster;
                }

                // Update the last cluster pointer to the current cluster.
                last_cluster = current_cluster;
            }
        }

        // Mark the end of the cluster chain in the FAT to indicate the end of the file.
        unsafe {
            self.set_next_cluster(last_cluster, 0x0FFFFFFF);
        }

        // Update the directory entry to reflect the new file size and modification time.
        self.update_entry(&entry, size);
    }

    /// Creates a new directory in the specified parent directory within the FAT file system.
    ///
    /// This function allocates a new cluster for the directory, creates the necessary directory entries
    /// (including the "." and ".." entries), and writes them to disk. It also ensures that the parent
    /// directory is updated accordingly.
    ///
    /// # Arguments
    ///
    /// * `parent_cluster` - The cluster number of the parent directory where the new directory will be created.
    /// * `name` - The name of the new directory to be created.
    ///
    /// # Safety
    ///
    /// This function is marked as `unsafe` because it directly manipulates raw memory and interacts
    /// with low-level disk sectors. The caller must ensure that the provided arguments are valid and
    /// that the operations do not cause unintended side effects or data corruption.
    pub fn create_dir(&self, parent_cluster: u32, name: &str) {
        unsafe {
            // Allocate a new cluster for the new directory.
            let new_dir_cluster = match self.alloc_cluster() {
                Some(cluster) if cluster != 0 => cluster, // Successfully allocated a cluster.
                _ => {
                    // If allocation fails, print an error message and return early.
                    println!("Failed to allocate cluster for new directory");
                    return;
                }
            };

            // Create a new directory entry in the parent directory for the new directory.
            // The entry includes the directory's name, attribute (directory flag), and starting cluster.
            self.create_entry(parent_cluster, name, ATTR_DIRECTORY, new_dir_cluster);

            // Determine the first sector of the newly allocated directory cluster.
            let sector = self.get_sector(new_dir_cluster);

            // Create the "." entry, which points to the directory itself.
            let dot_entry = DirectoryEntry::create_dot_entry(new_dir_cluster);

            // Create the ".." entry, which points to the parent directory.
            let dotdot_entry = DirectoryEntry::create_dotdot_entry(parent_cluster);

            // Allocate a DMA buffer to read and write the directory sector.
            let buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;

            // Read the current sector contents where the new directory is located.
            self.device.read_sectors(buffer, sector as u64, 1);

            // Write the "." entry at the beginning of the directory.
            core::ptr::write_volatile(buffer as *mut DirectoryEntry, dot_entry);

            // Write the ".." entry immediately after the "." entry.
            core::ptr::write_volatile(
                buffer.add(size_of::<DirectoryEntry>() as usize) as *mut DirectoryEntry,
                dotdot_entry,
            );

            // Write the modified buffer back to the disk to finalize the new directory creation.
            self.device.write_sectors(buffer, sector as u64, 1);
        }
    }

    /// Deletes a directory entry from the FAT file system by marking it as deleted.
    ///
    /// This function marks the specified directory entry as deleted in its respective sector
    /// on the disk. The actual data is not removed; instead, the entry is marked with a special
    /// character to indicate that it has been deleted. This operation does not deallocate clusters.
    ///
    /// # Arguments
    ///
    /// * `node` - A reference to a `VfsDirectoryEntry` representing the directory entry to be deleted.
    ///
    /// The function calculates the sector and offset of the entry, reads the sector into memory,
    /// marks the entry as deleted, and writes the modified sector back to the disk.
    pub fn delete_entry(&self, node: &VfsDirectoryEntry) {
        // Retrieve the sector number and offset within the sector for the directory entry.
        let sector = node.sector;
        let offset = node.offset;

        // Allocate a buffer to read the sector containing the directory entry.
        let read_buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;

        // Read the sector containing the directory entry into the buffer.
        self.device.read_sectors(read_buffer, sector as u64, 1);

        // Calculate the pointer to the specific directory entry within the buffer.
        let entry_ptr = unsafe { read_buffer.add(offset as usize) as *mut DirectoryEntry };

        // Mark the directory entry as deleted by setting its first character to ENTRY_DELETED.
        // ENTRY_DELETED is typically 0xE5 in the FAT file system, indicating the entry is deleted.
        unsafe {
            (*entry_ptr).name[0] = ENTRY_DELETED;
        }

        // Write the modified buffer back to the disk to update the directory entry.
        self.device.write_sectors(read_buffer, sector as u64, 1);
    }

    fn read_boot_sector(device: &AhciDevice) -> Fat32BootSector {
        let read_buffer = crate::memory::allocate_dma_buffer(512) as *mut u8;
        device.read_sectors(read_buffer, 0, 1);
        unsafe { *(read_buffer as *const Fat32BootSector) }
    }

    fn is_valid_cluster(&self, cluster: u32) -> bool {
        (cluster != CLUSTER_FREE) && (cluster < CLUSTER_LAST)
    }

    fn get_sector(&self, cluster: u32) -> u32 {
        self.fs.first_data_sector + (cluster - 2) * self.fs.sectors_per_cluster as u32
    }

    fn read_cluster_entries(&self, sector: u32) -> Vec<VfsDirectoryEntry> {
        let mut entries = Vec::new();

        for i in 0..self.fs.sectors_per_cluster {
            let read_buffer = self.read_sector(sector, i as u32);
            let sector_entries = self.read_sector_entries(sector, read_buffer);

            if sector_entries.is_empty() {
                return entries;
            }

            entries.extend(sector_entries);
        }

        entries
    }

    fn read_sector(&self, sector: u32, offset: u32) -> *mut u8 {
        let read_buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;
        self.device
            .read_sectors(read_buffer, sector as u64 + offset as u64, 1);
        read_buffer
    }

    fn read_sector_entries(&self, sector: u32, buffer: *const u8) -> Vec<VfsDirectoryEntry> {
        let mut entries = Vec::new();
        let mut lfn_entries = Vec::new();

        for i in 0..(self.fs.bytes_per_sector / size_of::<DirectoryEntry>() as u16) {
            let entry_ptr = unsafe { buffer.add(i as usize * size_of::<DirectoryEntry>()) };
            let entry = unsafe { *(entry_ptr as *const DirectoryEntry) };

            if entry.name[0] == ENTRY_END {
                return entries;
            }

            if entry.name[0] == ENTRY_FREE || entry.name[0] == ENTRY_DELETED {
                continue;
            }

            if entry.attributes == ENTRY_LONG {
                lfn_entries.push(unsafe { *(entry_ptr as *const LongDirectoryEntry) });
                continue;
            };

            let offset = i as u32 * size_of::<DirectoryEntry>() as u32;
            let node = VfsDirectoryEntry::from_entry(entry, &mut lfn_entries, sector, offset);

            entries.push(node);
        }

        entries
    }

    fn search_in_dir(&self, cluster: u32, name: &str) -> Option<VfsDirectoryEntry> {
        let entries = unsafe { self.get_dir_entries(cluster) };
        entries
            .into_iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(name))
    }

    unsafe fn alloc_cluster(&self) -> Option<u32> {
        (self.fs.root_dir_cluster..self.fs.total_clusters)
            .find(|&cluster| self.get_next_cluster(cluster) == CLUSTER_FREE)
            .map(|cluster| {
                self.set_next_cluster(cluster, CLUSTER_LAST);
                cluster
            })
    }

    fn get_next_cluster(&self, cluster: u32) -> u32 {
        // Calculate the sector in the FAT that contains the entry for the given cluster
        let (fat_sector, fat_offset) = self.get_fat_sector(cluster);

        // Allocate a buffer to read the FAT sector
        let read_buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;

        // Read the FAT sector into the buffer
        self.device.read_sectors(read_buffer, fat_sector as u64, 1);

        // Extract the next cluster value from the FAT entry
        let next_cluster = unsafe {
            let cluster_ptr = read_buffer.add(fat_offset as usize) as *const u32;
            *cluster_ptr & 0x0FFFFFFF
        };

        next_cluster
    }

    unsafe fn set_next_cluster(&self, cluster: u32, next_cluster: u32) {
        // Calculate the sector in the FAT that contains the entry for the given cluster
        let (fat_sector, fat_offset) = self.get_fat_sector(cluster);

        // Allocate a buffer to read the FAT sector
        let buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;

        // Read the FAT sector into the buffer
        self.device.read_sectors(buffer, fat_sector as u64, 1);

        // Calculate the pointer to the FAT entry within the buffer
        let entry_ptr = buffer.add(fat_offset as usize) as *mut u32;

        // Update the FAT entry to point to the new next cluster
        *entry_ptr = (*entry_ptr & 0xF0000000) | (next_cluster & 0x0FFFFFFF);

        // Write the modified FAT sector back to the device
        self.device.write_sectors(buffer, fat_sector as u64, 1);
    }

    fn get_fat_sector(&self, cluster: u32) -> (u32, u32) {
        let fat_sector = self.fs.first_fat_sector + (cluster * 4) / self.fs.bytes_per_sector as u32;
        let fat_offset = (cluster * 4) % self.fs.bytes_per_sector as u32;
        (fat_sector, fat_offset)
    }

    unsafe fn create_entry(
        &self,
        parent_cluster: u32,
        name: &str,
        attributes: u8,
        cluster: u32,
    ) -> Option<u32> {
        if name.is_empty() {
            println!("Error: Name is empty");
            return None;
        }

        // Generate short filename and calculate checksum
        let short_name = create_short_filename(name);
        let checksum = calculate_checksum(&short_name);

        // Generate LFN entries
        let lfn_entries = create_lfn_entries(name, checksum);

        // Find a free location to write the entries
        let (mut entry_cluster, mut entry_sector, mut sector_offset) =
            match self.locate_free_entries(parent_cluster, lfn_entries.len() + 1) {
                Some((cluster, sector, offset)) => (cluster, sector, offset),
                None => {
                    println!("Error: Failed to locate free entries");
                    return None;
                }
            };

        // Load the sector into memory
        let read_buffer = self.read_sector(entry_sector, 0);

        // Write the LFN entries to the buffer
        for lfn_entry in lfn_entries.iter() {
            let entry_ptr = read_buffer.add(sector_offset as usize);
            core::ptr::write_volatile(entry_ptr as *mut LongDirectoryEntry, *lfn_entry);
            sector_offset += size_of::<LongDirectoryEntry>() as u32;

            if sector_offset >= self.fs.bytes_per_sector as u32 {
                sector_offset %= self.fs.bytes_per_sector as u32;
                entry_sector += 1;

                if entry_sector >= self.fs.sectors_per_cluster as u32 {
                    entry_sector = 0;
                    entry_cluster = match self.next_cluster(entry_cluster) {
                        Some(next) => next,
                        None => {
                            println!("Error: Failed to allocate new cluster");
                            return None;
                        }
                    };
                }

                entry_sector = self.get_sector(entry_cluster);
                // Load the next sector into the buffer
                self.device
                    .read_sectors(read_buffer, entry_sector as u64, 1);
            }
        }

        // Write the short name entry into the buffer
        let entry_ptr = read_buffer.add(sector_offset as usize);

        core::ptr::write_volatile(
            entry_ptr as *mut DirectoryEntry,
            DirectoryEntry::new(short_name, cluster, attributes),
        );

        // Write the buffer back to disk
        self.device
            .write_sectors(read_buffer, entry_sector as u64, 1);

        Some(cluster) // Return the cluster of the new entry
    }

    unsafe fn locate_free_entries(
        &self,
        start_cluster: u32,
        required_entries: usize,
    ) -> Option<(u32, u32, u32)> {
        let mut current_cluster = start_cluster;
        let mut sector_found = 0;
        let mut offset_found = 0;
        let mut free_entries = 0;

        while self.is_valid_cluster(current_cluster) {
            let first_sector_of_cluster = self.get_sector(current_cluster);

            for sector_idx in 0..self.fs.sectors_per_cluster {
                let buffer = self.read_sector(first_sector_of_cluster, sector_idx as u32);

                for entry_idx in 0..(self.fs.bytes_per_sector / size_of::<DirectoryEntry>() as u16)
                {
                    let entry_ptr = buffer.add(entry_idx as usize * size_of::<DirectoryEntry>());
                    let dir_entry = *(entry_ptr as *const DirectoryEntry);

                    if dir_entry.is_free() {
                        if free_entries == 0 {
                            sector_found = first_sector_of_cluster + sector_idx as u32;
                            offset_found = entry_idx as u32 * size_of::<DirectoryEntry>() as u32;
                        }

                        free_entries += 1;
                        if free_entries == required_entries {
                            return Some((current_cluster, sector_found, offset_found));
                        }
                    } else {
                        free_entries = 0;
                        sector_found = 0;
                        offset_found = 0;
                    }
                }
            }

            current_cluster = self.next_cluster(current_cluster).unwrap();
        }

        None
    }

    unsafe fn next_cluster(&self, current_cluster: u32) -> Option<u32> {
        let next_cluster = self.get_next_cluster(current_cluster);
        if next_cluster >= CLUSTER_LAST {
            match self.alloc_cluster() {
                Some(new_cluster) => {
                    self.set_next_cluster(current_cluster, new_cluster);
                    Some(new_cluster)
                }
                None => {
                    println!("Error: Failed to allocate new cluster");
                    None
                }
            }
        } else {
            Some(next_cluster)
        }
    }

    fn write_to_sector(
        &self,
        buffer: *const u8,
        write_buffer: *mut u8,
        sector_start: u32,
        sector_offset: u8,
        written_bytes: usize,
        bytes_to_write: usize,
    ) {
        let sector = sector_start as u64 + sector_offset as u64;

        if bytes_to_write < self.fs.bytes_per_sector as usize {
            // Partial sector write
            self.device.read_sectors(write_buffer, sector, 1);
            unsafe {
                core::ptr::copy_nonoverlapping(
                    buffer.add(written_bytes),
                    write_buffer,
                    bytes_to_write,
                );
            }
        } else {
            // Full sector write
            unsafe {
                core::ptr::copy_nonoverlapping(
                    buffer.add(written_bytes),
                    write_buffer,
                    self.fs.bytes_per_sector as usize,
                );
            }
        }

        self.device.write_sectors(write_buffer, sector, 1);
    }

    fn update_entry(&self, node: &VfsDirectoryEntry, size: usize) {
        let sector = node.sector;
        let offset = node.offset;

        // Create a new entry with the updated size
        let mut updated_entry = node.entry;
        updated_entry.size = size as u32;

        // Allocate a buffer to read the sector containing the directory entry
        let read_buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;

        // Read the sector into the buffer
        self.device.read_sectors(read_buffer, sector as u64, 1);

        // Get the pointer to the entry location in the buffer
        let entry_ptr = unsafe { read_buffer.add(offset as usize) } as *mut DirectoryEntry;

        unsafe {
            // Update the metadata in the entry
            (*entry_ptr).update_metadata();
            // Write the updated entry back to the buffer
            core::ptr::write_volatile(entry_ptr, updated_entry);
        }

        // Write the modified sector back to the device
        self.device.write_sectors(read_buffer, sector as u64, 1);
    }

    unsafe fn clear_cluster(&self, cluster: u32) {
        let sector = self.get_sector(cluster);
        let buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;

        // Zero out the buffer
        core::ptr::write_bytes(buffer, 0, self.fs.bytes_per_sector as usize);

        // Write zeroed buffer to all sectors in the cluster
        for i in 0..self.fs.sectors_per_cluster {
            self.device
                .write_sectors(buffer, sector as u64 + i as u64, 1);
        }
    }

    fn dump_buffer(buffer: *const u8, size: usize) {
        unsafe {
            for i in 0..size {
                if i % 32 == 0 {
                    // Start of a new directory entry
                    println!();
                    print!("{:08x}: ", i);

                    // Print the name (first 11 bytes of the 32-byte directory entry)
                    let name = from_raw_parts(buffer.add(i), 11);
                    print!("Name: ");
                    name.iter().for_each(|&b| print!("{} ", b as char));
                }

                print!("{:02x} ", *buffer.add(i));

                if i % 16 == 15 || i == size - 1 {
                    println!();
                }
            }
        }
    }
}
