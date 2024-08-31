use super::{hba::DeviceSignature, read_sectors, sata_ident::SataIdentity, write_sectors};
use crate::memory;

/// Represents a device connected to an AHCI port, providing read and write capabilities.
#[derive(Clone, Copy)]
pub struct AhciDevice {
    pub port_number: usize, // The port number on the AHCI controller where the device is connected.
    pub signature: DeviceSignature, // The device signature indicating the type of device (e.g., ATA, ATAPI).
    pub sata_ident: SataIdentity, // The identity information of the SATA device, including serial number, model, etc.
}

impl AhciDevice {
    /// Creates a new `AhciDevice` instance.
    ///
    /// # Parameters
    ///
    /// - `port_number`: The port number on the AHCI controller where the device is connected.
    /// - `signature`: The device signature indicating the type of device (e.g., ATA, ATAPI).
    /// - `sata_ident`: The identity information of the SATA device, including details like the serial number, model, and firmware version.
    ///
    /// # Returns
    ///
    /// Returns a new instance of `AhciDevice`.
    pub fn new(port_number: usize, signature: DeviceSignature, sata_ident: SataIdentity) -> Self {
        AhciDevice {
            port_number,
            signature,
            sata_ident,
        }
    }

    /// Reads a specified number of sectors from the SATA device into the provided buffer.
    ///
    /// # Parameters
    ///
    /// - `buffer`: A mutable pointer (`*mut u8`) to the destination buffer where the data read from the device will be copied.
    /// - `start_sector`: The starting sector on the SATA device from which to begin reading.
    /// - `sectors_count`: The number of sectors to read from the device.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `buffer` is valid and large enough to hold the data for the specified number of sectors.
    pub fn read_sectors(&self, buffer: *mut u8, start_sector: u64, sectors_count: u64) {
        read_sectors(
            self.port_number,
            &self.sata_ident,
            buffer,
            start_sector,
            sectors_count,
        );
    }

    /// Writes a specified number of sectors from the provided buffer to the SATA device.
    ///
    /// After writing, the function verifies the write operation by reading back the sectors into a check buffer.
    ///
    /// # Parameters
    ///
    /// - `buffer`: A constant pointer (`*const u8`) to the source buffer containing the data to be written to the device.
    /// - `start_sector`: The starting sector on the SATA device where the write operation should begin.
    /// - `sectors_count`: The number of sectors to write to the device.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `buffer` is valid and contains the correct data for the specified number of sectors.
    pub fn write_sectors(&self, buffer: *mut u8, start_sector: u64, sectors_count: u64) {
        write_sectors(self.port_number, buffer, start_sector, sectors_count);

        // Check if write was successful by reading the written sectors
        let check_buffer = memory::allocate_dma_buffer(512) as *mut u8;
        self.read_sectors(check_buffer, start_sector, sectors_count);
    }
}
