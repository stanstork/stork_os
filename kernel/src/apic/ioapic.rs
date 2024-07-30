use alloc::vec::Vec;

use crate::{
    acpi::{
        madt::{InterruptSourceOverride, Madt, MadtEntry},
        rsdp::RSDP_MANAGER,
    },
    memory::{
        self,
        addr::{PhysAddr, VirtAddr},
        paging::{page_table_manager::PageTableManager, table::PageTable, PAGE_TABLE_MANAGER},
    },
    println,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ApicType {
    ProcessorLocalApic = 0x0,
    IOApic = 0x1,
    InterruptSourceOverride = 0x2,
    NonMaskableInterrupts = 0x3,
    LocalApicAddressOverride = 0x4,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C, packed)]
pub struct ApicHeader {
    pub apic_type: ApicType,
    pub length: u8,
}

#[derive(Clone)]
#[repr(C, packed)]
pub struct IoApic {
    apic_header: ApicHeader,
    ioapic_id: u8,
    reserved: u8,
    ioapic_address: u32,
    global_system_interrupt_base: u32,
}

pub const IO_APIC_ID_REG: usize = 0x0;
pub const IO_APIC_VERSION_REG: usize = 0x1;
pub const IO_APIC_REDIRECTION_TABLE: u32 = 0x10;
pub const IO_APIC_INTERRUPT_DISABLE: u32 = 0x00010000;
pub const BASE_IRQ: u32 = 0x20;

impl IoApic {
    pub fn new() -> Self {
        // Get the MADT address from the RSDP manager
        let madt_addr = unsafe { RSDP_MANAGER.sdt_headers.apic.unwrap() };

        // Create a MADT object from the address
        let madt = Madt::from_address(madt_addr);

        // Get the total length of the MADT structure
        let length = madt.header.length as usize;

        // Calculate the starting point of the entries (right after the MADT header)
        let mut start = madt_addr as usize + core::mem::size_of::<Madt>();

        // Calculate the end address of the MADT
        let end = madt_addr as usize + length;

        let mut ioapics = Vec::new();

        // Iterate through the entries
        while start < end {
            // Read the entry at the current start address
            let entry = unsafe { &*(start as *const MadtEntry) };

            // Handle specific entry types
            match entry.apic_type {
                1 => {
                    let ioapic = unsafe { &*(start as *const IoApic) };

                    let ioapic_address = ioapic.ioapic_address;
                    println!("IOAPIC Address: {:#X}", ioapic_address);

                    ioapics.push(ioapic.clone());
                }
                _ => {}
            }

            // Move to the next entry by adding the length of the current entry
            start += entry.length as usize;
        }

        if ioapics.len() == 0 {
            panic!("No IOAPIC found in the MADT");
        }

        if ioapics.len() > 1 {
            panic!("Multiple IOAPICs are not supported");
        }

        ioapics[0].clone()
    }

    pub fn irq_ovrride(&self) {
        let max_redirection_entry = unsafe { self.max_redirection_entry() };
        println!("Max Redirection Entry: {}", max_redirection_entry);

        for i in 0..=max_redirection_entry {
            unsafe {
                self.ioapic_write(
                    IO_APIC_REDIRECTION_TABLE + (2 * i) as u32,
                    IO_APIC_INTERRUPT_DISABLE | (BASE_IRQ + i as u32),
                );
                self.ioapic_write(IO_APIC_REDIRECTION_TABLE + (i * 2 + 1) as u32, 0);
            }
        }
    }

    pub fn init(&self) {
        println!("Initializing IOAPIC");
        unsafe { Self::map_ioapic_memory(self.ioapic_address) };

        self.irq_ovrride();
    }

    pub fn enable_irq(&self, irq: u8) {
        unsafe {
            self.ioapic_write(
                IO_APIC_REDIRECTION_TABLE + (2 * irq as u32) as u32,
                BASE_IRQ + irq as u32,
            );
        }
    }

    unsafe fn ioapic_write(&self, register: u32, value: u32) {
        let reg_address = self.ioapic_address as *mut u32;
        let data_address = (self.ioapic_address + 0x10) as *mut u32;

        // Write the register number to the register select address
        core::ptr::write_volatile(reg_address, register);

        // Write the value to the data register
        core::ptr::write_volatile(data_address, value);
    }

    unsafe fn ioapic_read(&self, register: u32) -> u32 {
        let reg_address = self.ioapic_address as *mut u32;
        let data_address = (self.ioapic_address + 0x10) as *mut u32;

        // Write the register number to the register select address
        core::ptr::write_volatile(reg_address, register);

        // Read the value from the data register
        core::ptr::read_volatile(data_address)
    }

    unsafe fn max_redirection_entry(&self) -> u8 {
        ((self.ioapic_read(1) >> 16) & 0xFF) as u8
    }

    unsafe fn map_ioapic_memory(ioapic_base: u32) {
        let root_page_table = memory::active_level_4_table();
        let mut page_table_manager = PageTableManager::new(root_page_table);
        let mut frame_alloc =
            || PAGE_TABLE_MANAGER.as_mut().unwrap().alloc_zeroed_page().0 as *mut PageTable;

        let virt_addr = VirtAddr(ioapic_base as usize);
        let phys_addr = PhysAddr(ioapic_base as usize);

        page_table_manager.map_memory(virt_addr, phys_addr, &mut frame_alloc, false);
    }
}
