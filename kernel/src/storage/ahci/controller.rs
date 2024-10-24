use super::{
    byte_swap_string, fis::FisRegisterHostToDevice, hba::HbaRegs, print_device_info,
    sata_ident::SataIdentity,
};
use crate::{
    io::sleep_for,
    memory,
    pci::device::pci_device::PciDevice,
    println,
    storage::{
        ahci::{device::AhciDevice, hba::DeviceSignature},
        register_ahci_device,
    },
};

// AHCI_ENABLE is a bitmask used to enable AHCI mode in the controller's global host control register.
pub const AHCI_ENABLE: u32 = 0x80000000; // Bit 31 is typically used to enable AHCI mode.

// AHCI_ENABLE_TIMEOUT is the maximum number of iterations to wait for AHCI mode to be enabled.
pub const AHCI_ENABLE_TIMEOUT: u32 = 100000; // Timeout value for enabling AHCI mode (e.g., for polling loops).

// AHCI_BASE_ADDR_REG is the offset of the Base Address Register (BAR) in the PCI configuration space
// where the base address of the AHCI controller's registers is stored.
pub const AHCI_BASE_ADDR_REG: u8 = 0x24; // Offset for AHCI Base Address Register in PCI config space.

/// Structure representing an AHCI controller.
/// AHCI controllers are used for interfacing with SATA devices (hard drives, SSDs).
pub struct AhciController {
    pub device: PciDevice, // The PCI device structure representing the AHCI controller.
    pub hba: *mut HbaRegs, // A raw pointer to the HBA (Host Bus Adapter) registers in memory.
                           // `HbaRegs` represents the memory-mapped registers used to control the AHCI controller.
}

impl AhciController {
    // Initializes an AHCI controller.
    // This function takes a `PciDevice` representing the AHCI controller, reads the necessary configuration,
    // and prepares the controller for use.
    pub unsafe fn init(device: PciDevice) -> Self {
        // Step 1: Read the AHCI Base Address Register (ABAR) to get the physical memory address
        // where the AHCI controller's registers are mapped.
        let abar = Self::get_base_address(&device);

        // Step 2: Map the ABAR to a virtual address that the kernel can access.
        memory::map_io(abar as u64);

        // Convert the mapped ABAR physical address to a raw pointer to the HBA (Host Bus Adapter) registers.
        let hba_ptr = abar as *mut HbaRegs;

        // Step 3: Print the detected AHCI version for debugging purposes.
        // The AHCI version is stored in the `version` register.
        println!(
            "AHCI {:x}.{:x}.{:x} compatible controller found",
            (((*hba_ptr).version & 0xffff0000) >> 16), // Major version
            ((*hba_ptr).version & 0x0000ff00 >> 8),    // Minor version
            (*hba_ptr).version & 0x000000ff            // Revision
        );

        // Step 4: Enable the AHCI controller.
        // The `enable_ahci` method is called on the HBA registers to enable AHCI mode.
        // If enabling AHCI mode fails, the function panics with an error message.
        if !(*hba_ptr).enable_ahci() {
            panic!("Failed to enable AHCI");
        }

        // Step 5: Determine the number of available ports.
        let port_count = (*hba_ptr).ports_count();

        // Step 6: Initialize each port of the AHCI controller.
        for i in 0..port_count {
            Self::init_port(i, hba_ptr);
        }

        Self {
            device,
            hba: hba_ptr,
        }
    }

    /// Performs a read operation from a SATA device connected to a specific port on the AHCI controller.
    ///
    /// # Safety
    ///
    /// This function is marked as `unsafe` because it involves raw pointer manipulation and direct memory access,
    /// which can lead to undefined behavior if not handled properly.
    ///
    /// # Parameters
    ///
    /// - `port_number`: The port number of the AHCI controller to read from. This specifies which port of the AHCI
    ///   controller is connected to the target SATA device.
    ///
    /// - `sata_ident`: A reference to a `SataIdentity` struct containing information about the SATA device.
    ///   This includes details such as the sector size (in bytes) which is required to calculate the read buffer size.
    ///
    /// - `buffer`: A mutable pointer (`*mut u8`) to the destination buffer where the data read from the SATA device will be copied.
    ///   The caller must ensure that this buffer is valid and large enough to hold the data for `sector_count` sectors.
    ///
    /// - `sector`: The starting sector on the SATA device from which to begin reading. This specifies the logical block
    ///   address (LBA) of the first sector to read.
    ///
    /// - `sector_count`: The number of sectors to read from the SATA device. This defines the total amount of data
    ///   to be read, based on the sector size provided by `sata_ident`.
    pub unsafe fn read(
        &self,
        port_number: usize,
        sata_ident: &SataIdentity,
        buffer: *mut u8,
        sector: u64,
        sector_count: u64,
    ) {
        // Create a FIS (Frame Information Structure) for the READ command.
        let fis = FisRegisterHostToDevice::read_command(sector, sector_count);

        // Calculate the total buffer size required for the read operation.
        let buf_size = (sector_count * sata_ident.sector_bytes as u64) as usize;

        // Allocate a DMA buffer for the read operation.
        let dma_buffer = memory::allocate_dma_buffer(buf_size) as *mut u8;

        // Perform the device I/O operation to read data from the SATA device.
        Self::perform_device_io(self.hba, port_number, fis, buf_size, false, dma_buffer).map(
            |buf_phys_addr| {
                // If the I/O operation succeeds, copy the data from the DMA buffer to the destination buffer.
                let dma_buffer = buf_phys_addr as *mut u8;
                dma_buffer.copy_to(buffer, buf_size);
            },
        );
    }

