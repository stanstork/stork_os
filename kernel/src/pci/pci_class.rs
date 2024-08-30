#[derive(Debug, Copy, Clone)]
pub struct PciClassCodeInfo {
    pub base_class: u8,
    pub sub_class: u8,
    pub prog_if: u8,
    pub base_desc: &'static str,
    pub sub_desc: &'static str,
    pub prog_desc: &'static str,
}

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

pub fn get_class_code_info(base_class: u8, sub_class: u8, prog_if: u8) -> Option<PciClassCodeInfo> {
    // Find the matching entry in the PCI_CLASS_CODE_TABLE
    PCI_CLASS_CODE_TABLE
        .iter()
        .find(|&entry| {
            entry.base_class == base_class
                && entry.sub_class == sub_class
                && entry.prog_if == prog_if
        })
        .copied() // Convert the reference to an owned value
}
