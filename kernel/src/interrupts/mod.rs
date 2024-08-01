use crate::{apic::APIC, cpu::io::pic_end_master};
use core::arch::asm;

pub(crate) mod idt;
pub(crate) mod isr;
pub(crate) mod timer;

pub fn disable_interrupts() {
    unsafe {
        asm!("cli");
    }
}

pub fn enable_interrupts() {
    unsafe {
        asm!("sti");
    }
}

pub fn no_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    disable_interrupts();
    let result = f();
    enable_interrupts();
    result
}

pub fn end_of_interrupt() {
    unsafe {
        if APIC.lock().is_enabled() {
            APIC.lock().lapic_eoi();
        } else {
            pic_end_master();
        }
    }
}
