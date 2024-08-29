use super::{
    fat_driver::{ATTR_DIRECTORY, ENTRY_END, ENTRY_FREE},
    get_current_fat_time_date,
};

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
    /// Creates a new directory entry with the given short name, cluster, and attributes.
    pub fn new(short_name: [u8; 11], cluster: u32, attributes: u8) -> Self {
        let (creation_time, creation_date) = get_current_fat_time_date();

        Self {
            name: short_name,
            attributes,
            reserved: 0,
            creation_time_tenths: 100,
            creation_time,
            creation_date,
            access_date: 0,
            high_cluster: (cluster >> 16) as u16,
            modification_time: 0,
            modification_date: 0,
            low_cluster: (cluster & 0xFFFF) as u16,
            size: 0,
        }
    }

    /// Checks if the directory entry is free.
    pub fn is_free(&self) -> bool {
        self.name[0] == ENTRY_END || self.name[0] == ENTRY_FREE
    }

    /// Updates the metadata (creation time and date) of the directory entry.
    pub fn update_metadata(&mut self) {
        let (creation_time, creation_date) = get_current_fat_time_date();
        self.creation_time = creation_time;
        self.creation_date = creation_date;
    }

    /// Creates a "." entry for a new directory.
    pub fn create_dot_entry(new_dir_cluster: u32) -> Self {
        Self {
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

    /// Creates a ".." entry pointing to the parent directory.
    pub fn create_dotdot_entry(parent_cluster: u32) -> Self {
        let mut entry = Self::create_dot_entry(parent_cluster);
        entry.name[1] = b'.'; // Change from "." to ".."
        entry
    }
}
