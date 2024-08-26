use crate::{
    cpu::io::sleep_for,
    memory::{self},
    pci::device::{self, PciDevice},
    print, println,
    storage::{
        ahci,
        ahci_device::{AhciDevice, SATAIdent},
        STORAGE_MANAGER,
    },
    sync::mutex::SpinMutex,
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
    ATA_FLUSH_CACHE = 0xE7,
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
    pub hba: *mut HbaRegs,
}

impl Default for FisRegisterHostToDevice {
    fn default() -> Self {
        Self {
            type_: FisType::REGISTER_HOST_TO_DEVICE,
            flags: FisRegisterHostToDeviceType::new(),
            command: 0,
            feature_low: 0,
            lba0: 0,
            lba1: 0,
            lba2: 0,
            device: 0,
            lba3: 0,
            lba4: 0,
            lba5: 0,
            feature_high: 0,
            count_low: 0,
            count_high: 0,
            isochronous_command_completion: 0,
            control: 0,
            reserved2: 0,
        }
    }
}

pub const AHCI_ENABLE: u32 = 0x80000000;
pub const AHCI_ENABLE_TIMEOUT: u32 = 100000;
const AHCI_BASE_ADDR_REG: u8 = 0x24;

impl AhciController {
    pub unsafe fn init(device: PciDevice) -> Self {
        // Read the AHCI Base Address Register (ABAR)
        let abar = device.read_dword(AHCI_BASE_ADDR_REG) & 0xFFFFFFFC;
        // Map the AHCI Base Address Register (ABAR) to a virtual address
        memory::map_io(abar as u64);

        let hba_ptr = abar as *mut HbaRegs;
        println!(
            "AHCI {:x}.{:x}.{:x} compatible controller found",
            (((*hba_ptr).version & 0xffff0000) >> 16),
            ((*hba_ptr).version & 0x0000ff00 >> 8),
            (*hba_ptr).version & 0x000000ff
        );

        // Enable AHCI
        Self::enable(hba_ptr);

        (*hba_ptr).interrupt_status = 0xFFFFFFFF; // Clear pending interrupts

        let port_count = ((*hba_ptr).host_capabilities & 0x1F) + 1;
        let port_count = port_count.min((*hba_ptr).ports.len() as u32);

        println!("AHCI Ports: {}", port_count);

        for i in 0..port_count {
            let port = &mut (*hba_ptr).ports[i as usize];
            let port_type = port.get_type();

            if port_type == DeviceSignature::ATA || port_type == DeviceSignature::ATAPI {
                Self::rebase_port(hba_ptr, i as usize);

                port.sata_error = 0xffffffff; // Clear SATA error register
                port.interrupt_status = 0xffffffff; // Clear interrupt status register
                port.interrupt_enable = 0x00000000; // Disable all port interrupts

                let identity = Self::identify_device(hba_ptr, i as usize).unwrap();

                println!(
                    "Serial No: {:?}",
                    core::str::from_utf8(&identity.serial_no).unwrap().trim()
                );
                println!(
                    "Model: {:?}",
                    core::str::from_utf8(&identity.model).unwrap().trim()
                );
                println!(
                    "Firmware Revision: {:?}",
                    core::str::from_utf8(&identity.fw_rev).unwrap().trim()
                );

                let sector_bytes = identity.sector_bytes as u64;
                println!("Sector Size: {} bytes", sector_bytes);

                let sectors = identity.lba_capacity as u64 * sector_bytes;
                let size = sectors / 1024 / 1024;
                println!("Size: {} Mb", size);

                let ahci_device = AhciDevice::new(i as usize, DeviceSignature::ATA, identity);
                STORAGE_MANAGER.register_ahci_device(ahci_device, alloc::format!("AHCI{}", i));
            } else {
                println!(
                    "AHCI Port {} is an unknown or unsupported device type: {:?}",
                    i, port_type
                );
                port.stop_cmd();
            }
        }

        Self {
            device,
            hba: hba_ptr,
        }
    }

    unsafe fn rebase_port(hba: *mut HbaRegs, port_no: usize) {
        let port = &mut (*hba).ports[port_no];

        // Make sure no commands are running
        port.stop_cmd();

        // Allocate memory for the command list
        port.command_list_base = memory::map_io_pages(1) as u32;
        port.command_list_base_upper = 0;

        // Port may now process commands
        port.start_cmd();
    }

