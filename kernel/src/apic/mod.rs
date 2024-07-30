use ioapic::IoApic;
use lapic::Lapic;

use crate::{
    acpi::{madt::Madt, rsdp::RSDP_MANAGER},
    println,
};

mod ioapic;
mod lapic;

pub(crate) struct Apic {
    pub lapic: Lapic,
    pub ioapic: IoApic,
}

pub static mut APIC: Option<Apic> = None;

impl Apic {
    pub fn init() -> Self {
        let madt_addr = unsafe { RSDP_MANAGER.sdt_headers.apic.unwrap() };
        let madt = Madt::from_address(madt_addr);
        let local_apic_addr = madt.local_apic_address as u64;

        println!("Local APIC Address: {:#X}", local_apic_addr);

        unsafe { Lapic::enable_apic_mode(local_apic_addr) };

        let lapic = Lapic::new(local_apic_addr);

        unsafe { lapic.init() };

        let ioapic = IoApic::new();
        ioapic.init();

        Apic { lapic, ioapic }
    }
}
