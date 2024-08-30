use super::fat_driver::ENTRY_LONG;
use alloc::vec::Vec;

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

pub fn create_lfn_entries(name: &str, checksum: u8) -> Vec<LongDirectoryEntry> {
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
