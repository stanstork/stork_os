use crate::hardware::port_io::{byte_in, byte_out};

// Constants for VGA memory and settings
const VGA_START: *mut VgaChar = 0xB8000 as *mut VgaChar; // Start address of VGA text mode memory
const VGA_EXTENT: usize = 80 * 25; // Total number of characters in VGA text mode (80 columns x 25 rows)
const VGA_WIDTH: usize = 80; // Width of the VGA text mode screen in characters

// Ports for controlling the VGA cursor
const CURSOR_PORT_COMMAND: u16 = 0x3D4;
const CURSOR_PORT_DATA: u16 = 0x3D5;

// Enum representing the different color codes for VGA text mode
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

// Representation of a character in VGA text mode
#[repr(C)]
#[derive(Copy, Clone)]
struct VgaChar {
    ascii_char: u8,
    color_code: u8,
}

pub struct VgaWriter {}

impl VgaWriter {
    pub fn new() -> Self {
        Self {}
    }

    /// Clears the VGA text mode screen by writing spaces to every character cell
    pub fn clear_screen(&self) {
        let clear_color = self.get_color_code(Color::Blk, Color::Cyn);
        let clear_char = VgaChar {
            ascii_char: b' ',
            color_code: clear_color,
        };

        unsafe {
            for i in 0..VGA_EXTENT {
                *VGA_START.add(i) = clear_char;
            }
        }
    }

    /// Writes a single byte to the VGA text mode screen at the current cursor position.
    /// `byte` is the ASCII character to write.
    /// `fg_color` is the foreground color of the character.
    /// `bg_color` is the background color of the character.
    pub fn write_byte(&mut self, byte: u8, fg_color: Color, bg_color: Color) {
        let color_code = self.get_color_code(fg_color, bg_color);
        let printed = VgaChar {
            ascii_char: byte,
            color_code,
        };

        let position = self.get_cursor_pos();
        unsafe {
            *VGA_START.add(position as usize) = printed;
        }
    }

    /// Writes a string to the VGA text mode screen at the current cursor position.
    /// Parameters:
    ///  - `string`: is the string to write.
    ///  - `fg_color`: is the foreground color of the string.
    ///  - `bg_color`: is the background color of the string.
    /// The cursor is advanced after the string is written.
    pub fn write(&mut self, string: &str, fg_color: Color, bg_color: Color) {
        for byte in string.bytes() {
            self.write_byte(byte, fg_color, bg_color);
            self.advance_cursor();
        }
    }

    /// ///////////////////////////////////////////////////////////////////////////////////////////////////
    /// VGA Register Ports (0x3D4 and 0x3D5)                                                            ///
    ///                                                                                                 ///
    /// CURSOR_PORT_COMMAND (0x3D4):                                                                    ///        
    ///     This is the I/O port used for selecting the VGA internal register to be accessed.           ///
    ///     It's a common port for various VGA functionalities, not just the cursor.                    ///
    /// CURSOR_PORT_DATA (0x3D5):                                                                       ///
    ///     This port is used to read from or write data to the selected VGA internal register.         ///
    ///                                                                                                 ///
    /// Cursor Location Registers (0x0F and 0x0E)                                                       ///
    ///                                                                                                 ///
    /// 0x0F: This is the code for the Cursor Location Low Register.                                    ///
    ///     When written to CURSOR_PORT_COMMAND, it allows access to the lower 8 bits                   ///
    ///     of the cursor's position in the VGA buffer (the specific horizontal position).              ///
    /// 0x0E: This is the code for the Cursor Location High Register.                                   ///
    ///     It allows access to the higher 8 bits of the cursor position                                ///
    ///     (the specific vertical position).                                                           ///
    ///                                                                                                 ///  
    /// Cursor Start and End Registers (0x0A and 0x0B)                                                  ///
    ///                                                                                                 ///
    /// 0x0A: This is the code for the Cursor Start Register.                                           ///
    ///     It controls where on each character cell the cursor starts displaying.                      ///
    ///     The value written to this register can be used to hide or show the cursor.                  ///
    /// 0x0B: This is the code for the Cursor End Register.                                             ///
    ///     It defines where on each character cell the cursor stops displaying.                        ///
    /// ///////////////////////////////////////////////////////////////////////////////////////////////////