    unsafe fn setup_command(
        hba: *mut HbaRegs,
        port_no: usize,
        command_fis: FisRegisterHostToDevice,
        prdt_len: u16,
        buffer_size: usize,
    ) -> Option<(*mut HbaCommandHeader, u64)> {
        let port = &mut (*hba).ports[port_no];
        if let Some(slot) = Self::find_cmd_slot(port) {
            let cmd_header = &mut *((port.command_list_base as u64
                + (slot as u64 * size_of::<HbaCommandHeader>() as u64))
                as *mut HbaCommandHeader);

            let buf_phys_addr = memory::allocate_dma_buffer(buffer_size);
            cmd_header.command_table_base = buf_phys_addr as u32;
            cmd_header.command_table_base_upper = (buf_phys_addr >> 32) as u32;
            cmd_header.dword0.set_command_fis_length(
                (size_of::<FisRegisterHostToDevice>() / size_of::<u32>()) as u8,
            );
            cmd_header.dword0.set_prdt_length(prdt_len);

            let cmd_tbl = &mut *((cmd_header.command_table_base as u64
                + cmd_header.command_table_base_upper as u64)
                as *mut HbaCommandTable);

            cmd_tbl.physical_region_descriptor_table[0] = HbaPhysicalRegionDescriptorTableEntry {
                data_base_address: buf_phys_addr as u32,
                data_base_address_upper: (buf_phys_addr >> 32) as u32,
                reserved1: 0,
                data_byte_count_reserved2_interrupt: DataByteCountReserved2Interrupt::new()
                    .with_data_byte_count(buffer_size as u32)
                    .with_reserved2(0)
                    .with_interrupt_on_completion(0),
            };

            let fis = cmd_tbl.command_fis.as_ptr() as *mut FisRegisterHostToDevice;
            fis.write_volatile(command_fis);

            Some((cmd_header, buf_phys_addr))
        } else {
            None
        }
    }

    unsafe fn read_from_device(
        hba: *mut HbaRegs,
        port_no: usize,
        command_fis: FisRegisterHostToDevice,
        prdt_len: u16,
        buffer_size: usize,
    ) -> Option<u64> {
        if let Some((_, buf_phys_addr)) =
            Self::setup_command(hba, port_no, command_fis, prdt_len, buffer_size)
        {
            let slot = Self::find_cmd_slot(&mut (*hba).ports[port_no]).unwrap();
            Self::issue_command(port_no, hba, slot);
            Some(buf_phys_addr)
        } else {
            None
        }
    }

    unsafe fn write_to_device(
        hba: *mut HbaRegs,
        port_no: usize,
        buffer_size: usize,
        buffer: *const u8,
        sector: u64,
        sector_count: u64,
    ) {
        let fis = FisRegisterHostToDevice {
            type_: FisType::REGISTER_HOST_TO_DEVICE,
            flags: FisRegisterHostToDeviceType::new()
                .with_port_multiplier_port(0)
                .with_reserved1(0)
                .with_command_control(true),
            command: Command::ATA_WRITE as u8,
            device: 1 << 6, // LBA mode
            feature_low: 1, // DMA mode
            lba0: (sector & 0xFF) as u8,
            lba1: ((sector >> 8) & 0xFF) as u8,
            lba2: ((sector >> 16) & 0xFF) as u8,
            lba3: (sector >> 24) as u8,
            count_low: (sector_count & 0xff) as u8,
            count_high: ((sector_count >> 8) & 0xff) as u8,
            control: 1,
            ..Default::default()
        };

        let dma_size = sector_count * 512;
        let dma_buffer = memory::allocate_dma_buffer(dma_size as usize) as *mut u8;
        dma_buffer.copy_from(buffer, dma_size as usize);

        let port = &mut (*hba).ports[port_no];
        let slot = Self::find_cmd_slot(port).unwrap();

        let cmd_header = &mut *((port.command_list_base as u64
            + (slot as u64 * size_of::<HbaCommandHeader>() as u64))
            as *mut HbaCommandHeader);

        let buf_phys_addr = memory::allocate_dma_buffer(buffer_size);
        cmd_header.command_table_base = buf_phys_addr as u32;
        cmd_header.command_table_base_upper = (buf_phys_addr >> 32) as u32;
        cmd_header.dword0.set_command_fis_length(
            (size_of::<FisRegisterHostToDevice>() / size_of::<u32>()) as u8,
        );

        let cmd_tbl = &mut *((cmd_header.command_table_base as u64
            + cmd_header.command_table_base_upper as u64)
            as *mut HbaCommandTable);

        cmd_tbl.physical_region_descriptor_table[0] = HbaPhysicalRegionDescriptorTableEntry {
            data_base_address: dma_buffer as u32,
            data_base_address_upper: (dma_buffer as u64 >> 32) as u32,
            reserved1: 0,
            data_byte_count_reserved2_interrupt: DataByteCountReserved2Interrupt::new()
                .with_data_byte_count(dma_size as u32)
                .with_reserved2(0)
                .with_interrupt_on_completion(0),
        };

        let cmd_fis = cmd_tbl.command_fis.as_ptr() as *mut FisRegisterHostToDevice;
        cmd_fis.write_volatile(fis);

        port.command_issue = 1 << slot;

        let mut timeout = 100_000;
        while (port.command_issue & (1 << slot)) != 0 && timeout > 0 {
            sleep_for(10);
            timeout -= 10;
        }

        println!("Write completed");
    }

