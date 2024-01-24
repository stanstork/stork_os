use crate::hardware::port_io::{byte_in, byte_out};

// Constants for VGA memory and settings
const VGA_START: *mut VgaChar = 0xB8000 as *mut VgaChar;
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;
const VGA_SIZE: usize = VGA_WIDTH * VGA_HEIGHT;

// Ports for controlling the VGA cursor
const CURSOR_PORT_COMMAND: u16 = 0x3D4;
const CURSOR_PORT_DATA: u16 = 0x3D5;

const VGA_BUFFER: *mut VgaChar = VGA_START as *mut VgaChar;

// Representation of a VGA character
#[repr(C)]
#[derive(Copy, Clone)]
struct VgaChar {
    ascii_char: u8,
    color_code: ColorCode,
}

// Color codes for VGA text mode
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

// Wrapper for color code (foreground and background)
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub fn new(foreground: Color, background: Color) -> Self {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

pub struct VgaWriter {
    pub cursor: VgaCursor,
}

pub struct VgaCursor {}

impl VgaWriter {
    pub fn new() -> Self {
        Self {
            cursor: VgaCursor::new(),
        }
    }

    /// Clears the VGA text mode screen by writing spaces to every character cell
    pub fn clear_screen(&self) {
        let clear_char = VgaChar {
            ascii_char: b' ',
            color_code: ColorCode::new(Color::Blk, Color::Cyn),
        };

        unsafe {
            for i in 0..VGA_SIZE {
                VGA_BUFFER.add(i).write_volatile(clear_char);
            }
        }
    }

    /// Writes a single byte to the VGA text mode screen at the current cursor position.
    pub fn write_byte(&mut self, byte: u8, color_code: ColorCode) {
        match byte {
            b'\n' => self.write_new_line(),
            b'\r' => self.write_carriage_return(),
            b'\t' => self.write_tab(color_code),
            _ => self.write_char(byte, color_code),
        }
    }

    /// Writes a string to the VGA text mode screen at the current cursor position.
    /// The cursor is advanced after the string is written.
    pub fn write(&mut self, string: &str, color_code: ColorCode) {
        for byte in string.bytes() {
            self.write_byte(byte, color_code);
        }
    }

    fn write_new_line(&mut self) {
        let position = self.cursor.get_position();
        let row = position as usize / VGA_WIDTH;

        if row + 1 >= VGA_HEIGHT {
            self.scroll();
        } else {
            self.cursor.set_position(0, row + 1);
        }
    }

    fn write_carriage_return(&mut self) {
        let position = self.cursor.get_position();
        let row = position / VGA_WIDTH as u16;
        self.cursor.set_position(0, row as usize);
    }

    fn write_tab(&mut self, color_code: ColorCode) {
        for _ in 0..4 {
            self.write_byte(b' ', color_code);
        }
        self.cursor.advance();
    }

    fn write_char(&mut self, byte: u8, color_code: ColorCode) {
        let position = self.cursor.get_position();
        let vga_char = VgaChar {
            ascii_char: byte,
            color_code,
        };

        unsafe {
            VGA_START.add(position as usize).write_volatile(vga_char);
        }
        self.cursor.advance();
    }

    fn scroll(&mut self) {
        // Move all lines up by one
        for row in 1..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                let to = col + (row - 1) * VGA_WIDTH;
                let from = col + row * VGA_WIDTH;

                unsafe {
                    let vga_char = VGA_BUFFER.add(from).read_volatile();
                    VGA_BUFFER.add(to).write_volatile(vga_char);
                }
            }
        }

        // Clear the last line
        let row = VGA_HEIGHT - 1;
        for col in 0..VGA_WIDTH {
            let to = col + row * VGA_WIDTH;
            let vga_char = VgaChar {
                ascii_char: b' ',
                color_code: ColorCode::new(Color::Blk, Color::Cyn),
            };

            unsafe {
                VGA_BUFFER.add(to).write_volatile(vga_char);
            }
        }

        self.cursor.set_position(0, row);
    }
}

impl VgaCursor {
    pub fn new() -> Self {
        Self {}
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
    pub fn set_position(&self, x: usize, y: usize) {
        let mut pos = x + (VGA_WIDTH * y);

        if pos >= VGA_SIZE {
            pos = VGA_SIZE - 1;
        }

        byte_out(CURSOR_PORT_COMMAND, 0x0F);
        byte_out(CURSOR_PORT_DATA, (pos & 0xFF) as u8);
        byte_out(CURSOR_PORT_COMMAND, 0x0E);
        byte_out(CURSOR_PORT_DATA, ((pos >> 8) & 0xFF) as u8);
    }

    /// Retrieves the current cursor position from the VGA hardware.
    /// Returns:
    /// - A 16-bit unsigned integer representing the cursor's position.
    /// This function reads from two VGA ports (CURSOR_PORT_COMMAND and CURSOR_PORT_DATA)
    /// to get the high and low bytes of the cursor's current position in the VGA buffer.
    /// These bytes are combined to form the complete position value.
    pub fn get_position(&self) -> u16 {
        let mut position: u16 = 0;

        byte_out(CURSOR_PORT_COMMAND, 0x0F);
        position |= byte_in(CURSOR_PORT_DATA) as u16;

        byte_out(CURSOR_PORT_COMMAND, 0x0E);
        position |= (byte_in(CURSOR_PORT_DATA) as u16) << 8;

        position
    }

    /// Shows the text cursor on the screen.
    /// This function writes to the VGA cursor control registers to enable the cursor's visibility.
    /// It uses the CURSOR_PORT_COMMAND and CURSOR_PORT_DATA ports to manipulate cursor display settings.
    fn show(&self) {
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
    fn hide(&self) {
        byte_out(CURSOR_PORT_COMMAND, 0x0A);
        byte_out(CURSOR_PORT_DATA, 0x20);
    }

    /// Advances the cursor to the next position on the screen.
    /// The function calculates the new cursor position by incrementing the current position.
    /// If the new position exceeds the bounds of the VGA screen, it's clamped to the maximum value.
    /// The updated position is then written to the VGA cursor position ports.
    pub fn advance(&self) {
        let mut pos = self.get_position();
        pos += 1;

        if pos >= VGA_SIZE as u16 {
            pos = VGA_SIZE as u16 - 1;
        }

        byte_out(CURSOR_PORT_COMMAND, 0x0F);
        byte_out(CURSOR_PORT_DATA, (pos & 0xFF) as u8);

        byte_out(CURSOR_PORT_COMMAND, 0x0E);
        byte_out(CURSOR_PORT_DATA, ((pos >> 8) & 0xFF) as u8);
    }
}
