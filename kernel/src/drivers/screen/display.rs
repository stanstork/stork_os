use super::framebuffer::Framebuffer;
use crate::structures::PSF1Font;
use core::fmt::Write;

struct Console {
    pub cursor: (usize, usize), // Cursor position (x, y)
    pub fg_color: u32,          // Foreground color
    pub bg_color: u32,          // Background color
    pub width: usize,           // Width of the console in characters
    pub height: usize,          // Height of the console in characters
}

pub struct Display {
    framebuffer: &'static Framebuffer, // Framebuffer for the display.
    backbuffer: Framebuffer,           // Backbuffer for the display.
    font: &'static PSF1Font,           // Font for the display.
    console: Console,                  // Console for the display.
    char_width: usize,                 // Width of each character in the font.
    char_height: usize,                // Height of each character in the font.
}

// The static mutable DISPLAY variable is used to store the display state.
static mut DISPLAY: Display = Display::default();

impl Display {
    /// Creates a new Display struct with all fields set to 0.
    pub const fn default() -> Display {
        Display {
            framebuffer: &Framebuffer {
                pointer: 0 as *mut u32,
                width: 0,
                height: 0,
                pixels_per_scanline: 0,
            },
            backbuffer: Framebuffer {
                pointer: 0 as *mut u32,
                width: 0,
                height: 0,
                pixels_per_scanline: 0,
            },
            font: &PSF1Font {
                glyph_buffer: 0 as *const u8,
                psf1_header: crate::structures::PSF1Header {
                    magic: [0; 2],
                    mode: 0,
                    char_size: 0,
                },
            },
            console: Console {
                cursor: (0, 0),
                fg_color: 0,
                bg_color: 0,
                width: 0,
                height: 0,
            },
            char_width: 0,
            char_height: 0,
        }
    }

    /// Clears the screen by writing spaces to every position in the framebuffer memory.
    pub unsafe fn clear_screen(&mut self) {
        let clear_char = 0xFF000000; // Black space character to clear with
        let address = self.backbuffer.pointer as *mut u32;

        for i in 0..(self.backbuffer.width * self.backbuffer.height) {
            *address.add(i as usize) = clear_char;
        }
    }

    /// Puts a character at the current cursor position on the display.
    unsafe fn put_char(char: char) {
        // Get the current cursor position
        let (mut x, mut y) = DISPLAY.console.cursor;

        match char {
            '\n' => {
                // Move the cursor to the start of the next line
                x = 0;
                y += 1;
            }
            _ => {
                // Put the character at the current cursor position
                Self::put_char_at(x as usize, y as usize, char);

                // Move the cursor to the next position
                x += 1;

                // Check if the cursor has reached or passed the end of the line
                if x >= DISPLAY.console.width {
                    // Move cursor to the start of the next line
                    x = 0;
                    y += 1;
                }
            }
        }

        // Update the cursor position
        DISPLAY.console.cursor = (x, y);

        // Handle scrolling if the cursor goes beyond the bottom of the display
        if y >= DISPLAY.console.height {
            Self::scroll_up();
        }
    }

    /// Puts a character at a specific position on the display.
    unsafe fn put_char_at(x_off: usize, y_off: usize, char: char) {
        // Get the pointer to the backbuffer where pixels are drawn
        let address = DISPLAY.backbuffer.pointer as *mut u32;

        // Get the width and height of each character
        let char_width = DISPLAY.char_width;
        let char_height = DISPLAY.char_height;

        // Get the number of pixels per scanline in the backbuffer
        let pitch = DISPLAY.backbuffer.pixels_per_scanline as usize;

        // Calculate the starting pointer in the glyph buffer for the given character
        let font_ptr = unsafe {
            DISPLAY
                .font
                .glyph_buffer
                .add(char as usize * DISPLAY.font.psf1_header.char_size as usize)
        };

        for y in (y_off * char_height)..(y_off * char_height + char_height) {
            for x in (x_off * char_width)..(x_off * char_width + char_width) {
                // Get the byte value from the font's glyph buffer for the current row
                let font_byte = unsafe { *font_ptr.add(y % char_height) };

                // The if logic:
                // 1. `(x % char_width)` calculates the current column within the character glyph.
                // 2. `0b10000000 >> (x % char_width)` creates a mask to check the specific bit in the font_byte.
                //    It shifts the bit 0b10000000 to the right based on the column. For example, for the first column,
                //    the mask is 0b10000000, for the second column 0b01000000, and so on.
                // 3. `font_byte & [mask]` checks if the specific bit at the column in the font byte is set.
                //    If the bit is set (i.e., > 0), it means this pixel should be drawn.
                if font_byte & (0b10000000 >> (x % char_width)) > 0 {
                    // Set the pixel at the current position to the foreground color
                    *address.add(y * pitch + x) = DISPLAY.console.fg_color;
                }
            }
        }
    }

