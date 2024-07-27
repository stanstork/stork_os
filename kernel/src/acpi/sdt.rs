use core::fmt;

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct SdtHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

impl fmt::Debug for SdtHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SdtHeader {{ ")?;
        write!(
            f,
            "signature: {:?}, ",
            core::str::from_utf8(&self.signature).unwrap_or("Invalid")
        )?;

        let length = self.length;
        write!(f, "length: {}, ", length)?;
        write!(f, "revision: {}, ", self.revision)?;
        write!(f, "checksum: {}, ", self.checksum)?;
        write!(
            f,
            "oem_id: {:?}, ",
            core::str::from_utf8(&self.oem_id).unwrap_or("Invalid")
        )?;
        write!(
            f,
            "oem_table_id: {:?}, ",
            core::str::from_utf8(&self.oem_table_id).unwrap_or("Invalid")
        )?;

        let oem_revision = self.oem_revision;
        write!(f, "oem_revision: {}, ", oem_revision)?;

        let creator_id = self.creator_id;
        write!(f, "creator_id: {}, ", creator_id)?;

        let creator_revision = self.creator_revision;
        write!(f, "creator_revision: {} ", creator_revision)?;
        write!(f, "}}")
    }
}
