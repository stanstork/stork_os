use super::isr::{InterruptStackFrame, IDT, KERNEL_CS};
use crate::{
    cpu::io::{outb, pic_end_master},
    tasks::schedule,
};

const CLOCK_TICK_RATE: u32 = 1193182u32; // The PIT's input frequency is 1.193182 MHz.
const TIMER_TICK_RATE: u32 = 100; // The timer interrupt frequency is 100 Hz.

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
    schedule();
}

pub fn init_timer() {
    unsafe {
        // Set the timer interrupt handler in the IDT.
        IDT[32].set_gate(timer_interrupt_handler as u64, 0x8E, KERNEL_CS);

        // Calculate the latch value for the given frequency.
        let latch = ((CLOCK_TICK_RATE + TIMER_TICK_RATE / 2) / TIMER_TICK_RATE) as u16;

        // Set the PIT to the desired frequency.
        outb(0x43, 0x34);
        outb(0x40, (latch & 0xFF) as u8);
        outb(0x40, (latch > 8) as u8);
    }
}
