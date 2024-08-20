use crate::{memory, print, println};

use super::{ahci::AHCI_CONTROLLER, ahci_controller::DeviceSignature};

// https://forum.osdev.org/viewtopic.php?f=1&t=30118
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct SATAIdent {
    config: u16,                    // Lots of obsolete bit flags
    cyls: u16,                      // Obsolete
    reserved2: u16,                 // Special (word 2)
    heads: u16,                     // Physical heads
    track_bytes: u16,               // Unformatted bytes per track
    pub sector_bytes: u16,          // Unformatted bytes per sector
    sectors: u16,                   // Physical sectors per track
    vendor0: u16,                   // Vendor unique
    vendor1: u16,                   // Vendor unique
    vendor2: u16,                   // Vendor unique
    pub(crate) serial_no: [u8; 20], // 20 bit serial number
    buf_type: u16,                  // Buffer type
    buf_size: u16,                  // Buffer size in 512-byte increments
    ecc_bytes: u16,                 // ECC bytes
    pub(crate) fw_rev: [u8; 8],     // Firmware revision
    pub(crate) model: [u8; 40],     // Model name
    multi_count: u16,               // Multiple count
    dword_io: u16,                  // IORDY may be disabled
    capability1: u16,               // Bit 8: LBA supported
    capability2: u16,               // Bit 8: IORDY may be disabled
    vendor5: u8,                    // Vendor unique
    tPIO: u8,                       // Time to ready after power-on, 0 = slow, 1 = medium, 2 = fast
    vendor6: u8,                    // Vendor unique
    tDMA: u8,                       // Time to ready after power-on, 0 = slow, 1 = medium, 2 = fast
    field_valid: u16,               // bits 0: cur_ok, 1: config_ok
    cur_cyls: u16,                  // Logical cylinders
    cur_heads: u16,                 // Logical heads
    cur_sectors: u16,               // Logical sectors per track
    cur_capacity0: u16,             // Logical total sectors on drive
    cur_capacity1: u16,             // (2 words)
    multsect: u8,                   // Multiple sector count
    multsect_valid: u8,             // Multiple sector setting valid
    pub(crate) lba_capacity: u32,   // Total number of sectors
    dma_1word: u16,                 // Single-word DMA info
    dma_mword: u16,                 // Multi-word DMA info
    eide_pio_modes: u16,            // Advanced PIO modes
    eide_dma_min: u16,              // Min multiword DMA transfer cycle time in ns
    eide_dma_time: u16,             // Recommended multiword DMA transfer cycle time in ns
    eide_pio: u16,                  // Min PIO transfer cycle time in ns
    eide_pio_iordy: u16,            // Min IORDY cycle time in ns
    words69_70: [u16; 2],           // Reserved words 69-70
    words71_74: [u16; 4],           // Reserved words 71-74
    queue_depth: u16,               // Queue depth
    sata_capability: u16,           // SATA capabilities
    sata_additional: u16,           // SATA additional capabilities
    sata_supported: u16,            // SATA supported features
    features_enabled: u16,          // Features enabled
    major_rev_num: u16,             // Major rev number
    minor_rev_num: u16,             // Minor rev number
    command_set_1: u16,             // bits 0:Smart 1:Security 2:Removable 3:PM
    command_set_2: u16,             // bits 14:Smart Enabled 13:0 zero
    cfsse: u16,                     // Command set-feature supported extensions
    cfs_enable_1: u16,              // Command set-feature enabled
    cfs_enable_2: u16,              // Command set-feature enabled
    csf_default: u16,               // Command set-feature default
    dma_ultra: u16,                 // Ultra DMA mode
    word89: u16,                    // Reserved word 89
    word90: u16,                    // Reserved word 90
    cur_apm_values: u16,            // Current APM values
    word92: u16,                    // Reserved word 92
    comreset: u16,                  // Command flag
    acoustic: u16,                  // Acoustic management
    min_req_sz: u16,                // Stream minimum request size
    transfer_time_dma: u16,         // Streaming transfer time - DMA
    access_latency: u16,            // Streaming access latency - DMA and PIO
    perf_granularity: u32,          // Streaming performance granularity
    total_usr_sectors: [u32; 2],    // Total user addressable sectors for 48-bit LBA
    transfer_time_pio: u16,         // Streaming transfer time - PIO
    reserved105: u16,               // Reserved word 105
    sector_sz: u16,                 // Physical sector size / logical sector size
    inter_seek_delay: u16,          // Inter-seek delay for ISO7779
    words108_116: [u16; 9],         // Reserved words 108-116
    words_per_sector: u32,          // Words per logical sector
    supported_settings: u16,        // Supported settings
    command_set_3: u16,             // Command set 3
    words121_126: [u16; 6],         // Reserved words 121-126
    word127: u16,                   // Reserved word 127
    security_status: u16,           // Security status
    csfo: u16,                      // Current setting for features
    words130_155: [u16; 26],        // Reserved words 130-155
    word156: u16,                   // Reserved word 156
    words157_159: [u16; 3],         // Reserved words 157-159
    cfa: u16,                       // CFA power mode
    words161_175: [u16; 15],        // Reserved words 161-175
    media_serial: [u8; 60],         // Media serial number
    sct_cmd_transport: u16,         // SCT command transport
    words207_208: [u16; 2],         // Reserved words 207-208
    block_align: u16,               // Block alignment
    wrv_sec_count: u32,             // Write-read-verify sector count mode 3 only
    verf_sec_count: u32,            // Verify sector count mode 2 only
    nv_cache_capability: u16,       // NV Cache capabilities
    nv_cache_sz: u16,               // NV Cache size in 512-byte blocks
    nv_cache_sz2: u16,              // NV Cache size in 512-byte blocks
    rotation_rate: u16,             // Rotation rate
    reserved218: u16,               // Reserved word 218
    nv_cache_options: u16,          // NV Cache options
    words220_221: [u16; 2],         // Reserved words 220-221
    transport_major_rev: u16,       // Transport major revision number
    transport_minor_rev: u16,       // Transport minor revision number
    words224_233: [u16; 10],        // Reserved words 224-233
    min_dwnload_blocks: u16, // Minimum number of 512-byte data blocks per download microcode command
    max_dwnload_blocks: u16, // Maximum number of 512-byte data blocks per download microcode command
    words236_254: [u16; 19], // Reserved words 236-254
    integrity: u16,          // Integrity word
}

