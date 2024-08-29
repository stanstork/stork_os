use crate::{pci::PCI, println, sync::mutex::SpinMutex};
use ahci_controller::AhciController;
use sata_ident::SataIdentity;

pub mod ahci_controller;
pub mod ahci_device;
pub mod fis;
pub mod hba;
pub mod sata_ident;

pub const MASS_STORAGE: u8 = 0x01;
pub const PCI_SUBCLASS_AHCI: u8 = 0x06;

pub static mut AHCI_CONTROLLER: SpinMutex<Option<AhciController>> = SpinMutex::new(None);

pub fn init_ahci_controller() {
    let device = PCI::search_device(MASS_STORAGE, PCI_SUBCLASS_AHCI);
    if let Some(device) = device {
        let controller = unsafe { AhciController::init(device) };
        unsafe { AHCI_CONTROLLER = SpinMutex::new(Some(controller)) };
    } else {
        println!("AHCI Controller not found");
    }
}

pub fn read_sectors(
    port: usize,
    sata_ident: &SataIdentity,
    buffer: *mut u8,
    start_sector: u64,
    sectors_count: u64,
) {
    let controller = unsafe { AHCI_CONTROLLER.lock() };
    if let Some(ref controller) = *controller {
        unsafe { controller.read(port, sata_ident, buffer, start_sector, sectors_count) };
    }
}

pub fn write_sectors(port: usize, buffer: *mut u8, start_sector: u64, sectors_count: u64) {
    let controller = unsafe { AHCI_CONTROLLER.lock() };
    if let Some(ref controller) = *controller {
        unsafe { controller.write(port, buffer, start_sector, sectors_count) };
    }
}
