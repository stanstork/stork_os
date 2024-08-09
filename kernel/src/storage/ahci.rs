use super::ahci_controller::AhciController;
use crate::pci::PCI;

pub const MASS_STORAGE: u8 = 0x01;
pub const PCI_SUBCLASS_AHCI: u8 = 0x06;

pub fn init() {
    let devices = PCI::search_devices(MASS_STORAGE, PCI_SUBCLASS_AHCI);
    for device in devices {
        unsafe { AhciController::initialize_ahci_controller(device) };
    }
}
