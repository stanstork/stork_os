use crate::{
    cpu::io::sleep_for,
    memory::{self},
    pci::device::PciDevice,
    println,
    storage::ahci_device::SATAIdent,
};
use alloc::vec::Vec;
use bitfield_struct::bitfield;
use core::{mem::size_of, u32};

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

#[repr(u8)]
pub enum FisType {
    REGISTER_HOST_TO_DEVICE = 0x27,
    REGISTER_DEVICE_TO_HOST = 0x34,
    DMA_ACTIVATE = 0x39,
    DMA_SETUP = 0x41,
    DMA_DATA = 0x46,
    BIST_ACTIVATE = 0x58,
    PIO_SETUP = 0x5f,
    DEVICE_BITS = 0xa1,
}

#[repr(u8)]
pub enum Command {
    ATA_IDENTIFY = 0xEC,
    ATA_READ = 0x25,
    ATA_WRITE = 0x35,
}

#[bitfield(u8)]
pub struct FisRegisterHostToDeviceType {
    #[bits(4)]
    pub port_multiplier_port: u8,
    #[bits(3)]
    pub reserved1: u8,
    pub command_control: bool,
}

#[repr(C, packed)]
pub struct FisRegisterHostToDevice {
    pub type_: FisType,
    pub flags: FisRegisterHostToDeviceType,
    pub command: u8,
    pub feature_low: u8,

    pub lba0: u8,
    pub lba1: u8,
    pub lba2: u8,
    pub device: u8,

    pub lba3: u8,
    pub lba4: u8,
    pub lba5: u8,
    pub feature_high: u8,

    pub count_low: u8,
    pub count_high: u8,
    pub isochronous_command_completion: u8,
    pub control: u8,

    pub reserved2: u32,
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

    pub fn get_status(&self) -> u32 {
        self.sata_status
    }

