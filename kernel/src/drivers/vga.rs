use core::fmt::Write;

use super::cpu::{byte_out, CURSOR_PORT_COMMAND, CURSOR_PORT_DATA};

// Constants for VGA memory and settings
const VGA_START: *mut VgaChar = 0xB8000 as *mut VgaChar; // VGA text mode memory starts at 0xB8000
const VGA_WIDTH: usize = 80; // VGA text mode has 80 columns
const VGA_HEIGHT: usize = 25; // VGA text mode has 25 lines
const VGA_SIZE: usize = VGA_WIDTH * VGA_HEIGHT; // Total number of characters that can be displayed

// Pointer to the VGA text mode memory
const VGA_BUFFER: *mut VgaChar = VGA_START as *mut VgaChar;

// Global instance of the VGA buffer writer
pub static mut VGA: VgaBufferWriter = VgaBufferWriter {
    cursor_x: 0,
    cursor_y: 0,
    color_code: (Color::Blk as u8) << 4 | (Color::Wht as u8),
};

/// Structure representing a character in VGA text mode, including its ASCII value and color code
#[repr(C)]
#[derive(Copy, Clone)]
struct VgaChar {
    ascii_char: u8,
    color_code: u8,
}

/// Color codes for VGA text mode
#[derive(Copy, Clone)]
pub enum Color {
    Blk = 0,  // Black
    Blu = 1,  // Blue
    Grn = 2,  // Green
    Cyn = 3,  // Cyan
    Red = 4,  // Red
    Prp = 5,  // Purple
    Brn = 6,  // Brown
    Gry = 7,  // Gray
    Dgy = 8,  // Dark Gray
    Lbu = 9,  // Light Blue
    Lgr = 10, // Light Green
    Lcy = 11, // Light Cyan
    Lrd = 12, // Light Red
    Lpp = 13, // Light Purple
    Yel = 14, // Yellow
    Wht = 15, // White
}

/// Writer struct for the VGA buffer, managing cursor position and color code
pub struct VgaBufferWriter {
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub color_code: u8,
}

impl VgaBufferWriter {
    pub fn clear_screen(&mut self) {
        let clear_char = VgaChar {
            ascii_char: b' ', // Space character to clear with
            color_code: self.color_code,
        };

        unsafe {
            // Write the clear character to every position in the VGA text mode memory
            for i in 0..VGA_SIZE {
                VGA_BUFFER.add(i).write_volatile(clear_char);
            }
        }

        // Move the cursor to the top left corner
        self.move_cursor(0, 0);
    }

    /// Writes a single byte to the VGA text mode screen at the current cursor position.
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.cursor_x = 0;
                self.cursor_y += 1;
            }
            b'\r' => {
                self.cursor_x = 0;
            }
            b'\t' => {
                self.cursor_x += 4;
            }
            _ => {
                // Calculate the linear position index from the coordinates
                let position = self.cursor_y * VGA_WIDTH + self.cursor_x;
                unsafe {
                    // Write the character to the VGA text mode memory
                    VGA_START.add(position as usize).write_volatile(VgaChar {
                        ascii_char: byte,
                        color_code: self.color_code,
                    });
                }
                self.cursor_x += 1;
            }
        }

        // If the cursor is at the end of the line, move it to the next line
        self.scroll();
        self.move_cursor(self.cursor_x, self.cursor_y);
    }

    /// Writes a string to the VGA text mode screen at the current cursor position.
    pub fn write_str(&mut self, str: &str) {
        for byte in str.bytes() {
            self.write_byte(byte);
        }
    }

    /// Sets the color code for the VGA text mode screen.
    pub fn set_color(&mut self, fg: Color, bg: Color) {
        self.color_code = (bg as u8) << 4 | (fg as u8);
    }

    /// Scrolls the screen up by one line.
    /// This function moves all lines up by one, and clears the last line.
    fn scroll(&mut self) {
        if self.cursor_y < VGA_HEIGHT {
            return;
        }

        // Move all lines up by one
        for x in 1..VGA_HEIGHT {
            for y in 0..VGA_WIDTH {
                let to = y + (x - 1) * VGA_WIDTH;
                let from = y + x * VGA_WIDTH;
                unsafe {
                    let vga_char = VGA_BUFFER.add(from).read_volatile();
                    VGA_BUFFER.add(to).write_volatile(vga_char);
                }
            }
        }

        // Clear the last line
        let y = VGA_HEIGHT - 1;
        let color_code = self.color_code;

        for x in 0..VGA_WIDTH {
            let pos = x + y * VGA_WIDTH;
            unsafe {
                VGA_BUFFER.add(pos).write_volatile(VgaChar {
                    ascii_char: b' ',
                    color_code,
                });
            }
        }

        self.move_cursor(0, y);
    }

    /// Moves the cursor to the specified position on the screen.
    fn move_cursor(&mut self, x: usize, y: usize) {
        self.cursor_x = x;
        self.cursor_y = y;

        let position = (x + (VGA_WIDTH * y)) as u16; // Calculate the linear position index from the coordinates

        byte_out(CURSOR_PORT_COMMAND, 0x0F); // Set the Cursor Location Low Register
        byte_out(CURSOR_PORT_DATA, (position & 0xFF) as u8); // Write the lower 8 bits of the cursor position
        byte_out(CURSOR_PORT_COMMAND, 0x0E); // Set the Cursor Location High Register
        byte_out(CURSOR_PORT_DATA, ((position >> 8) & 0xFF) as u8); // Write the higher 8 bits of the cursor position
    }
}

// Implementation of the Write trait from the core library
// This allows the use of Rust's formatting macros with the VGA buffer
impl Write for VgaBufferWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
        Ok(())
    }
}

// Custom macros to provide convenient printing functions to the VGA text buffer

/// Prints to the VGA text buffer.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {$crate::drivers::vga::print(format_args!($($arg)*))};
}

/// Prints a line to the VGA text buffer.
#[macro_export]
macro_rules! println {
    () => {$crate::print!("\n")};
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)))
}

/// Clears the screen by writing spaces to every position in the VGA text mode memory.
#[macro_export]
macro_rules! cls {
    () => {
        $crate::drivers::vga::clear_screen()
    };
}

// Helper function to handle formatted printing to the VGA text buffer
#[doc(hidden)]
pub fn print(args: core::fmt::Arguments) {
    unsafe {
        VGA.write_fmt(args).unwrap();
    }
}

// Helper function to clear the screen
#[doc(hidden)]
pub fn clear_screen() {
    unsafe {
        VGA.clear_screen();
    }
}
