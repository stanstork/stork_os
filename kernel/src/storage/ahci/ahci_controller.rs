use super::{
    byte_swap_string, fis::FisRegisterHostToDevice, hba::HbaRegs, print_device_info,
    sata_ident::SataIdentity,
};
use crate::{
    cpu::io::sleep_for,
    memory::{self},
    pci::pci_device::PciDevice,
    println,
    storage::{
        ahci::{ahci_device::AhciDevice, hba::DeviceSignature},
        register_ahci_device,
    },
};

pub const AHCI_ENABLE: u32 = 0x80000000;
pub const AHCI_ENABLE_TIMEOUT: u32 = 100000;
pub const AHCI_BASE_ADDR_REG: u8 = 0x24;

pub struct AhciController {
    pub device: PciDevice,
    pub hba: *mut HbaRegs,
}

impl AhciController {
    pub unsafe fn init(device: PciDevice) -> Self {
        // Step 1: Read the AHCI Base Address Register (ABAR)
        let abar = Self::get_base_address(&device);

        // Step 2: Map the ABAR to a virtual address for kernel access
        memory::map_io(abar as u64);

        // Convert the mapped ABAR to a pointer to the HBA registers
        let hba_ptr = abar as *mut HbaRegs;

        // Step 3: Print the detected AHCI version for debugging
        println!(
            "AHCI {:x}.{:x}.{:x} compatible controller found",
            (((*hba_ptr).version & 0xffff0000) >> 16),
            ((*hba_ptr).version & 0x0000ff00 >> 8),
            (*hba_ptr).version & 0x000000ff
        );

        // Step 4: Enable the AHCI controller
        if !(*hba_ptr).enable_ahci() {
            panic!("Failed to enable AHCI");
        }

        // Step 5: Determine the number of available ports
        let port_count = (*hba_ptr).ports_count();

        // Step 6: Initialize each port
        for i in 0..port_count {
            Self::init_port(i, hba_ptr);
        }

        // Return the initialized AHCI controller
        Self {
            device,
            hba: hba_ptr,
        }
    }

    pub unsafe fn read(
        &self,
        port_number: usize,
        sata_ident: &SataIdentity,
        buffer: *mut u8,
        sector: u64,
        sector_count: u64,
    ) {
        let fis = FisRegisterHostToDevice::read_command(sector, sector_count);
        let buf_size = (sector_count * sata_ident.sector_bytes as u64) as usize;
        let dma_buffer = memory::allocate_dma_buffer(buf_size) as *mut u8;

        Self::perform_device_io(self.hba, port_number, fis, buf_size, false, dma_buffer).map(
            |buf_phys_addr| {
                let dma_buffer = buf_phys_addr as *mut u8;
                dma_buffer.copy_to(buffer, buf_size);
            },
        );
    }

    pub unsafe fn write(&self, port_no: usize, buffer: *const u8, sector: u64, sector_count: u64) {
        let fis = FisRegisterHostToDevice::write_command(sector, sector_count);
        let buf_size = (sector_count * 512) as usize;
        let dma_buffer = memory::allocate_dma_buffer(buf_size) as *mut u8;

        dma_buffer.copy_from(buffer, buf_size);
        Self::perform_device_io(self.hba, port_no, fis, buf_size, true, dma_buffer);
    }

    fn get_base_address(device: &PciDevice) -> u32 {
        device.read_dword(AHCI_BASE_ADDR_REG) & 0xFFFFFFFC
    }

    unsafe fn init_port(port_number: usize, hba_ptr: *mut HbaRegs) {
        let port = (*hba_ptr).port_mut(port_number as usize);
        let port_type = port.device_signature();

        match port_type {
            DeviceSignature::ATA | DeviceSignature::ATAPI => {
                port.rebase();
                port.clear_errors();

                let identity = Self::identify_device(hba_ptr, port_number as usize).unwrap();

                print_device_info(&identity);

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

    unsafe fn identify_device(hba: *mut HbaRegs, port_number: usize) -> Option<SataIdentity> {
        let fis = FisRegisterHostToDevice::identify_command();
        let dma_buf = memory::allocate_dma_buffer(512) as *mut u8;
        let buf_size = 512;

        // Send the command and read the result
        let identity = Self::perform_device_io(hba, port_number, fis, buf_size, false, dma_buf)
            .map(|buf_phys_addr| *(buf_phys_addr as *mut SataIdentity));

        // If the identity structure is successfully read, swap the strings and return
        identity.map(|mut identity| {
            byte_swap_string(&mut identity.serial_no);
            byte_swap_string(&mut identity.model);
            byte_swap_string(&mut identity.fw_rev);
            identity
        })
    }

    unsafe fn perform_device_io(
        hba: *mut HbaRegs,
        port_num: usize,
        fis: FisRegisterHostToDevice,
        buf_size: usize,
        is_write: bool,
        dma_buf: *mut u8,
    ) -> Option<u64> {
        let port = (*hba).port_mut(port_num);

        // Find an available command slot
        if let Some(slot) = port.find_cmd_slot() {
            let cmd_header = port.get_cmd_header(slot);
            let buf_phys_addr = memory::allocate_dma_buffer(buf_size);

            // Set up the command header (includes FIS setup)
            (*cmd_header).setup(buf_phys_addr, 1, fis);

            // Retrieve the command table
            let cmd_tbl = (*cmd_header).get_command_table();
            // Set up the Physical Region Descriptor Table entry for the buffer
            (*cmd_tbl).setup(buf_phys_addr, buf_size, is_write, dma_buf);

            // Issue the command to the port
            Self::issue_command(port_num, hba, slot);

            if !is_write {
                Some(buf_phys_addr)
            } else {
                None
            }
        } else {
            println!(
                "Failed to perform {} on device: No command slots available for port {}.",
                if is_write { "write" } else { "read" },
                port_num
            );
            None
        }
    }

    unsafe fn issue_command(port_no: usize, hba: *mut HbaRegs, slot: usize) {
        let port = (*hba).port_mut(port_no);
        port.sata_error = 0xFFFF_FFFF;

        port.command_issue = 1 << slot;

        let mut timeout = 100_000;
        while (port.command_issue & (1 << slot)) != 0 && timeout > 0 {
            sleep_for(10);
            timeout -= 10;
        }
    }
}
