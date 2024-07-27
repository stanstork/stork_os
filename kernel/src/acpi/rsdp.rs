use core::intrinsics::size_of;

use super::sdt::SdtHeader;
use crate::{println, structures::BootInfo};

/// Root System Description Pointer (RSDP) structure.
/// The RSDP is a structure that is used to locate the Root System Description Table (RSDT)
/// or the Extended System Description Table (XSDT).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct Rsdp {
    /// Signature identifying the table as the RSDP. This should be "RSD PTR ".
    pub signature: [u8; 8],
    /// Checksum of the first 20 bytes of the table. All bytes must sum to zero.
    pub checksum: u8,
    /// OEM ID, which is a string that identifies the system's manufacturer.
    pub oem_id: [u8; 6],
    /// Revision of the ACPI. 0 for ACPI 1.0, 2 for ACPI 2.0 and later.
    pub revision: u8,
    /// Physical address of the Root System Description Table (RSDT).
    pub rsdt_address: u32,
    /// Length of the table, including the extended fields, if applicable.
    pub length: u32,
    /// Physical address of the Extended System Description Table (XSDT).
    pub xsdt_address: u64,
    /// Checksum of the entire table, including the extended fields.
    pub extended_checksum: u8,
    /// Reserved bytes, must be zero.
    pub reserved: [u8; 3],
}

pub struct RsdpManager {
    pub rsdp: *const Rsdp,
    pub rsdt: *const SdtHeader,
    pub entry_count: usize,
}

impl RsdpManager {
    pub fn new(rsdp: *const Rsdp) -> RsdpManager {
        let rsdt = unsafe { (*rsdp).get_root_sdt() };
        let entry_count = unsafe { (*rsdp).get_table_count() };

        RsdpManager {
            rsdp,
            rsdt,
            entry_count,
        }
    }

    pub fn get_entry(&self, index: usize) -> Option<&SdtHeader> {
        if index < self.entry_count {
            let entries_base =
                unsafe { (self.rsdt as *const u8).add(size_of::<SdtHeader>()) } as *const u32;
            let entry_address = unsafe { *entries_base.add(index) } as *const SdtHeader;
            unsafe { Some(&*entry_address) }
        } else {
            None
        }
    }
}

impl Rsdp {
    pub fn validate_checksum(&self) -> bool {
        let mut sum = 0u8;

        for byte in unsafe { core::slice::from_raw_parts(self as *const Rsdp as *const u8, 20) } {
            sum = sum.wrapping_add(*byte);
        }

        sum == 0
    }

    fn get_root_sdt(&self) -> *const SdtHeader {
        self.rsdt_address as *const SdtHeader
    }

    fn get_table_count(&self) -> usize {
        unsafe {
            let rsdt = &*(self.get_root_sdt());
            (rsdt.length as usize - size_of::<SdtHeader>()) / size_of::<u32>()
        }
    }
}

pub unsafe fn init_rsdp(boot_info: &'static BootInfo) {
    let rsdp = boot_info.rsdp;

    if !(*rsdp).validate_checksum() {
        println!("Invalid RSDP checksum");
    }

    println!("RSDP Details:");
    println!(
        "  Signature: {:?}",
        core::str::from_utf8_unchecked(&(*rsdp).signature)
    );
    println!("  Checksum: {}", (*rsdp).checksum);
    println!(
        "  OEM ID: {:?}",
        core::str::from_utf8_unchecked(&(*rsdp).oem_id)
    );
    println!("  Revision: {}", (*rsdp).revision);

    let rsdt_address = (*rsdp).rsdt_address;
    println!("  RSDT Address: {:#X}", rsdt_address);

    let length = (*rsdp).length;
    println!("  Length: {}", length);

    let xsdt_address = (*rsdp).xsdt_address;
    println!("  XSDT Address: {:#X}", xsdt_address);
    println!("  Extended Checksum: {}", (*rsdp).extended_checksum);
    println!();

    let table_count = (*rsdp).get_table_count();
    println!("RSDT Table Count: {}", table_count);

    let rsdp_manager = RsdpManager::new(rsdp);
    for i in 0..rsdp_manager.entry_count {
        if let Some(entry) = rsdp_manager.get_entry(i) {
            println!("Entry {}: {:?}", i, entry);
        }
    }
}
