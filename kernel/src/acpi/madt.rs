use core::str;

use super::sdt::SdtHeader;

#[repr(C, packed)]
pub struct Madt {
    pub header: SdtHeader,
    pub local_apic_address: u32,
    pub flags: u32,
    pub entries: [u8; 0],
}

#[repr(C, packed)]
pub struct MadtEntry {
    pub apic_type: u8,
    pub length: u8,
}

#[repr(C, packed)]
pub struct InterruptSourceOverride {
    pub header: SdtHeader,
    pub bus: u8,
    pub source: u8,
    pub global_system_interrupt: u32,
    pub flags: u16,
}

impl Madt {
    pub fn from_address(address: u64) -> &'static Madt {
        unsafe { &*(address as *const Madt) }
    }
}