    pub fn is_operable(&self) -> bool {
        self.sata_status & 0x1 == 0
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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HbaCommandHeader {
    pub dword0: HbaCommandHeaderDword0, // Bitfields for DWORD 0
    pub prdb_count: u32,                // Physical region descriptor byte count transferred
    pub command_table_base: u32,        // Command table descriptor base address
    pub command_table_base_upper: u32,  // Command table descriptor base address upper 32 bits
    pub reserved1: [u32; 4],            // Reserved
}

#[repr(C, packed)]
pub struct HbaPhysicalRegionDescriptorTableEntry {
    pub data_base_address: u32,
    pub data_base_address_upper: u32,
    pub reserved1: u32,
    pub data_byte_count_reserved2_interrupt: DataByteCountReserved2Interrupt,
}

#[bitfield(u32)]
pub struct DataByteCountReserved2Interrupt {
    #[bits(22)]
    pub data_byte_count: u32,
    #[bits(9)]
    pub reserved2: u32,
    #[bits(1)]
    pub interrupt_on_completion: u32,
}

#[repr(C, packed)]
pub struct HbaCommandTable {
    pub command_fis: [u8; 64],
    pub atapi_command: [u8; 16],
    pub reserved: [u8; 48],
    pub physical_region_descriptor_table: [HbaPhysicalRegionDescriptorTableEntry; 1],
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
    pub unsafe fn initialize_ahci_controller(device: PciDevice) {
        const AHCI_BASE_ADDR_REG: u8 = 0x24;
        let abar = device.read_dword(AHCI_BASE_ADDR_REG) & 0xFFFFFFFC;
        println!("AHCI Base Address: {:X}", abar);

        memory::map_io(abar as u64);

        let hba_ptr = abar as *mut HbaRegs;
        if hba_ptr.is_null() {
            println!("Invalid HBA register address.");
            return;
        }

        Self::enable(hba_ptr);

        let port_count = ((*hba_ptr).ports_implemented & 0x1F) + 1;
        let port_count = port_count.min((*hba_ptr).ports.len() as u32);

        println!("AHCI Ports: {}", port_count);

        for i in 0..port_count {
            let port = &mut (*hba_ptr).ports[i as usize];
            let port_type = port.get_type();

            if port_type == DeviceSignature::ATA || port_type == DeviceSignature::ATAPI {
                port.stop_cmd();

                let phys_addr = memory::map_io_pages(1);

                let command_list_base_addr = phys_addr as u32;
                port.command_list_base = command_list_base_addr;
                port.command_list_base_upper = 0;

                port.fis_base = command_list_base_addr + size_of::<HbaCommandHeader>() as u32;
                port.fis_base_upper = 0;

                let command_header_ptr = command_list_base_addr as *mut HbaCommandHeader;
                for j in 0..32 {
                    let cmd_hdr = command_header_ptr.add(j);
                    (*cmd_hdr).dword0 = HbaCommandHeaderDword0::new()
                        .with_command_fis_length(
                            (size_of::<FisRegisterHostToDevice>() / size_of::<u32>()) as u8,
                        )
                        .with_atapi(0)
                        .with_write(0)
                        .with_prefetchable(0)
                        .with_reset(0)
                        .with_bist(0)
                        .with_clear_busy(0)
                        .with_port_multiplier_port(0)
                        .with_prdt_length(1);
                    (*cmd_hdr).prdb_count = 1;

                    let base_addr = memory::map_io_pages(1) as u64;
                    (*cmd_hdr).command_table_base = base_addr as u32;
                    (*cmd_hdr).command_table_base_upper = (base_addr >> 32) as u32;
                }

                port.start_cmd();

                let identify_buffer = [0u8; 512];
                let slot = Self::find_cmd_slot(port);

                println!("Slot: {:?}", slot);

                if let Some(slot) = slot {
                    let cmd_header = &mut *((port.command_list_base as u64
                        + (slot as u64 * size_of::<HbaCommandHeader>() as u64))
                        as *mut HbaCommandHeader);

                    let buf_phys_addr = identify_buffer.as_ptr() as u64;
                    cmd_header.command_table_base = buf_phys_addr as u32;
                    cmd_header.command_table_base_upper = (buf_phys_addr >> 32) as u32;
                    cmd_header.dword0.set_command_fis_length(
                        (size_of::<FisRegisterHostToDevice>() / size_of::<u32>()) as u8,
                    );
                    cmd_header.dword0.set_prdt_length(1);

                    let cmd_tbl = &mut *((cmd_header.command_table_base as u64
                        + cmd_header.command_table_base_upper as u64)
                        as *mut HbaCommandTable);

                    cmd_tbl.physical_region_descriptor_table[0] =
                        HbaPhysicalRegionDescriptorTableEntry {
                            data_base_address: buf_phys_addr as u32,
                            data_base_address_upper: (buf_phys_addr >> 32) as u32,
                            reserved1: 0,
                            data_byte_count_reserved2_interrupt:
                                DataByteCountReserved2Interrupt::new()
                                    .with_data_byte_count(512)
                                    .with_reserved2(0)
                                    .with_interrupt_on_completion(0),
                        };

                    let fis = cmd_tbl.command_fis.as_ptr() as *mut FisRegisterHostToDevice;
                    (*fis).type_ = FisType::REGISTER_HOST_TO_DEVICE;
                    (*fis).flags = FisRegisterHostToDeviceType::new()
                        .with_port_multiplier_port(0)
                        .with_reserved1(0)
                        .with_command_control(true);
                    (*fis).command = Command::ATA_IDENTIFY as u8;
                    (*fis).device = 0;
                    (*fis).lba0 = 0;
                    (*fis).lba1 = 0;
                    (*fis).lba2 = 0;
                    (*fis).lba3 = 0;
                    (*fis).lba4 = 0;
                    (*fis).lba5 = 0;
                    (*fis).control = 0;

                    port.command_issue = 1 << slot;

                    let mut timeout = 100_000;
                    while (port.command_issue & (1 << slot)) != 0 && timeout > 0 {
                        sleep_for(10);
                        timeout -= 10;
                    }

                    let identity = identify_buffer.as_ptr() as *const SATAIdent as *mut SATAIdent;

                    Self::byte_swap_string(&mut (*identity).serial_no);
                    Self::byte_swap_string(&mut (*identity).model);
                    Self::byte_swap_string(&mut (*identity).fw_rev);

                    println!(
                        "Serial No: {:?}",
                        core::str::from_utf8(&(*identity).serial_no).unwrap().trim()
                    );
                    println!(
                        "Model: {:?}",
                        core::str::from_utf8(&(*identity).model).unwrap().trim()
                    );
                    println!(
                        "Firmware Revision: {:?}",
                        core::str::from_utf8(&(*identity).fw_rev).unwrap().trim()
                    );

                    let sectors = (*identity).lba_capacity * 512;
                    let size = sectors / 1024 / 1024;
                    println!("Size: {} Mb", size);
                }
            } else {
                println!(
                    "AHCI Port {} is an unknown or unsupported device type: {:?}",
                    i, port_type
                );
                port.stop_cmd();
            }
        }
    }

    fn byte_swap_string(string: &mut [u8]) {
        let length = string.len();
        for i in (0..length).step_by(2) {
            if i + 1 < length {
                string.swap(i, i + 1);
            }
        }
    }

    fn find_cmd_slot(port: &HbaPort) -> Option<u8> {
        let slots = (port.sata_active | port.command_issue) as u8;
        for i in 0..32 {
            if (slots & (1 << i)) == 0 {
                return Some(i);
            }
        }
        None
    }

    unsafe fn enable(hba: *mut HbaRegs) -> bool {
        let mut time = 0;

        println!("Enabling AHCI");
        (*hba).global_host_control |= AHCI_ENABLE;

        while ((*hba).global_host_control & AHCI_ENABLE) == 0 && time < AHCI_ENABLE_TIMEOUT {
            sleep_for(10);
            time += 10;
        }

        if ((*hba).global_host_control & AHCI_ENABLE) == 0 {
            println!("Failed to enable AHCI");
            return false;
        }

        println!("Time to enable AHCI: {}ms", time);
        println!("AHCI enabled");
        true
    }
}
