use crate::pci::PCI;

use super::ahci_controller::AhciController;

pub const MASS_STORAGE: u8 = 0x01;
pub const PCI_SUBCLASS_AHCI: u8 = 0x06;

pub fn init() {
    let devices = PCI::search_devices(MASS_STORAGE, PCI_SUBCLASS_AHCI);
    for device in devices {
        AhciController::init(device);
    }
}
