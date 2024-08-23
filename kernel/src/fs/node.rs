use alloc::string::String;

use super::fat32::{DirectoryEntry, LongDirectoryEntry};

pub struct VfsEntry {
    pub entry: DirectoryEntry,
    pub name: String,
    pub sector: u32,
    pub offset: u32,
}

impl VfsEntry {
    pub fn get_cluster(&self) -> u32 {
        self.entry.high_cluster as u32 | self.entry.low_cluster as u32
    }
}
