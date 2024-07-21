use self::{
    addr::{PhysAddr, VirtAddr},
    memory_descriptor::EFIMemoryDescriptor,
    physical_page_allocator::PhysicalPageAllocator,
    region::Region,
};
use crate::{println, structures::BootInfo, ALLOCATOR};
use alloc::boxed::Box;

pub(crate) mod addr;
pub(crate) mod bitmap;
pub(crate) mod global_allocator;
pub(crate) mod heap;
pub(crate) mod memory_descriptor;
pub(crate) mod paging;
pub(crate) mod physical_page_allocator;
pub(crate) mod region;

pub const PAGE_SIZE: usize = 4096; // 4 KB
pub const KERNEL_PHYS_START: PhysAddr = PhysAddr(0x100000); // 1 MB
pub const HEAP_START: VirtAddr = VirtAddr(0x0000100000000000); // 1 TB
pub const HEAP_PAGES: usize = 1024 * 16; // 64 MB

pub static mut PAGE_FRAME_ALLOCATOR: Option<PhysicalPageAllocator> = None;

/// Initializes the system's memory management unit, setting up the allocator and paging.
///
/// This function sets up the physical page frame allocator, reads the EFI memory map,
/// locks the memory pages used by the kernel, initializes paging, and sets up the heap
/// for dynamic memory allocation. After setting up the heap, it initializes the global allocator
/// which allows for dynamic memory allocation throughout the system.
///
/// # Safety
///
/// This function is unsafe because it performs various low-level memory operations, including
/// writing to the EFI memory map, locking memory pages, initializing the page table, and
/// setting up the heap. Incorrect handling of any of these operations can corrupt the system state.
///
/// # Arguments
///
/// * `boot_info` - A reference to the boot information provided by the bootloader. This includes
///   details about the memory map, kernel start and end addresses, and other boot parameters.
pub unsafe fn init(boot_info: &'static crate::structures::BootInfo) {
    let mut page_frame_allocator = PhysicalPageAllocator::new();

    // Read and process the EFI memory map to understand the available and used memory regions.
    page_frame_allocator.read_efi_memory_map(boot_info);

    println!("Free memory: {} MB", page_frame_allocator.free_memory_mb());

    // Calculate the size and number of pages used by the kernel based on its start and end addresses.
    let kernel_size = boot_info.kernel_end as usize - KERNEL_PHYS_START.0;
    let kernel_pages = (kernel_size / PAGE_SIZE) + 1;

    // Lock the memory pages occupied by the kernel to prevent their use by other processes.
    page_frame_allocator.lock_pages(KERNEL_PHYS_START, kernel_pages);

    // Initialize paging, setting up the necessary page tables and entries.
    paging::init(boot_info, &mut page_frame_allocator);

    // Initialize the heap by allocating and mapping a specified number of pages.
    let heap = heap::init(HEAP_START, HEAP_PAGES, &mut page_frame_allocator);

    // Initialize the global allocator with the heap to enable dynamic memory allocations.
    ALLOCATOR.init(heap);

    // Store the physical page frame allocator in a global static variable for future use.
    PAGE_FRAME_ALLOCATOR = Some(page_frame_allocator);

    // Optionally test heap allocation and modification to verify the allocator's functionality.
    test_heap_allocation();

    println!("Memory initialized");
}

/// Calculates the total memory size based on the EFI memory map.
pub fn get_memory_size(boot_info: &'static BootInfo) -> usize {
    let mut total_memory = 0;

    // Iterate over each memory descriptor and sum up the memory sizes.
    iter_and_apply(boot_info, |descriptor| {
        total_memory += descriptor.number_of_pages as usize * PAGE_SIZE;
    });

    total_memory
}

/// Finds the largest usable memory region from the EFI memory map.
pub(super) fn largest_usable_memory_region(boot_info: &'static BootInfo) -> Region {
    let mut largest_region_start = 0usize;
    let mut largest_region_size = 0usize;

    // Iterate over each memory descriptor to find the largest usable region.
    iter_and_apply(boot_info, |descriptor| {
        if descriptor.is_usable() {
            let region_start = descriptor.physical_start as usize;
            let region_size = (descriptor.number_of_pages * PAGE_SIZE as u64) as usize;

            if region_size > largest_region_size {
                largest_region_size = region_size;
                largest_region_start = region_start;
            }
        }
    });

    Region::new(PhysAddr(largest_region_start), largest_region_size)
}

/// Utility function to iterate over the EFI memory map and apply a function to each memory descriptor.
pub(super) fn iter_and_apply<F>(boot_info: &'static BootInfo, mut f: F)
where
    F: FnMut(&EFIMemoryDescriptor),
{
    let total_entries = boot_info.memory_map_size / boot_info.memory_map_descriptor_size;
    let memory_map_start = boot_info.memory_map as *const u8;

    // Iterate over the memory map entries and apply the function `f` to each.
    for i in 0..total_entries {
        let descriptor_addr =
            memory_map_start.wrapping_add(i * boot_info.memory_map_descriptor_size);
        let descriptor = unsafe { &*(descriptor_addr as *const EFIMemoryDescriptor) };
        f(descriptor);
    }
}

// This function demonstrates heap allocation and manipulation using a Box in a `no_std` environment.
// It performs the following steps to test heap allocation:
// 1. Allocates an integer on the heap and initializes it with the value 42.
// 2. Modifies the value on the heap by adding 10 to it.
// 3. Prints the modified value to demonstrate that the heap-allocated value has been successfully modified.
fn test_heap_allocation() {
    // Allocate an integer on the heap, initializing it with the value 42
    let mut v = Box::new(42);

    // Modify the value on the heap by adding 10
    *v += 10;

    // Print the modified value to demonstrate successful heap allocation and modification
    println!("Heap value: {}", *v);
}
