use crate::{
    acpi::{madt::Madt, rsdp::RSDP_MANAGER},
    sync::mutex::SpinMutex,
};
use ioapic::IoApic;
use lapic::Lapic;

mod ioapic;
mod lapic;

/// Represents the APIC system, including both the Local APIC (LAPIC) and I/O APIC.
pub(crate) struct Apic {
    lapic: Lapic,
    ioapic: IoApic,
    is_enabled: bool,
}

/// Global static instance of the APIC, protected by a SpinMutex for safe concurrent access.
pub static mut APIC: SpinMutex<Apic> = SpinMutex::new(Apic {
    lapic: Lapic::new(0), // Default LAPIC initialized with base address 0 (placeholder)
    ioapic: IoApic::default(), // Default I/O APIC (needs to be initialized properly)
    is_enabled: false,    // Indicates whether the APIC system is enabled
});

impl Apic {
    /// Returns whether the APIC system is currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    /// Sends an End-of-Interrupt (EOI) signal to the LAPIC.
    pub fn lapic_eoi(&self) {
        self.lapic.eoi();
    }

    /// Enables a specific IRQ line in the I/O APIC.
    pub fn enable_irq(&self, irq: u8) {
        self.ioapic.enable_irq(irq);
    }
}

/// Initializes and enables the APIC system by configuring the LAPIC and I/O APIC.
pub fn enable_apic_mode() {
    // Retrieve the MADT (Multiple APIC Description Table) address from the RSDP Manager
    let madt_addr = unsafe { RSDP_MANAGER.sdt_headers.apic.unwrap() };
    let madt = Madt::from_address(madt_addr);

    // Initialize the LAPIC with its base address from the MADT
    let lapic = Lapic::new(madt.local_apic_address);
    // Retrieve and configure the I/O APIC based on the MADT entries
    let ioapic = IoApic::get_from_madt();

    // Enable the LAPIC
    lapic.enable();
    // Set up the I/O APIC
    ioapic.setup();

    // Create the APIC system instance and mark it as enabled
    let apic = Apic {
        lapic,
        ioapic,
        is_enabled: true,
    };

    // Store the initialized APIC system in the global APIC static variable
    unsafe {
        APIC = SpinMutex::new(apic);
    }
}
