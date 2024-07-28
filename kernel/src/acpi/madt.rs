use super::sdt::SdtHeader;

#[repr(C, packed)]
pub struct Madt {
    pub header: SdtHeader,
    pub local_apic_address: u32,
    pub flags: u32,
    pub entries: [u8; 0],
}

impl Madt {
    pub fn from_address(address: u64) -> &'static Madt {
        unsafe { &*(address as *const Madt) }
    }
}