    unsafe fn create_cmd_tbl(buffer_size: usize, buffer: *const u8) -> *mut HbaCommandTable {
        let cmd_tbl_phys_addr = memory::allocate_dma_buffer(size_of::<HbaCommandTable>());
        let cmd_tbl = &mut *((cmd_tbl_phys_addr as u64) as *mut HbaCommandTable);

        let descriptors = buffer_size / 4096 + 1;
        for i in 0..descriptors {
            let data_phys_addr = memory::allocate_dma_buffer(4096);
            let data = &mut *((data_phys_addr as u64) as *mut [u8; 4096]);
            data.copy_from_slice(core::slice::from_raw_parts(buffer.add(i * 4096), 4096));

            cmd_tbl.physical_region_descriptor_table[i] = HbaPhysicalRegionDescriptorTableEntry {
                data_base_address: data_phys_addr as u32,
                data_base_address_upper: (data_phys_addr >> 32) as u32,
                reserved1: 0,
                data_byte_count_reserved2_interrupt: DataByteCountReserved2Interrupt::new()
                    .with_data_byte_count(4096)
                    .with_reserved2(0)
                    .with_interrupt_on_completion(0),
            };
        }

        cmd_tbl
    }

    unsafe fn issue_command(port_no: usize, hba: *mut HbaRegs, slot: u8) {
        let port = &mut (*hba).ports[port_no];
        port.sata_error = 0xFFFF_FFFF;

        port.command_issue = 1 << slot;

        let mut timeout = 100_000;
        while (port.command_issue & (1 << slot)) != 0 && timeout > 0 {
            sleep_for(10);
            timeout -= 10;
        }
    }

    unsafe fn identify_device(hba: *mut HbaRegs, port_no: usize) -> Option<SATAIdent> {
        let command_fis = FisRegisterHostToDevice {
            type_: FisType::REGISTER_HOST_TO_DEVICE,
            flags: FisRegisterHostToDeviceType::new()
                .with_port_multiplier_port(0)
                .with_reserved1(0)
                .with_command_control(true),
            command: Command::ATA_IDENTIFY as u8,
            ..Default::default()
        };

        // Send the command and read the result
        let identity = Self::read_from_device(hba, port_no, command_fis, 1, 512)
            .map(|buf_phys_addr| *(buf_phys_addr as *mut SATAIdent));

        // If the identity structure is successfully read, swap the strings and return
        identity.map(|mut identity| {
            Self::byte_swap_string(&mut identity.serial_no);
            Self::byte_swap_string(&mut identity.model);
            Self::byte_swap_string(&mut identity.fw_rev);
            identity
        })
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

        (*hba).global_host_control |= AHCI_ENABLE;

        while ((*hba).global_host_control & AHCI_ENABLE) == 0 && time < AHCI_ENABLE_TIMEOUT {
            sleep_for(10);
            time += 10;
        }

        if ((*hba).global_host_control & AHCI_ENABLE) == 0 {
            println!("Failed to enable AHCI");
            return false;
        }

        println!("AHCI enabled");
        true
    }

