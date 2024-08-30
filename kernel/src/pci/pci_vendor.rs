pub struct PciVendorTable {
    pub vendor_id: u16,
    pub vendor_name: &'static str,
}

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

pub fn get_vendor_name(vendor_id: u16) -> Option<&'static str> {
    // Use `.find` for a more idiomatic search
    PCI_VENDOR_TABLE
        .iter()
        .find(|&vendor| vendor.vendor_id == vendor_id)
        .map(|vendor| vendor.vendor_name)
}
