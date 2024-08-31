use super::fat_driver::ENTRY_LONG;
use alloc::vec::Vec;

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct LongDirectoryEntry {
    // The order of this entry in the sequence of LFN entries. The highest bit (0x40) is set for the last entry in the sequence.
    pub order: u8,
    // The first part of the long file name, stored as UTF-16 characters (5 characters).
    pub name1: [u16; 5],
    // Attributes field, set to 0x0F for LFN entries.
    pub attributes: u8,
    // Reserved byte, typically set to 0.
    pub reserved1: u8,
    // Checksum of the associated short file name; used to associate LFN entries with their corresponding short name entry.
    pub checksum: u8,
    // The second part of the long file name, stored as UTF-16 characters (6 characters).
    pub name2: [u16; 6],
    // Reserved word, typically set to 0.
    pub reserved2: u16,
    // The third part of the long file name, stored as UTF-16 characters (2 characters).
    pub name3: [u16; 2],
}

impl Default for LongDirectoryEntry {
    // Provides a default, zeroed-out instance of LongDirectoryEntry.
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

/// Creates a sequence of Long File Name (LFN) entries to represent a long file name in a FAT file system.
///
/// # Arguments
///
/// * `name` - The long file name to be stored, as a UTF-8 string.
/// * `checksum` - The checksum of the short file name entry associated with this long file name.
///
/// # Returns
///
/// A vector of `LongDirectoryEntry` structures that represent the long file name in multiple entries,
/// in reverse order to facilitate writing to disk.
///
/// The FAT file system represents long file names using multiple directory entries. Each LFN entry
/// can store up to 13 UTF-16 characters (5 in `name1`, 6 in `name2`, and 2 in `name3`).
/// The entries are stored in reverse order on disk.
pub fn create_lfn_entries(name: &str, checksum: u8) -> Vec<LongDirectoryEntry> {
    let name_len = name.len();
    let long_entries = (name_len / 13) + 1; // Calculate the number of LFN entries needed.

    // Create a buffer for LongDirectoryEntry structures
    let mut entries = Vec::with_capacity(long_entries);

    // Convert the UTF-8 string to a UTF-16 vector.
    let utf16_name: Vec<u16> = name.encode_utf16().collect();
    let mut counter = 0;

    // Loop to create each LFN entry.
    for j in 0..long_entries {
        let mut lfn_entry = LongDirectoryEntry {
            order: (j + 1) as u8, // Order starts from 1
            name1: [0; 5],
            attributes: ENTRY_LONG, // Set attributes to 0x0F to mark as LFN entry
            reserved1: 0,
            checksum, // Checksum of the associated short file name
            name2: [0; 6],
            reserved2: 0,
            name3: [0; 2],
        };

        // If this is the last LFN entry, mark it by setting the highest bit in the order.
        if j == (long_entries - 1) {
            lfn_entry.order |= 0x40; // Mark as the last LFN entry
        }

        // Fill `name1` array with up to 5 UTF-16 characters.
        for i in 0..5 {
            if counter >= utf16_name.len() {
                lfn_entry.name1[i] = 0; // Padding with zeros if the name is shorter.
            } else {
                lfn_entry.name1[i] = utf16_name[counter];
                counter += 1;
            }
        }

        // Fill `name2` array with up to 6 UTF-16 characters.
        for i in 0..6 {
            if counter >= utf16_name.len() {
                lfn_entry.name2[i] = 0; // Padding with zeros if the name is shorter.
            } else {
                lfn_entry.name2[i] = utf16_name[counter];
                counter += 1;
            }
        }

        // Fill `name3` array with up to 2 UTF-16 characters.
        for i in 0..2 {
            if counter >= utf16_name.len() {
                lfn_entry.name3[i] = 0; // Padding with zeros if the name is shorter.
            } else {
                lfn_entry.name3[i] = utf16_name[counter];
                counter += 1;
            }
        }

        // Append the entry to the vector.
        entries.push(lfn_entry);
    }

    // Reverse the order to match the FAT file system's storage order on disk.
    entries.reverse();
    entries
}
