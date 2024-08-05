use core::{ptr, u32};

use alloc::vec::Vec;
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
    pub vendor_specific: [u32; 0],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DeviceSignature {
    NONE = 0x00000000,
    ATA = 0x00000101,
    ATAPI = 0xeb140101,
    ENCLOSURE_POWER_MANAGEMENT_BRIDGE = 0xc33c0101,
    PORT_MULTIPLIER = 0x96690101,
}

const HBA_PxCMD_ST: u32 = 1 << 0; // Start bit
const HBA_PxCMD_FRE: u32 = 1 << 4; // FIS Receive Enable
const HBA_PxCMD_FR: u32 = 1 << 14; // FIS Receive Running
const HBA_PxCMD_CR: u32 = 1 << 15; // Command List Running

impl HbaPort {
    pub fn get_type(&self) -> DeviceSignature {
        match self.signature {
            0x00000101 => DeviceSignature::ATA,
            0xeb140101 => DeviceSignature::ATAPI,
            0xc33c0101 => DeviceSignature::ENCLOSURE_POWER_MANAGEMENT_BRIDGE,
            0x96690101 => DeviceSignature::PORT_MULTIPLIER,
            _ => DeviceSignature::NONE,
        }
    }

    pub fn stop_cmd(&mut self) {
        // Clear ST (bit 0)
        self.command &= !HBA_PxCMD_ST;

        // Clear FRE (bit 4)
        self.command &= !HBA_PxCMD_FRE;

        // Wait until FR (bit 14) and CR (bit 15) are cleared
        while {
            let cmd = self.command;
            (cmd & HBA_PxCMD_FR != 0) || (cmd & HBA_PxCMD_CR != 0)
        } {
            sleep_for(10); // Busy-wait or sleep for 10 milliseconds
        }
    }

    pub fn start_cmd(&mut self) {
        // Wait until CR (bit15) is cleared
        while self.command & HBA_PxCMD_CR != 0 {
            sleep_for(10); // Busy-wait or sleep for 10 milliseconds
        }

        // Set FRE (bit4) and ST (bit0)c
        self.command |= HBA_PxCMD_FRE;
        self.command |= HBA_PxCMD_ST;
    }
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
    pub command_lists: Vec<HbaCommandHeader>,
}

pub const AHCI_ENABLE: u32 = 0x80000000;
pub const AHCI_ENABLE_TIMEOUT: u32 = 100000;

impl AhciController {
    pub fn init(device: PciDevice) {
        let mut command = device.read_word(0x04);
        command |= 0x02; // Enable IO Space
        command |= 0x04; // Enable Bus Master

        device.write_word(0x04, command);

        // Get the AHCI Base Address
        let abar = device.read_dword(0x24) & 0xFFFFFFF0;
        println!("AHCI Base Address: {:X}", abar);

        // Map the AHCI Base Address to a virtual address
        memory::map_io(abar as u64);

        // Get the AHCI Controller Registers
        let mut hba = unsafe { *(abar as *mut HbaRegs) } as HbaRegs;

        println!(
            "AHCI Version: [{:X}.{:X}.{:X}]",
            hba.version >> 16,
            (hba.version >> 8) & 0xFF,
            hba.version & 0xFF
        );

        if !Self::enable(&mut hba) {
            return;
        }

        hba.interrupt_status = 0xFFFFFFFF; // Clear pending interrupts

        // Read maximum number of supported ports from lowest 5 bits of capabilities registers
        let port_count = (hba.host_capabilities & 0x1F) + 1;

        println!("AHCI Ports: {}", port_count);

        let pi = hba.ports_implemented;
        println!("AHCI Ports Implemented: {:X}", pi);

        let max_ports = hba.ports.len() as u32;

        let mut commands_list: Vec<*mut HbaCommandHeader> =
            alloc::vec![ptr::null_mut(); max_ports as usize];

        for i in 0..max_ports {
            if (pi & (1 << i)) != 0 {
                let port = &mut hba.ports[i as usize];
                let port_type = port.get_type();

                if port_type == DeviceSignature::ATA || port_type == DeviceSignature::ATAPI {
                    Self::rebase_port(port, i as usize, &mut commands_list);

                    port.sata_error = 0xFFFFFFFF; // Clear SATA error register
                    port.interrupt_status = 0xFFFFFFFF; // Clear pending interrupts
                    port.interrupt_enable = 0; // Disable all port interrupts

                    if port_type == DeviceSignature::ATA {
                        println!("AHCI Port {} is an ATA device", i);
                    } else if port_type == DeviceSignature::ATAPI {
                        println!("AHCI Port {} is an ATAPI device", i);
                    }
                } else {
                    match port_type {
                        DeviceSignature::ENCLOSURE_POWER_MANAGEMENT_BRIDGE => {
                            println!("AHCI Port {} is an Enclosure Management device", i);
                        }
                        DeviceSignature::PORT_MULTIPLIER => {
                            println!("AHCI Port {} is a Port Multiplier device", i);
                        }
                        _ => {
                            println!("AHCI Port {} is an unknown device", i);
                        }
                    }
                    port.stop_cmd();
                }
            }
        }
    }

    fn rebase_port(
        port: &mut HbaPort,
        port_num: usize,
        commands_list: &mut Vec<*mut HbaCommandHeader>,
    ) {
        println!("Rebasing port");
        port.stop_cmd();

        let phys_addr = memory::map_io_pages(1);
        commands_list[port_num] = phys_addr as *mut HbaCommandHeader;

        port.command_list_base = phys_addr as u32;

        port.start_cmd();
    }

    fn enable(hba: &mut HbaRegs) -> bool {
        let mut time = 0;

        println!("Enabling AHCI");
        hba.global_host_control |= AHCI_ENABLE;

        while (hba.global_host_control & AHCI_ENABLE) == 0 && time < AHCI_ENABLE_TIMEOUT {
            sleep_for(10);
            time += 10;
        }

        if (hba.global_host_control & AHCI_ENABLE) == 0 {
            println!("Failed to enable AHCI");
            return false;
        }

        println!("Time to enable AHCI: {}ms", time);
        println!("AHCI enabled");
        true
    }
}
