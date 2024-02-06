use crate::{
    cpu::io::{pic_end_master, Port, PortIO},
    interrupts::isr::{IDT, KERNEL_CS},
    print, println,
};

// Ports and Commands for Keyboard Encoder and Controller
const KYBRD_ENC_CMD_SET_LED: u8 = 0xED; // Command to set the keyboard LEDs (Num Lock, Caps Lock, Scroll Lock)
const KYBRD_ENC_CMD_REG: Port = Port::new(0x60); // Port for sending commands to the keyboard encoder

const KYBRD_CTRL_STATS_REG: Port = Port::new(0x64); // Port for accessing the keyboard controller's status register

// Masks for Keyboard Controller Status Register
const KYBRD_CTRL_STATS_MASK_OUT_BUF: u8 = 1; // Mask for checking the output buffer status (1 = full, data can be read)
const KYBRD_CTRL_STATS_MASK_IN_BUF: u8 = 2; // Mask for checking the input buffer status (1 = full, data can't be written)

// Keyboard Interrupt and Data
const KEYBOARD_INTERRUPT_VECTOR: u8 = 0x21; // The interrupt vector (IRQ) number used by the keyboard
const KEYBOARD_DATA_PORT: Port = Port::new(0x60); // Port for receiving data from the keyboard

// Keyboard Buffer
const MAX_KEYB_BUFFER_SIZE: usize = 255; // Maximum size of the keyboard buffer

/// Represents the key codes for keyboard keys.
pub enum KeyCode {
    KeyReserved = 0,
    KeyEsc = 1,
    Key1 = 2,
    Key2 = 3,
    Key3 = 4,
    Key4 = 5,
    Key5 = 6,
    Key6 = 7,
    Key7 = 8,
    Key8 = 9,
    Key9 = 10,
    Key0 = 11,
    KeyMinus = 12,
    KeyEqual = 13,
    KeyBackspace = 14,
    KeyTab = 15,
    KeyQ = 16,
    KeyW = 17,
    KeyE = 18,
    KeyR = 19,
    KeyT = 20,
    KeyY = 21,
    KeyU = 22,
    KeyI = 23,
    KeyO = 24,
    KeyP = 25,
    KeyLeftBrace = 26,
    KeyRightBrace = 27,
    KeyEnter = 28,
    KeyLeftCtrl = 29,
    KeyA = 30,
    KeyS = 31,
    KeyD = 32,
    KeyF = 33,
    KeyG = 34,
    KeyH = 35,
    KeyJ = 36,
    KeyK = 37,
    KeyL = 38,
    KeySemicolon = 39,
    KeyApostrophe = 40,
    KeyGrave = 41,
    KeyLeftShift = 42,
    KeyBackslash = 43,
    KeyZ = 44,
    KeyX = 45,
    KeyC = 46,
    KeyV = 47,
    KeyB = 48,
    KeyN = 49,
    KeyM = 50,
    KeyComma = 51,
    KeyDot = 52,
    KeySlash = 53,
    KeyRightShift = 54,
    KeyKpAsterisk = 55,
    KeyLeftAlt = 56,
    KeySpace = 57,
    KeyCapsLock = 58,
    KeyF1 = 59,
    KeyF2 = 60,
    KeyF3 = 61,
    KeyF4 = 62,
    KeyF5 = 63,
    KeyF6 = 64,
    KeyF7 = 65,
    KeyF8 = 66,
    KeyF9 = 67,
    KeyF10 = 68,
    KeyNumLock = 69,
    KeyScrollLock = 70,
    KeyKp7 = 71,
    KeyKp8 = 72,
    KeyKp9 = 73,
    KeyKpMinus = 74,
    KeyKp4 = 75,
    KeyKp5 = 76,
    KeyKp6 = 77,
    KeyKpPlus = 78,
    KeyKp1 = 79,
    KeyKp2 = 80,
    KeyKp3 = 81,
    KeyKp0 = 82,
    KeyKpDot = 83,
}

