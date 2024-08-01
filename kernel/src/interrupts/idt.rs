use crate::{
    cpu::io::{PortIO, PIC_COMMAND_MASTER, PIC_DATA_MASTER},
    structures::DescriptorTablePointer,
};
use core::{
    arch::asm,
    mem::size_of,
    ops::{Index, IndexMut},
};

/// An entry in the Interrupt Descriptor Table (IDT).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct IdtEntry {
    pub(crate) offset0: u16,   // offset bits 0..15
    pub(crate) selector: u16,  // a code segment selector in GDT or LDT
    pub(crate) ist: u8,        // bits 0..2 hold Interrupt Stack Table offset, rest of bits zero.
    pub(crate) types_attr: u8, // type and attributes
    pub(crate) offset1: u16,   // offset bits 16..31
    pub(crate) offset2: u32,   // offset bits 32..63
    pub(crate) ignore: u32,    // reserved
}

impl IdtEntry {
    /// Create a new IDT entry with default values.
    pub const fn default() -> IdtEntry {
        IdtEntry {
            offset0: 0,
            selector: 0,
            ist: 0,
            types_attr: 0,
            offset1: 0,
            offset2: 0,
            ignore: 0,
        }
    }

    /// Set the offset of the IDT entry.
    pub fn set_offset(&mut self, offset: u64) {
        self.offset0 = (offset & 0x000000000000FFFF) as u16;
        self.offset1 = ((offset & 0x00000000FFFF0000) >> 16) as u16;
        self.offset2 = ((offset & 0xFFFFFFFF00000000) >> 32) as u32;
    }

    /// Set the gate of the IDT entry.
    pub fn set_gate(&mut self, handler: u64, types_attr: u8, selector: u16) {
        self.set_offset(handler);
        self.selector = selector;
        self.types_attr = types_attr;
    }
}

/// The Interrupt Descriptor Table (IDT).
#[derive(Clone)]
#[repr(C)]
#[repr(align(16))]
pub struct InterruptDescriptorTable {
    /// Divide by zero exception: Triggered when an integer division operation results in a quotient of zero.
    pub div_by_zero: IdtEntry,
    /// Debug exception: Occurs during program execution under a debugger, allowing for program interruption and inspection.
    pub debug: IdtEntry,
    /// Non-maskable interrupt: Raised by hardware signaling an interrupt that cannot be ignored, often for critical hardware errors.
    pub non_maskable_interrupt: IdtEntry,
    /// Breakpoint exception: Triggered by an INT3 instruction, commonly used by debuggers to temporarily halt program execution.
    pub breakpoint: IdtEntry,
    /// Overflow exception: Signaled when the result of a signed integer operation exceeds the representable range.
    pub overflow: IdtEntry,
    /// Bound range exceeded exception: Occurs when an array index is outside the bounds specified by the BOUND instruction.
    pub bound_range_exceeded: IdtEntry,
    /// Invalid opcode exception: Indicates that the executed instruction is not recognized or supported by the current CPU mode.
    pub invalid_opcode: IdtEntry,
    /// Device not available exception: Triggered when a required hardware device is not ready or available.
    pub device_not_available: IdtEntry,
    /// Double fault exception: Occurs when an exception arises while the CPU is trying to call an exception handler.
    pub double_fault: IdtEntry,
    /// Coprocessor segment overrun exception: A legacy exception related to floating-point operations, rarely used in modern CPUs.
    pub coprocessor_segment_overrun: IdtEntry,
    /// Invalid TSS exception: Triggered when the CPU encounters an invalid Task State Segment during task switching.
    pub invalid_tss: IdtEntry,
    /// Segment not present exception: Raised when the CPU attempts to use a segment that is marked as not present in the segment descriptor.
    pub segment_not_present: IdtEntry,
    /// Stack-segment fault exception: Similar to a segment-not-present exception, but specifically for stack segment errors.
    pub stack_segment_fault: IdtEntry,
    /// General protection fault: A broad exception for various protection violations, often related to memory access errors.
    pub general_protection_fault: IdtEntry,
    /// Page fault: Occurs when a program tries to access a region of memory that is not currently mapped to physical memory or lacks the required permissions.
    pub page_fault: IdtEntry,
    /// Unassigned: A placeholder for unassigned interrupts.
    pub unassigned: IdtEntry,
    /// x87 floating-point exception: Triggered by errors in legacy x87 floating-point operations.
    pub x87_floating_point_exception: IdtEntry,
    /// Alignment check exception: Raised when unaligned memory access is performed, and alignment checking is enabled.
    pub alignment_check: IdtEntry,
    /// Machine check exception: Signals severe hardware errors, such as overheating or hardware malfunctions.
    pub machine_check: IdtEntry,
    /// SIMD floating-point exception: Related to errors in SIMD (Single Instruction, Multiple Data) floating-point operations.
    pub simd_floating_point_exception: IdtEntry,
    /// Reserved: Space reserved for future use or for specific system use.
    reserved: [IdtEntry; 12],
    /// The rest of the IDT entries: Additional entries for user-defined or system-specific interrupts.
    entries: [IdtEntry; 256 - 32],
}

