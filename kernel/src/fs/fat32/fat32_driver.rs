use super::{DirectoryEntry, Fat32BootSector};
use crate::{
    fs::{
        entry,
        fat32::LongDirectoryEntry,
        node::{self, VfsEntry},
    },
    memory, print, println,
    storage::ahci_device::AhciDevice,
};
use alloc::{string::String, vec::Vec};
use core::{intrinsics::size_of, slice::from_raw_parts};

pub struct FatFileSystem {
    pub device: AhciDevice,
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
    pub(crate) volume_id: u32,
    pub(crate) fs: FatFileSystem,
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
    pub fn mount(device: AhciDevice, start_sector: u64, sectors_count: u64) -> Self {
        let read_buffer = crate::memory::allocate_dma_buffer(512) as *mut u8;

        device.read(read_buffer, start_sector, 1);

        let boot_sector = read_buffer as *const Fat32BootSector;
        let boot_sector = unsafe { *boot_sector };

        let root_sectors = ((boot_sector.root_dir_entries as u32 * 32)
            + (boot_sector.bytes_per_sector as u32 - 1))
            / boot_sector.bytes_per_sector as u32;
        let first_data_sector = boot_sector.reserved_sectors as u32
            + (boot_sector.fat_count as u32 * boot_sector.sectors_per_fat_large as u32);
        let total_clusters = (boot_sector.total_sectors_large - first_data_sector as u32)
            / boot_sector.sectors_per_cluster as u32;

        FatDriver {
            volume_id: boot_sector.serial_number,
            fs: FatFileSystem {
                device,
                start_sector,
                sectors_count,
                bytes_per_sector: boot_sector.bytes_per_sector,
                root_sectors,
                sectors_per_cluster: boot_sector.sectors_per_cluster,
                cluster_size: boot_sector.bytes_per_sector as u32
                    * boot_sector.sectors_per_cluster as u32,
                first_data_sector,
                first_fat_sector: boot_sector.reserved_sectors as u32,
                root_dir_cluster: boot_sector.root_dir_start,
                total_clusters,
                root_dir_entries: boot_sector.root_dir_entries,
            },
            device,
        }
    }

    pub fn get_node(&self, path: &str) -> Option<VfsEntry> {
        let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        println!("Path: {:?}", path_parts);

        // If the path is "/", return the root directory node
        if path_parts.is_empty() {
            println!("Returning root directory");
            return self.search_in_dir(self.fs.root_dir_cluster, "/", true);
        }

        let mut current_cluster = self.fs.root_dir_cluster;

        for (i, part) in path_parts.iter().enumerate() {
            let node = self.search_in_dir(current_cluster, part, i == 0)?;
            let is_dir = node.entry.attributes & 0x10 != 0;
            if is_dir {
                current_cluster = node.entry.high_cluster as u32 | node.entry.low_cluster as u32;
            } else {
                return Some(node);
            }
        }

        None
    }

    fn search_in_dir(&self, cluster: u32, name: &str, root: bool) -> Option<VfsEntry> {
        println!("Cluster: {}, Name: {}", cluster, name);
        let entries = unsafe { self.get_dir_entries(cluster) };

        for node in entries {
            let entry_name = self.parse_short_filename(node.entry.name.as_ptr());
            if entry_name.eq_ignore_ascii_case(name) {
                return Some(VfsEntry {
                    entry: node.entry,
                    name: node.name,
                    sector: node.sector,
                    offset: node.offset,
                });
            }
        }

        None
    }

