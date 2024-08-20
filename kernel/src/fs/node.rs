use alloc::string::String;

use super::fat32::{DirectoryEntry, LongDirectoryEntry};

pub struct VfsEntry {
    pub entry: DirectoryEntry,
    pub name: String,
    pub sector: u32,
    pub offset: u32,
}
