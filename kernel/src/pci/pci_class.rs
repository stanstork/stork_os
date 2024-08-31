// Structure to store detailed information about a PCI device's class, subclass, and programming interface.
// This struct is used to provide human-readable descriptions of PCI device types.
#[derive(Debug, Copy, Clone)]
pub struct PciClassCodeInfo {
    pub base_class: u8, // Base class code of the PCI device (e.g., 0x01 for Mass Storage Controller).
    pub sub_class: u8, // Subclass code that provides more specific classification (e.g., 0x06 for SATA Controller).
    pub prog_if: u8,   // Programming interface code (e.g., 0x01 for AHCI 1.0).
    pub base_desc: &'static str, // Human-readable description of the base class.
    pub sub_desc: &'static str, // Human-readable description of the subclass.
    pub prog_desc: &'static str, // Human-readable description of the programming interface.
}

// Static array that holds predefined `PciClassCodeInfo` entries for common PCI device types.
// This table is used to map PCI class codes to human-readable descriptions.
pub static PCI_CLASS_CODE_TABLE: [PciClassCodeInfo; 10] = [
    PciClassCodeInfo {
        base_class: 0x01,
        sub_class: 0x06,
        prog_if: 0x01,
        base_desc: "Mass Storage Controller",
        sub_desc: "SATA Controller",
        prog_desc: "AHCI 1.0",
    },
    PciClassCodeInfo {
        base_class: 0x02,
        sub_class: 0x00,
        prog_if: 0x00,
        base_desc: "Network Controller",
        sub_desc: "Ethernet Controller",
        prog_desc: "Ethernet",
    },
    PciClassCodeInfo {
        base_class: 0x03,
        sub_class: 0x00,
        prog_if: 0x00,
        base_desc: "Display Controller",
        sub_desc: "VGA Compatible Controller",
        prog_desc: "VGA",
    },
    PciClassCodeInfo {
        base_class: 0x06,
        sub_class: 0x04,
        prog_if: 0x00,
        base_desc: "Bridge Device",
        sub_desc: "PCI-to-PCI Bridge",
        prog_desc: "Normal Decode",
    },
    PciClassCodeInfo {
        base_class: 0x0C,
        sub_class: 0x03,
        prog_if: 0x00,
        base_desc: "Serial Bus Controller",
        sub_desc: "USB Controller",
        prog_desc: "UHCI",
    },
    PciClassCodeInfo {
        base_class: 0x06,
        sub_class: 0x00,
        prog_if: 0x00,
        base_desc: "Bridge Device",
        sub_desc: "Host Bridge",
        prog_desc: "No specific programming interface",
    },
    PciClassCodeInfo {
        base_class: 0x03,
        sub_class: 0x00,
        prog_if: 0x02,
        base_desc: "Display Controller",
        sub_desc: "VGA Compatible Controller",
        prog_desc: "8514-Compatible",
    },
    PciClassCodeInfo {
        base_class: 0x0C,
        sub_class: 0x03,
        prog_if: 0x03,
        base_desc: "Serial Bus Controller",
        sub_desc: "USB Controller",
        prog_desc: "USB 1.1 (OHCI)",
    },
    PciClassCodeInfo {
        base_class: 0x06,
        sub_class: 0x01,
        prog_if: 0x02,
        base_desc: "Bridge Device",
        sub_desc: "ISA Bridge",
        prog_desc: "PCI-to-ISA Bridge with ISA bus interface",
    },
    PciClassCodeInfo {
        base_class: 0x01,
        sub_class: 0x06,
        prog_if: 0x02,
        base_desc: "Mass Storage Controller",
        sub_desc: "SATA Controller",
        prog_desc: "AHCI 1.0",
    },
];

// Function to retrieve the class code information from the `PCI_CLASS_CODE_TABLE` based on class, subclass, and programming interface.
pub fn get_class_code_info(base_class: u8, sub_class: u8, prog_if: u8) -> Option<PciClassCodeInfo> {
    PCI_CLASS_CODE_TABLE
        .iter()
        .find(|&entry| {
            entry.base_class == base_class
                && entry.sub_class == sub_class
                && entry.prog_if == prog_if
        })
        .copied()
}
