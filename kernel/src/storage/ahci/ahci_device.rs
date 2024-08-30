use super::{hba::DeviceSignature, read_sectors, sata_ident::SataIdentity, write_sectors};
use crate::memory;

#[derive(Clone, Copy)]
pub struct AhciDevice {
    pub port_number: usize,
    pub signature: DeviceSignature,
    pub sata_ident: SataIdentity,
}

impl AhciDevice {
    pub fn new(port_number: usize, signature: DeviceSignature, sata_ident: SataIdentity) -> Self {
        AhciDevice {
            port_number,
            signature,
            sata_ident,
        }
    }

    pub fn read_sectors(&self, buffer: *mut u8, start_sector: u64, sectors_count: u64) {
        read_sectors(
            self.port_number,
            &self.sata_ident,
            buffer,
            start_sector,
            sectors_count,
        );
    }

    pub fn write_sectors(&self, buffer: *mut u8, start_sector: u64, sectors_count: u64) {
        write_sectors(self.port_number, buffer, start_sector, sectors_count);

        // Check if write was successful by reading the written sectors
        let check_buffer = memory::allocate_dma_buffer(512) as *mut u8;
        self.read_sectors(check_buffer, start_sector, sectors_count);
    }
}
