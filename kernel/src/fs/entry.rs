#[derive(Clone, Copy)]
pub struct VfsEntry {
    pub name: [u8; 255],
    pub size: u32,
    pub is_dir: bool,
    pub creation_time: VfsEntryCreationTime,
    pub creation_date: VfsEntryCreationDate,
}

#[derive(Clone, Copy)]
pub struct VfsEntryCreationTime {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
}

#[derive(Clone, Copy)]
pub struct VfsEntryCreationDate {
    pub day: u8,
    pub month: u8,
    pub year: u16,
}
