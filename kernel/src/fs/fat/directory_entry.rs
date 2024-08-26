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
