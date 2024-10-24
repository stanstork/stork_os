#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct Fat32BootSector {
    // Jump instruction to boot code, typically three bytes to jump past the header.
    pub jump_instruction: [u8; 3],
    // OEM name, usually the name of the formatting software.
    pub oem_name: [u8; 8],
    // Bytes per sector; common values are 512, 1024, 2048, or 4096.
    pub bytes_per_sector: u16,
    // Number of sectors per cluster. Cluster is the basic unit of file allocation.
    pub sectors_per_cluster: u8,
    // Number of reserved sectors in the volume starting from the first sector; the boot sector is included in the count of reserved sectors.
    pub reserved_sectors: u16,
    // Number of FAT data structures on the volume, usually 2.
    pub fat_count: u8,
    // Maximum number of root directory entries; typically 0 for FAT32, which indicates that the root directory is a cluster chain.
    pub root_dir_entries: u16,
    // Total number of sectors in the file system; if this field is 0, use `total_sectors_large`.
    pub total_sectors: u16,
    // Media descriptor; provides information about the media type. Commonly used value for fixed disks is 0xF8.
    pub media_descriptor: u8,
    // Number of sectors per FAT; if 0, use `sectors_per_fat_large`.
    pub sectors_per_fat: u16,
    // Number of sectors per track for interrupt 0x13. This is relevant for drives with CHS addressing.
    pub sectors_per_track: u16,
    // Number of heads for interrupt 0x13. This is relevant for drives with CHS addressing.
    pub head_count: u16,
    // Number of hidden sectors preceding the partition that contains this FAT volume.
    pub hidden_sectors: u32,
    // Total number of sectors in the file system if `total_sectors` is 0.
    pub total_sectors_large: u32,
    // Number of sectors per FAT (File Allocation Table) if `sectors_per_fat` is 0; relevant for FAT32.
    pub sectors_per_fat_large: u32,
    // Flags indicating mirroring and FAT configuration.
    pub flags: u16,
    // File system version; usually 0 for FAT32.
    pub version: u16,
    // Cluster number of the root directory's start; relevant for FAT32.
    pub root_dir_start: u32,
    // Sector number of the FSInfo structure; typically 1.
    pub fs_info_sector: u16,
    // Sector number of the backup boot sector; typically 6.
    pub backup_boot_sector: u16,
    // Reserved for future expansion; should be set to 0.
    pub reserved0: u32,
    // Reserved for future expansion; should be set to 0.
    pub reserved1: u32,
    // Reserved for future expansion; should be set to 0.
    pub reserved2: u32,
    // Physical drive number (0x00 for floppy disks, 0x80 for hard disks).
    pub drive_number: u8,
    // Reserved; should be set to 0.
    pub reserved3: u8,
    // Extended boot signature; should be 0x29 to indicate that the fields following are valid.
    pub ext_signature: u8,
    // Volume serial number; typically created randomly during formatting.
    pub serial_number: u32,
    // Volume label; a user-readable label for the volume.
    pub volume_label: [u8; 11],
    // File system type identifier; typically "FAT32".
    pub system_id: [u8; 8],
    // Boot code; contains the bootloader code for the operating system.
    pub boot_code: [u8; 420],
    // Boot sector signature; should be 0xAA55 to indicate a valid boot sector.
    pub boot_signature: u16,
}
