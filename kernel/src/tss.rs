use super::gdt::SegmentAttributes;
use super::gdt::SegmentType;
use crate::gdt::PrivilegeLevel;
use crate::gdt::SegmentSelector;
use crate::{gdt::GDT, println};
use alloc::vec;
use core::arch::asm;
use core::ptr::addr_of;

/// Represents the Task State Segment (TSS).
///
/// The TSS is used to hold information about the state of a task, including
/// stack pointers and I/O map base addresses. It is crucial for handling
/// hardware task switching and interrupt handling in x86_64 architecture.
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct TaskStateSegment {
    _reserved0: u32,
    /// Stack pointers for privilege levels 0-2.
    pub privilege_stack_table: [u64; 3],
    _reserved1: u64,
    /// Stack pointers for handling interrupts.
    pub interrupt_stack_table: [u64; 7],
    _reserved2: u64,
    _reserved3: u16,
    /// Offset to the I/O permission bitmap.
    pub io_bitmap_offset: u16,
    /// I/O permission bitmap.  
    pub io_bitmap: [u8; 8193],
}

impl TaskStateSegment {
    /// Creates a new Task State Segment (TSS).
    ///
    /// # Arguments
    ///
    /// * `kernel_rsp` - The stack pointer for the kernel privilege level.
    ///
    /// # Returns
    ///
    /// A new `TaskStateSegment` instance with the provided kernel stack pointer.
    #[inline]
    pub const fn new(kernel_rsp: u64) -> Self {
        Self {
            _reserved0: 0,
            privilege_stack_table: [kernel_rsp; 3],
            _reserved1: 0,
            interrupt_stack_table: [kernel_rsp; 7],
            _reserved2: 0,
            _reserved3: 0,
            io_bitmap_offset: 112,
            io_bitmap: [0; 8193],
        }
    }
}

/// Represents a TSS descriptor in the GDT.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct TssDescriptor {
    pub length: u16,
    pub base_low: u16,
    pub base_middle: u8,
    pub attributes: SegmentAttributes,
    pub base_high: u8,
    pub base_upper: u32,
    __: u32,
}

impl TssDescriptor {
    /// Creates a null TSS descriptor.
    ///
    /// A null TSS descriptor is used to represent an invalid or unused TSS.
    #[inline]
    pub const fn null() -> Self {
        Self {
            length: 104,
            base_low: 0,
            base_middle: 0,
            attributes: SegmentAttributes::new()
                .with_segment_type(SegmentType::Task)
                .with_long_mode(true),
            base_high: 0,
            base_upper: 0,
            __: 0,
        }
    }
}

/// Loads the Task State Segment (TSS) into the CPU.
///
/// This function initializes the TSS with a kernel stack pointer, updates the
/// GDT entry for the TSS, and loads the TSS into the CPU's TR register. This
/// is necessary for proper task switching and interrupt handling.
///
/// # Safety
/// This function is unsafe because it modifies global state and interacts with
/// CPU registers directly.
pub unsafe fn load_task_state_segment() {
    // Allocate a stack for the kernel
    let kernel_stack = vec![0; 0x14000];

    // Initialize the TSS with the kernel stack pointer
    // The privilege stack table and interrupt stack table are initialized with the kernel stack pointer
    TSS = TaskStateSegment::new(kernel_stack.as_ptr() as u64 + kernel_stack.len() as u64);

    // Calculate the address of the TSS
    let tss_addr = addr_of!(TSS) as *const _ as u64;

    // Update the GDT entry for the TSS
    GDT.tss.base_low = tss_addr as u16;
    GDT.tss.base_middle = (tss_addr >> 16) as u8;
    GDT.tss.attributes = GDT.tss.attributes.with_present(true);
    GDT.tss.base_high = (tss_addr >> 24) as u8;
    GDT.tss.base_upper = (tss_addr >> 32) as u32;

    // Load the TSS into the CPU's TR register
    asm!(
        "ltr ax",
        in("ax") SegmentSelector::new(5, PrivilegeLevel::Kernel).0,
        options(nostack, preserves_flags),
    );

    // Verify that TSS is loaded by reading the TR register
    let tr: u16;
    asm!("str ax", out("ax") tr);
    println!("TR register: {:04x}", tr);
}

/// The global Task State Segment (TSS).
pub static mut TSS: TaskStateSegment = TaskStateSegment::new(0);
