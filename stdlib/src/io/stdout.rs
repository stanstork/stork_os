use crate::sys::syscall;
use core::fmt::{self, Write};

use self::syscall::syscall2;

/// Syscall number for `write`
const SYS_WRITE: usize = 2;

/// `Stdout` struct for formatted output.
pub struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe { syscall2(SYS_WRITE, s.as_ptr() as usize, s.len()) };
        Ok(())
    }
}

/// Print formatted output to stdout.
pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}
