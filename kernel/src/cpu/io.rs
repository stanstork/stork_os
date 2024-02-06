use core::arch::asm;

// Ports for PIC command and data registers.
pub const PIC1_DATA: Port = Port::new(0x21);
pub const PIC2_DATA: Port = Port::new(0xA1);
pub const PIC1_COMMAND: Port = Port::new(0x20);
pub const PIC2_COMMAND: Port = Port::new(0xA0);

// Initialization command words for PIC.
pub const ICW1_INIT: u8 = 0x10;
pub const ICW1_ICW4: u8 = 0x01;
pub const ICW4_8086: u8 = 0x01;

/// Reads a byte from a specified hardware port.
pub fn inb(port: u16) -> u8 {
    let result: u8;

    unsafe {
        asm!(
            // Inline assembly instruction "in al, dx".
            // "in" is an x86 assembly instruction used for input from an I/O port.
            // "al" is the lower 8 bits of the "ax" register, used here to store the input byte.
            // "dx" is a register where the port number must be placed before executing this instruction.
            "in al, dx",

            // The `out("al") result` tells Rust to pbyte_outut the value from the "al" register into the `result` variable after executing the instruction.
            out("al") result,

            // The `in("dx") port` tells Rust to use the value of `port` as the input for the "dx" register.
            in("dx") port,

            // Options for the inline assembly:
            // `nomem` - Indicates that the assembly code does not perform any memory reads or writes.
            // `nostack` - Indicates that the assembly does not use the stack.
            // `preserves_flags` - Indicates that the assembly does not affect the CPU's flags.
            options(nomem, nostack, preserves_flags)
        );
    }

    // Return the byte that was read from the specified port.
    result
}

/// Writes a byte to a specified hardware port.
/// This is used for sending data directly to hardware devices.
pub fn outb(port: u16, data: u8) {
    // As with `byte_in`, we are dealing with low-level hardware access, so we use an unsafe block.
    unsafe {
        asm!(
            // Inline assembly instruction "out dx, al".
            // "out" is an x86 assembly instruction used for output to an I/O port.
            // "dx" is the register that should contain the port number to which we want to send data.
            // "al" is used to supply the byte of data to be sent to the port.
            "out dx, al",

            // The `in("dx") port` operand tells Rust to load the port number into the "dx" register.
            in("dx") port,

            // The `in("al") data` operand tells Rust to load the byte of data into the "al" register.
            in("al") data,

            options(nostack)
        );
    }
}

/// Waits for I/O operations to complete.
pub fn io_wait() {
    Port::new(0x80).write_port(0);
}

pub fn pic_end_master() {
    PIC1_COMMAND.write_port(0x20);
}

pub fn pic_end_slave() {
    PIC2_COMMAND.write_port(0x20);
    PIC1_COMMAND.write_port(0x20);
}

/// Trait defining basic I/O port operations.
pub trait PortIO {
    /// Reads a byte from the port.
    fn read_port(&self) -> u8;

    /// Writes a byte to the port.
    fn write_port(&self, data: u8);
}

/// Represents a hardware I/O port.
pub struct Port {
    port: u16,
}

impl Port {
    /// Creates a new Port instance with the specified port number.
    pub const fn new(port: u16) -> Port {
        Port { port }
    }
}

impl PortIO for Port {
    /// Reads a byte from the port.
    fn read_port(&self) -> u8 {
        inb(self.port)
    }

    /// Writes a byte to the port.
    fn write_port(&self, data: u8) {
        outb(self.port, data)
    }
}

impl PortIO for u16 {
    /// Reads a byte from a specified hardware port.
    fn read_port(&self) -> u8 {
        inb(*self)
    }

    /// Writes a byte to a specified hardware port.
    fn write_port(&self, data: u8) {
        outb(*self, data)
    }
}
