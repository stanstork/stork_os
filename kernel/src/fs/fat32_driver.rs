use core::fmt::Debug;

use crate::storage::ahci_device::AhciDevice;

#[repr(C, packed)]
pub struct FAT32_BootSector {
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

impl Debug for FAT32_BootSector {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let bytes_per_sector = self.bytes_per_sector;
        let sectors_per_cluster = self.sectors_per_cluster;
        let reserved_sectors = self.reserved_sectors;
        let total_sectors = self.total_sectors;
        let sectors_per_track = self.sectors_per_track;
        let root_dir_entries = self.root_dir_entries;
        let media_descriptor = self.media_descriptor;
        let sectors_per_fat = self.sectors_per_fat;
        let head_count = self.head_count;
        let hidden_sectors = self.hidden_sectors;
        let total_sectors_large = self.total_sectors_large;
        let sectors_per_fat_large = self.sectors_per_fat_large;
        let flags = self.flags;
        let version = self.version;
        let root_dir_start = self.root_dir_start;
        let fs_info_sector = self.fs_info_sector;
        let backup_boot_sector = self.backup_boot_sector;
        let serial_number = self.serial_number;
        let volume_label = core::str::from_utf8(&self.volume_label).unwrap();
        let system_id = core::str::from_utf8(&self.system_id).unwrap();
        let boot_signature = self.boot_signature;
        let oem_name = core::str::from_utf8(&self.oem_name).unwrap();

        f.debug_struct("FAT32_BootSector")
            .field("oem_name", &oem_name)
            .field("bytes_per_sector", &bytes_per_sector)
            .field("sectors_per_cluster", &sectors_per_cluster)
            .field("reserved_sectors", &reserved_sectors)
            .field("fat_count", &self.fat_count)
            .field("root_dir_entries", &root_dir_entries)
            .field("total_sectors", &total_sectors)
            .field("media_descriptor", &media_descriptor)
            .field("sectors_per_fat", &sectors_per_fat)
            .field("sectors_per_track", &sectors_per_track)
            .field("head_count", &head_count)
            .field("hidden_sectors", &hidden_sectors)
            .field("total_sectors_large", &total_sectors_large)
            .field("sectors_per_fat_large", &sectors_per_fat_large)
            .field("flags", &flags)
            .field("version", &version)
            .field("root_dir_start", &root_dir_start)
            .field("fs_info_sector", &fs_info_sector)
            .field("backup_boot_sector", &backup_boot_sector)
            .field("drive_number", &self.drive_number)
            .field("ext_signature", &self.ext_signature)
            .field("serial_number", &serial_number)
            .field("volume_label", &volume_label)
            .field("system_id", &system_id)
            .field("boot_signature", &boot_signature)
            .finish()
    }
}

pub struct Fat32Driver {
    pub device: AhciDevice,
}
