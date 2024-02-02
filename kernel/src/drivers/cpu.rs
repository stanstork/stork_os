use core::arch::asm;

// Ports for controlling the VGA cursor
pub const CURSOR_PORT_COMMAND: u16 = 0x3D4;
pub const CURSOR_PORT_DATA: u16 = 0x3D5;

/// Reads a byte from a specified hardware port.
pub fn byte_in(port: u16) -> u8 {
    let result: u8;

    unsafe {
        asm!(
            // Inline assembly instruction "in al, dx".
            // "in" is an x86 assembly instruction used for input from an I/O port.
            // "al" is the lower 8 bits of the "ax" register, used here to store the input byte.
            // "dx" is a register where the port number must be placed before executing this instruction.
            "in al, dx",

            // The `out("al") result` tells Rust to put the value from the "al" register into the `result` variable after executing the instruction.
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
pub fn byte_out(port: u16, data: u8) {
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

            // The same options as in `byte_in` are used here.
            options(nomem, nostack, preserves_flags)
        );
    }
}