#[derive(Clone, Copy)]
pub struct AhciDevice {
    pub port_no: usize,
    pub signature: DeviceSignature,
    pub sata_ident: SATAIdent,
}

impl AhciDevice {
    pub fn new(port_no: usize, signature: DeviceSignature, sata_ident: SATAIdent) -> Self {
        AhciDevice {
            port_no,
            signature,
            sata_ident,
        }
    }

    pub fn read(&self, buffer: *mut u8, start_sector: u64, sectors_count: u64) {
        unsafe {
            AHCI_CONTROLLER.lock().as_mut().unwrap().read(
                self.port_no,
                &self.sata_ident,
                buffer,
                start_sector,
                sectors_count,
            )
        };
    }

    pub fn write(&self, buffer: *mut u8, start_sector: u64, sectors_count: u64) {
        unsafe {
            AHCI_CONTROLLER.lock().as_mut().unwrap().write(
                self.port_no,
                buffer,
                start_sector,
                sectors_count,
            )
        };

        let check_buffer = memory::allocate_dma_buffer(512) as *mut u8;
        self.read(check_buffer, start_sector, sectors_count);

        // Dump the buffer to the screen
        // for i in 0..512 {
        //     print!("{:02X} ", unsafe { *check_buffer.add(i) });
        //     if i % 16 == 15 {
        //         println!();
        //     }
        // }
    }
}
