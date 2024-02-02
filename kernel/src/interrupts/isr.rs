use super::idt::InterruptDescriptorTable;
use crate::println;
use core::fmt;

/// The InterruptStackFrame struct represents the stack frame that is pushed to the stack when an interrupt occurs.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct InterruptStackFrameValue {
    pub instruction_pointer: u64,
    pub code_segment: u64,
    pub cpu_flags: u64,
    pub stack_pointer: u64,
    pub stack_segment: u64,
}

/// Wrapper struct for the InterruptStackFrameValue struct
#[repr(C)]
pub struct InterruptStackFrame {
    value: InterruptStackFrameValue,
}

// Custom Debug implementation for the InterruptStackFrame struct
impl fmt::Debug for InterruptStackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Helper struct to format the CPU flags as a hexadecimal value
        struct Hex(u64);
        impl fmt::Debug for Hex {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "0x{:x}", self.0)
            }
        }

        // Format the InterruptStackFrame struct
        f.debug_struct("InterruptStackFrame")
            .field("instruction_pointer", &self.value.instruction_pointer)
            .field("code_segment", &self.value.code_segment)
            .field("cpu_flags", &Hex(self.value.cpu_flags))
            .field("stack_pointer", &self.value.stack_pointer)
            .field("stack_segment", &self.value.stack_segment)
            .finish()
    }
}

/// Installs the Interrupt Service Routines (ISRs) for the CPU
pub fn isr_install() {
    let mut idt = InterruptDescriptorTable::new();

    idt.breakpoint.set_handler(breakpoint_handler as u64);
    idt.non_maskable_interrupt
        .set_handler(non_maskable_handler as u64);

    // Set the generic handler for all other interrupts
    for i in 32..256 {
        idt[i].set_handler(isr_handler as u64);
    }

    // Load the IDT to start using the interrupt handlers
    idt.load();
}

/// Generic handler for all interrupts
pub extern "x86-interrupt" fn isr_handler(stack_frame: InterruptStackFrame) {
    println!("Interrupt: Generic handler");
}

/// Handler for the breakpoint interrupt
pub extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("Interrupt: Breakpoint\n {:#?}", stack_frame);
}

/// Handler for the non-maskable interrupt
pub extern "x86-interrupt" fn non_maskable_handler(stack_frame: InterruptStackFrame) {
    println!("Interrupt: Non Maskable\n {:#?}", stack_frame);
}
