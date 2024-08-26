use crate::{
    cpu::rtc::RTC,
    fs::{
        fat::{directory_entry::DirectoryEntry, long_directory_entry::LongDirectoryEntry},
        vfs_directory_entry::VfsDirectoryEntry,
    },
    memory, print, println,
    storage::ahci_device::AhciDevice,
};
use alloc::{string::String, vec::Vec};
use core::{intrinsics::size_of, slice::from_raw_parts};

use super::boot_sector::Fat32BootSector;

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

    fn read_boot_sector(device: &AhciDevice) -> Fat32BootSector {
        let read_buffer = crate::memory::allocate_dma_buffer(512) as *mut u8;
        device.read(read_buffer, 0, 1);
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
            .read(read_buffer, sector as u64 + offset as u64, 1);
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

    pub fn get_node(&self, path: &str) -> Option<VfsDirectoryEntry> {
        let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        println!("Path: {:?}", path_parts);

        // If the path is "/", return the root directory node
        if path_parts.is_empty() {
            println!("Returning root directory");
            return self.search_in_dir(self.fs.root_dir_cluster, "/");
        }

        let mut current_cluster = self.fs.root_dir_cluster;
        let mut last_node = None;

        for (i, part) in path_parts.iter().enumerate() {
            let node = self.search_in_dir(current_cluster, part)?;
            let is_dir = node.entry.attributes & 0x10 != 0;
            if is_dir {
                current_cluster = node.entry.high_cluster as u32 | node.entry.low_cluster as u32;
                last_node = Some(node);
            } else {
                return Some(node);
            }
        }

        last_node
    }

    fn parse_short_filename(&self, filename_ptr: *const u8) -> String {
        let mut filename = Vec::new();

        unsafe {
            // Read the main filename part (first 8 bytes)
            for i in 0..8 {
                let byte = *filename_ptr.add(i);
                if byte != b' ' && byte != 0x00 {
                    filename.push(byte);
                }
            }

            // Read the extension (next 3 bytes)
            let mut has_extension = false;
            for i in 8..11 {
                let byte = *filename_ptr.add(i);
                if byte != b' ' && byte != 0x00 {
                    if !has_extension {
                        filename.push(b'.');
                        has_extension = true;
                    }
                    filename.push(byte);
                }
            }
        }

        // Convert to String and return
        String::from_utf8_lossy(&filename).to_lowercase()
    }

    fn parse_long_filename(&self, long_entries: &Vec<LongDirectoryEntry>) -> String {
        let mut long_name = Vec::with_capacity(128);

        // Iterate in reverse as LFNs are stored in reverse order
        let mut l_entry_iter = long_entries.iter().rev();
        while let Some(l_entry) = l_entry_iter.next() {
            for i in 0..5 {
                long_name.push(l_entry.name1[i] as u8);
            }
            for i in 0..6 {
                long_name.push(l_entry.name2[i] as u8);
            }
            for i in 0..2 {
                long_name.push(l_entry.name3[i] as u8);
            }

            if (l_entry.order & 0x40) == 0x40 {
                // Check if this is the last LFN entry
                break;
            }
        }

        // Trim the vector to remove invalid characters (e.g., null characters)
        long_name.retain(|&x| x != 0x00 && x != 0xFF);

        String::from_utf8(long_name).unwrap()
    }

    fn get_next_cluster(&self, cluster: u32) -> u32 {
        let fat_sector = self.fs.first_fat_sector + (cluster * 4) / self.fs.bytes_per_sector as u32;
        let fat_offset = (cluster * 4) % self.fs.bytes_per_sector as u32;

        let read_buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;
        self.device.read(read_buffer, fat_sector as u64, 1);

        let next_cluster = unsafe {
            let next_cluster = read_buffer.offset(fat_offset as isize) as *const u32;
            *next_cluster & 0x0FFFFFFF
        };

        next_cluster
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

    pub fn read_file(&self, path: &str, buffer: *mut u8) {
        let node = self.get_node(path).unwrap();
        let cluster = node.entry.high_cluster as u32 | node.entry.low_cluster as u32;

        let mut cluster = cluster;
        let mut buffer_offset = 0;

        while cluster < CLUSTER_LAST {
            let sector =
                self.fs.first_data_sector + (cluster - 2) * self.fs.sectors_per_cluster as u32;

            for i in 0..self.fs.sectors_per_cluster {
                self.device.read(
                    unsafe { buffer.add(buffer_offset) },
                    sector as u64 + i as u64,
                    1,
                );
                buffer_offset += self.fs.bytes_per_sector as usize;
            }

            cluster = self.get_next_cluster(cluster);
        }
    }

    pub fn size(&self, path: &str) -> Option<usize> {
        let node = self.get_node(path)?;
        if node.entry.attributes & ATTR_DIRECTORY != 0 {
            return None;
        }
        Some(node.entry.size as usize)
    }

    pub fn file_exists(&self, path: &str) -> bool {
        let node = self.get_node(path);
        if let Some(node) = node {
            let is_dir = node.entry.attributes & 0x10 != 0;
            return !is_dir;
        }
        false
    }

    pub fn create_file(&self, path: &str) {
        let file_exists = self.file_exists(path);
        if file_exists {
            println!("File already exists");
            return;
        }

        let path_parts: Vec<&str> = path.split('/').collect();
        let parent_path = path_parts[..path_parts.len() - 1].join("/");
        let parent_cluster = if parent_path.is_empty() {
            self.fs.root_dir_cluster
        } else {
            println!("Parent path: {}", parent_path);
            let parent_node = self.get_node(&parent_path).unwrap();
            parent_node.entry.high_cluster as u32 | parent_node.entry.low_cluster as u32
        };

        unsafe {
            // Allocate a cluster for the new file (even if it's initially empty)
            let new_file_cluster = self.alloc_cluster();
            if new_file_cluster == 0 {
                println!("Failed to allocate cluster for new file");
                return;
            }

            // Create the file entry in the parent directory
            self.create_entry(
                parent_cluster,
                path_parts.last().unwrap(),
                0,
                new_file_cluster,
            );

            // Initialize the allocated cluster for the new file to zero (optional but recommended)
            self.clear_cluster(new_file_cluster);
        }
    }

    pub fn create_dir(&self, path: &str) {
        let file_exists = self.file_exists(path);
        if file_exists {
            println!("Directory already exists");
            return;
        }

        let path_parts: Vec<&str> = path.split('/').collect();
        let parent_path = path_parts[..path_parts.len() - 1].join("/");
        let parent_cluster = if parent_path.is_empty() {
            self.fs.root_dir_cluster
        } else {
            let parent_node = self.get_node(&parent_path).unwrap();
            parent_node.entry.high_cluster as u32 | parent_node.entry.low_cluster as u32
        };

        unsafe {
            // Allocate a new cluster for the directory
            let new_dir_cluster = self.alloc_cluster();
            if new_dir_cluster == 0 {
                println!("Failed to allocate cluster for new directory");
                return;
            }

            // Create the directory entry in the parent directory
            self.create_entry(
                parent_cluster,
                path_parts.last().unwrap(),
                ATTR_DIRECTORY,
                new_dir_cluster,
            );

            // Calculate the sector for the new directory
            let sector = self.fs.first_data_sector
                + (new_dir_cluster - 2) * self.fs.sectors_per_cluster as u32;

            // Create the "." entry for the new directory
            let mut dot_entry = DirectoryEntry {
                name: [
                    b'.', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
                ],
                attributes: ATTR_DIRECTORY,
                reserved: 0,
                creation_time_tenths: 0,
                creation_time: 0,
                creation_date: 0,
                access_date: 0,
                high_cluster: (new_dir_cluster >> 16) as u16,
                modification_time: 0,
                modification_date: 0,
                low_cluster: (new_dir_cluster & 0xFFFF) as u16,
                size: 0,
            };

            // Read the sector where the new directory is located
            let read_buffer =
                memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;
            self.device.read(read_buffer, sector as u64, 1);

            // Write the "." entry at the beginning of the directory
            core::ptr::write_volatile(read_buffer as *mut DirectoryEntry, dot_entry);

            // Update the ".." entry to point to the parent directory
            dot_entry.name[1] = b'.';
            dot_entry.high_cluster = (parent_cluster >> 16) as u16;
            dot_entry.low_cluster = (parent_cluster & 0xFFFF) as u16;

            // Write the ".." entry just after the "." entry
            core::ptr::write_volatile(
                read_buffer.add(size_of::<DirectoryEntry>() as usize) as *mut DirectoryEntry,
                dot_entry,
            );

            // Write the buffer back to the disk
            self.device.write(read_buffer, sector as u64, 1);
        }
    }

    fn clear_cluster(&self, cluster: u32) {
        let sector = self.fs.first_data_sector + (cluster - 2) * self.fs.sectors_per_cluster as u32;
        let buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;

        unsafe {
            // Zero out the buffer
            core::ptr::write_bytes(buffer, 0, self.fs.bytes_per_sector as usize);
        }

        // Write zeroed buffer to all sectors in the cluster
        for i in 0..self.fs.sectors_per_cluster {
            self.device.write(buffer, sector as u64 + i as u64, 1);
        }
    }

    pub fn write_file(&self, path: &str, buffer: *const u8, size: usize) {
        let file_exists = self.file_exists(path);
        println!("File exists: {}", file_exists);
        if !file_exists {
            self.create_file(path);
        }

        let mut node = self.get_node(path).unwrap();
        println!("Node: {}", node.name);

        let mut clusters = (size as u32) / self.fs.cluster_size;
        if (size as u32) % self.fs.cluster_size != 0 {
            clusters += 1;
        }

        let mut written_bytes = 0;
        let mut cluster = (node.entry.high_cluster as u32) << 16 | node.entry.low_cluster as u32;
        let mut last_cluster = cluster;

        for _ in 0..clusters {
            let sector =
                self.fs.first_data_sector + (cluster - 2) * self.fs.sectors_per_cluster as u32;

            for s in 0..self.fs.sectors_per_cluster {
                let bytes_left = size - written_bytes;
                let bytes_to_write = if bytes_left < self.fs.bytes_per_sector as usize {
                    bytes_left
                } else {
                    self.fs.bytes_per_sector as usize
                };

                let write_buffer =
                    memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;

                if bytes_to_write < self.fs.bytes_per_sector as usize {
                    // Partial write: read existing sector, modify, and write back
                    self.device.read(write_buffer, sector as u64 + s as u64, 1);
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            buffer.add(written_bytes),
                            write_buffer,
                            bytes_to_write,
                        );
                    }

                    // println!("Buffer content before partial write:");
                    // Self::print_buffer(unsafe { buffer.add(written_bytes) }, bytes_to_write);

                    self.device.write(write_buffer, sector as u64 + s as u64, 1);
                } else {
                    // Full sector write
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            buffer.add(written_bytes),
                            write_buffer,
                            self.fs.bytes_per_sector as usize,
                        );
                    }

                    // println!("Buffer content before full write:");
                    // Self::print_buffer(unsafe { buffer.add(written_bytes) }, bytes_to_write);

                    self.device.write(write_buffer, sector as u64 + s as u64, 1);
                }

                written_bytes += bytes_to_write;

                if written_bytes >= size {
                    break;
                }
            }

            // Allocate new cluster if needed and update FAT
            if written_bytes < size {
                let next_cluster = self.get_next_cluster(cluster);
                if next_cluster == 0x0FFFFFFF {
                    // Allocate a new cluster if needed
                    let new_cluster = unsafe { self.alloc_cluster() };
                    if new_cluster == 0 {
                        println!("Failed to allocate new cluster");
                        return;
                    }

                    unsafe { self.set_next_cluster(cluster, new_cluster) };
                    cluster = new_cluster;
                } else {
                    cluster = next_cluster;
                }
                last_cluster = cluster;
            }
        }

        // Mark the end of the cluster chain in the FAT
        unsafe { self.set_next_cluster(last_cluster, 0x0FFFFFFF) };

        // Update the file size and modification time
        unsafe { Self::update_file_metadata(&mut node.entry) };

        // Update the directory entry with the new file size and modification time
        self.update_entry(&node, size);
    }

    fn print_buffer(buffer: *const u8, size: usize) {
        for i in 0..size {
            unsafe {
                print!("{:02x} ", *buffer.add(i));
            }
            if i % 16 == 15 {
                println!();
            }
        }
        println!();
    }

    fn update_entry(&self, node: &VfsDirectoryEntry, size: usize) {
        let sector = node.sector;
        let offset = node.offset;

        println!("Sector: {}, Offset: {}", sector, offset);

        let mut new_entry = node.entry;
        new_entry.size = size as u32;

        let read_buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;
        self.device.read(read_buffer, sector as u64, 1);

        let entry_ptr = unsafe { read_buffer.add(offset as usize) };

        // Dump the buffer starting from the entry position
        // Self::dump_buffer(
        //     entry_ptr as *const u8, // Start dumping from the entry
        //     self.fs.bytes_per_sector as usize - offset as usize, // Dump from the entry to the end of the sector
        // );

        let entry = entry_ptr as *mut DirectoryEntry;

        unsafe {
            println!(
                "Updating entry: {}",
                self.parse_short_filename((*entry).name.as_ptr())
            );
        }

        unsafe {
            core::ptr::write_volatile(entry_ptr as *mut DirectoryEntry, new_entry);
        }

        self.device.write(read_buffer, sector as u64, 1);
    }

    fn convert_to_fat_date(year: u16, month: u16, day: u16) -> u16 {
        ((year - 1980) << 9) | (month << 5) | day
    }

    fn convert_to_fat_time(hours: u16, minutes: u16) -> u16 {
        (hours << 11) | (minutes << 5) | (0) // Assuming seconds are zero
    }

    unsafe fn update_file_metadata(node: &mut DirectoryEntry) {
        let (hour, minute, second) = RTC.lock().read_time();
        let (day, month, year) = RTC.lock().read_date();

        println!(
            "Updating file metadata: {}-{}-{} {}:{}:{}",
            year, month, day, hour, minute, second
        );

        node.modification_date = Self::convert_to_fat_date(year as u16, month as u16, day as u16);
        node.modification_time = Self::convert_to_fat_time(hour as u16, minute as u16);
    }

    fn update_node(&self, node: &VfsDirectoryEntry) {
        let sector = node.sector;
        let offset = node.offset;

        let read_buffer = memory::allocate_dma_buffer(node.entry.size as usize) as *mut u8;
        self.device.read(read_buffer, sector as u64, 1);

        let entry_ptr = unsafe { read_buffer.add(offset as usize) };
        unsafe {
            core::ptr::write_volatile(entry_ptr as *mut DirectoryEntry, node.entry);
        }

        self.device.write(read_buffer, sector as u64, 1);
    }

    unsafe fn create_dir_entry(&self, name: &str) {
        let path_parts: Vec<&str> = name.split('/').collect();
        let parent_path = path_parts[..path_parts.len() - 1].join("/");
        let parent_cluster = if parent_path.is_empty() {
            self.fs.root_dir_cluster
        } else {
            let parent_node = self.get_node(&parent_path).unwrap();
            parent_node.entry.high_cluster as u32 | parent_node.entry.low_cluster as u32
        };
        let name = path_parts.last().unwrap();
        Self::create_entry(self, parent_cluster, &name, ATTR_DIRECTORY, CLUSTER_FREE);
    }

    unsafe fn create_entry(
        &self,
        parent_cluster: u32,
        name: &str,
        attributes: u8,
        cluster: u32,
    ) -> u32 {
        if name.is_empty() {
            return 0;
        }

        // Generate the short filename
        let short_name = self.create_short_filename(name);
        println!("Short filename: {:?}", short_name);

        // Calculate the checksum for the short filename
        let checksum = self.calculate_checksum(&short_name);
        println!("Checksum: 0x{:02X}", checksum);

        // Generate LFN entries
        let lfn_entries = Self::create_lfn_entries(name, checksum);

        // Find a free location to write the entries
        let (mut entry_cluster, mut entry_sector, mut sector_offset) =
            self.find_free_loc(parent_cluster, lfn_entries.len() + 1);

        if entry_cluster == 0 {
            return 0;
        }

        // Load the sector into memory
        let read_buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;
        self.device.read(read_buffer, entry_sector as u64, 1);

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
                    let next_cluster = self.get_next_cluster(entry_cluster);

                    if next_cluster >= CLUSTER_LAST {
                        let new_cluster = self.alloc_cluster();
                        self.set_next_cluster(entry_cluster, new_cluster);
                        entry_cluster = new_cluster;
                    } else {
                        entry_cluster = next_cluster;
                    }
                }

                entry_sector = self.fs.first_data_sector
                    + (entry_cluster - 2) * self.fs.sectors_per_cluster as u32
                    + entry_sector;

                // Load the next sector into the buffer
                self.device.read(read_buffer, entry_sector as u64, 1);
            }
        }

        // Write the short name entry into the buffer
        let entry_ptr = read_buffer.add(sector_offset as usize);
        let (hour, minute, second) = RTC.lock().read_time();
        let (day, month, year) = RTC.lock().read_date();

        core::ptr::write_volatile(
            entry_ptr as *mut DirectoryEntry,
            DirectoryEntry {
                name: short_name,
                attributes,
                reserved: 0,
                creation_time_tenths: 100,
                creation_time: Self::convert_to_fat_time(hour as u16, minute as u16),
                creation_date: Self::convert_to_fat_date(year as u16, month as u16, day as u16),
                access_date: 0,
                high_cluster: (cluster >> 16) as u16,
                modification_time: 0,
                modification_date: 0,
                low_cluster: (cluster & 0xFFFF) as u16,
                size: 0,
            },
        );

        // Write the buffer back to disk
        self.device.write(read_buffer, entry_sector as u64, 1);

        println!("Entry written successfully");
        entry_cluster
    }

    pub fn delete_file(&self, path: &str) {
        let node = self.get_node(path).unwrap();
        unsafe { self.delete_entry(&node) };
    }

    unsafe fn delete_entry(&self, node: &VfsDirectoryEntry) {
        let sector = node.sector;
        let offset = node.offset;

        let read_buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;
        self.device.read(read_buffer, sector as u64, 1);

        let entry_ptr = read_buffer.add(offset as usize);
        let entry = entry_ptr as *mut DirectoryEntry;

        (*entry).name[0] = ENTRY_DELETED;

        self.device.write(read_buffer, sector as u64, 1);
    }

    unsafe fn find_free_loc(&self, cluster: u32, needed: usize) -> (u32, u32, u32) {
        println!("Finding free location for {} entries", needed);
        println!("Cluster: {}", cluster);

        let mut cluster = cluster;
        let mut sector = 0;
        let mut offset = 0;
        let mut free_count = 0;

        while (cluster != CLUSTER_FREE) && (cluster < CLUSTER_LAST) {
            let first_sector =
                self.fs.first_data_sector + (cluster - 2) * self.fs.sectors_per_cluster as u32;

            for sector_idx in 0..self.fs.sectors_per_cluster {
                let buffer =
                    memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;
                self.device
                    .read(buffer, first_sector as u64 + sector_idx as u64, 1);

                for j in 0..(self.fs.bytes_per_sector / size_of::<DirectoryEntry>() as u16) {
                    let entry_ptr = buffer.add(j as usize * size_of::<DirectoryEntry>());
                    let entry = *(entry_ptr as *const DirectoryEntry);

                    if entry.name[0] == ENTRY_END || entry.name[0] == ENTRY_FREE {
                        if free_count == 0 {
                            sector = first_sector + sector_idx as u32;
                            offset = j as u32 * size_of::<DirectoryEntry>() as u32;
                        }

                        free_count += 1;
                        if free_count == needed {
                            return (cluster, sector, offset);
                        }
                    } else {
                        free_count = 0;
                        sector = 0;
                        offset = 0;
                    }
                }
            }

            let next_cluster = self.get_next_cluster(cluster);
            cluster = if next_cluster >= CLUSTER_LAST {
                let new_cluster = self.alloc_cluster();
                self.set_next_cluster(cluster, new_cluster);
                new_cluster
            } else {
                next_cluster
            };
        }

        (0, 0, 0)
    }

    fn create_short_filename(&self, name: &str) -> [u8; 11] {
        let mut short_name = [b' '; 11];
        let mut short_name_idx = 0;

        for c in name.chars() {
            if short_name_idx == 11 {
                break;
            }

            if c == '.' {
                short_name_idx = 8;
                continue;
            }

            short_name[short_name_idx] = c as u8;
            short_name_idx += 1;
        }

        short_name
    }

    fn calculate_checksum(&self, short_name: &[u8]) -> u8 {
        let mut checksum = 0u8;
        for &byte in short_name {
            checksum = ((checksum & 1) << 7).wrapping_add((checksum >> 1).wrapping_add(byte));
        }
        checksum
    }

    fn create_lfn_entries(name: &str, checksum: u8) -> Vec<LongDirectoryEntry> {
        let name_len = name.len();
        let long_entries = (name_len / 13) + 1;

        // Create a buffer for LongDirectoryEntry structures
        let mut entries = Vec::with_capacity(long_entries);

        let utf16_name: Vec<u16> = name.encode_utf16().collect();
        let mut counter = 0;

        for j in 0..long_entries {
            let mut lfn_entry = LongDirectoryEntry {
                order: (j + 1) as u8,
                name1: [0; 5],
                attributes: ENTRY_LONG,
                reserved1: 0,
                checksum,
                name2: [0; 6],
                reserved2: 0,
                name3: [0; 2],
            };

            if j == (long_entries - 1) {
                lfn_entry.order |= 0x40; // Mark as the last LFN entry
            }

            // Fill name1 (5 characters)
            for i in 0..5 {
                if counter >= utf16_name.len() {
                    lfn_entry.name1[i] = 0;
                } else {
                    lfn_entry.name1[i] = utf16_name[counter];
                    counter += 1;
                }
            }

            // Fill name2 (6 characters)
            for i in 0..6 {
                if counter >= utf16_name.len() {
                    lfn_entry.name2[i] = 0;
                } else {
                    lfn_entry.name2[i] = utf16_name[counter];
                    counter += 1;
                }
            }

            // Fill name3 (2 characters)
            for i in 0..2 {
                if counter >= utf16_name.len() {
                    lfn_entry.name3[i] = 0;
                } else {
                    lfn_entry.name3[i] = utf16_name[counter];
                    counter += 1;
                }
            }

            entries.push(lfn_entry);
        }

        entries.reverse(); // Reverse the order to prepare for writing to disk
        entries
    }

    unsafe fn alloc_cluster(&self) -> u32 {
        (self.fs.root_dir_cluster..self.fs.total_clusters)
            .find(|&cluster| self.get_next_cluster(cluster) == CLUSTER_FREE)
            .map(|cluster| {
                self.set_next_cluster(cluster, CLUSTER_LAST);
                cluster
            })
            .unwrap_or(0)
    }

    unsafe fn set_next_cluster(&self, cluster: u32, next_cluster: u32) {
        let fat_sector = self.fs.first_fat_sector + (cluster * 4) / self.fs.bytes_per_sector as u32;
        let fat_offset = (cluster * 4) % self.fs.bytes_per_sector as u32;

        let buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;
        self.device.read(buffer, fat_sector as u64, 1);

        let entry_ptr = buffer.add(fat_offset as usize) as *mut u32;
        *entry_ptr = (*entry_ptr & 0xF0000000) | (next_cluster & 0x0FFFFFFF);

        self.device.write(buffer, fat_sector as u64, 1);
    }
}
