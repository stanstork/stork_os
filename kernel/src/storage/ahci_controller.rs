use core::ptr;

use bitfield_struct::bitfield;

use crate::{
    cpu::io::sleep_for,
    memory::{self},
    pci::device::PciDevice,
    println,
};

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct HbaRegs {
    pub host_capabilities: u32,
    pub global_host_control: u32,
    pub interrupt_status: u32,
    pub ports_implemented: u32,
    pub version: u32,
    pub ccc_control: u32,
    pub ccc_ports: u32,
    pub em_location: u32,
    pub em_control: u32,
    pub ext_capabilities: u32,
    pub bohc: u32,
    pub reserved: [u8; 0xA0 - 0x2C],
    pub vendor_specific: [u8; 0x100 - 0xA0],
    pub ports: [HbaPort; 1],
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct HbaPort {
    pub command_list_base: u32,
    pub command_list_base_upper: u32,
    pub fis_base: u32,
    pub fis_base_upper: u32,
    pub interrupt_status: u32,
    pub interrupt_enable: u32,
    pub command: u32,
    pub reserved0: u32,
    pub task_file_data: u32,
    pub signature: u32,
    pub sata_status: u32,
    pub sata_control: u32,
    pub sata_error: u32,
    pub sata_active: u32,
    pub command_issue: u32,
    pub sata_notification: u32,
    pub fis_switch_control: u32,
    pub device_sleep: u32,
    pub reserved1: [u32; 10],
    pub vendor_specific: [u32; 4],
}

#[bitfield(u32)]
pub struct HbaCommandHeaderDword0 {
    #[bits(5)]
    pub command_fis_length: u8, // Command FIS length in DWORDS, 2 ~ 16
    #[bits(1)]
    pub atapi: u8, // ATAPI
    #[bits(1)]
    pub write: u8, // Write, 1: H2D, 0: D2H
    #[bits(1)]
    pub prefetchable: u8, // Prefetchable
    #[bits(1)]
    pub reset: u8, // Reset
    #[bits(1)]
    pub bist: u8, // BIST
    #[bits(1)]
    pub clear_busy: u8, // Clear busy upon R_OK
    #[bits(1)]
    pub reserved0: u8, // Reserved
    #[bits(4)]
    pub port_multiplier_port: u8, // Port multiplier port
    #[bits(16)]
    pub prdt_length: u16, // Physical region descriptor table length in entries
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct HbaCommandHeader {
    pub dword0: HbaCommandHeaderDword0, // Bitfields for DWORD 0
    pub prdb_count: u32,                // Physical region descriptor byte count transferred
    pub command_table_base: u32,        // Command table descriptor base address
    pub command_table_base_upper: u32,  // Command table descriptor base address upper 32 bits
    pub reserved1: [u32; 4],            // Reserved
}

pub struct AhciController {
    pub device: PciDevice,
    pub hba: HbaRegs,
    pub port_count: u32,
}

pub const AHCI_ENABLE: u32 = 0x80000000;
pub const AHCI_ENABLE_TIMEOUT: u32 = 100000;

impl AhciController {
    pub fn new(device: PciDevice) -> Self {
        Self {
            device,
            hba: HbaRegs {
                host_capabilities: 0,
                global_host_control: 0,
                interrupt_status: 0,
                ports_implemented: 0,
                version: 0,
                ccc_control: 0,
                ccc_ports: 0,
                em_location: 0,
                em_control: 0,
                ext_capabilities: 0,
                bohc: 0,
                reserved: [0; 116],
                vendor_specific: [0; 96],
                ports: [HbaPort {
                    command_list_base: 0,
                    command_list_base_upper: 0,
                    fis_base: 0,
                    fis_base_upper: 0,
                    interrupt_status: 0,
                    interrupt_enable: 0,
                    command: 0,
                    reserved0: 0,
                    task_file_data: 0,
                    signature: 0,
                    sata_status: 0,
                    sata_control: 0,
                    sata_error: 0,
                    sata_active: 0,
                    command_issue: 0,
                    sata_notification: 0,
                    fis_switch_control: 0,
                    device_sleep: 0,
                    reserved1: [0; 10],
                    vendor_specific: [0; 4],
                }; 1],
            },
            port_count: 0,
        }
    }

    pub fn init(&mut self) {
        let mut command = self.device.read_word(0x04);
        command |= 0x02; // Enable IO Space
        command |= 0x04; // Enable Bus Master

        self.device.write_word(0x04, command);

        // Get the AHCI Base Address
        let abar = self.device.read_dword(0x24) & 0xFFFFFFF0;
        println!("AHCI Base Address: {:X}", abar);

        // Map the AHCI Base Address to a virtual address
        memory::map_io(abar as u64);

        // Get the AHCI Controller Registers
        self.hba = unsafe { *(abar as *mut HbaRegs) } as HbaRegs;

        println!(
            "AHCI Version: [{:X}.{:X}.{:X}]",
            self.hba.version >> 16,
            (self.hba.version >> 8) & 0xFF,
            self.hba.version & 0xFF
        );

        if !self.enable() {
            return;
        }

        self.hba.interrupt_status = 0xFFFFFFFF; // Clear pending interrupts

        // Read maximum number of supported ports from lowest 5 bits of capabilities registers
        self.port_count = (self.hba.host_capabilities & 0x1F) + 1;

        println!("AHCI Ports: {}", self.port_count);

        let mut pi = self.hba.ports_implemented;
        println!("AHCI Ports Implemented: {:X}", pi);

        for i in 0..self.port_count {
            let port = &self.hba.ports[i as usize];

            if (port.sata_status & 0x0F) != 3 {
                continue;
            }

            println!("AHCI Port {} is a SATA drive", i);
        }
    }

    fn enable(&mut self) -> bool {
        let mut time = 0;

        println!("Enabling AHCI");
        self.hba.global_host_control |= AHCI_ENABLE;

        while (self.hba.global_host_control & AHCI_ENABLE) == 0 && time < AHCI_ENABLE_TIMEOUT {
            sleep_for(10);
            time += 10;
        }

        if (self.hba.global_host_control & AHCI_ENABLE) == 0 {
            println!("Failed to enable AHCI");
            return false;
        }

        println!("Time to enable AHCI: {}ms", time);
        println!("AHCI enabled");
        true
    }
}
