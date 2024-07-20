use self::heap::Heap;
use super::{
    addr::{PhysAddr, VirtAddr},
    paging::{table::PageTable, PAGE_TABLE_MANAGER},
    physical_page_allocator::PhysicalPageAllocator,
    region::Region,
    PAGE_SIZE,
};
use crate::{memory::addr::ToPhysAddr, println};

pub mod heap;

/// Initializes the heap by allocating and mapping a specified number of pages.
///
/// This function allocates physical pages and maps them to virtual addresses,
/// starting from `start_addr`, to create a heap region. It then adds this region
/// to the heap manager for future allocations.
///
/// # Safety
///
/// This function is unsafe because it directly manipulates memory and relies on
/// the caller to ensure that the `start_addr` and `page_frame_allocator` are valid
/// and that the memory region is not already in use or reserved.
///
/// # Parameters
///
/// - `start_addr`: The starting virtual address where the heap region will begin.
/// - `pages`: The number of pages to allocate and map for the heap.
/// - `page_frame_allocator`: A mutable reference to the physical page allocator.
pub unsafe fn init(
    start_addr: VirtAddr,
    pages: usize,
    page_frame_allocator: &mut PhysicalPageAllocator,
) -> Heap<32> {
    // Calculate the total size of the region to be allocated for the heap
    let region_size = pages * PAGE_SIZE;
    // Map the allocated pages to the starting virtual address
    map_pages(start_addr, pages, page_frame_allocator);
    // Create a new region representing the heap space
    let region = Region::new(start_addr.to_phys_addr(), region_size);

    // Initialize the heap manager and add the region to it
    let mut heap: Heap<32> = Heap::new();
    heap.add_region(region);

    println!("Heap initialized");

    heap
}

// Maps the allocated pages to the starting virtual address.
unsafe fn map_pages(
    mut start_addr: VirtAddr,
    pages: usize,
    page_frame_allocator: &mut PhysicalPageAllocator,
) {
    let mut frame_alloc = || page_frame_allocator.alloc_page().unwrap().0 as *mut PageTable;
    for i in 0..pages {
        let page = frame_alloc();
        if i == 0 {
            println!("Heap start address: {:#x}", page as usize);
        }
        PAGE_TABLE_MANAGER.as_mut().unwrap().map_memory(
            start_addr,
            PhysAddr(page as usize),
            &mut frame_alloc,
            true,
        );
        start_addr += PAGE_SIZE;
    }
}
