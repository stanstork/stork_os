// Structure to store information about a PCI vendor.
// This struct contains the vendor ID and the human-readable name of the vendor.
pub struct PciVendorTable {
    pub vendor_id: u16, // The unique identifier for the vendor, as defined by the PCI SIG.
    pub vendor_name: &'static str, // A static string representing the vendor's name.
}

// Static array that holds predefined `PciVendorTable` entries for known PCI vendors.
// This table is used to map vendor IDs to human-readable names.
pub static PCI_VENDOR_TABLE: [PciVendorTable; 3] = [
    PciVendorTable {
        vendor_id: 0x1234,
        vendor_name: "QEMU Virtual Device",
    },
    PciVendorTable {
        vendor_id: 0x8086,
        vendor_name: "Intel Corporation",
    },
    PciVendorTable {
        vendor_id: 0x10EC,
        vendor_name: "Realtek Semiconductor",
    },
];

// Function to retrieve the vendor name from the `PCI_VENDOR_TABLE` based on the vendor ID.
pub fn get_vendor_name(vendor_id: u16) -> Option<&'static str> {
    PCI_VENDOR_TABLE
        .iter()
        .find(|&vendor| vendor.vendor_id == vendor_id)
        .map(|vendor| vendor.vendor_name)
}
