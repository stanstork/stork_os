use super::sdt::SdtHeader;

/// Multiple APIC Description Table (MADT) structure.
/// The MADT is used in ACPI to describe the system's interrupt controllers,
/// including the local APIC, I/O APICs, and other interrupt sources.
#[repr(C, packed)]
pub struct Madt {
    /// The standard ACPI System Description Table (SDT) header.
    pub header: SdtHeader,
    /// Physical address of the Local APIC.
    pub local_apic_address: u32,
    /// Flags indicating system-wide configuration options.
    pub flags: u32,
    /// Variable-length array of APIC entries, such as Local APIC, I/O APIC, etc.
    /// This is a flexible array member used to access the entries within the table.
    pub entries: [u8; 0],
}

/// Represents an entry in the MADT.
/// Each entry describes a specific interrupt controller or source in the system.
#[repr(C, packed)]
pub struct MadtEntry {
    /// Type of the APIC entry. This field indicates the kind of structure,
    /// such as Local APIC, I/O APIC, Interrupt Source Override, etc.
    pub apic_type: ApicType,
    /// Length of the entry, including the type and length fields.
    /// This is used to parse through the variable-length entries in the MADT.
    pub length: u8,
}

/// Enum representing different types of APIC entries in the MADT.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ApicType {
    ProcessorLocalApic = 0x0,       // Local APIC
    IOApic = 0x1,                   // I/O APIC
    InterruptSourceOverride = 0x2,  // Interrupt Source Override
    NonMaskableInterrupts = 0x3,    // Non-Maskable Interrupts (NMI)
    LocalApicAddressOverride = 0x4, // Local APIC Address Override
}

/// Common header for all APIC entries in the MADT.
/// This header is used to identify and parse the various APIC structures in the MADT.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C, packed)]
pub struct ApicHeader {
    pub apic_type: ApicType,
    pub length: u8,
}

impl Madt {
    /// Constructs a reference to a `Madt` structure from a given physical address.
    /// This is typically used to map and access the MADT in memory after its address
    /// has been retrieved from the RSDT/XSDT.
    pub fn from_address(address: u64) -> &'static Madt {
        unsafe { &*(address as *const Madt) }
    }
}
