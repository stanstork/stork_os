use bitfield_struct::bitfield;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FisType {
    REGISTER_HOST_TO_DEVICE = 0x27,
    REGISTER_DEVICE_TO_HOST = 0x34,
    DMA_ACTIVATE = 0x39,
    DMA_SETUP = 0x41,
    DMA_DATA = 0x46,
    BIST_ACTIVATE = 0x58,
    PIO_SETUP = 0x5f,
    DEVICE_BITS = 0xa1,
}

#[repr(u8)]
pub enum Command {
    ATA_IDENTIFY = 0xEC,
    ATA_READ = 0x25,
    ATA_WRITE = 0x35,
    ATA_FLUSH_CACHE = 0xE7,
}

#[bitfield(u8)]
pub struct FisRegisterHostToDeviceType {
    #[bits(4)]
    pub port_multiplier_port: u8,
    #[bits(3)]
    pub reserved1: u8,
    pub command_control: bool,
}

#[derive(Clone)]
#[repr(C, packed)]
pub struct FisRegisterHostToDevice {
    pub type_: FisType,
    pub flags: FisRegisterHostToDeviceType,
    pub command: u8,
    pub feature_low: u8,

    pub lba0: u8,
    pub lba1: u8,
    pub lba2: u8,
    pub device: u8,

    pub lba3: u8,
    pub lba4: u8,
    pub lba5: u8,
    pub feature_high: u8,

    pub count_low: u8,
    pub count_high: u8,
    pub isochronous_command_completion: u8,
    pub control: u8,

    pub reserved2: u32,
}

impl Default for FisRegisterHostToDevice {
    fn default() -> Self {
        Self {
            type_: FisType::REGISTER_HOST_TO_DEVICE,
            flags: FisRegisterHostToDeviceType::new(),
            command: 0,
            feature_low: 0,
            lba0: 0,
            lba1: 0,
            lba2: 0,
            device: 0,
            lba3: 0,
            lba4: 0,
            lba5: 0,
            feature_high: 0,
            count_low: 0,
            count_high: 0,
            isochronous_command_completion: 0,
            control: 0,
            reserved2: 0,
        }
    }
}
