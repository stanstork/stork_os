use crate::cpu::gdt::PrivilegeLevel;
use crate::cpu::gdt::SegmentSelector;
use crate::{cpu::gdt::GDT, println};
use alloc::vec;
use core::{arch::asm, cell::SyncUnsafeCell};

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct TaskStateSegment {
    _reserved0: u32,
    pub privilege_stack_table: [u64; 3],
    _reserved1: u64,
    pub interrupt_stack_table: [u64; 7],
    _reserved2: u64,
    _reserved3: u16,
    pub io_bitmap_offset: u16,
    pub io_bitmap: [u8; 8193],
}

impl TaskStateSegment {
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

pub static mut TSS: SyncUnsafeCell<TaskStateSegment> =
    SyncUnsafeCell::new(TaskStateSegment::new(0));

pub unsafe fn load_task_state_segment() {
    let kernel_stack = vec![0; 0x14000];
    let gdt = &mut *GDT.get();

    (*TSS.get()) = TaskStateSegment::new(kernel_stack.as_ptr() as u64 + kernel_stack.len() as u64);
    let tss_addr = TSS.get() as u64;
    gdt.tss.base_low = tss_addr as u16;
    gdt.tss.base_middle = (tss_addr >> 16) as u8;
    gdt.tss.attributes = gdt.tss.attributes.with_present(true);
    gdt.tss.base_high = (tss_addr >> 24) as u8;
    gdt.tss.base_upper = (tss_addr >> 32) as u32;

    asm!(
        "ltr ax",
        in("ax") SegmentSelector::new(5, PrivilegeLevel::Supervisor).0,
        options(nostack, preserves_flags),
    );

    // Verify that TSS is loaded
    let tr: u16;
    asm!("str ax", out("ax") tr);
    println!("TR register: {:04x}", tr);
}
