use crate::{
    io::sleep_for,
    memory, println,
    storage::ahci::controller::{AHCI_ENABLE, AHCI_ENABLE_TIMEOUT},
};

use super::fis::FisRegisterHostToDevice;
use bitfield_struct::bitfield;
use core::mem::size_of;

/// Enum representing various device signatures used to identify the type of device
/// connected to a SATA port in AHCI mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DeviceSignature {
    NONE = 0x00000000,
    ATA = 0x00000101,
    ATAPI = 0xeb140101,
    ENCLOSURE_POWER_MANAGEMENT_BRIDGE = 0xc33c0101,
    PORT_MULTIPLIER = 0x96690101,
}

// AHCI command and status constants for controlling and querying the AHCI controller's state.
const CMD_START_BIT: u32 = 0x0001; // Bit 0 represents the start bit
const CMD_FIS_RECEIVE_ENABLE_BIT: u32 = 0x0010; // Bit 4 represents FIS receive enable
const CMD_FIS_RECEIVE_RUNNING_BIT: u32 = 0x4000; // Bit 14 represents FIS receive running
const CMD_LIST_RUNNING_BIT: u32 = 0x8000; // Bit 15 represents command list running

/// Represents an AHCI Host Bus Adapter (HBA) port. Contains all necessary registers
/// to control and monitor the state and data flow of a single SATA port.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct HbaPort {
    pub command_list_base: u32,       // Base address of the command list
    pub command_list_base_upper: u32, // Upper 32 bits of the command list base address
    pub fis_base: u32,                // Base address of the FIS (Frame Information Structure) area
    pub fis_base_upper: u32,          // Upper 32 bits of the FIS base address
    pub interrupt_status: u32,        // Interrupt status register
    pub interrupt_enable: u32,        // Interrupt enable register
    pub command: u32,                 // Command and status register
    pub reserved0: u32,               // Reserved
    pub task_file_data: u32,          // Task file data register
    pub signature: u32,               // Signature to identify the type of connected device
    pub sata_status: u32,             // SATA status register
    pub sata_control: u32,            // SATA control register
    pub sata_error: u32,              // SATA error register
    pub sata_active: u32,             // SATA active register
    pub command_issue: u32,           // Command issue register
    pub sata_notification: u32,       // SATA notification register
    pub fis_switch_control: u32,      // FIS-based switching control register
    pub device_sleep: u32,            // Device sleep register
    pub reserved1: [u32; 10],         // Reserved space
    pub vendor_specific: [u32; 0],    // Vendor-specific registers
}

/// Represents the registers of an AHCI Host Bus Adapter (HBA). Contains global
/// control and status registers, as well as an array of port structures.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct HbaRegs {
    pub host_capabilities: u32,              // HBA capabilities
    pub global_host_control: u32,            // Global HBA control
    pub interrupt_status: u32,               // Interrupt status register
    pub ports_implemented: u32,              // Bit mask of implemented ports
    pub version: u32,                        // AHCI version
    pub ccc_control: u32,                    // Command completion coalescing control
    pub ccc_ports: u32,                      // Command completion coalescing ports
    pub em_location: u32,                    // Enclosure management location
    pub em_control: u32,                     // Enclosure management control
    pub ext_capabilities: u32,               // Extended capabilities
    pub bohc: u32,                           // BIOS/OS handoff control and status
    pub reserved: [u8; 0xA0 - 0x2C],         // Reserved space
    pub vendor_specific: [u8; 0x100 - 0xA0], // Vendor-specific registers
    pub ports: [HbaPort; 1],                 // Array of ports
}

/// Represents a command header for an AHCI command. Contains the details needed
/// to issue a command to a SATA device.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HbaCommandHeader {
    pub dword0: HbaCommandHeaderDword0, // Bitfields for DWORD 0
    pub prdb_count: u32,                // Physical region descriptor byte count transferred
    pub command_table_base: u32,        // Command table descriptor base address
    pub command_table_base_upper: u32,  // Command table descriptor base address upper 32 bits
    pub reserved1: [u32; 4],            // Reserved
}