    pub unsafe fn get_dir_entries(&self, cluster: u32) -> Vec<VfsEntry> {
        let mut entries = Vec::new();
        let mut long_filename_entries = Vec::new();
        let mut cluster = cluster;

        while (cluster != CLUSTER_FREE) && (cluster < CLUSTER_LAST) {
            let sector =
                self.fs.first_data_sector + (cluster - 2) * self.fs.sectors_per_cluster as u32;

            println!("Cluster: {}, Sector: {}", cluster, sector);
            println!("Sectors per cluster: {}", self.fs.sectors_per_cluster);

            for i in 0..self.fs.sectors_per_cluster {
                let read_buffer =
                    memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;
                self.device.read(read_buffer, sector as u64 + i as u64, 1);

                // Debugging: Dump the buffer to the console
                // Self::dump_buffer(read_buffer, self.fs.bytes_per_sector as usize);

                for j in 0..(self.fs.bytes_per_sector / size_of::<DirectoryEntry>() as u16) {
                    let entry_ptr = read_buffer.add(j as usize * size_of::<DirectoryEntry>());
                    let entry = *(entry_ptr as *const DirectoryEntry);

                    if entry.name[0] == ENTRY_END {
                        return entries;
                    }

                    if entry.name[0] == ENTRY_FREE || entry.name[0] == ENTRY_DELETED {
                        continue;
                    }

                    if entry.attributes == ENTRY_LONG {
                        long_filename_entries.push(*(entry_ptr as *const LongDirectoryEntry));
                        continue;
                    };

                    let name = if long_filename_entries.is_empty() {
                        self.parse_short_filename(entry.name.as_ptr())
                    } else {
                        let long_name = self.parse_long_filename(&long_filename_entries);
                        long_filename_entries.clear();
                        long_name
                    };

                    let node = VfsEntry {
                        entry,
                        name,
                        sector: sector + i as u32,
                        offset: j as u32 + size_of::<DirectoryEntry>() as u32,
                    };
                    entries.push(node);
                }
            }

            cluster = self.get_next_cluster(cluster);
        }

        entries
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

    pub fn write_file(&self, path: &str, buffer: *mut u8, create: bool) {
        let file_exists = self.file_exists(path);
        if create && !file_exists {
            unsafe {
                self.create_entry(self.fs.root_dir_cluster, path, 0, self.fs.root_dir_cluster);
            }
        }
    }

    fn create_dir_entry(&self, name: &str, attributes: u8) -> DirectoryEntry {
        let path_parts: Vec<&str> = name.split('/').collect();
        todo!()
    }

    unsafe fn create_entry(&self, parent_cluster: u32, name: &str, attributes: u8, cluster: u32) {
        if name.is_empty() {
            return;
        }

        let required_entries = 1; // Only one entry needed for the short name
        let (mut entry_cluster, mut entry_sector, mut sector_offset) =
            self.find_free_loc(parent_cluster, required_entries);

        if entry_cluster == 0 {
            return;
        }

        println!(
            "Free location: {:?}",
            (entry_cluster, entry_sector, sector_offset)
        );

        let short_name = self.create_short_filename(name);

        // Load the sector into memory
        let read_buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;
        self.device.read(read_buffer, entry_sector as u64, 1);

        // Handle sector overflow and cluster allocation
        if sector_offset >= self.fs.bytes_per_sector as u32 {
            println!("Sector overflow");
            sector_offset %= self.fs.bytes_per_sector as u32;
            entry_sector += 1;

            println!("Entry sector: {}", entry_sector);

            if entry_sector >= self.fs.sectors_per_cluster as u32 {
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

        // Write the short name entry into the buffer
        let entry_ptr = read_buffer.add(sector_offset as usize);
        let entry = entry_ptr as *mut DirectoryEntry;
        *entry = DirectoryEntry {
            name: short_name,
            attributes,
            reserved: 0,
            creation_time_tenths: 0,
            creation_time: 0,
            creation_date: 0,
            access_date: 0,
            high_cluster: (cluster >> 16) as u16,
            modification_time: 0,
            modification_date: 0,
            low_cluster: (cluster & 0xFFFF) as u16,
            size: 0,
        };

        // Write the buffer back to disk
        self.device.write(read_buffer, entry_sector as u64, 1);

        println!("Short entry written");
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

    fn create_lfn_entries(&self, name: &str) -> Vec<LongDirectoryEntry> {
        let mut entries = Vec::new();
        let name_chars = name.chars().collect::<Vec<char>>();
        let mut name_chars_idx = 0;

        let mut entry = LongDirectoryEntry::default();
        entry.order = 0x40;

        while name_chars_idx < name_chars.len() {
            for i in 0..5 {
                entry.name1[i] = if name_chars_idx < name_chars.len() {
                    name_chars[name_chars_idx] as u16
                } else {
                    0xFFFF
                };
                name_chars_idx += 1;
            }

            for i in 0..6 {
                entry.name2[i] = if name_chars_idx < name_chars.len() {
                    name_chars[name_chars_idx] as u16
                } else {
                    0xFFFF
                };
                name_chars_idx += 1;
            }

            for i in 0..2 {
                entry.name3[i] = if name_chars_idx < name_chars.len() {
                    name_chars[name_chars_idx] as u16
                } else {
                    0xFFFF
                };
                name_chars_idx += 1;
            }

            if name_chars_idx == name_chars.len() {
                entry.order |= 0x40;
            }

            entries.push(entry);
            entry.order += 1;
        }

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

        let mut buffer = memory::allocate_dma_buffer(self.fs.bytes_per_sector as usize) as *mut u8;
        self.device.read(buffer, fat_sector as u64, 1);

        buffer = buffer.add(fat_offset as usize);
        *buffer = next_cluster as u8;

        self.device.write(buffer, fat_sector as u64, 1);
    }
}

pub struct DiskLocation {
    pub cluster: u32,
    pub sector: u32,
    pub offset: u32,
}
