use self::{
    addr::PhysAddr, memory_descriptor::EFIMemoryDescriptor,
    physical_page_allocator::PhysicalPageAllocator, region::Region,
};
use crate::{println, structures::BootInfo};

pub(crate) mod addr;
pub(crate) mod bitmap;
pub(crate) mod memory_descriptor;
pub(crate) mod paging;
pub(crate) mod physical_page_allocator;
pub(crate) mod region;

pub const PAGE_SIZE: usize = 4096;
pub const KERNEL_PHYS_START: PhysAddr = 0x100000;

/// Initializes the system's memory management unit.
///
/// This function sets up the physical page frame allocator, reads the EFI memory map,
/// locks the memory pages used by the kernel, and initializes paging.
///
/// # Safety
///
/// This function is unsafe because it performs various low-level memory operations, including
/// writing to the EFI memory map, locking memory pages, and initializing the page table.
///
/// # Arguments
///
/// * `boot_info` - A reference to the boot information provided by the bootloader.
pub unsafe fn init(boot_info: &'static crate::structures::BootInfo) {
    let mut page_frame_allocator = PhysicalPageAllocator::new();

    // Read and process the EFI memory map to understand the available and used memory regions.
    page_frame_allocator.read_efi_memory_map(boot_info);

    println!("Free memory: {} MB", page_frame_allocator.free_memory_mb());

    // Calculate the size and number of pages used by the kernel based on its start and end addresses.
    let kernel_size = boot_info.kernel_end as usize - KERNEL_PHYS_START;
    let kernel_pages = (kernel_size / PAGE_SIZE) + 1;

    // Lock the memory pages occupied by the kernel to prevent their use by other processes.
    page_frame_allocator.lock_pages(KERNEL_PHYS_START, kernel_pages);

    // Initialize paging, setting up the necessary page tables and entries.
    paging::init(boot_info, &mut page_frame_allocator);
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

    Region::new(largest_region_start, largest_region_size)
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