/// Represents an entry in the Physical Region Descriptor Table (PRDT) for data transfer in AHCI.
#[repr(C, packed)]
pub struct HbaPhysicalRegionDescriptorTableEntry {
    pub data_base_address: u32,       // Base address of data
    pub data_base_address_upper: u32, // Upper 32 bits of the base address
    pub reserved1: u32,               // Reserved
    pub data_byte_count_reserved2_interrupt: DataByteCountReserved2Interrupt, // Data byte count and flags
}

/// Represents the command table for an AHCI command. Contains the command FIS, ATAPI command, and
/// Physical Region Descriptor Table (PRDT) entries for data transfer.
#[repr(C, packed)]
pub struct HbaCommandTable {
    pub command_fis: [u8; 64],   // Command FIS
    pub atapi_command: [u8; 16], // ATAPI command
    pub reserved: [u8; 48],      // Reserved
    pub physical_region_descriptor_table: [HbaPhysicalRegionDescriptorTableEntry; 1], // PRDT entries
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

#[bitfield(u32)]
pub struct DataByteCountReserved2Interrupt {
    #[bits(22)]
    pub data_byte_count: u32, // Data byte count
    #[bits(9)]
    pub reserved2: u32, // Reserved
    #[bits(1)]
    pub interrupt_on_completion: u32, // Interrupt on completion
}
impl HbaPort {
    /// Returns the device signature type for the HBA port.
    ///
    /// # Returns
    ///
    /// The `DeviceSignature` enum variant representing the type of device connected to the port.
    pub fn device_signature(&self) -> DeviceSignature {
        match self.signature {
            0x00000101 => DeviceSignature::ATA,
            0xeb140101 => DeviceSignature::ATAPI,
            0xc33c0101 => DeviceSignature::ENCLOSURE_POWER_MANAGEMENT_BRIDGE,
            0x96690101 => DeviceSignature::PORT_MULTIPLIER,
            _ => DeviceSignature::NONE,
        }
    }

    /// Stops the command engine for this HBA port.
    ///
    /// Clears the ST (Start) and FRE (FIS Receive Enable) bits,
    /// and waits for FR (FIS Receive Running) and CR (Command List Running) bits to clear.
    ///
    /// # Note
    ///
    /// This method is blocking and may take some time if the command engine is busy.
    pub fn stop_command(&mut self) {
        // Clear ST (bit 0) to stop the port
        self.command &= !CMD_START_BIT;

        // Clear FRE (bit 4) to disable FIS receive
        self.command &= !CMD_FIS_RECEIVE_ENABLE_BIT;

        // Wait until both FR (bit 14) and CR (bit 15) are cleared
        let mut retry_count = 100; // Maximum retries to avoid infinite loop
        while {
            let cmd = self.command;
            (cmd & CMD_FIS_RECEIVE_RUNNING_BIT != 0) || (cmd & CMD_LIST_RUNNING_BIT != 0)
        } {
            if retry_count == 0 {
                println!("Warning: Timeout while stopping command engine.");
                break;
            }
            sleep_for(10);
            retry_count -= 1;
        }
    }

    /// Starts the command engine for this HBA port.
    ///
    /// Waits for the CR (Command List Running) bit to clear, then sets FRE and ST bits to start the port.
    ///
    /// # Note
    ///
    /// This method is blocking and may take some time if the command engine is busy.
    pub fn start_command(&mut self) {
        // Wait until CR (bit 15) is cleared
        let mut retry_count = 100; // Maximum retries to avoid infinite loop
        while self.command & CMD_LIST_RUNNING_BIT != 0 {
            if retry_count == 0 {
                println!("Warning: Timeout while starting command engine.");
                break;
            }
            sleep_for(10);
            retry_count -= 1;
        }

        // Set FRE (bit 4) and ST (bit 0) to start the port
        self.command |= CMD_FIS_RECEIVE_ENABLE_BIT;
        self.command |= CMD_START_BIT;
    }

