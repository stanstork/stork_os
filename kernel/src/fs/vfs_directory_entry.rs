use super::fat::{
    directory_entry::DirectoryEntry, fat_driver::ATTR_DIRECTORY,
    long_directory_entry::LongDirectoryEntry,
};
use alloc::{string::String, vec::Vec};

pub struct VfsDirectoryEntry {
    pub entry: DirectoryEntry,
    pub name: String,
    pub sector: u32,
    pub offset: u32,
}

impl VfsDirectoryEntry {
    pub fn from_entry(
        entry: DirectoryEntry,
        lfn_entries: &mut Vec<LongDirectoryEntry>,
        sector: u32,
        offset: u32,
    ) -> Self {
        VfsDirectoryEntry {
            entry,
            name: Self::parse_name(entry, lfn_entries),
            sector,
            offset,
        }
    }

    pub fn get_cluster(&self) -> u32 {
        self.entry.high_cluster as u32 | self.entry.low_cluster as u32
    }

    pub fn is_dir(&self) -> bool {
        self.entry.attributes & ATTR_DIRECTORY != 0
    }

    fn parse_name(entry: DirectoryEntry, lfn_entries: &mut Vec<LongDirectoryEntry>) -> String {
        if lfn_entries.is_empty() {
            Self::parse_short_filename(entry.name.as_ptr())
        } else {
            let long_name = Self::parse_long_filename(lfn_entries);
            lfn_entries.clear();
            long_name
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
