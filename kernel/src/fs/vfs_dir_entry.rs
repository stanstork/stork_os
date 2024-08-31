use super::fat::{
    dir_entry::DirectoryEntry, fat_driver::ATTR_DIRECTORY, long_dir_entry::LongDirectoryEntry,
};
use alloc::{string::String, vec::Vec};

// Represents the type of a directory entry (file or directory).
pub enum EntryType {
    Directory,
    File,
}

pub struct VfsDirectoryEntry {
    // The actual directory entry as read from the FAT file system.
    pub entry: DirectoryEntry,
    // The name of the file or directory.
    pub name: String,
    // The sector number where the directory entry is located.
    pub sector: u32,
    // The offset of the directory entry within the sector.
    pub offset: u32,
}

impl VfsDirectoryEntry {
    /// Creates a `VfsDirectoryEntry` from a given `DirectoryEntry` and corresponding long file name (LFN) entries.
    ///
    /// # Arguments
    ///
    /// * `entry` - The directory entry containing metadata and location information.
    /// * `lfn_entries` - A mutable vector of `LongDirectoryEntry` representing the long file name entries.
    /// * `sector` - The sector number where the directory entry is located.
    /// * `offset` - The byte offset within the sector where the directory entry starts.
    ///
    /// The function constructs a `VfsDirectoryEntry` and clears the LFN entries vector after use.
    pub fn from_entry(
        entry: DirectoryEntry,
        lfn_entries: &mut Vec<LongDirectoryEntry>,
        sector: u32,
        offset: u32,
    ) -> Self {
        let name = Self::parse_name(&entry, lfn_entries);
        lfn_entries.clear(); // Clear LFN entries after parsing name

        VfsDirectoryEntry {
            entry,
            name,
            sector,
            offset,
        }
    }

    /// Retrieves the cluster number of the file or directory.
    pub fn get_cluster(&self) -> u32 {
        (self.entry.high_cluster as u32) << 16 | (self.entry.low_cluster as u32)
    }

    /// Checks if the entry represents a directory.
    pub fn is_dir(&self) -> bool {
        self.entry.attributes & ATTR_DIRECTORY != 0
    }

    // Parses the name of the file or directory from the directory entry and LFN entries.
    fn parse_name(entry: &DirectoryEntry, lfn_entries: &mut Vec<LongDirectoryEntry>) -> String {
        if lfn_entries.is_empty() {
            Self::parse_short_filename(entry.name.as_ptr())
        } else {
            Self::parse_long_filename(lfn_entries)
        }
    }

    fn parse_short_filename(filename_ptr: *const u8) -> String {
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

    fn parse_long_filename(long_entries: &Vec<LongDirectoryEntry>) -> String {
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
}