impl KeyCode {
    /// Converts a scan code to a key code.
    pub fn from_index(code: u8) -> KeyCode {
        match code {
            0 => KeyCode::KeyReserved,
            1 => KeyCode::KeyEsc,
            2 => KeyCode::Key1,
            3 => KeyCode::Key2,
            4 => KeyCode::Key3,
            5 => KeyCode::Key4,
            6 => KeyCode::Key5,
            7 => KeyCode::Key6,
            8 => KeyCode::Key7,
            9 => KeyCode::Key8,
            10 => KeyCode::Key9,
            11 => KeyCode::Key0,
            12 => KeyCode::KeyMinus,
            13 => KeyCode::KeyEqual,
            14 => KeyCode::KeyBackspace,
            15 => KeyCode::KeyTab,
            16 => KeyCode::KeyQ,
            17 => KeyCode::KeyW,
            18 => KeyCode::KeyE,
            19 => KeyCode::KeyR,
            20 => KeyCode::KeyT,
            21 => KeyCode::KeyY,
            22 => KeyCode::KeyU,
            23 => KeyCode::KeyI,
            24 => KeyCode::KeyO,
            25 => KeyCode::KeyP,
            26 => KeyCode::KeyLeftBrace,
            27 => KeyCode::KeyRightBrace,
            28 => KeyCode::KeyEnter,
            29 => KeyCode::KeyLeftCtrl,
            30 => KeyCode::KeyA,
            31 => KeyCode::KeyS,
            32 => KeyCode::KeyD,
            33 => KeyCode::KeyF,
            34 => KeyCode::KeyG,
            35 => KeyCode::KeyH,
            36 => KeyCode::KeyJ,
            37 => KeyCode::KeyK,
            38 => KeyCode::KeyL,
            39 => KeyCode::KeySemicolon,
            40 => KeyCode::KeyApostrophe,
            41 => KeyCode::KeyGrave,
            42 => KeyCode::KeyLeftShift,
            43 => KeyCode::KeyBackslash,
            44 => KeyCode::KeyZ,
            45 => KeyCode::KeyX,
            46 => KeyCode::KeyC,
            47 => KeyCode::KeyV,
            48 => KeyCode::KeyB,
            49 => KeyCode::KeyN,
            50 => KeyCode::KeyM,
            51 => KeyCode::KeyComma,
            52 => KeyCode::KeyDot,
            53 => KeyCode::KeySlash,
            54 => KeyCode::KeyRightShift,
            55 => KeyCode::KeyKpAsterisk,
            56 => KeyCode::KeyLeftAlt,
            57 => KeyCode::KeySpace,
            58 => KeyCode::KeyCapsLock,
            59 => KeyCode::KeyF1,
            60 => KeyCode::KeyF2,
            61 => KeyCode::KeyF3,
            62 => KeyCode::KeyF4,
            63 => KeyCode::KeyF5,
            64 => KeyCode::KeyF6,
            65 => KeyCode::KeyF7,
            66 => KeyCode::KeyF8,
            67 => KeyCode::KeyF9,
            68 => KeyCode::KeyF10,
            69 => KeyCode::KeyNumLock,
            70 => KeyCode::KeyScrollLock,
            71 => KeyCode::KeyKp7,
            72 => KeyCode::KeyKp8,
            73 => KeyCode::KeyKp9,
            74 => KeyCode::KeyKpMinus,
            75 => KeyCode::KeyKp4,
            76 => KeyCode::KeyKp5,
            77 => KeyCode::KeyKp6,
            78 => KeyCode::KeyKpPlus,
            79 => KeyCode::KeyKp1,
            80 => KeyCode::KeyKp2,
            81 => KeyCode::KeyKp3,
            82 => KeyCode::KeyKp0,
            83 => KeyCode::KeyKpDot,
            _ => KeyCode::KeyReserved,
        }
    }

