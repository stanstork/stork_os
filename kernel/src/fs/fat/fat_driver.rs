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
    pub start_sector: u64,
    pub sectors_count: u64,
    pub bytes_per_sector: u16,
    pub root_sectors: u32,
    pub sectors_per_cluster: u8,
    pub cluster_size: u32,
    pub first_data_sector: u32,
    pub first_fat_sector: u32,
    pub root_dir_cluster: u32,
    pub total_clusters: u32,
    pub root_dir_entries: u16,
}

pub struct FatDriver {
    pub(crate) fs: FileSystemInfo,
    pub(crate) device: AhciDevice,
}

pub const CLUSTER_FREE: u32 = 0x00000000;
pub const CLUSTER_RESERVED: u32 = 0x0FFFFFF0;
pub const CLUSTER_BAD: u32 = 0x0FFFFFF7;
pub const CLUSTER_LAST: u32 = 0x0FFFFFF8;

pub const ENTRY_END: u8 = 0x00;
pub const ENTRY_FREE: u8 = 0xE5;
pub const ENTRY_DELETED: u8 = 0x05;
pub const ENTRY_LONG: u8 = 0x0F;

pub const ATTR_DIRECTORY: u8 = 0x10;

impl FatDriver {
    pub fn mount(device: AhciDevice) -> Self {
        let boot_sector = Self::read_boot_sector(&device);

        // Compute values based on the boot sector
        let bytes_per_sector = boot_sector.bytes_per_sector as u32;
        let root_sectors = ((boot_sector.root_dir_entries as u32 * 32) + (bytes_per_sector - 1))
            / bytes_per_sector;
        let first_fat_sector = boot_sector.reserved_sectors as u32;
        let first_data_sector = first_fat_sector
            + (boot_sector.fat_count as u32 * boot_sector.sectors_per_fat_large as u32);
        let cluster_size = bytes_per_sector * boot_sector.sectors_per_cluster as u32;
        let total_clusters = (boot_sector.total_sectors_large - first_data_sector)
            / boot_sector.sectors_per_cluster as u32;

        // Initialize FileSystemInfo with computed values
        let fs_info = FileSystemInfo {
            start_sector: 0,
            sectors_count: boot_sector.total_sectors_large as u64,
            bytes_per_sector: boot_sector.bytes_per_sector,
            root_sectors,
            sectors_per_cluster: boot_sector.sectors_per_cluster,
            cluster_size,
            first_data_sector,
            first_fat_sector,
            root_dir_cluster: boot_sector.root_dir_start,
            total_clusters,
            root_dir_entries: boot_sector.root_dir_entries,
        };

        FatDriver {
            fs: fs_info,
            device,
        }
    }

    pub unsafe fn get_dir_entries(&self, cluster: u32) -> Vec<VfsDirectoryEntry> {
        let mut entries = Vec::new();
        let mut current_cluster = cluster;

        while self.is_valid_cluster(current_cluster) {
            let sector = self.get_sector(current_cluster);
            let cluster_entries = self.read_cluster_entries(sector);

            entries.extend(cluster_entries);

            current_cluster = self.get_next_cluster(current_cluster);
        }

        entries
    }

    pub fn get_dir_entry(&self, path: &str) -> Option<VfsDirectoryEntry> {
        let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        // If the path is "/", search the root directory
        if path_parts.is_empty() {
            return self.search_in_dir(self.fs.root_dir_cluster, "/");
        }

        let mut current_cluster = self.fs.root_dir_cluster;
        let mut last_entry = None;

        for (_i, part) in path_parts.iter().enumerate() {
            let entry = self.search_in_dir(current_cluster, part)?;
            if entry.is_dir() {
                current_cluster = entry.get_cluster();
                last_entry = Some(entry);
            } else {
                return Some(entry);
            }
        }

        last_entry
    }

    pub fn read_file(&self, cluster: u32, buffer: *mut u8) {
        let mut cluster = cluster;
        let mut buffer_offset = 0;

        while cluster < CLUSTER_LAST {
            let sector = self.get_sector(cluster);

            for i in 0..self.fs.sectors_per_cluster {
                self.device.read_sectors(
                    unsafe { buffer.add(buffer_offset) },
                    sector as u64 + i as u64,
                    1,
                );
                buffer_offset += self.fs.bytes_per_sector as usize;
            }

            cluster = self.get_next_cluster(cluster);
        }
    }

