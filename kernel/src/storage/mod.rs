use crate::sync::mutex::SpinMutex;
use ahci::{device::AhciDevice, init_ahci_controller};
use alloc::string::String;
use manager::StorageManager;

pub mod ahci;
pub mod manager;

/// A global mutable instance of `StorageManager` wrapped in a `SpinMutex`.
/// This static variable is used to manage access to the `StorageManager` across multiple threads.
pub static mut STORAGE_MANAGER: SpinMutex<Option<StorageManager>> = SpinMutex::new(None);

/// Initializes the storage manager.
pub fn init_storage_manager() {
    let storage_manager = StorageManager::new();
    unsafe { STORAGE_MANAGER = SpinMutex::new(Some(storage_manager)) };
}

/// Registers an AHCI device with the storage manager.
///
/// # Parameters
///
/// - `device`: The `AhciDevice` to be registered with the storage manager.
/// - `name`: A `String` representing the name of the device to register.
pub fn register_ahci_device(device: AhciDevice, name: String) {
    let mut storage_manager = unsafe { STORAGE_MANAGER.lock() };
    if let Some(ref mut storage_manager) = *storage_manager {
        storage_manager.register_ahci_device(device, name);
    }
}

/// Retrieves an AHCI device by name from the storage manager.
///
/// # Parameters
///
/// - `name`: A string slice representing the name of the device to retrieve.
///
/// # Returns
///
/// An `Option<AhciDevice>` containing the device if found, or `None` if the device is not registered.
pub fn get_ahci_device(name: &str) -> Option<AhciDevice> {
    let storage_manager = unsafe { STORAGE_MANAGER.lock() };
    if let Some(ref storage_manager) = *storage_manager {
        storage_manager.get_ahci_device(name).cloned()
    } else {
        None
    }
}

/// Initializes the storage subsystem, including the storage manager and AHCI controller.
pub fn init() {
    init_storage_manager();
    init_ahci_controller();
}