    /// Converts a key code to an ASCII character.
    /// Returns '\0' if the key code does not correspond to an ASCII character.
    pub fn to_ascii(&self) -> char {
        match self {
            KeyCode::Key1 => '1',
            KeyCode::Key2 => '2',
            KeyCode::Key3 => '3',
            KeyCode::Key4 => '4',
            KeyCode::Key5 => '5',
            KeyCode::Key6 => '6',
            KeyCode::Key7 => '7',
            KeyCode::Key8 => '8',
            KeyCode::Key9 => '9',
            KeyCode::Key0 => '0',
            KeyCode::KeyMinus => '-',
            KeyCode::KeyEqual => '=',
            KeyCode::KeyTab => '\t',
            KeyCode::KeyQ => 'q',
            KeyCode::KeyW => 'w',
            KeyCode::KeyE => 'e',
            KeyCode::KeyR => 'r',
            KeyCode::KeyT => 't',
            KeyCode::KeyY => 'y',
            KeyCode::KeyU => 'u',
            KeyCode::KeyI => 'i',
            KeyCode::KeyO => 'o',
            KeyCode::KeyP => 'p',
            KeyCode::KeyLeftBrace => '[',
            KeyCode::KeyRightBrace => ']',
            KeyCode::KeyA => 'a',
            KeyCode::KeyS => 's',
            KeyCode::KeyD => 'd',
            KeyCode::KeyF => 'f',
            KeyCode::KeyG => 'g',
            KeyCode::KeyH => 'h',
            KeyCode::KeyJ => 'j',
            KeyCode::KeyK => 'k',
            KeyCode::KeyL => 'l',
            KeyCode::KeySemicolon => ';',
            KeyCode::KeyApostrophe => '\'',
            KeyCode::KeyGrave => '`',
            KeyCode::KeyBackslash => '\\',
            KeyCode::KeyZ => 'z',
            KeyCode::KeyX => 'x',
            KeyCode::KeyC => 'c',
            KeyCode::KeyV => 'v',
            KeyCode::KeyB => 'b',
            KeyCode::KeyN => 'n',
            KeyCode::KeyM => 'm',
            KeyCode::KeyComma => ',',
            KeyCode::KeyDot => '.',
            KeyCode::KeySlash => '/',
            KeyCode::KeyKpAsterisk => '*',
            KeyCode::KeySpace => ' ',
            KeyCode::KeyKp7 => '7',
            KeyCode::KeyKp8 => '8',
            KeyCode::KeyKp9 => '9',
            KeyCode::KeyKpMinus => '-',
            KeyCode::KeyKp4 => '4',
            KeyCode::KeyKp5 => '5',
            KeyCode::KeyKp6 => '6',
            KeyCode::KeyKpPlus => '+',
            KeyCode::KeyKp1 => '1',
            KeyCode::KeyKp2 => '2',
            KeyCode::KeyKp3 => '3',
            KeyCode::KeyKp0 => '0',
            KeyCode::KeyKpDot => '.',
            KeyCode::KeyEnter => '\n',
            _ => '\0',
        }
    }
}

/// Represents a key event.
/// It contains the scan code of the key and the state of the modifier keys (Shift, Caps Lock, Ctrl, Alt).
#[derive(Clone, Copy)]
pub struct KeyEvent {
    scan_code: u8,
    shift: bool,
    caps_lock: bool,
    ctrl: bool,
    alt: bool,
}

impl KeyEvent {
    /// Creates a new KeyEvent struct with a scan code of 0 and all modifier keys set to false.
    pub const fn new() -> KeyEvent {
        KeyEvent {
            scan_code: 0,
            shift: false,
            caps_lock: false,
            ctrl: false,
            alt: false,
        }
    }

    /// Converts the scan code to an ASCII character.
    /// Returns '\0' if the scan code does not correspond to an ASCII character.
    pub fn to_ascii(&self) -> char {
        let code = KeyCode::from_index(self.scan_code).to_ascii() as u8;

        if code == '\n' as u8 {
            // New line
            '\n'
        } else if code >= 0x20 && code <= 0x7E {
            // ASCII printable characters

            match code {
                b'a'..=b'z' => {
                    // Convert to uppercase if Shift is pressed or Caps Lock is active
                    if self.shift || self.caps_lock {
                        (code - 32) as char
                    } else {
                        code as char
                    }
                }
                b'0'..=b'9' if self.shift => match code {
                    b'0' => ')',
                    b'1' => '!',
                    b'2' => '@',
                    b'3' => '#',
                    b'4' => '$',
                    b'5' => '%',
                    b'6' => '^',
                    b'7' => '&',
                    b'8' => '*',
                    b'9' => '(',
                    _ => '\0',
                },
                b'`' | b'-' | b'=' | b'[' | b']' | b'\\' | b';' | b'\'' | b',' | b'.' | b'/'
                    if self.shift =>
                {
                    match code {
                        b'`' => '~',
                        b'-' => '_',
                        b'=' => '+',
                        b'[' => '{',
                        b']' => '}',
                        b'\\' => '|',
                        b';' => ':',
                        b'\'' => '"',
                        b',' => '<',
                        b'.' => '>',
                        b'/' => '?',
                        _ => '\0',
                    }
                }
                _ => code as char, // Return the character as-is
            }
        } else {
            '\0' // Non-printable character
        }
    }
}

/// Represents the state of the keyboard.
pub enum State {
    Normal,
    Prefix,
}

/// Represents a keyboard with its state and buffer.
pub struct Keyboard {
    buffer: [KeyEvent; MAX_KEYB_BUFFER_SIZE], // Buffer to store key events
    buffer_index: usize,                      // Current position in the buffer
    current_state: State,                     // Current state of the keyboard
    shift: bool,                              // Shift key state
    caps_lock: bool,                          // Caps Lock key state
    ctrl: bool,                               // Control key state
    alt: bool,                                // Alt key state
}

