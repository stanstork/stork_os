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