    unsafe fn create_fis_register_command(
        command: Command,
        start_sector: u64,
        sector_count: u64,
    ) -> FisRegisterHostToDevice {
        FisRegisterHostToDevice {
            type_: FisType::REGISTER_HOST_TO_DEVICE,
            flags: FisRegisterHostToDeviceType::new()
                .with_port_multiplier_port(0)
                .with_reserved1(0)
                .with_command_control(true),
            command: command as u8,
            device: 1 << 6, // LBA mode
            feature_low: 1, // DMA mode
            lba0: (start_sector & 0xFF) as u8,
            lba1: ((start_sector >> 8) & 0xFF) as u8,
            lba2: ((start_sector >> 16) & 0xFF) as u8,
            lba3: (start_sector >> 24) as u8,
            count_low: (sector_count & 0xff) as u8,
            count_high: ((sector_count >> 8) & 0xff) as u8,
            control: 1,
            ..Default::default()
        }
    }

    pub unsafe fn read(
        &self,
        port_no: usize,
        sat_ident: &SATAIdent,
        buffer: *mut u8,
        start_sector: u64,
        sector_count: u64,
    ) {
        let command_fis =
            Self::create_fis_register_command(Command::ATA_READ, start_sector, sector_count);
        let dma_buffer = Self::read_from_device(
            self.hba,
            port_no,
            command_fis,
            1,
            (sector_count * 512) as usize,
        );

        if let Some(dma_buffer) = dma_buffer {
            let sector_bytes = sat_ident.sector_bytes as u64;
            let data_size = (sector_count * sector_bytes) as usize;
            buffer.copy_from(dma_buffer as *const u8, data_size);
        }
    }

    pub unsafe fn write(
        &self,
        port_no: usize,
        buffer: *const u8,
        start_sector: u64,
        sector_count: u64,
    ) {
        Self::write_to_device(
            self.hba,
            port_no,
            (sector_count * 512) as usize,
            buffer,
            start_sector,
            sector_count,
        );
    }

    pub unsafe fn flush(&self, port_no: usize) {
        let port = &mut (*self.hba).ports[port_no];
        let command_fis = FisRegisterHostToDevice {
            type_: FisType::REGISTER_HOST_TO_DEVICE,
            flags: FisRegisterHostToDeviceType::new()
                .with_port_multiplier_port(0)
                .with_reserved1(0)
                .with_command_control(true),
            command: Command::ATA_FLUSH_CACHE as u8,
            ..Default::default()
        };

        let slot = Self::find_cmd_slot(port).unwrap();
        let cmd_header = &mut *((port.command_list_base as u64
            + (slot as u64 * size_of::<HbaCommandHeader>() as u64))
            as *mut HbaCommandHeader);

        let buf_phys_addr = memory::allocate_dma_buffer(512);
        cmd_header.command_table_base = buf_phys_addr as u32;
        cmd_header.command_table_base_upper = (buf_phys_addr >> 32) as u32;
        cmd_header.dword0.set_command_fis_length(
            (size_of::<FisRegisterHostToDevice>() / size_of::<u32>()) as u8,
        );

        let cmd_tbl = &mut *((cmd_header.command_table_base as u64
            + cmd_header.command_table_base_upper as u64)
            as *mut HbaCommandTable);

        cmd_tbl.physical_region_descriptor_table[0] = HbaPhysicalRegionDescriptorTableEntry {
            data_base_address: 0,
            data_base_address_upper: 0,
            reserved1: 0,
            data_byte_count_reserved2_interrupt: DataByteCountReserved2Interrupt::new()
                .with_data_byte_count(0)
                .with_reserved2(0)
                .with_interrupt_on_completion(0),
        };

        let cmd_fis = cmd_tbl.command_fis.as_ptr() as *mut FisRegisterHostToDevice;
        cmd_fis.write_volatile(command_fis);

        port.command_issue = 1 << slot;

        let mut timeout = 100_000;
        while (port.command_issue & (1 << slot)) != 0 && timeout > 0 {
            sleep_for(10);
            timeout -= 10;
        }

        println!("Flush completed");
    }
}