impl Keyboard {
    /// Creates a new Keyboard instance.
    pub const fn new() -> Keyboard {
        Keyboard {
            buffer: [KeyEvent::new(); MAX_KEYB_BUFFER_SIZE],
            buffer_index: 0,
            current_state: State::Normal,
            shift: false,
            caps_lock: false,
            ctrl: false,
            alt: false,
        }
    }

    /// Pushes a KeyEvent into the buffer and updates the buffer index.
    /// This method also handles wrapping around the buffer when it's full.
    pub fn push(&mut self, scan_code: u8) {
        self.buffer[self.buffer_index] = KeyEvent {
            scan_code,
            shift: self.shift,
            caps_lock: self.caps_lock,
            ctrl: self.ctrl,
            alt: self.alt,
        };

        // Optionally, print the ASCII character for debugging.
        print!("{}", self.buffer[self.buffer_index].to_ascii());

        // Update buffer index, wrapping around if necessary.
        self.buffer_index = (self.buffer_index + 1) % MAX_KEYB_BUFFER_SIZE;
    }

    /// Reads the current status from the keyboard controller.
    pub fn read_status(&self) -> u8 {
        KYBRD_CTRL_STATS_REG.read_port()
    }

    /// Reads a scan code from the keyboard data port.
    pub fn read(&mut self) -> u8 {
        KEYBOARD_DATA_PORT.read_port()
    }

    /// Sets the LEDs for Num Lock, Caps Lock, and Scroll Lock.
    pub fn set_leds(&self, num: bool, caps: bool, scroll: bool) {
        let mut data = 0;
        data = data | if scroll { 1 } else { 0 };
        data = data | if num { 2 } else { 0 };
        data = data | if caps { 4 } else { 0 };

        self.send_command(KYBRD_ENC_CMD_SET_LED);
        self.send_command(data);
    }

    /// Sends a command to the keyboard encoder.
    fn send_command(&self, cmd: u8) {
        // Wait until the input buffer is clear.
        while (self.read_status() & KYBRD_CTRL_STATS_MASK_IN_BUF) != 0 {}
        KYBRD_ENC_CMD_REG.write_port(cmd);
    }
}

// Global keyboard instance
pub static mut KEYBOARD: Keyboard = Keyboard::new();

/// Initializes the keyboard.
pub fn init_keyboard() {
    println!("Initializing keyboard");
    unsafe {
        // Set the keyboard interrupt handler in the IDT
        IDT[KEYBOARD_INTERRUPT_VECTOR as usize].set_gate(
            keyboard_irq_handler as u64,
            0x8E, // Present, Ring 0, 32-bit interrupt gate
            KERNEL_CS,
        );
    }
}

/// Interrupt handler for keyboard IRQs.
///
/// This function is called by the CPU when a keyboard interrupt occurs.
/// It reads the scan code from the keyboard, updates the state of the keyboard,
/// and adds the key event to the keyboard's buffer.
unsafe extern "x86-interrupt" fn keyboard_irq_handler() {
    let kybrd_status = KEYBOARD.read_status();

    // Check if the keyboard's output buffer is full
    if (kybrd_status & KYBRD_CTRL_STATS_MASK_OUT_BUF) != 0 {
        let mut scan_code = KEYBOARD.read();
        pic_end_master(); // Send EOI signal to the PIC.

        // Check for prefix scan codes (used for special keys)
        if scan_code == 0xE0 || scan_code == 0xE1 {
            KEYBOARD.current_state = State::Prefix;
            return;
        }

        let key_released = (scan_code & 0x80) != 0;
        if key_released {
            scan_code &= 0x7F; // Clear the highest bit to get the actual scan code.
        }

        // Update the state of modifier keys
        let key = KeyCode::from_index(scan_code);
        match key {
            KeyCode::KeyLeftCtrl => {
                KEYBOARD.ctrl = !key_released;
            }
            KeyCode::KeyLeftShift | KeyCode::KeyRightShift => {
                KEYBOARD.shift = !key_released;
            }
            KeyCode::KeyLeftAlt => {
                KEYBOARD.alt = !key_released;
            }
            KeyCode::KeyCapsLock if !key_released => {
                KEYBOARD.caps_lock = !KEYBOARD.caps_lock;
                KEYBOARD.set_leds(false, KEYBOARD.caps_lock, false);
            }
            _ => {}
        }

        // Add key event to buffer if the key is pressed
        if !key_released {
            KEYBOARD.push(scan_code);
        }

        KEYBOARD.current_state = State::Normal;
    } else {
        pic_end_master(); // Send EOI even if no key was pressed
    }
}