    /// Scrolls the display up by one line.
    unsafe fn scroll_up() {
        // Get the pointer to the backbuffer where pixels are drawn
        let address = DISPLAY.backbuffer.pointer as *mut u32;

        let width = DISPLAY.backbuffer.width as usize;
        let height = DISPLAY.backbuffer.height as usize;
        let pitch = DISPLAY.backbuffer.pixels_per_scanline as usize;

        // Move all pixels up by char_height
        // Start from the second line and copy the pixels from the line above
        for y in 0..(height - DISPLAY.char_height) {
            for x in 0..width {
                let position = y * pitch + x;
                let new_position = (y + DISPLAY.char_height) * pitch + x;
                *address.add(position) = *address.add(new_position);
            }
        }

        // Clear the last line by setting its pixels to a specific color (e.g., black)
        // The color is represented in ARGB format, where 0xFF000000 is opaque black
        // let clear_color = 0xFF000000;
        let clear_color = DISPLAY.console.bg_color;
        for y in (height - DISPLAY.char_height)..height {
            for x in 0..width {
                let position = y * pitch + x;
                *address.add(position) = clear_color;
            }
        }

        // Move the cursor up by one line
        // This adjusts the cursor position to stay within the new screen bounds
        if DISPLAY.console.cursor.1 > 0 {
            DISPLAY.console.cursor.1 -= 1;
        } else {
            DISPLAY.console.cursor.1 = 0;
        }
    }
}

/// Initializes the display with the given framebuffer and font.
pub fn init(framebuffer: &'static Framebuffer, font: &'static PSF1Font) {
    let backbuffer = Framebuffer {
        pointer: framebuffer.pointer as *mut u32,
        width: framebuffer.width,
        height: framebuffer.height,
        pixels_per_scanline: framebuffer.pixels_per_scanline,
    };

    unsafe {
        DISPLAY = Display {
            framebuffer,
            backbuffer,
            font,
            console: Console {
                cursor: (0, 0),
                fg_color: 0xFFFFFFFF, // White
                bg_color: 0xFF000000, // Black
                width: (framebuffer.width / 8) as usize,
                height: (framebuffer.height / 16) as usize,
            },
            char_width: 8,
            char_height: 16,
        };
    }
}

// Implement the Write trait for the Display struct
// This allows the use of Rust's formatting macros with the Display struct
impl Write for Display {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for ch in s.chars() {
            unsafe {
                Display::put_char(ch);
            }
        }
        Ok(())
    }
}

/// Prints to the Framebuffer memory.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {$crate::drivers::screen::display::print(format_args!($($arg)*))};
}

/// Prints to the Framebuffer memory with a newline.
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n");
    };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*));
    };
}

/// Clears the screen by writing spaces to every position in the framebuffer memory.
#[macro_export]
macro_rules! cls {
    () => {
        $crate::drivers::screen::display::clear_screen();
    };
}

// Helper function to clear the screen
#[doc(hidden)]
pub fn clear_screen() {
    unsafe {
        DISPLAY.clear_screen();
    }
}

// Helper function to print text to the screen
#[doc(hidden)]
pub fn print(args: core::fmt::Arguments) {
    unsafe {
        DISPLAY.write_fmt(args).unwrap();
    }
}
