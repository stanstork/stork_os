use super::sdt::SdtHeader;
use crate::{acpi::fadt::Fadt, println, structures::BootInfo};
use core::intrinsics::size_of;

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

/// Struct to hold pointers to various System Description Tables.
pub struct StdHeaders {
    pub facp: Option<u64>,
    pub apic: Option<u64>,
    pub hpet: Option<u64>,
    pub mcfg: Option<u64>,
    pub waet: Option<u64>,
    pub bgrt: Option<u64>,
}

/// Manager for handling RSDP and related System Description Tables.
pub struct RsdpManager {
    pub rsd_ptr: *const Rsdp,       // Pointer to the RSDP.
    pub root_sdt: *const SdtHeader, // Pointer to the Root System Description Table (RSDT or XSDT).
    pub entry_count: usize,         // Number of entries in the RSDT or XSDT.
    pub sdt_headers: StdHeaders,    // Pointers to various System Description Tables.
}

impl StdHeaders {
    pub const fn default() -> StdHeaders {
        StdHeaders {
            facp: None,
            apic: None,
            hpet: None,
            mcfg: None,
            waet: None,
            bgrt: None,
        }
    }
}

impl RsdpManager {
    /// Creates a new RsdpManager with the given RSDP pointer.
    pub fn new(rsdp: *const Rsdp) -> RsdpManager {
        let rsdt = unsafe { (*rsdp).get_root_sdt() };
        let entry_count = unsafe { (*rsdp).get_table_count() };

        RsdpManager {
            rsd_ptr: rsdp,
            root_sdt: rsdt,
            entry_count,
            sdt_headers: StdHeaders::default(),
        }
    }

    /// Retrieves an entry from the RSDT by index.
    pub fn get_entry(&self, index: usize) -> Option<*const SdtHeader> {
        if index < self.entry_count {
            let entries_base =
                unsafe { (self.root_sdt as *const u8).add(size_of::<SdtHeader>()) } as *const u32;
            let entry_address = unsafe { *entries_base.add(index) } as *const SdtHeader;
            unsafe { Some(&*entry_address) }
        } else {
            None
        }
    }

    /// Adds a System Description Table header to the manager based on its signature.
    pub fn add_sdt_header(&mut self, header: *const SdtHeader) {
        let signature = core::str::from_utf8(unsafe { &(*header).signature }).unwrap();
        match signature {
            "FACP" => self.sdt_headers.facp = Some(header as u64),
            "APIC" => self.sdt_headers.apic = Some(header as u64),
            "HPET" => self.sdt_headers.hpet = Some(header as u64),
            "MCFG" => self.sdt_headers.mcfg = Some(header as u64),
            "WAET" => self.sdt_headers.waet = Some(header as u64),
            "BGRT" => self.sdt_headers.bgrt = Some(header as u64),
            _ => {}
        }
    }
}

impl Rsdp {
    /// Validates the checksum of the RSDP.
    pub fn validate_checksum(&self) -> bool {
        let mut sum = 0u8;

        // Sum the first 20 bytes to validate the checksum.
        for byte in unsafe { core::slice::from_raw_parts(self as *const Rsdp as *const u8, 20) } {
            sum = sum.wrapping_add(*byte);
        }

        // The sum should be zero if the checksum is valid.
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

/// Global RSDP manager instance.
pub static mut RSDP_MANAGER: RsdpManager = RsdpManager {
    rsd_ptr: core::ptr::null(),
    root_sdt: core::ptr::null(),
    entry_count: 0,
    sdt_headers: StdHeaders::default(),
};

/// Initializes and processes the RSDP (Root System Description Pointer).
pub unsafe fn init_rsdp(boot_info: &'static BootInfo) {
    // Get the RSDP from the boot information.
    let rsdp = boot_info.rsdp;

    // Validate the checksum of the RSDP.
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

    // Initialize the RSDP manager.
    let mut rsdp_manager = RsdpManager::new(rsdp);

    // Iterate through the entries in the RSDT and add them to the RSDP manager.
    for i in 0..rsdp_manager.entry_count {
        if let Some(entry) = rsdp_manager.get_entry(i) {
            rsdp_manager.add_sdt_header(entry);
        }
    }

    // Ensure the ACPI is enabled if the FADT is present.
    if let Some(fadt) = rsdp_manager.sdt_headers.facp {
        let fadt = Fadt::from_address(fadt); // Create an FADT object from the address.
        fadt.ensure_acpi_enabled();
        println!("ACPI Enabled");
    }

    // Store the RSDP manager in the global static variable.
    RSDP_MANAGER = rsdp_manager;
}
