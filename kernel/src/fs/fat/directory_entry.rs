use super::{
    convert_to_fat_date, convert_to_fat_time,
    fat_driver::{ATTR_DIRECTORY, ENTRY_END, ENTRY_FREE},
};
use crate::cpu::rtc::RTC;

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

impl DirectoryEntry {
    pub fn new(short_name: [u8; 11], cluster: u32, attributes: u8) -> Self {
        let (hour, minute, _) = unsafe { RTC.lock().read_time() };
        let (day, month, year) = unsafe { RTC.lock().read_date() };

        Self {
            name: short_name,
            attributes,
            reserved: 0,
            creation_time_tenths: 100,
            creation_time: convert_to_fat_time(hour as u16, minute as u16),
            creation_date: convert_to_fat_date(year as u16, month as u16, day as u16),
            access_date: 0,
            high_cluster: (cluster >> 16) as u16,
            modification_time: 0,
            modification_date: 0,
            low_cluster: (cluster & 0xFFFF) as u16,
            size: 0,
        }
    }

    pub fn is_free(&self) -> bool {
        self.name[0] == ENTRY_END || self.name[0] == ENTRY_FREE
    }

    pub fn update_metadata(&mut self) {
        let (hour, minute, _) = unsafe { RTC.lock().read_time() };
        let (day, month, year) = unsafe { RTC.lock().read_date() };

        self.creation_time = convert_to_fat_time(hour as u16, minute as u16);
        self.creation_date = convert_to_fat_date(year as u16, month as u16, day as u16);
    }

    pub fn create_dot_entry(new_dir_cluster: u32) -> DirectoryEntry {
        DirectoryEntry {
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
        }
    }

    pub fn create_dotdot_entry(parent_cluster: u32) -> DirectoryEntry {
        let mut entry = Self::create_dot_entry(parent_cluster);
        entry.name[1] = b'.'; // Change from "." to ".."
        entry
    }

    pub fn create_short_filename(name: &str) -> [u8; 11] {
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

    pub fn calculate_checksum(short_name: &[u8]) -> u8 {
        let mut checksum = 0u8;
        for &byte in short_name {
            checksum = ((checksum & 1) << 7).wrapping_add((checksum >> 1).wrapping_add(byte));
        }
        checksum
    }
}
