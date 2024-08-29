use crate::sync::mutex::SpinMutex;
use ahci::{ahci_device::AhciDevice, init_ahci_controller};
use alloc::string::String;
use storage_manager::StorageManager;

pub mod ahci;
pub mod storage_manager;

pub static mut STORAGE_MANAGER: SpinMutex<Option<StorageManager>> = SpinMutex::new(None);

pub fn init_storage_manager() {
    let storage_manager = StorageManager::new();
    unsafe { STORAGE_MANAGER = SpinMutex::new(Some(storage_manager)) };
}

pub fn register_ahci_device(device: AhciDevice, name: String) {
    let mut storage_manager = unsafe { STORAGE_MANAGER.lock() };
    if let Some(ref mut storage_manager) = *storage_manager {
        storage_manager.register_ahci_device(device, name);
    }
}

pub fn get_ahci_device(name: &str) -> Option<AhciDevice> {
    let storage_manager = unsafe { STORAGE_MANAGER.lock() };
    if let Some(ref storage_manager) = *storage_manager {
        storage_manager.get_ahci_device(name).cloned()
    } else {
        None
    }
}

pub fn init() {
    init_storage_manager();
    init_ahci_controller();
}
