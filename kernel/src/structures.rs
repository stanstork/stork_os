/// A structure representing a descriptor table pointer (GDTR, IDTR, LDTR, or TR)
/// Format is suitable for direct loading into the corresponding x86 control register.
#[repr(C, packed)]
pub struct DescriptorTablePointer {
    /// Size of the table.
    pub limit: u16,
    /// Pointer to the memory region containing the DT.
    pub base: u64,
}
