use core::arch::asm;

use crate::{apic::APIC, io::pic_end_master};

pub(crate) mod handlers;
pub(crate) mod idt;

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
