// Framebuffer structure representing a basic framebuffer.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Framebuffer {
    pub pointer: *mut u32, // Pointer to the beginning of the framebuffer in memory.
    pub width: u32,        // Width of the framebuffer in pixels.
    pub height: u32,       // Height of the framebuffer in pixels.
    pub pixels_per_scanline: u32, // Number of pixels per scanline (often equals width but can be larger).
}
