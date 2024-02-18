/// A structure representing a descriptor table pointer (GDTR, IDTR, LDTR, or TR)
/// Format is suitable for direct loading into the corresponding x86 control register.
#[repr(C, packed)]
pub struct DescriptorTablePointer {
    /// Size of the table.
    pub limit: u16,
    /// Pointer to the memory region containing the DT.
    pub base: u64,
}

// Framebuffer structure representing a basic framebuffer.
#[repr(C)]
pub struct Framebuffer {
    pub pointer: *mut u32, // Pointer to the beginning of the framebuffer in memory.
    pub width: u32,        // Width of the framebuffer in pixels.
    pub height: u32,       // Height of the framebuffer in pixels.
    pub pixels_per_scanline: u32, // Number of pixels per scanline (often equals width but can be larger).
}

// Boot_Info structure containing information passed to the OS at boot time.
#[repr(C)]
pub struct BootInfo {
    pub memory_map: *mut u32,              // Pointer to the system's memory map.
    pub memory_map_size: usize,            // Total size of the memory map.
    pub memory_map_descriptor_size: usize, // Size of an individual memory descriptor in the memory map.
    pub framebuffer: Framebuffer,          // Framebuffer information for the display.
}
