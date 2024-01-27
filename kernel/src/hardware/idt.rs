use core::arch::asm;

use super::isr::{int_handler, isr_install};
use core::ptr::addr_of;

// Define the interrupt handler gate descriptor
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    limit_high_flags: u8,
    base_high: u8,
}

// Define the IDT register
#[repr(C, packed)]
pub struct GdtPtr {
    limit: u16, // upper 16 bits of all selector limits
    base: u64,  // address of the first GdtEntry struct
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IdtEntry {
    base_low: u16,  // lower 16 bits of the offset to the interrupt handler
    selector: u16,  // segment selector in GDT or LDT
    zero: u8,       // unused, set to 0
    flags: u8,      // flags, determine what type of interrupt this is
    base_mid: u16,  // next 16 bits of the offset to the interrupt handler
    base_high: u32, // upper 16 bits of the offset to the interrupt handler
    reserved: u32,  // reserved, set to 0
}

#[repr(C, packed)]
pub struct IdtPtr {
    limit: u16, // upper 16 bits of all selector limits
    base: u64,  // address of the first IdtEntry struct
}

const KERNEL_CS: u16 = 0x08;
const INT_ATTR: u8 = 0x8E;

static mut GDT: [GdtEntry; 3] = [
    GdtEntry::new(0, 0, 0),          // Null descriptor
    GdtEntry::new(0, 0xFFFFF, 0x9A), // Code segment descriptor
    GdtEntry::new(0, 0xFFFFF, 0x92), // Data segment descriptor
];

static mut IDT_ENTRIES: [IdtEntry; 256] = [IdtEntry::new(); 256];

impl GdtEntry {
    const fn new(base: u32, limit: u32, access: u8) -> GdtEntry {
        GdtEntry {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (base & 0xFFFF) as u16,
            base_middle: ((base >> 16) & 0xFF) as u8,
            access,
            limit_high_flags: (((limit >> 16) & 0xF) as u8) | 0xC0, // 0xC0 sets the granularity and size flag
            base_high: ((base >> 24) & 0xFF) as u8,
        }
    }
}

impl IdtEntry {
    pub const fn new() -> Self {
        Self {
            base_low: 0,
            selector: 0,
            zero: 0,
            flags: 0,
            base_mid: 0,
            base_high: 0,
            reserved: 0,
        }
    }
}

pub fn set_idt_gate(index: usize, handler_address: u64) {
    unsafe {
        IDT_ENTRIES[index].base_low = (handler_address & 0xFFFF) as u16;
        IDT_ENTRIES[index].base_mid = ((handler_address >> 16) & 0xFFFF) as u16;
        IDT_ENTRIES[index].base_high = ((handler_address >> 32) & 0xFFFFFFFF) as u32;

        IDT_ENTRIES[index].selector = KERNEL_CS;
        IDT_ENTRIES[index].zero = 0;
        IDT_ENTRIES[index].flags = INT_ATTR;
        IDT_ENTRIES[index].reserved = 0;
    }
}

pub fn init_gdt() {
    let gdt_ptr = GdtPtr {
        limit: (unsafe { GDT.len() } * core::mem::size_of::<GdtEntry>() - 1) as u16,
        base: unsafe { GDT.as_ptr() } as u64,
    };

    unsafe {
        asm!("lgdt [{}]", in(reg) &gdt_ptr, options(readonly, nostack));
    }
}

pub fn set_idt() {
    // for i in 32..=255 {
    //     set_idt_gate(i, int_handler as u64);
    // }

    let idt_ptr = IdtPtr {
        limit: (unsafe { IDT_ENTRIES.len() } * core::mem::size_of::<IdtEntry>() - 1) as u16,
        base: unsafe { IDT_ENTRIES.as_ptr() as u64 },
    };

    unsafe {
        asm!("lidt [{}]", in(reg) &idt_ptr, options(readonly, nostack));
    }
}