impl InterruptDescriptorTable {
    /// Create a new IDT with default values.
    pub const fn new() -> InterruptDescriptorTable {
        InterruptDescriptorTable {
            div_by_zero: IdtEntry::default(),
            debug: IdtEntry::default(),
            non_maskable_interrupt: IdtEntry::default(),
            breakpoint: IdtEntry::default(),
            overflow: IdtEntry::default(),
            bound_range_exceeded: IdtEntry::default(),
            invalid_opcode: IdtEntry::default(),
            device_not_available: IdtEntry::default(),
            double_fault: IdtEntry::default(),
            coprocessor_segment_overrun: IdtEntry::default(),
            invalid_tss: IdtEntry::default(),
            segment_not_present: IdtEntry::default(),
            stack_segment_fault: IdtEntry::default(),
            general_protection_fault: IdtEntry::default(),
            page_fault: IdtEntry::default(),
            unassigned: IdtEntry::default(),
            x87_floating_point_exception: IdtEntry::default(),
            alignment_check: IdtEntry::default(),
            machine_check: IdtEntry::default(),
            simd_floating_point_exception: IdtEntry::default(),
            reserved: [IdtEntry::default(); 12],
            entries: [IdtEntry::default(); 256 - 32],
        }
    }

    /// Returns a pointer to the IDT for use with the `lidt` instruction.
    pub fn get_pointer(&self) -> DescriptorTablePointer {
        DescriptorTablePointer {
            limit: size_of::<Self>() as u16 - 1,
            base: self as *const _ as u64,
        }
    }

    /// Load the IDT by setting the IDTR register with the address of the IDT.
    pub unsafe fn load(&self) {
        let idt_ptr = self.get_pointer();
        asm!("lidt [{}]", in(reg) &idt_ptr, options(readonly, nostack));
    }

    pub fn disable_pic_interrupt(&self, int_no: usize) {
        let mask = 1 << int_no;
        let current_mask = PIC_DATA_MASTER.read_port();
        PIC_DATA_MASTER.write_port(current_mask | mask);
    }
}

// Implement indexing for the IDT to allow for easy access to IDT entries by index.
impl Index<usize> for InterruptDescriptorTable {
    type Output = IdtEntry;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.div_by_zero,
            1 => &self.debug,
            2 => &self.non_maskable_interrupt,
            3 => &self.breakpoint,
            4 => &self.overflow,
            5 => &self.bound_range_exceeded,
            6 => &self.invalid_opcode,
            7 => &self.device_not_available,
            8 => &self.double_fault,
            9 => &self.coprocessor_segment_overrun,
            10 => &self.invalid_tss,
            11 => &self.segment_not_present,
            12 => &self.stack_segment_fault,
            13 => &self.general_protection_fault,
            14 => &self.page_fault,
            15 => &self.unassigned,
            16 => &self.x87_floating_point_exception,
            17 => &self.alignment_check,
            18 => &self.machine_check,
            19 => &self.simd_floating_point_exception,
            20..=31 => &self.reserved[index - 20], // todo: panic on reserved
            _ => &self.entries[index - 32],
        }
    }
}

// Implement mutable indexing for the IDT to allow for easy access to IDT entries by index.
impl IndexMut<usize> for InterruptDescriptorTable {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.div_by_zero,
            1 => &mut self.debug,
            2 => &mut self.non_maskable_interrupt,
            3 => &mut self.breakpoint,
            4 => &mut self.overflow,
            5 => &mut self.bound_range_exceeded,
            6 => &mut self.invalid_opcode,
            7 => &mut self.device_not_available,
            8 => &mut self.double_fault,
            9 => &mut self.coprocessor_segment_overrun,
            10 => &mut self.invalid_tss,
            11 => &mut self.segment_not_present,
            12 => &mut self.stack_segment_fault,
            13 => &mut self.general_protection_fault,
            14 => &mut self.page_fault,
            15 => &mut self.unassigned,
            16 => &mut self.x87_floating_point_exception,
            17 => &mut self.alignment_check,
            18 => &mut self.machine_check,
            19 => &mut self.simd_floating_point_exception,
            20..=31 => &mut self.reserved[index - 20], // todo: panic on reserved
            _ => &mut self.entries[index - 32],
        }
    }
}