    /// Performs a write operation to a SATA device connected to a specific port on the AHCI controller.
    ///
    /// # Safety
    ///
    /// This function is marked as `unsafe` because it involves raw pointer manipulation and direct memory access,
    /// which can lead to undefined behavior if not handled properly.
    ///
    /// # Parameters
    ///
    /// - `port_no`: The port number of the AHCI controller to write to. This specifies which port of the AHCI
    ///   controller is connected to the target SATA device.
    ///
    /// - `buffer`: A constant pointer (`*const u8`) to the source buffer containing the data to be written to the SATA device.
    ///   The caller must ensure that this buffer is valid and contains the correct data for `sector_count` sectors.
    ///
    /// - `sector`: The starting sector on the SATA device where the data will be written. This specifies the logical block
    ///   address (LBA) of the first sector to write.
    ///
    /// - `sector_count`: The number of sectors to write to the SATA device. This defines the total amount of data
    ///   to be written, based on a standard sector size (512 bytes).
    pub unsafe fn write(&self, port_no: usize, buffer: *const u8, sector: u64, sector_count: u64) {
        // Create a FIS (Frame Information Structure) for the WRITE command.
        let fis = FisRegisterHostToDevice::write_command(sector, sector_count);

        // Calculate the total buffer size required for the write operation.
        // Assumes a standard sector size of 512 bytes.
        // TODO: parameterize sector size based on SATA device information.
        let buf_size = (sector_count * 512) as usize;

        //Allocate a DMA buffer for the write operation.
        let dma_buffer = memory::allocate_dma_buffer(buf_size) as *mut u8;

        // Copy the data from the source buffer to the DMA buffer.
        dma_buffer.copy_from(buffer, buf_size);

        //  Perform the device I/O operation to write data to the SATA device.
        // The `perform_device_io` function handles the actual write operation, interacting with the hardware.
        // The 'true' flag indicates that this is a write operation.
        Self::perform_device_io(self.hba, port_no, fis, buf_size, true, dma_buffer);
    }

    // Gets the base address of the AHCI controller's registers from the PCI device's configuration space.
    fn get_base_address(device: &PciDevice) -> u32 {
        // Read the 32-bit value from the AHCI Base Address Register (BAR) and mask the lower 2 bits.
        // The mask `0xFFFFFFFC` ensures that the address is properly aligned as required by PCI specifications.
        device.read_dword(AHCI_BASE_ADDR_REG) & 0xFFFFFFFC
    }

    // Initializes a specific port on the AHCI controller.
    // This function configures the port based on the type of device connected to it.
    unsafe fn init_port(port_number: usize, hba_ptr: *mut HbaRegs) {
        // Get a mutable reference to the port using the port number.
        let port = (*hba_ptr).port_mut(port_number);

        // Retrieve the device signature from the port to determine the type of connected device.
        let port_type = port.device_signature();

        match port_type {
            // If the device is an ATA or ATAPI device, perform initialization.
            DeviceSignature::ATA | DeviceSignature::ATAPI => {
                port.rebase(); // Rebase the port to prepare it for use.
                port.clear_errors(); // Clear any existing errors on the port.

                // Identify the device connected to the port.
                let identity = Self::identify_device(hba_ptr, port_number).unwrap();

                // Print the device information for debugging purposes.
                print_device_info(&identity);

                // Create a new AHCI device instance with the identified device information.
                let ahci_device = AhciDevice::new(port_number, DeviceSignature::ATA, identity);

                // Register the AHCI device with a formatted name based on the port number.
                register_ahci_device(ahci_device, alloc::format!("AHCI{}", port_number));
            }
            // If the device type is unknown or unsupported, log a message and stop the port's command engine.
            _ => {
                println!(
                    "AHCI Port {} is an unknown or unsupported device type: {:?}",
                    port_number, port_type
                );
                port.stop_command(); // Stop any ongoing command processing on the port.
            }
        }
    }

