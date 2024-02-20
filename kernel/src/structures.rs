use crate::drivers::screen::framebuffer::Framebuffer;

/// A structure representing a descriptor table pointer (GDTR, IDTR, LDTR, or TR)
/// Format is suitable for direct loading into the corresponding x86 control register.
#[repr(C, packed)]
pub struct DescriptorTablePointer {
    /// Size of the table.
    pub limit: u16,
    /// Pointer to the memory region containing the DT.
    pub base: u64,
}

// PSF1Header structure representing the header of a PSF1 font.
#[repr(C)]
pub struct PSF1Header {
    pub magic: [u8; 2], // Magic number identifying the file as a PSF1 font.
    pub mode: u8,       // Mode of the PSF1 font.
    pub char_size: u8,  // Size of each character in the PSF1 font.
}

#[repr(C)]
pub struct PSF1Font {
    pub psf1_header: PSF1Header, // Header of the PSF1 font.
    pub glyph_buffer: *const u8, // Pointer to the buffer containing the glyphs of the PSF1 font.
}

// Boot_Info structure containing information passed to the OS at boot time.
#[repr(C)]
pub struct BootInfo {
    pub memory_map: *mut u32,              // Pointer to the system's memory map.
    pub memory_map_size: usize,            // Total size of the memory map.
    pub memory_map_descriptor_size: usize, // Size of an individual memory descriptor in the memory map.
    pub framebuffer: Framebuffer,          // Framebuffer information for the display.
    pub font: PSF1Font,                    // PSF1 font information for the display.
}
