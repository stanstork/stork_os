use crate::{
    acpi::{
        madt::{ApicHeader, ApicType, Madt, MadtEntry},
        rsdp::RSDP_MANAGER,
    },
    memory::{self},
    println,
};

/// Represents an I/O APIC entry in the MADT.
#[derive(Clone)]
#[repr(C, packed)]
pub struct IoApic {
    apic_header: ApicHeader,           // Common header for all APIC entries
    ioapic_id: u8,                     // I/O APIC ID
    reserved: u8,                      // Reserved, must be zero
    ioapic_address: u32,               // Physical address of the I/O APIC
    global_system_interrupt_base: u32, // Global system interrupt base
}

// Constants for I/O APIC registers and IRQ settings
pub const IO_APIC_VERSION_REG: usize = 0x1;
pub const IO_APIC_REDIRECTION_TABLE: u32 = 0x10;
pub const IO_APIC_INTERRUPT_DISABLE: u32 = 0x00010000;
pub const BASE_IRQ: u32 = 0x20;

impl IoApic {
    /// Creates a new I/O APIC with default values.
    pub const fn default() -> Self {
        Self {
            apic_header: ApicHeader {
                apic_type: ApicType::IOApic,
                length: core::mem::size_of::<IoApic>() as u8,
            },
            ioapic_id: 0,
            reserved: 0,
            ioapic_address: 0,
            global_system_interrupt_base: 0,
        }
    }

    /// Reads the I/O APIC from the MADT.
    /// This function scans the MADT for an I/O APIC entry and returns it.
    pub fn get_from_madt() -> IoApic {
        // Get the MADT from the RSDP manager
        let madt_addr = unsafe { RSDP_MANAGER.sdt_headers.apic.unwrap() };
        let madt = Madt::from_address(madt_addr);
        let length = madt.header.length as usize;

        // Start at the beginning of the MADT entries, skipping the MADT header
        let mut start = madt_addr as usize + core::mem::size_of::<Madt>();
        let end = madt_addr as usize + length;

        // Iterate through the MADT entries to find the I/O APIC
        while start < end {
            let entry = unsafe { &*(start as *const MadtEntry) };

            // Check if the entry is an I/O APIC
            if entry.apic_type == ApicType::IOApic {
                let ioapic = unsafe { &*(start as *const IoApic) };
                return ioapic.clone();
            }

            start += entry.length as usize;
        }

        // Panic if no I/O APIC was found in the MADT
        panic!("No I/O APIC found in the MADT");
    }

    /// Sets up the I/O APIC by mapping its memory and configuring IRQ redirection entries.
    pub fn setup(&self) {
        println!("Setting up I/O APIC");

        // Map the I/O APIC memory
        memory::map_io(self.ioapic_address as u64);

        // Determine the maximum number of IRQ redirection entries
        let max_irq_entries = unsafe { self.max_irq_entries() };

        // Disable all IRQ redirection entries
        for i in 0..=max_irq_entries {
            unsafe {
                self.write(
                    // Calculate the address for the redirection table entry for the current IRQ
                    // The redirection table starts at IO_APIC_REDIRECTION_TABLE (0x10).
                    // Each entry is 2 registers wide (64 bits), so we multiply the index `i` by 2.
                    // For example, the first entry (i = 0) would be at offset 0x10, the second (i = 1) at 0x18, and so on.
                    IO_APIC_REDIRECTION_TABLE + (2 * i) as u32,
                    // Disable the interrupt by setting the interrupt disable bit (IO_APIC_INTERRUPT_DISABLE)
                    // and set the interrupt vector. The vector is calculated as BASE_IRQ (0x20) + `i`,
                    // where `i` is the IRQ number being configured.
                    // This ensures that each IRQ is mapped to a unique vector starting from BASE_IRQ.
                    IO_APIC_INTERRUPT_DISABLE | (BASE_IRQ + i as u32),
                );
                // Write zero to the higher 32 bits of the redirection entry, typically used for destination fields.
                self.write(IO_APIC_REDIRECTION_TABLE + (i * 2 + 1) as u32, 0);
            }
        }
    }

    /// Enables a specific IRQ by updating the I/O APIC's redirection table.
    pub fn enable_irq(&self, irq: u8) {
        unsafe {
            self.write(
                IO_APIC_REDIRECTION_TABLE + (2 * irq as u32) as u32,
                BASE_IRQ + irq as u32,
            );
        }
    }

    /// Writes a value to a specific I/O APIC register.
    unsafe fn write(&self, register: u32, value: u32) {
        let reg_address = self.ioapic_address as *mut u32;
        let data_address = (self.ioapic_address + 0x10) as *mut u32;

        // Write the register number to the register select address
        core::ptr::write_volatile(reg_address, register);

        // Write the value to the data register
        core::ptr::write_volatile(data_address, value);
    }

    /// Reads a value from a specific I/O APIC register.
    unsafe fn read(&self, register: u32) -> u32 {
        let reg_address = self.ioapic_address as *mut u32;
        let data_address = (self.ioapic_address + 0x10) as *mut u32;

        // Write the register number to the register select address
        core::ptr::write_volatile(reg_address, register);

        // Read the value from the data register
        core::ptr::read_volatile(data_address)
    }

    /// Returns the maximum number of IRQ redirection entries supported by the I/O APIC.
    /// This value is extracted from the I/O APIC version register.
    unsafe fn max_irq_entries(&self) -> u8 {
        let version_register = self.read(IO_APIC_VERSION_REG as u32);
        ((version_register >> 16) & 0xFF) as u8
    }
}