    pub fn create_file(&self, parent_cluster: u32, name: &str) {
        // Attempt to allocate a cluster for the new file.
        let new_file_cluster = match unsafe { self.alloc_cluster() } {
            Some(cluster) => cluster,
            None => {
                println!("Failed to allocate cluster for new file");
                return;
            }
        };

        unsafe {
            // Safely create the file entry in the parent directory.
            self.create_entry(parent_cluster, name, 0, new_file_cluster);
            // Optionally, clear the newly allocated cluster to zero to initialize it.
            self.clear_cluster(new_file_cluster);
        };
    }

    pub fn write_file(&self, entry: &mut VfsDirectoryEntry, buffer: *const u8, size: usize) {
        let clusters_needed = (size as u32 + self.fs.cluster_size - 1) / self.fs.cluster_size;
        let mut written_bytes = 0;
        let mut current_cluster = entry.get_cluster();
        let mut last_cluster = current_cluster;

        for _ in 0..clusters_needed {
            let sector = self.get_sector(current_cluster);

            for sector_offset in 0..self.fs.sectors_per_cluster {
                let bytes_left = size - written_bytes;
                let bytes_to_write = bytes_left.min(self.fs.bytes_per_sector as usize);

                let write_buffer =
                    memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;

                self.write_to_sector(
                    buffer,
                    write_buffer,
                    sector,
                    sector_offset,
                    written_bytes,
                    bytes_to_write,
                );

                written_bytes += bytes_to_write;
                if written_bytes >= size {
                    break;
                }
            }

            // Allocate new cluster if needed and update FAT
            if written_bytes < size {
                let next_cluster = self.get_next_cluster(current_cluster);
                if next_cluster == 0x0FFFFFFF {
                    current_cluster = unsafe { self.next_cluster(current_cluster).unwrap() };
                } else {
                    current_cluster = next_cluster;
                }
                last_cluster = current_cluster;
            }
        }

        // Mark the end of the cluster chain in the FAT
        unsafe {
            self.set_next_cluster(last_cluster, 0x0FFFFFFF);
        }

        // Update the directory entry with the new file size and modification time
        self.update_entry(&entry, size);
    }

    pub fn create_dir(&self, parent_cluster: u32, name: &str) {
        unsafe {
            // Allocate a new cluster for the directory
            let new_dir_cluster = match self.alloc_cluster() {
                Some(cluster) if cluster != 0 => cluster,
                _ => {
                    println!("Failed to allocate cluster for new directory");
                    return;
                }
            };

            // Create the directory entry in the parent directory
            self.create_entry(parent_cluster, name, ATTR_DIRECTORY, new_dir_cluster);

            // Calculate the sector for the new directory
            let sector = self.get_sector(new_dir_cluster);

            // Create the "." entry for the new directory
            let dot_entry = DirectoryEntry::create_dot_entry(new_dir_cluster);
            let dotdot_entry = DirectoryEntry::create_dotdot_entry(parent_cluster);

            // Read the sector where the new directory is located
            let buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;
            self.device.read_sectors(buffer, sector as u64, 1);

            // Write the "." entry at the beginning of the directory
            core::ptr::write_volatile(buffer as *mut DirectoryEntry, dot_entry);

            // Write the ".." entry just after the "." entry
            core::ptr::write_volatile(
                buffer.add(size_of::<DirectoryEntry>() as usize) as *mut DirectoryEntry,
                dotdot_entry,
            );

            // Write the buffer back to the disk
            self.device.write_sectors(buffer, sector as u64, 1);
        }
    }

    pub fn delete_entry(&self, node: &VfsDirectoryEntry) {
        let sector = node.sector;
        let offset = node.offset;

        // Allocate a buffer to read the sector containing the directory entry
        let read_buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;

        // Read the sector into the buffer
        self.device.read_sectors(read_buffer, sector as u64, 1);

        // Get a pointer to the directory entry within the buffer
        let entry_ptr = unsafe { read_buffer.add(offset as usize) as *mut DirectoryEntry };

        // Mark the directory entry as deleted
        unsafe {
            (*entry_ptr).name[0] = ENTRY_DELETED;
        }

        // Write the modified sector back to the device
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
