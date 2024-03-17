use super::isr::{InterruptStackFrame, IDT, KERNEL_CS};
use crate::{
    cpu::io::{outb, pic_end_master},
    print,
};

/// Represents a system timer.
pub struct Timer {
    /// The number of ticks since the timer was initialized.
    pub tick: u32,
}

/// The global timer instance.
pub static mut TIMER: Timer = Timer { tick: 0 };

/// Interrupt handler for the system timer.
///
/// This function is called on each timer interrupt.
/// It increments the `tick` count of the system timer.
/// After handling the interrupt, it sends the End of Interrupt (EOI) signal.
pub extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        TIMER.tick += 1;
    }
    // Print a dot for each timer tick (for debugging).
    // print!(".");

    // Send EOI signal to the PIC.
    pic_end_master();
}

/// Initializes the system timer to a given frequency.
///
/// The frequency determines how often the timer interrupt is triggered.
///
/// # Arguments
///
/// * `freq` - The frequency at which the timer should tick.
pub fn init_timer(freq: u32) {
    unsafe {
        // Set the timer interrupt handler in the IDT.
        IDT[32].set_gate(timer_interrupt_handler as u64, 0x8E, KERNEL_CS);

        // Calculate the divisor for the given frequency.
        let divisor = 1193180 / freq; // The PIT's input frequency is 1.193180 MHz.
        let low = (divisor & 0xFF) as u8; // Get the low byte of the divisor.
        let high = ((divisor >> 8) & 0xFF) as u8; // Get the high byte of the divisor.

        // Configure the PIT to operate in square wave mode.
        outb(0x43, 0x36); // Command port: channel 0, access mode lobyte/hibyte, square wave generator.
        outb(0x40, low); // Channel 0 data port (low byte of divisor).
        outb(0x40, high); // Channel 0 data port (high byte of divisor).
    }
}
