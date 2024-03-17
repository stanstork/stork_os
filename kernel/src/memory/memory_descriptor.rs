use core::fmt::Debug;

/// Representation of a memory descriptor in the context of EFI (Extensible Firmware Interface).
#[repr(C)]
pub struct EFIMemoryDescriptor {
    pub r#type: u32, // The type of memory region, using the EFI memory type enumeration.
    pub pad: u32,    // Padding to maintain structure alignment.
    pub physical_start: u64, // The physical start address of the memory region.
    pub virtual_start: u64, // The virtual start address of the memory region.
    pub number_of_pages: u64, // The number of pages in the memory region.
    pub attribute: u64, // Attributes of the memory region.
}

// Enumeration representing different types of memory in the EFI specification.
#[repr(u32)]
pub enum EfiMemoryType {
    // Other memory types omitted for brevity...
    EfiConventionalMemory = 7, // Represents conventional memory that can be used for any purpose.
                               // Other memory types omitted for brevity...
}

// Implementation of the Debug trait for EFIMemoryDescriptor to enable formatted output.
impl Debug for EFIMemoryDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Custom formatting for the EFIMemoryDescriptor struct to display its fields.
        f.debug_struct("EFIMemoryDescriptor")
            .field("r#type", &self.r#type)
            .field("pad", &self.pad)
            .field("physical_start", &self.physical_start)
            .field("virtual_start", &self.virtual_start)
            .field("number_of_pages", &self.number_of_pages)
            .field("attribute", &self.attribute)
            .finish()
    }
}

impl EFIMemoryDescriptor {
    /// Returns whether the memory region is usable.
    pub fn is_usable(&self) -> bool {
        self.r#type == EfiMemoryType::EfiConventionalMemory as u32
    }
}
