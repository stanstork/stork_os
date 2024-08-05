use super::ahci_controller::{AhciController, DeviceSignature};

struct DeviceInfo {
    config: u16,
}

pub struct AhciDevice {
    pub port: u8,
    pub controller: &'static AhciController,
    pub signature: DeviceSignature,
}
