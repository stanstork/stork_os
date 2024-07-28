use crate::{
    acpi::{madt::Madt, rsdp::RSDP_MANAGER},
    cpu::io::{
        PortIO, ICW2_MASTER, ICW2_SLAVE, ICW3_MASTER, ICW3_SLAVE, ICW4_8086, ICW_1,
        PIC_COMMAND_MASTER, PIC_COMMAND_SLAVE, PIC_DATA_MASTER, PIC_DATA_SLAVE,
    },
    memory::{
        addr::{PhysAddr, VirtAddr},
        paging::{
            page_table_manager::PageTableManager, table::PageTable, PAGE_TABLE_MANAGER,
            ROOT_PAGE_TABLE,
        },
    },
    println,
};
use core::arch::asm;
use lapic::Lapic;

mod lapic;
mod regs;

pub static mut LOCAL_APIC: Option<&mut Lapic> = None;

/// Checks if the CPU supports APIC by examining the CPUID feature bits.
fn check_apic() -> bool {
    let eax: u32;
    let edx: u32;
    unsafe {
        asm!(
            "cpuid",
            in("eax") 1,
            lateout("eax") eax,
            lateout("edx") edx,
        );
    }
    // APIC is supported if bit 9 of edx is set.
    (edx & (1 << 9)) != 0
}

fn disable_pic() {
    println!("Disabling PIC...");

    // Send ICW1 (Initialization Command Word 1)
    PIC_COMMAND_MASTER.write_port(ICW_1);
    PIC_COMMAND_SLAVE.write_port(ICW_1);

    // Send ICW2 (Interrupt Vector Offset)
    PIC_DATA_MASTER.write_port(ICW2_MASTER);
    PIC_DATA_MASTER.write_port(ICW2_SLAVE);

    // Send ICW3 (Tell PIC1 about PIC2 at IRQ2 (0000 0100))
    PIC_DATA_MASTER.write_port(ICW3_MASTER);
    PIC_DATA_SLAVE.write_port(ICW3_SLAVE);

    // Send ICW4 (8086/88 (MCS-80/85) mode).
    PIC_DATA_MASTER.write_port(ICW4_8086);
    PIC_DATA_SLAVE.write_port(ICW4_8086);

    // Mask all interrupts.
    PIC_DATA_MASTER.write_port(0xFF);
    PIC_DATA_SLAVE.write_port(0xFF);

    println!("PIC disabled.");
}

fn map_lapic_base(local_apic_address: u64) {
    println!("Mapping local APIC base...");

    let lapic_base = PhysAddr(local_apic_address as usize);
    let lapic_virtual_base = VirtAddr(local_apic_address as usize);

    // Retrieve the root page table pointer
    let root_table = unsafe { ROOT_PAGE_TABLE as *mut PageTable };
    let mut page_table_manager = PageTableManager::new(root_table);

    // Frame allocator closure.
    let mut frame_alloc = || {
        let phys_page = unsafe { PAGE_TABLE_MANAGER.as_ref().unwrap().alloc_zeroed_page().0 };
        phys_page as *mut PageTable
    };

    // Map the LAPIC base address
    unsafe {
        page_table_manager.map_memory(lapic_virtual_base, lapic_base, &mut frame_alloc, false)
    };
}

fn setup_local_apic() {
    let madt = Madt::from_address(unsafe { RSDP_MANAGER.sdt_headers.apic.unwrap() });
    let local_apic_address = madt.local_apic_address as u64;

    map_lapic_base(local_apic_address);

    println!("Setting up local APIC at 0x{:X}...", local_apic_address);

    unsafe {
        LOCAL_APIC = Some(Lapic::new(local_apic_address as usize));

        // Enable the local APIC by setting the SVR (Spurious Interrupt Vector Register) bit 8.
        LOCAL_APIC.as_mut().unwrap().enable();
    }
}

pub fn setup_apic() {
    if !check_apic() {
        println!("APIC is not supported.");
        return;
    }

    disable_pic();
    setup_local_apic();
}