    /// Clears any existing errors and disables interrupts for this HBA port.
    ///
    /// # Safety
    ///
    /// This method is unsafe because it directly manipulates hardware registers.
    pub unsafe fn clear_errors(&mut self) {
        self.sata_error = 0xffffffff; // Clear SATA error register
        self.interrupt_status = 0xffffffff; // Clear interrupt status register
        self.interrupt_enable = 0x00000000; // Disable all port interrupts
    }

    /// Reinitializes the port by stopping any active commands, allocating a new command list,
    /// and restarting the command engine.
    pub fn rebase(&mut self) {
        // Ensure no commands are running before rebasing
        self.stop_command();

        // Allocate memory for the command list (1 page) and map it to an I/O accessible address
        let command_list_base = memory::map_io_pages(1) as u32;

        if command_list_base == 0 {
            println!("Failed to allocate memory for the command list.");
            return;
        }

        // Set the command list base and upper base address for the port
        self.command_list_base = command_list_base;
        self.command_list_base_upper = 0;

        // Port is ready to process commands
        self.start_command();
    }

    /// Finds an available command slot on the port.
    ///
    /// # Returns
    ///
    /// An `Option<usize>` containing the index of an available command slot, or `None` if no slots are available.
    pub fn find_cmd_slot(&self) -> Option<usize> {
        let slots = (self.sata_active | self.command_issue) as u8;
        for i in 0..32 {
            if (slots & (1 << i)) == 0 {
                return Some(i);
            }
        }
        None
    }

    /// Gets a mutable reference to the command header for a specific command slot.
    ///
    /// # Parameters
    ///
    /// - `slot`: The index of the command slot for which to retrieve the command header.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `HbaCommandHeader` for the specified command slot.
    pub fn get_cmd_header(&self, slot: usize) -> &mut HbaCommandHeader {
        unsafe {
            &mut *((self.command_list_base as u64
                + (slot as u64 * size_of::<HbaCommandHeader>() as u64))
                as *mut HbaCommandHeader)
        }
    }
}

impl HbaRegs {
    /// Returns a reference to the HBA port at the specified index.
    ///
    /// # Parameters
    ///
    /// - `index`: The index of the port to retrieve.
    ///
    /// # Returns
    ///
    /// A reference to the `HbaPort` at the specified index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn port(&self, index: usize) -> &HbaPort {
        &self.ports[index]
    }

    /// Returns a mutable reference to the HBA port at the specified index.
    ///
    /// # Parameters
    ///
    /// - `index`: The index of the port to retrieve.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `HbaPort` at the specified index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn port_mut(&mut self, index: usize) -> &mut HbaPort {
        &mut self.ports[index]
    }

    /// Returns the number of implemented ports.
    ///
    /// # Returns
    ///
    /// The number of ports that are implemented and available for use.
    pub fn ports_count(&self) -> usize {
        let max_ports = self.ports.len() as u32;
        let port_count = (self.host_capabilities & 0x1F) + 1;
        port_count.min(max_ports) as usize
    }

    /// Enables AHCI mode for the HBA controller.
    ///
    /// Sets the AHCI Enable bit in the global host control register and waits for it to be set.
    /// This method also clears all pending interrupts once AHCI mode is enabled.
    ///
    /// # Returns
    ///
    /// `true` if AHCI mode is enabled successfully, `false` if it fails to enable within the timeout period.
    pub fn enable_ahci(&mut self) -> bool {
        let mut elapsed_time = 0;

        // Set the AHCI Enable bit in the global host control register
        self.global_host_control |= AHCI_ENABLE;

        // Wait until the AHCI Enable bit is set or until timeout
        while (self.global_host_control & AHCI_ENABLE) == 0 && elapsed_time < AHCI_ENABLE_TIMEOUT {
            sleep_for(10); // Sleep for 10 milliseconds
            elapsed_time += 10;
        }

        // Check if AHCI Enable bit is still not set after the timeout
        if (self.global_host_control & AHCI_ENABLE) == 0 {
            println!("Failed to enable AHCI after {} ms", elapsed_time);
            return false;
        }

        // Clear all pending interrupts
        self.interrupt_status = 0xFFFFFFFF;

        println!("AHCI enabled successfully");
        true
    }
}

