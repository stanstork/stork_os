use crate::{
    boot::BootInfo,
    memory::{
        addr::{PhysAddr, VirtAddr},
        get_memory_size,
        paging::{manager::PageTableManager, table::PageTable},
        KERNEL_VIRT_START, PAGE_SIZE,
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

    // Identity mapping ensures the kernel can access physical memory directly during early initialization.
    // This is crucial before transitioning to higher-half memory, as the kernel may still access
    // physical addresses directly (e.g., for device initialization or debugging).
    map_kernel(
        VirtAddr(0),
        PhysAddr(0),
        total_memory,
        &mut pt_manager,
        &mut frame_alloc,
    );

    // Remap the framebuffer memory.
    remap_frame_buffer(boot_info, &mut pt_manager, &mut frame_alloc);

    // Update the CR3 register to use the new page table.
    Cr3::write(pml4 as u64);

    // Mapping the kernel to the higher half separates kernel space from user space.
    // This provides better isolation, security, and compatibility with modern operating systems.
    // After the transition, all kernel code and data will be accessed using higher-half virtual addresses.
    map_kernel(
        KERNEL_VIRT_START,
        PhysAddr(0),
        boot_info.kernel_end as usize,
        &mut pt_manager,
        &mut frame_alloc,
    );

    // Store the page table manager in a global static variable.
    unsafe {
        PAGE_TABLE_MANAGER = Some(pt_manager);
        ROOT_PAGE_TABLE = pml4 as usize;
    }

    println!("Page table initialized");
}

// Maps the kernel memory to the specified physical address range.
unsafe fn map_kernel<F: FnMut() -> *mut PageTable>(
    virt_addr_start: VirtAddr,
    phys_addr_start: PhysAddr,
    size: usize,
    pt_manager: &mut PageTableManager,
    frame_alloc: &mut F,
) {
    for i in (0..size).step_by(PAGE_SIZE) {
        pt_manager.map_memory(virt_addr_start + i, phys_addr_start + i, frame_alloc, false);
    }
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
