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

pub struct StdHeaders {
    pub facp: Option<u64>,
    pub apic: Option<u64>,
    pub hpet: Option<u64>,
    pub mcfg: Option<u64>,
    pub waet: Option<u64>,
    pub bgrt: Option<u64>,
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

pub struct RsdpManager {
    pub rsd_ptr: *const Rsdp,
    pub root_sdt: *const SdtHeader,
    pub entry_count: usize,
    pub sdt_headers: StdHeaders,
}

impl RsdpManager {
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

pub static mut RSDP_MANAGER: RsdpManager = RsdpManager {
    rsd_ptr: core::ptr::null(),
    root_sdt: core::ptr::null(),
    entry_count: 0,
    sdt_headers: StdHeaders::default(),
};

/// Initializes and processes the RSDP (Root System Description Pointer).
///
/// This function validates the checksum of the RSDP, prints its details,
/// and initializes the RSDP manager to handle the ACPI tables. It then
/// retrieves the Fixed ACPI Description Table (FADT) and enables ACPI.
///
/// ACPI Table Locations in BIOS:
///
/// ACPI tables are located in specific regions of memory, defined by the system firmware.
///
/// 1. **RSDP (Root System Description Pointer)**:
///    - **Location**: Typically found in the first 1KB of the Extended BIOS Data Area (EBDA),
///      in the last 128KB of the system's main memory (below 1MB), or within the first 1MB of system memory.
///    - **Purpose**: Contains pointers to the RSDT (Root System Description Table) or XSDT (Extended System Description Table).
///
/// 2. **RSDT/XSDT (Root System Description Table/Extended System Description Table)**:
///    - **Location**: Address provided by the RSDP.
///    - **Purpose**: Contains pointers to other ACPI tables such as the FADT, DSDT, SSDT, and more.
///
/// 3. **Other ACPI Tables (e.g., FADT, DSDT)**:
///    - **Location**: Addresses provided by the RSDT/XSDT.
///    - **Purpose**: Provide detailed information about various system components and power management features.
///
/// Example Schema:
///
/// ```
/// +---------------------------+
/// | Extended BIOS Data Area   |
/// | (EBDA)                    |
/// |                           |
/// | - RSDP (if present)       |
/// +---------------------------+
/// | System Memory (Below 1MB) |
/// |                           |
/// | - RSDP (if present)       |
/// |                           |
/// | Last 128KB of memory      |
/// | below 1MB (e.g., 0xF0000) |
/// | - RSDP (if present)       |
/// +---------------------------+
/// | Memory Above 1MB          |
/// |                           |
/// | - RSDT/XSDT (address from |
/// |   RSDP)                   |
/// | - Other ACPI Tables       |
/// |   (addresses from         |
/// |    RSDT/XSDT)             |
/// +---------------------------+
/// ```
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers from
/// the BootInfo structure.
pub unsafe fn init_rsdp(boot_info: &'static BootInfo) {
    // Get the RSDP from the boot information.
    let rsdp = boot_info.rsdp;

    // Validate the checksum of the RSDP.
    if !(*rsdp).validate_checksum() {
        println!("Invalid RSDP checksum");
    }

    // Print the RSDP details.
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

    // Get the count of ACPI tables from the RSDP.
    let table_count = (*rsdp).get_table_count();
    println!("RSDT Table Count: {}", table_count);

    // Initialize the RSDP manager.
    let mut rsdp_manager = RsdpManager::new(rsdp);

    // Iterate through the entries in the RSDT and add them to the RSDP manager.
    for i in 0..rsdp_manager.entry_count {
        if let Some(entry) = rsdp_manager.get_entry(i) {
            rsdp_manager.add_sdt_header(entry);
        }
    }

    // Store the RSDP manager in a static variable.
    RSDP_MANAGER = rsdp_manager;

    // Get the FADT (Fixed ACPI Description Table) and enable ACPI.
    if let Some(fadt) = RSDP_MANAGER.sdt_headers.facp {
        let fadt = Fadt::from_address(fadt);
        fadt.ensure_acpi_enabled();
        println!("ACPI Enabled");
    }
}
