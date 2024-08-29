use super::{
    fis::FisRegisterHostToDevice,
    hba::{
        DataByteCountReserved2Interrupt, HbaCommandHeader, HbaCommandTable,
        HbaPhysicalRegionDescriptorTableEntry, HbaPort, HbaRegs,
    },
    sata_ident::SataIdentity,
};
use crate::{
    cpu::io::sleep_for,
    memory::{self},
    pci::device::PciDevice,
    println,
    storage::{
        ahci::{
            ahci_device::AhciDevice,
            fis::{Command, FisRegisterHostToDeviceType, FisType},
            hba::DeviceSignature,
        },
        register_ahci_device,
    },
};
use core::{mem::size_of, u32};

pub const AHCI_ENABLE: u32 = 0x80000000;
pub const AHCI_ENABLE_TIMEOUT: u32 = 100000;
pub const AHCI_BASE_ADDR_REG: u8 = 0x24;

pub struct AhciController {
    pub device: PciDevice,
    pub hba: *mut HbaRegs,
}

impl AhciController {
    pub unsafe fn init(device: PciDevice) -> Self {
        // Read the AHCI Base Address Register (ABAR)
        let abar = Self::get_base_address(&device);
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
        if !Self::enable(hba_ptr) {
            panic!("Failed to enable AHCI");
        }

        let port_count = Self::get_port_count(hba_ptr);
        for i in 0..port_count {
            Self::init_port(i, hba_ptr);
        }

        Self {
            device,
            hba: hba_ptr,
        }
    }

    fn get_base_address(device: &PciDevice) -> u32 {
        device.read_dword(AHCI_BASE_ADDR_REG) & 0xFFFFFFFC
    }

    unsafe fn get_port_count(hba_ptr: *mut HbaRegs) -> u32 {
        let max_ports = (*hba_ptr).ports.len() as u32;
        let port_count = ((*hba_ptr).host_capabilities & 0x1F) + 1;
        port_count.min(max_ports)
    }

    unsafe fn init_port(port_number: u32, hba_ptr: *mut HbaRegs) {
        let port = &mut (*hba_ptr).ports[port_number as usize];
        let port_type = port.device_signature();

        match port_type {
            DeviceSignature::ATA | DeviceSignature::ATAPI => {
                Self::rebase_port(hba_ptr, port_number as usize);
                Self::clear_port_errors(port_number, hba_ptr);

                let identity = Self::identify_device(hba_ptr, port_number as usize).unwrap();

                Self::print_device_info(&identity);

                let ahci_device =
                    AhciDevice::new(port_number as usize, DeviceSignature::ATA, identity);
                register_ahci_device(ahci_device, alloc::format!("AHCI{}", port_number));
            }
            _ => {
                println!(
                    "AHCI Port {} is an unknown or unsupported device type: {:?}",
                    port_number, port_type
                );
                port.stop_command();
            }
        }
    }

    unsafe fn clear_port_errors(port_number: u32, hba_ptr: *mut HbaRegs) {
        let port = &mut (*hba_ptr).ports[port_number as usize];

        port.sata_error = 0xffffffff; // Clear SATA error register
        port.interrupt_status = 0xffffffff; // Clear interrupt status register
        port.interrupt_enable = 0x00000000; // Disable all port interrupts
    }

    fn print_device_info(identity: &SataIdentity) {
        println!(
            "Serial No: {:?}",
            core::str::from_utf8(&identity.serial_no)
                .unwrap_or("")
                .trim()
        );
        println!(
            "Model: {:?}",
            core::str::from_utf8(&identity.model).unwrap_or("").trim()
        );
        println!(
            "Firmware Revision: {:?}",
            core::str::from_utf8(&identity.fw_rev).unwrap_or("").trim()
        );
    }