    // Identifies the SATA device connected to a specific port on the AHCI controller.
    // This function sends an IDENTIFY DEVICE command to the device and retrieves its identity information.
    unsafe fn identify_device(hba: *mut HbaRegs, port_number: usize) -> Option<SataIdentity> {
        // Create a FIS (Frame Information Structure) for the IDENTIFY DEVICE command.
        let fis = FisRegisterHostToDevice::identify_command();

        // Allocate a DMA buffer of 512 bytes to receive the IDENTIFY DEVICE data from the SATA device.
        let dma_buf = memory::allocate_dma_buffer(512) as *mut u8;
        let buf_size = 512;

        // Perform the device I/O operation to send the IDENTIFY DEVICE command and read the response.
        // The `perform_device_io` function handles the command execution and returns the physical address
        // of the DMA buffer with the data if successful.
        let identity = Self::perform_device_io(hba, port_number, fis, buf_size, false, dma_buf)
            .map(|buf_phys_addr| *(buf_phys_addr as *mut SataIdentity)); // Convert the physical address to a pointer and dereference it to get the SataIdentity structure.

        // If the identity structure is successfully read, swap the byte order of the strings
        // (e.g., serial number, model, firmware revision) to correct endianness and return the structure.
        identity.map(|mut identity| {
            byte_swap_string(&mut identity.serial_no);
            byte_swap_string(&mut identity.model);
            byte_swap_string(&mut identity.fw_rev);
            identity
        })
    }

    // Performs a read or write operation to a SATA device connected to a specific port on the AHCI controller.
    // This function sets up the necessary data structures and issues the command to the controller.
    unsafe fn perform_device_io(
        hba: *mut HbaRegs,            // Pointer to the AHCI controller's HBA registers.
        port_num: usize, // The port number on the AHCI controller to perform the I/O on.
        fis: FisRegisterHostToDevice, // The FIS (Frame Information Structure) representing the command to be sent.
        buf_size: usize,              // The size of the buffer for the I/O operation.
        is_write: bool, // Flag indicating whether the operation is a write (`true`) or read (`false`).
        dma_buf: *mut u8, // Pointer to the DMA buffer to be used for the I/O operation.
    ) -> Option<u64> {
        // Returns an `Option<u64>` with the physical address of the DMA buffer on success (for reads).

        let port = (*hba).port_mut(port_num); // Get a mutable reference to the port using the port number.

        // Find an available command slot in the port's command list.
        if let Some(slot) = port.find_cmd_slot() {
            let cmd_header = port.get_cmd_header(slot); // Get the command header for the allocated command slot.
            let buf_phys_addr = memory::allocate_dma_buffer(buf_size); // Allocate a DMA buffer and get its physical address.

            // Set up the command header, including setting up the FIS and other command details.
            (*cmd_header).setup(buf_phys_addr, 1, fis);

            // Retrieve the command table for the command slot.
            let cmd_tbl = (*cmd_header).get_command_table();
            // Set up the Physical Region Descriptor Table (PRDT) entry for the DMA buffer.
            (*cmd_tbl).setup(buf_phys_addr, buf_size, is_write, dma_buf);

            // Issue the command to the specified port.
            Self::issue_command(port_num, hba, slot);

            // Return the physical address of the DMA buffer if the operation is a read; otherwise, return `None` for writes.
            if !is_write {
                Some(buf_phys_addr)
            } else {
                None
            }
        } else {
            // If no command slot is available, print an error message and return `None`.
            println!(
                "Failed to perform {} on device: No command slots available for port {}.",
                if is_write { "write" } else { "read" },
                port_num
            );
            None
        }
    }

    // Issues a command to a specific port on the AHCI controller.
    // This function sets the command issue bit and waits for the command to complete.
    unsafe fn issue_command(port_no: usize, hba: *mut HbaRegs, slot: usize) {
        let port = (*hba).port_mut(port_no); // Get a mutable reference to the port using the port number.

        port.sata_error = 0xFFFF_FFFF; // Clear any existing SATA errors by setting the SATA error register to all ones.

        port.command_issue = 1 << slot; // Set the command issue register to initiate the command in the specified slot.

        let mut timeout = 100_000; // Set a timeout value to prevent infinite waiting.

        // Wait for the command to complete by polling the command issue register.
        // The loop continues while the command issue bit for the specified slot is set and the timeout has not expired.
        while (port.command_issue & (1 << slot)) != 0 && timeout > 0 {
            sleep_for(10); // Sleep for a short period to avoid busy-waiting.
            timeout -= 10;
        }
    }
}
