use super::{idt::InterruptDescriptorTable, timer::init_timer};
use crate::{
    cpu::io::{
        io_wait, PortIO, ICW1_ICW4, ICW1_INIT, ICW4_8086, PIC1_COMMAND, PIC1_DATA, PIC2_COMMAND,
        PIC2_DATA,
    },
    drivers::keyboard::init_keyboard,
    println,
};

// Constants for kernel code segment and IDT entry count.
pub const KERNEL_CS: u16 = 0x08;
pub const IDT_ENTRIES: usize = 256;

/// The InterruptStackFrame struct represents the stack frame that is pushed to the stack when an interrupt occurs.
#[repr(C, packed)]
pub struct InterruptStackFrame {
    value: InterruptStackFrameValue,
}

/// Structure representing the stack frame values.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct InterruptStackFrameValue {
    instruction_pointer: u64,
    code_segment: u64,
    cpu_flags: u64,
    stack_pointer: u64,
    stack_segment: u64,
}

/// The Interrupt Descriptor Table (IDT).
pub static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

/// Initialize the Interrupt Descriptor Table (IDT) and the Programmable Interrupt Controller (PIC).
pub fn isr_install() {
    println!("Initializing IDT and PIC");

    unsafe {
        // Set specific interrupt handlers.
        IDT.page_fault
            .set_gate(page_fault_handler as u64, 0x8E, KERNEL_CS);
        IDT.double_fault
            .set_gate(double_fault_handler as u64, 0x8E, KERNEL_CS);
        IDT.general_protection_fault
            .set_gate(gpf_fault_handler as u64, 0x8E, KERNEL_CS);
        IDT.breakpoint
            .set_gate(breakpoint_handler as u64, 0x8E, KERNEL_CS);

        // Set all other entries to the default handler
        for i in 32..IDT_ENTRIES {
            IDT[i].set_gate(default_handler as u64, 0x8E, KERNEL_CS);
        }

        // Load the IDT
        IDT.load();
        println!("IDT loaded successfully");

        // Remap the PIC
        remap_pic();
        println!("PIC remapped successfully");

        // Initialize the timer
        init_timer(50);
        println!("Timer initialized with frequency: 50 Hz");

        // Initialize the keyboard
        init_keyboard();
    }
}

/// Remaps the PIC to avoid conflicts with CPU exceptions.
pub fn remap_pic() {
    // Save current masks.
    let pic1 = PIC1_DATA.read_port();
    let pic2 = PIC2_DATA.read_port();

    // Initialize PICs in cascade mode.
    PIC1_COMMAND.write_port(ICW1_INIT | ICW1_ICW4);
    io_wait();
    PIC2_COMMAND.write_port(ICW1_INIT | ICW1_ICW4);
    io_wait();

    // Set vector offsets.
    PIC1_DATA.write_port(0x20); // ICW2: Master PIC vector offset
    io_wait();
    PIC2_DATA.write_port(0x28); // ICW2: Slave PIC vector offset
    io_wait();

    // Configure PIC cascading.
    PIC1_DATA.write_port(4); // ICW3: tell Master PIC that there is a slave PIC at IRQ2 (0000 0100)
    io_wait();
    PIC2_DATA.write_port(2); // ICW3: tell Slave PIC its cascade identity (0000 0010)
    io_wait();

    // Set PICs to 8086 mode.
    PIC1_DATA.write_port(ICW4_8086);
    io_wait();
    PIC2_DATA.write_port(ICW4_8086);
    io_wait();

    // Restore saved masks.
    PIC1_DATA.write_port(pic1);
    PIC2_DATA.write_port(pic2);
}

// Various interrupt handlers follow:

/// Handler for breakpoint exceptions.
pub extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("Interrupt: Breakpoint");
}

/// Handler for page fault exceptions.
pub extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame) {
    println!("Interrupt: Page Fault");
    loop {}
}

/// Handler for double fault exceptions.
pub extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    println!("Interrupt: Double Fault");
    loop {}
}

/// Handler for general protection fault exceptions.
pub extern "x86-interrupt" fn gpf_fault_handler(stack_frame: InterruptStackFrame) {
    println!("Interrupt: General Protection fault");
    loop {}
}

/// Default handler for all other interrupts.
pub extern "x86-interrupt" fn default_handler(stack_frame: InterruptStackFrame) {
    println!("Interrupt: Default handler");
}