    unsafe fn rebase_port(hba: *mut HbaRegs, port_no: usize) {
        let port = &mut (*hba).ports[port_no];

        // Ensure no commands are running before rebasing
        port.stop_command();

        // Allocate memory for the command list (1 page) and map it to an I/O accessible address
        let command_list_base = memory::map_io_pages(1) as u32;

        if command_list_base == 0 {
            println!("Failed to allocate memory for the command list.");
            return;
        }

        // Set the command list base and upper base address for the port
        port.command_list_base = command_list_base;
        port.command_list_base_upper = 0;

        // Port is ready to process commands
        port.start_command();
    }

    unsafe fn setup_command(
        hba: *mut HbaRegs,
        port_no: usize,
        command_fis: FisRegisterHostToDevice,
        prdt_len: u16,
        buffer_size: usize,
    ) -> Option<(*mut HbaCommandHeader, u64)> {
        let port = &mut (*hba).ports[port_no];
        Self::find_cmd_slot(port)
        .map(|slot| Self::get_command_header(port, slot as usize))
        .and_then(|cmd_header_opt| {
            cmd_header_opt.map(|cmd_header| {
                let buf_phys_addr = memory::allocate_dma_buffer(buffer_size);
                Self::setup_command_header(cmd_header, buf_phys_addr, prdt_len, command_fis);
                Self::setup_command_table(cmd_header, buf_phys_addr, buffer_size);
                (cmd_header, buf_phys_addr)
            })
        })
        .or_else(|| {
            println!(
                "Failed to setup command for port {}: No command slots available or header allocation failed",
                port_no
            );
            None
        })
    }

    unsafe fn get_command_header(port: &mut HbaPort, slot: usize) -> Option<*mut HbaCommandHeader> {
        let cmd_list_base = port.command_list_base as u64;
        let cmd_header_offset = slot as u64 * size_of::<HbaCommandHeader>() as u64;
        let cmd_header_addr = cmd_list_base + cmd_header_offset;

        Some(cmd_header_addr as *mut HbaCommandHeader)
    }

    unsafe fn setup_command_header(
        cmd_header: *mut HbaCommandHeader,
        buf_phys_addr: u64,
        prdt_len: u16,
        command_fis: FisRegisterHostToDevice,
    ) {
        (*cmd_header).command_table_base = buf_phys_addr as u32;
        (*cmd_header).command_table_base_upper = (buf_phys_addr >> 32) as u32;
        (*cmd_header).dword0.set_command_fis_length(
            (size_of::<FisRegisterHostToDevice>() / size_of::<u32>()) as u8,
        );
        (*cmd_header).dword0.set_prdt_length(prdt_len);

        let fis_ptr = (*cmd_header).command_table_base as *mut FisRegisterHostToDevice;
        fis_ptr.write_volatile(command_fis);
    }

    unsafe fn setup_command_table(
        cmd_header: *mut HbaCommandHeader,
        buf_phys_addr: u64,
        buffer_size: usize,
    ) {
        let cmd_tbl = Self::get_command_table(cmd_header);

        (*cmd_tbl).physical_region_descriptor_table[0] = HbaPhysicalRegionDescriptorTableEntry {
            data_base_address: buf_phys_addr as u32,
            data_base_address_upper: (buf_phys_addr >> 32) as u32,
            reserved1: 0,
            data_byte_count_reserved2_interrupt: DataByteCountReserved2Interrupt::new()
                .with_data_byte_count(buffer_size as u32)
                .with_reserved2(0)
                .with_interrupt_on_completion(0),
        };
    }

    unsafe fn get_command_table(cmd_header: *mut HbaCommandHeader) -> *mut HbaCommandTable {
        let table_base = (*cmd_header).command_table_base as u64;
        let table_upper_base = (*cmd_header).command_table_base_upper as u64;
        (table_base + table_upper_base) as *mut HbaCommandTable
    }

