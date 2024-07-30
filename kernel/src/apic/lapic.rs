use crate::{
    cpu::io::{Port, PortIO},
    interrupts::isr::IDT,
    memory::{
        self,
        addr::{PhysAddr, VirtAddr},
        paging::{
            page_table_manager::{self, PageTableManager},
            table::PageTable,
            PAGE_TABLE_MANAGER,
        },
    },
};

pub const LVT_TIMER: u32 = 0x320;
pub const LVT_THERMAL: u32 = 0x330;
pub const LVT_PERFMON: u32 = 0x340;
pub const LVT_LINT0: u32 = 0x350;
pub const LVT_LINT1: u32 = 0x360;
pub const LVT_ERROR: u32 = 0x370;
pub const LVT_EOI: u32 = 0xB0;
pub const LVT_SPV: u32 = 0xF0;
pub const LVT_TPR: u32 = 0x80;

pub const APIC_LVT_MASKED: u32 = 0x10000;

pub struct Lapic {
    pub base: *mut u32,
}

impl Lapic {
    pub const fn new(lapic_base: u64) -> Lapic {
        Lapic {
            base: lapic_base as *mut u32,
        }
    }

    pub unsafe fn enable_apic_mode(lapic_base: u64) {
        IDT.disable_pic_interrupt(0);
        IDT.disable_pic_interrupt(1);

        Port::new(0x22).write_port(0x70); // Select IMCR at 0x70
        Port::new(0x23).write_port(0x01); // Write IMCR, 0x00 connects PIC to LINT0, 0x01 disconnects

        Self::map_lapic_memory(lapic_base);
    }

    pub unsafe fn init(&self) {
        self.init_vector_table();
        self.setup_timer(32, 1000000, 0x3);
        self.write_spurious_interrupt_vector();
        self.eoi();
        self.write_register(LVT_TPR, 0);
    }

    unsafe fn init_vector_table(&self) {
        // Mask all interrupts
        self.write_register(LVT_TIMER, APIC_LVT_MASKED); // Mask the timer interrupt
        self.write_register(LVT_THERMAL, APIC_LVT_MASKED); // Mask the thermal sensor interrupt
        self.write_register(LVT_PERFMON, APIC_LVT_MASKED); // Mask the performance monitoring interrupt
        self.write_register(LVT_LINT0, APIC_LVT_MASKED); // Mask the local interrupt 0
        self.write_register(LVT_LINT1, APIC_LVT_MASKED); // Mask the local interrupt 1
        self.write_register(LVT_ERROR, APIC_LVT_MASKED); // Mask the error interrupt
    }

    unsafe fn setup_timer(&self, vector: u32, initial_count: u32, divide_config: u32) {
        // Timer Divide Configuration Register (Divide by 1)
        self.write_register(0x3E0, divide_config);

        // Timer Initial Count Register (Initial count value)
        self.write_register(0x380, initial_count);

        // Timer LVT Register (Timer mode: periodic, vector number)
        let timer_lvt_value = vector | (1 << 17); // Bit 17 set to 1 for periodic mode
        self.write_register(0x320, timer_lvt_value);
    }

    /// Writes a value to a LAPIC register
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs raw pointer arithmetic and
    /// writes to a memory-mapped IO register.
    unsafe fn write_register(&self, reg: u32, value: u32) {
        // Calculate the register address
        let reg_ptr = self.base.add(reg as usize / 4); // Divide by 4 because the base is a pointer to u32

        // Write the value to the register using volatile to ensure the write is not optimized out
        core::ptr::write_volatile(reg_ptr, value);
    }

    /// Reads a value from a LAPIC register
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs raw pointer arithmetic and
    /// reads from a memory-mapped IO register.
    unsafe fn read_register(&self, reg: u32) -> u32 {
        // Calculate the register address
        let reg_ptr = self.base.add(reg as usize / 4); // Divide by 4 because the base is a pointer to u32

        // Read the value from the register using volatile to ensure the read is not optimized out
        core::ptr::read_volatile(reg_ptr)
    }

    unsafe fn write_spurious_interrupt_vector(&self) {
        let value = self.read_register(LVT_SPV) | 0x100;
        self.write_register(LVT_SPV, value);
    }

    pub unsafe fn eoi(&self) {
        self.write_register(LVT_EOI, 0);
    }

    unsafe fn map_lapic_memory(lapic_base: u64) {
        let root_page_table = memory::active_level_4_table();
        let mut page_table_manager = PageTableManager::new(root_page_table);
        let mut frame_alloc =
            || PAGE_TABLE_MANAGER.as_mut().unwrap().alloc_zeroed_page().0 as *mut PageTable;

        let virt_addr = VirtAddr(lapic_base as usize);
        let phys_addr = PhysAddr(lapic_base as usize);

        page_table_manager.map_memory(virt_addr, phys_addr, &mut frame_alloc, false);
    }
}
