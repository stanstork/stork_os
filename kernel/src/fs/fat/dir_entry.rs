use super::{
    driver::{ATTR_DIRECTORY, ENTRY_END, ENTRY_FREE},
    get_current_fat_time_date,
};

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct DirectoryEntry {
    // Short name in 8.3 format (8 characters for the name and 3 for the extension).
    pub name: [u8; 11],
    // File attributes (e.g., read-only, hidden, system, volume label, subdirectory, archive).
    pub attributes: u8,
    // Reserved byte, typically set to 0.
    pub reserved: u8,
    // Tenths of a second for the creation time; allows for more precise timestamping.
    pub creation_time_tenths: u8,
    // Time of creation (hours, minutes, seconds in FAT format).
    pub creation_time: u16,
    // Date of creation (year, month, day in FAT format).
    pub creation_date: u16,
    // Last access date (year, month, day in FAT format).
    pub access_date: u16,
    // High word of the first cluster number (used for FAT32).
    pub high_cluster: u16,
    // Time of last modification (hours, minutes, seconds in FAT format).
    pub modification_time: u16,
    // Date of last modification (year, month, day in FAT format).
    pub modification_date: u16,
    // Low word of the first cluster number (used for FAT32).
    pub low_cluster: u16,
    // File size in bytes. For directories, this field is typically 0.
    pub size: u32,
}

impl DirectoryEntry {
    /// Creates a new directory entry with the given short name, cluster, and attributes.
    ///
    /// # Arguments
    ///
    /// * `short_name` - An 11-byte array representing the short name in 8.3 format.
    /// * `cluster` - The starting cluster number for the file or directory.
    /// * `attributes` - The attributes of the file or directory (e.g., read-only, directory).
    ///
    /// The function initializes the entry with the current time and date for creation.
    pub fn new(short_name: [u8; 11], cluster: u32, attributes: u8) -> Self {
        let (creation_time, creation_date) = get_current_fat_time_date();

        Self {
            name: short_name,
            attributes,
            reserved: 0,
            creation_time_tenths: 100, // Set to 100 tenths of a second for the creation timestamp.
            creation_time,
            creation_date,
            access_date: 0,                       // Access date not initialized here.
            high_cluster: (cluster >> 16) as u16, // Upper 16 bits of the cluster number.
            modification_time: 0,                 // Modification time not initialized here.
            modification_date: 0,                 // Modification date not initialized here.
            low_cluster: (cluster & 0xFFFF) as u16, // Lower 16 bits of the cluster number.
            size: 0,                              // Size is 0 by default for new entries.
        }
    }

    /// Checks if the directory entry is free.
    ///
    /// A free entry is either marked by a specific value indicating that it is available
    /// (ENTRY_FREE) or that it is the last entry (ENTRY_END).
    pub fn is_free(&self) -> bool {
        self.name[0] == ENTRY_END || self.name[0] == ENTRY_FREE
    }

    /// Updates the metadata (creation time and date) of the directory entry.
    ///
    /// This function updates the entry's creation time and date to the current time and date,
    /// reflecting the time of the last modification.
    pub fn update_metadata(&mut self) {
        let (creation_time, creation_date) = get_current_fat_time_date();
        self.creation_time = creation_time;
        self.creation_date = creation_date;
    }

    /// Creates a "." entry for a new directory.
    ///
    /// The "." entry points to the directory itself, and it is commonly used in filesystem navigation.
    ///
    /// # Arguments
    ///
    /// * `new_dir_cluster` - The cluster number of the new directory.
    ///
    /// This function initializes the entry with the directory attribute set.
    pub fn create_dot_entry(new_dir_cluster: u32) -> Self {
        Self {
            name: [
                b'.', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            ],
            attributes: ATTR_DIRECTORY,
            reserved: 0,
            creation_time_tenths: 0, // Not used for "." entry.
            creation_time: 0,        // Not set for "." entry.
            creation_date: 0,        // Not set for "." entry.
            access_date: 0,          // Not used for "." entry.
            high_cluster: (new_dir_cluster >> 16) as u16, // Upper 16 bits of the cluster number.
            modification_time: 0,    // Not set for "." entry.
            modification_date: 0,    // Not set for "." entry.
            low_cluster: (new_dir_cluster & 0xFFFF) as u16, // Lower 16 bits of the cluster number.
            size: 0,                 // Size is 0 for directories.
        }
    }

    /// Creates a ".." entry pointing to the parent directory.
    ///
    /// The ".." entry points to the parent directory, allowing for navigation upwards in the filesystem hierarchy.
    ///
    /// # Arguments
    ///
    /// * `parent_cluster` - The cluster number of the parent directory.
    ///
    /// This function modifies a "." entry to represent a ".." entry.
    pub fn create_dotdot_entry(parent_cluster: u32) -> Self {
        let mut entry = Self::create_dot_entry(parent_cluster);
        entry.name[1] = b'.'; // Change from "." to ".."
        entry
    }
}