    unsafe fn read_from_device(
        hba: *mut HbaRegs,
        port_number: usize,
        command_fis: FisRegisterHostToDevice,
        prdt_len: u16,
        buffer_size: usize,
    ) -> Option<u64> {
        let (_, buf_phys_addr) =
            Self::setup_command(hba, port_number, command_fis, prdt_len, buffer_size)?;

        let port = &mut (*hba).ports[port_number];

        Self::find_cmd_slot(port).map(|slot| {
            Self::issue_command(port_number, hba, slot);

            buf_phys_addr
        })
    }

    unsafe fn write_to_device(
        hba: *mut HbaRegs,
        port_no: usize,
        buffer_size: usize,
        buffer: *const u8,
        sector: u64,
        sector_count: u64,
    ) {
        let fis = Self::create_fis(sector, sector_count, Command::ATA_WRITE);

        let dma_size = sector_count * 512;
        let dma_buffer = memory::allocate_dma_buffer(dma_size as usize) as *mut u8;
        dma_buffer.copy_from(buffer, dma_size as usize);

        let port = &mut (*hba).ports[port_no];
        Self::find_cmd_slot(port)
            .map(|slot| {
                let cmd_header = &mut *((port.command_list_base as u64
                    + (slot as u64 * size_of::<HbaCommandHeader>() as u64))
                    as *mut HbaCommandHeader);

                let buf_phys_addr = memory::allocate_dma_buffer(buffer_size);
                Self::setup_command_header(cmd_header, buf_phys_addr, 1, fis.clone());
                Self::setup_command_table_for_write(cmd_header, fis, dma_buffer, dma_size as u32);

                Self::issue_command(port_no, hba, slot);
            })
            .or_else(|| {
                println!("Failed to write to device: No command slots available");
                None
            });

        println!("Write completed");
    }

    unsafe fn setup_command_table_for_write(
        cmd_header: *mut HbaCommandHeader,
        fis: FisRegisterHostToDevice,
        dma_buffer: *mut u8,
        dma_size: u32,
    ) {
        let cmd_tbl = &mut *(((*cmd_header).command_table_base as u64
            + (*cmd_header).command_table_base_upper as u64)
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
    }

    fn create_fis(sector: u64, sector_count: u64, command: Command) -> FisRegisterHostToDevice {
        FisRegisterHostToDevice {
            type_: FisType::REGISTER_HOST_TO_DEVICE,
            flags: FisRegisterHostToDeviceType::new()
                .with_port_multiplier_port(0)
                .with_reserved1(0)
                .with_command_control(true),
            command: command as u8,
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
        }
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

    unsafe fn identify_device(hba: *mut HbaRegs, port_no: usize) -> Option<SataIdentity> {
        let command_fis = Self::create_fis(0, 0, Command::ATA_IDENTIFY);

        // Send the command and read the result
        let identity = Self::read_from_device(hba, port_no, command_fis, 1, 512)
            .map(|buf_phys_addr| *(buf_phys_addr as *mut SataIdentity));

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
        let mut elapsed_time = 0;

        // Set the AHCI Enable bit in the global host control register
        (*hba).global_host_control |= AHCI_ENABLE;

        // Wait until the AHCI Enable bit is set or until timeout
        while ((*hba).global_host_control & AHCI_ENABLE) == 0 && elapsed_time < AHCI_ENABLE_TIMEOUT
        {
            sleep_for(10); // Sleep for 10 milliseconds
            elapsed_time += 10;
        }

        // Check if AHCI Enable bit is still not set after the timeout
        if ((*hba).global_host_control & AHCI_ENABLE) == 0 {
            println!("Failed to enable AHCI after {} ms", elapsed_time);
            return false;
        }

        // Clear all pending interrupts
        (*hba).interrupt_status = 0xFFFFFFFF;

        println!("AHCI enabled successfully");
        true
    }

    pub unsafe fn read(
        &self,
        port_no: usize,
        sat_ident: &SataIdentity,
        buffer: *mut u8,
        start_sector: u64,
        sector_count: u64,
    ) {
        let command_fis = Self::create_fis(start_sector, sector_count, Command::ATA_READ);
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
}
