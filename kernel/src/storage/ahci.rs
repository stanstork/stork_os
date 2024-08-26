use super::ahci_controller::AhciController;
use crate::{pci::PCI, println, sync::mutex::SpinMutex};

pub const MASS_STORAGE: u8 = 0x01;
pub const PCI_SUBCLASS_AHCI: u8 = 0x06;

pub static mut AHCI_CONTROLLER: SpinMutex<Option<AhciController>> = SpinMutex::new(None);

pub fn init() {
    let device = PCI::search_device(MASS_STORAGE, PCI_SUBCLASS_AHCI);
    if let Some(device) = device {
        unsafe {
            let controller = AhciController::init(device);
            AHCI_CONTROLLER = SpinMutex::new(Some(controller));
        }
    } else {
        println!("AHCI Controller not found");
    }
}
