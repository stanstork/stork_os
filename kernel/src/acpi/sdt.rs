/// System Description Table (SDT) Header structure.
/// This header is a common structure used by various ACPI tables,
/// such as the RSDT, XSDT, FADT, MADT, etc.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct SdtHeader {
    /// Signature identifying the table. Each ACPI table has a unique 4-character signature
    /// (e.g., "RSDT", "XSDT", "FACP") that helps identify the table type.
    pub signature: [u8; 4],
    /// Length of the entire table, including the header and all table-specific data.
    pub length: u32,
    /// Revision number of the table. This indicates the version of the ACPI specification
    /// to which the table conforms.
    pub revision: u8,
    /// Checksum of the entire table. The sum of all bytes in the table, including the header,
    /// must equal zero for the checksum to be valid.
    pub checksum: u8,
    /// OEM ID string that identifies the system's manufacturer. This is a six-character ASCII string.
    pub oem_id: [u8; 6],
    /// OEM Table ID, which is an eight-character string that identifies the particular data table
    /// for the OEM. This is typically used for custom tables created by the OEM.
    pub oem_table_id: [u8; 8],
    /// OEM Revision number, which indicates the version of the OEM table.
    pub oem_revision: u32,
    /// Creator ID, which identifies the utility or vendor that created the table.
    pub creator_id: u32,
    /// Creator Revision number, which indicates the version of the utility or vendor
    /// that created the table.
    pub creator_revision: u32,
}
