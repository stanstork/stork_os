use crate::{pci::device::device::search_device, println, sync::mutex::SpinMutex};
use controller::AhciController;
use sata_ident::SataIdentity;

pub mod controller;
pub mod device;
pub mod fis;
pub mod hba;
pub mod sata_ident;

// Mass storage class code for PCI devices.
const MASS_STORAGE: u8 = 0x01;
// Subclass code for AHCI controllers under the mass storage class.
const PCI_SUBCLASS_AHCI: u8 = 0x06;

/// A global mutable instance of `AhciController` wrapped in a `SpinMutex`.
/// This static variable is used to manage access to the AHCI controller across multiple threads.
pub static mut AHCI_CONTROLLER: SpinMutex<Option<AhciController>> = SpinMutex::new(None);

/// Initializes the AHCI controller by searching for a compatible mass storage device.
///
/// If a device is found, it initializes the `AhciController` and stores it in a global static variable.
pub fn init_ahci_controller() {
    let device = search_device(MASS_STORAGE, PCI_SUBCLASS_AHCI);
    if let Some(device) = device {
        let controller = unsafe { AhciController::init(device) };
        unsafe { AHCI_CONTROLLER = SpinMutex::new(Some(controller)) };
    } else {
        println!("AHCI Controller not found");
    }
}

/// Reads a specified number of sectors from a SATA device using the AHCI controller.
///
/// # Parameters
///
/// - `port`: The port number of the SATA device to read from.
/// - `sata_ident`: A reference to the `SataIdentity` structure containing the SATA device's identity information.
/// - `buffer`: A mutable pointer (`*mut u8`) to the destination buffer where the data read from the device will be copied.
/// - `start_sector`: The starting sector (LBA) on the device from which to begin reading.
/// - `sectors_count`: The number of sectors to read from the device.
///
/// # Safety
///
/// This function is `unsafe` because it involves raw pointer manipulation and direct memory access.
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

/// Writes a specified number of sectors to a SATA device using the AHCI controller.
///
/// # Parameters
///
/// - `port`: The port number of the SATA device to write to.
/// - `buffer`: A mutable pointer (`*mut u8`) to the source buffer containing the data to be written to the device.
/// - `start_sector`: The starting sector (LBA) on the device where the write operation should begin.
/// - `sectors_count`: The number of sectors to write to the device.
///
/// # Safety
///
/// This function is `unsafe` because it involves raw pointer manipulation and direct memory access.
pub fn write_sectors(port: usize, buffer: *mut u8, start_sector: u64, sectors_count: u64) {
    let controller = unsafe { AHCI_CONTROLLER.lock() };
    if let Some(ref controller) = *controller {
        unsafe { controller.write(port, buffer, start_sector, sectors_count) };
    }
}

pub(crate) fn byte_swap_string(string: &mut [u8]) {
    let length = string.len();
    for i in (0..length).step_by(2) {
        if i + 1 < length {
            string.swap(i, i + 1);
        }
    }
}

pub(crate) fn print_device_info(identity: &SataIdentity) {
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
