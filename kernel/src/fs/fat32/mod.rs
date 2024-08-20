pub(crate) mod fat32_driver;
use core::{fmt::Debug, pin};

use alloc::{string::String, vec::Vec};

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct Fat32BootSector {
    pub jump_instruction: [u8; 3],
    pub oem_name: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub fat_count: u8,
    pub root_dir_entries: u16,
    pub total_sectors: u16,
    pub media_descriptor: u8,
    pub sectors_per_fat: u16,
    pub sectors_per_track: u16,
    pub head_count: u16,
    pub hidden_sectors: u32,
    pub total_sectors_large: u32,
    pub sectors_per_fat_large: u32,
    pub flags: u16,
    pub version: u16,
    pub root_dir_start: u32,
    pub fs_info_sector: u16,
    pub backup_boot_sector: u16,
    pub reserved0: u32,
    pub reserved1: u32,
    pub reserved2: u32,
    pub drive_number: u8,
    pub reserved3: u8,
    pub ext_signature: u8,
    pub serial_number: u32,
    pub volume_label: [u8; 11],
    pub system_id: [u8; 8],
    pub boot_code: [u8; 420],
    pub boot_signature: u16,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct DirectoryEntry {
    pub name: [u8; 11],
    pub attributes: u8,
    pub reserved: u8,
    pub creation_time_tenths: u8,
    pub creation_time: u16,
    pub creation_date: u16,
    pub access_date: u16,
    pub high_cluster: u16,
    pub modification_time: u16,
    pub modification_date: u16,
    pub low_cluster: u16,
    pub size: u32,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct LongDirectoryEntry {
    pub order: u8,
    pub name1: [u16; 5],
    pub attributes: u8,
    pub reserved1: u8,
    pub checksum: u8,
    pub name2: [u16; 6],
    pub reserved2: u16,
    pub name3: [u16; 2],
}

#[derive(Clone, Copy)]
pub struct Fat32FsInfo {
    pub lead_signature: u32,
    pub reserved0: [u8; 480],
    pub structure_signature: u32,
    pub free_cluster_count: u32,
    pub next_free_cluster: u32,
    pub reserved1: [u8; 12],
    pub trail_signature: u32,
}

impl Default for LongDirectoryEntry {
    fn default() -> Self {
        Self {
            order: 0,
            name1: [0; 5],
            attributes: 0,
            reserved1: 0,
            checksum: 0,
            name2: [0; 6],
            reserved2: 0,
            name3: [0; 2],
        }
    }
}