    /// Sets the cursor position on the screen.
    /// Parameters:
    /// - `x`: The horizontal position (column number) where the cursor should be placed.
    /// - `y`: The vertical position (row number) where the cursor should be placed.
    ///
    /// The screen is treated as a grid with coordinates (x, y).
    /// The function calculates the linear position index from these coordinates,
    /// clamps it to ensure it's within screen bounds, and then writes the position
    /// to the VGA cursor position ports.
    pub fn set_cursor_pos(&self, x: u8, y: u8) {
        let mut pos = x as u16 + (VGA_WIDTH as u16 * y as u16);

        if pos >= VGA_EXTENT as u16 {
            pos = VGA_EXTENT as u16 - 1;
        }

        byte_out(CURSOR_PORT_COMMAND, 0x0F);
        byte_out(CURSOR_PORT_DATA, (pos & 0xFF) as u8);
        byte_out(CURSOR_PORT_COMMAND, 0x0E);
        byte_out(CURSOR_PORT_DATA, ((pos >> 8) & 0xFF) as u8);
    }

    /// Gets the color code for a character in VGA text mode.
    /// Parameters:
    /// - `fg_color`: The foreground color of the character.
    /// - `bg_color`: The background color of the character.
    ///
    /// The color code is a single byte with the following format:
    /// - The first 4 bits are the background color.
    /// - The last 4 bits are the foreground color.
    fn get_color_code(&self, fg_color: Color, bg_color: Color) -> u8 {
        ((bg_color as u8) << 4) | ((fg_color as u8) & 0x0F)
    }

    /// Retrieves the current cursor position from the VGA hardware.
    /// Returns:
    /// - A 16-bit unsigned integer representing the cursor's position.
    /// This function reads from two VGA ports (CURSOR_PORT_COMMAND and CURSOR_PORT_DATA)
    /// to get the high and low bytes of the cursor's current position in the VGA buffer.
    /// These bytes are combined to form the complete position value.
    fn get_cursor_pos(&self) -> u16 {
        let mut position = 0;

        byte_out(CURSOR_PORT_COMMAND, 0x0F);
        position |= byte_in(CURSOR_PORT_DATA);

        byte_out(CURSOR_PORT_COMMAND, 0x0E);
        position |= ((byte_in(CURSOR_PORT_DATA) as u16) << 8) as u8;

        position as u16
    }

    /// Shows the text cursor on the screen.
    /// This function writes to the VGA cursor control registers to enable the cursor's visibility.
    /// It uses the CURSOR_PORT_COMMAND and CURSOR_PORT_DATA ports to manipulate cursor display settings.
    fn show_cursor(&self) {
        byte_out(CURSOR_PORT_COMMAND, 0x0A);
        let mut current = byte_in(CURSOR_PORT_DATA);
        byte_out(CURSOR_PORT_DATA, current & 0xC0);

        byte_out(CURSOR_PORT_COMMAND, 0x0B);
        current = byte_in(CURSOR_PORT_DATA);
        byte_out(CURSOR_PORT_DATA, current & 0xE0);
    }

    /// Hides the text cursor from the screen.
    /// This function writes to the VGA cursor control registers to disable the cursor's visibility.
    /// It specifically sets the cursor's start line to a value that makes it invisible on the screen.
    fn hide_cursor(&self) {
        byte_out(CURSOR_PORT_COMMAND, 0x0A);
        byte_out(CURSOR_PORT_DATA, 0x20);
    }

    /// Advances the cursor to the next position on the screen.
    /// The function calculates the new cursor position by incrementing the current position.
    /// If the new position exceeds the bounds of the VGA screen, it's clamped to the maximum value.
    /// The updated position is then written to the VGA cursor position ports.
    fn advance_cursor(&self) {
        let mut pos = self.get_cursor_pos();
        pos += 1;

        if pos >= VGA_EXTENT as u16 {
            pos = VGA_EXTENT as u16 - 1;
        }

        byte_out(CURSOR_PORT_COMMAND, 0x0F);
        byte_out(CURSOR_PORT_DATA, (pos & 0xFF) as u8);

        byte_out(CURSOR_PORT_COMMAND, 0x0E);
        byte_out(CURSOR_PORT_DATA, ((pos >> 8) & 0xFF) as u8);
    }
}
