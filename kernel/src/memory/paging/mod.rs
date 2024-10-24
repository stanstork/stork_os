use crate::{
    boot::BootInfo,
    memory::{
        addr::{PhysAddr, VirtAddr},
        get_memory_size,
        paging::{manager::PageTableManager, table::PageTable},
        PAGE_SIZE,
    },
    println,
    registers::cr3::Cr3,
};

use super::allocation::physical::PhysicalPageAllocator;

pub(crate) mod manager;
pub(crate) mod table;

pub static mut PAGE_TABLE_MANAGER: Option<PageTableManager> = None;
pub static mut ROOT_PAGE_TABLE: usize = 0;

/// Initializes the page table manager with the boot information and a page frame allocator.
///
/// This function sets up the initial PML4 table, maps all system memory, and ensures that the framebuffer
/// memory is also correctly mapped. Finally, it updates the CR3 register to use the new page table.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Performs raw pointer dereferences and writes to potentially arbitrary memory locations.
/// - Modifies global processor state (CR3 register).
/// - Assumes the boot_info structure provides valid and correct information.
///
/// # Arguments
///
/// * `boot_info` - A reference to the boot information provided by the bootloader.
/// * `page_frame_alloc` - A reference to a physical page frame allocator.
pub unsafe fn init(boot_info: &'static BootInfo, page_frame_alloc: &mut PhysicalPageAllocator) {
    // Allocate and zero-initialize a new PML4 table.
    let pml4 = page_frame_alloc.alloc_page().unwrap().0 as *mut PageTable;
    (pml4 as *mut u8).write_bytes(0, PAGE_SIZE);

    println!("PML4 allocated at: {:#x}", pml4 as usize);

    let mut pt_manager = PageTableManager::new(pml4);
    let total_memory = get_memory_size(boot_info);

    let mut frame_alloc = || page_frame_alloc.alloc_page().unwrap().0 as *mut PageTable;
    for i in (0..total_memory).step_by(PAGE_SIZE) {
        unsafe { pt_manager.map_memory(VirtAddr(i), PhysAddr(i), &mut frame_alloc, false) };
    }

    // Remap the framebuffer memory.
    remap_frame_buffer(boot_info, &mut pt_manager, &mut frame_alloc);

    // Update the CR3 register to use the new page table.
    Cr3::write(pml4 as u64);

    // Store the page table manager in a global static variable.
    unsafe {
        PAGE_TABLE_MANAGER = Some(pt_manager);
        ROOT_PAGE_TABLE = pml4 as usize;
    }

    println!("Page table initialized");
}

// Remaps the framebuffer memory to ensure it is accessible.
unsafe fn remap_frame_buffer<F: FnMut() -> *mut PageTable>(
    boot_info: &'static BootInfo,
    pt_manager: &mut PageTableManager,
    frame_alloc: &mut F,
) {
    let fb_start = boot_info.framebuffer.pointer.as_ptr() as usize;
    let fb_size =
        (boot_info.framebuffer.height * boot_info.framebuffer.width * 4) as usize + PAGE_SIZE;
    // page_frame_alloc.lock_pages(fb_start, fb_size / PAGE_SIZE + 1);
    for i in (fb_start..fb_start + fb_size).step_by(PAGE_SIZE) {
        pt_manager.map_memory(VirtAddr(i), PhysAddr(i), frame_alloc, true);
    }
}