impl HbaCommandHeader {
    /// Sets up the command header for a new command.
    ///
    /// Configures the command table base address, PRDT length, and FIS (Frame Information Structure).
    ///
    /// # Parameters
    ///
    /// - `buf_phys_addr`: The physical address of the buffer used for the command.
    /// - `prdt_len`: The length of the Physical Region Descriptor Table (PRDT) in entries.
    /// - `fis`: The `FisRegisterHostToDevice` structure representing the FIS for the command.
    pub fn setup(&mut self, buf_phys_addr: u64, prdt_len: u16, fis: FisRegisterHostToDevice) {
        // Set the base address for the command table.
        self.command_table_base = buf_phys_addr as u32;
        self.command_table_base_upper = (buf_phys_addr >> 32) as u32;

        // Set the command FIS length in DWORDS (each DWORD is 4 bytes).
        self.dword0.set_command_fis_length(
            (size_of::<FisRegisterHostToDevice>() / size_of::<u32>()) as u8,
        );

        // Set the length of the PRDT.
        self.dword0.set_prdt_length(prdt_len);

        // Write the FIS to the command table base.
        let fis_ptr = self.command_table_base as *mut FisRegisterHostToDevice;
        unsafe { fis_ptr.write_volatile(fis) };
    }

    /// Returns a mutable pointer to the command table associated with this command header.
    ///
    /// # Returns
    ///
    /// A mutable pointer to the `HbaCommandTable`.
    ///
    /// # Safety
    ///
    /// The returned pointer must be used with care, as improper use can lead to undefined behavior.
    pub fn get_command_table(&self) -> *mut HbaCommandTable {
        // Calculate the base address of the command table.
        let table_base = self.command_table_base as u64;
        let table_upper_base = self.command_table_base_upper as u64;
        let cmd_tbl_addr = (table_base + table_upper_base) as *mut HbaCommandTable;

        cmd_tbl_addr
    }
}

impl HbaCommandTable {
    /// Sets up the command table for a data transfer operation.
    ///
    /// Configures the Physical Region Descriptor Table (PRDT) entry for the data buffer.
    ///
    /// # Parameters
    ///
    /// - `buf_phys_addr`: The physical address of the buffer to be used for the data transfer.
    /// - `buf_size`: The size of the buffer in bytes.
    /// - `is_write`: A boolean flag indicating whether the operation is a write (`true`) or a read (`false`).
    /// - `dma_buf`: A mutable pointer to the DMA buffer used for data transfer.
    pub fn setup(&mut self, buf_phys_addr: u64, buf_size: usize, is_write: bool, dma_buf: *mut u8) {
        // Set up the Physical Region Descriptor Table entry for the buffer
        self.physical_region_descriptor_table[0] = HbaPhysicalRegionDescriptorTableEntry {
            data_base_address: if is_write {
                dma_buf as u32
            } else {
                buf_phys_addr as u32
            },
            data_base_address_upper: if is_write {
                (dma_buf as u64 >> 32) as u32
            } else {
                (buf_phys_addr >> 32) as u32
            },
            reserved1: 0,
            data_byte_count_reserved2_interrupt: DataByteCountReserved2Interrupt::new()
                .with_data_byte_count(buf_size as u32)
                .with_reserved2(0)
                .with_interrupt_on_completion(0),
        };
    }
}
