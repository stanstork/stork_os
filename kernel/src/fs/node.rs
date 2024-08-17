use alloc::string::String;

use super::fat32::{DirectoryEntry, LongDirectoryEntry};

pub enum NodeEntry {
    Short(DirectoryEntry),
    Long(LongDirectoryEntry),
}

pub struct Node {
    pub entry: DirectoryEntry,
    pub name: String,
    pub sector: u32,
    pub offset: u32,
}
