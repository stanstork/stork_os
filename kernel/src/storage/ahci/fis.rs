use bitfield_struct::bitfield;

/// Represents the different types of FIS (Frame Information Structure) used in AHCI communication.
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

/// Enum representing the various ATA commands used in AHCI communication.
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

impl FisRegisterHostToDevice {
    /// Creates a new FIS for a READ command to a SATA device.
    ///
    /// # Parameters
    ///
    /// - `sector`: The starting sector (LBA) on the device to read from.
    /// - `sector_count`: The number of sectors to read.
    ///
    /// # Returns
    ///
    /// A new `FisRegisterHostToDevice` configured for a READ command.
    pub fn read_command(sector: u64, sector_count: u64) -> Self {
        Self::new(Command::ATA_READ, sector, sector_count)
    }

    /// Creates a new FIS for a WRITE command to a SATA device.
    ///
    /// # Parameters
    ///
    /// - `sector`: The starting sector (LBA) on the device to write to.
    /// - `sector_count`: The number of sectors to write.
    ///
    /// # Returns
    ///
    /// A new `FisRegisterHostToDevice` configured for a WRITE command.
    pub fn write_command(sector: u64, sector_count: u64) -> Self {
        Self::new(Command::ATA_WRITE, sector, sector_count)
    }

    /// Creates a new FIS for an IDENTIFY DEVICE command to a SATA device.
    ///
    /// # Returns
    ///
    /// A new `FisRegisterHostToDevice` configured for an IDENTIFY DEVICE command.
    pub fn identify_command() -> Self {
        Self::new(Command::ATA_IDENTIFY, 0, 0)
    }

    /// Helper function to create a new `FisRegisterHostToDevice` with the specified command, sector, and sector count.
    fn new(command: Command, sector: u64, sector_count: u64) -> Self {
        FisRegisterHostToDevice {
            type_: FisType::REGISTER_HOST_TO_DEVICE,
            flags: FisRegisterHostToDeviceType::new()
                .with_port_multiplier_port(0)
                .with_reserved1(0)
                .with_command_control(true),
            command: command as u8,
            device: 1 << 6, // LBA mode
            feature_low: 1, // DMA mode
            lba0: (sector & 0xFF) as u8,
            lba1: ((sector >> 8) & 0xFF) as u8,
            lba2: ((sector >> 16) & 0xFF) as u8,
            lba3: (sector >> 24) as u8,
            count_low: (sector_count & 0xff) as u8,
            count_high: ((sector_count >> 8) & 0xff) as u8,
            control: 1,
            ..Default::default()
        }
    }
}
