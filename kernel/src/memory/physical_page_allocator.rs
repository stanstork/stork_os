use super::{
    addr::{PhysAddr, ToPhysAddr},
    bitmap::Bitmap,
    region::Region,
    PAGE_SIZE,
};
use crate::{
    memory::{
        get_memory_size, iter_and_apply, largest_usable_memory_region,
        memory_descriptor::EfiMemoryType, KERNEL_PHYS_START,
    },
    println,
    structures::BootInfo,
};
use core::{alloc::Layout, ptr};

/// PhysicalPageAllocator is responsible for managing physical memory allocation in page frames.
/// It uses a bitmap to keep track of which pages are free, reserved, or in use. This structure
/// is crucial for memory management tasks, such as allocating and deallocating memory pages,
/// and tracking memory usage statistics.
///
/// Fields:
/// - bitmap: A Bitmap struct that represents the usage status of each page frame. A set bit indicates
///   the page is occupied, and a cleared bit indicates the page is free.
/// - free_memory: The total amount of free memory in bytes. This value is updated as pages are allocated or freed.
/// - reserved_memory: The total amount of memory in bytes that has been reserved. Reserved pages are not
///   available for general allocation but are not considered in use yet.
/// - used_memory: The total amount of memory in bytes that is currently in use. This includes memory
///   that has been allocated and is actively being used.
/// - bitmap_index: An index into the bitmap, used to optimize the search for a free page. It indicates
///   the next starting point for scanning the bitmap for a free page.
pub struct PhysicalPageAllocator {
    bitmap: Bitmap,
    free_memory: usize,
    reserved_memory: usize,
    used_memory: usize,
    bitmap_index: usize,
}

impl PhysicalPageAllocator {
    pub const fn new() -> PhysicalPageAllocator {
        PhysicalPageAllocator {
            bitmap: Bitmap::new(ptr::null_mut(), 0),
            free_memory: 0,
            reserved_memory: 0,
            used_memory: 0,
            bitmap_index: 0,
        }
    }

    /// Reads the EFI memory map and initializes memory tracking structures.
    ///
    /// This function performs several key operations to set up memory tracking based on the EFI memory map:
    /// - Identifies the largest usable memory region.
    /// - Calculates the total memory size available.
    /// - Initializes a bitmap for memory management, marking memory regions appropriately.
    /// - Reserves necessary system pages.
    /// - Marks conventional memory regions as usable.
    /// - Locks the bitmap into memory to prevent its alteration.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs operations on raw pointers and manipulates memory directly.
    /// The caller must ensure that `boot_info` points to a valid `BootInfo` structure and that the memory operations
    /// do not lead to data races or invalid memory access.
    ///
    /// # Arguments
    ///
    /// * `boot_info` - A reference to the boot information structure, containing the EFI memory map.
    pub unsafe fn read_efi_memory_map(&mut self, boot_info: &'static BootInfo) {
        // Identify and log the largest usable memory region.
        let largest_region = largest_usable_memory_region(boot_info);
        println!(
            "Largest free memory region addr: 0x{:x}, size: {} Mb",
            largest_region.start().0,
            largest_region.size() / 1024 / 1024
        );

        // Calculate and log the total memory size.
        let memory_size = get_memory_size(boot_info);
        println!("Total memory: {} Mb", memory_size / 1024 / 1024);

        // Initialize the bitmap for memory tracking.
        self.init_bitmap(&largest_region, memory_size);

        self.reserve_pages(PhysAddr(0), memory_size / PAGE_SIZE + 1);

        // Mark usable regions in the memory map.
        self.mark_usable(boot_info);

        // Reserve system pages.
        self.lock_pages(PhysAddr(0), 0x100);
        self.lock_pages(
            KERNEL_PHYS_START,
            (boot_info.kernel_end as usize - KERNEL_PHYS_START.0) / PAGE_SIZE + 1,
        );

        // Lock the bitmap to ensure it remains unchanged.
        self.lock_bitmap();
    }

    /// Allocates a single page of memory using a bitmap to track free and used pages.
    ///
    /// This function iterates over the bitmap starting from the current index (`self.bitmap_index`) to find
    /// a free page. Once a free page is found, it locks the page, updates the bitmap to mark the page as used,
    /// and then returns the physical address of the allocated page.
    ///
    /// # Returns
    ///
    /// An `Option<u64>` representing the physical address of the allocated page. If no free page is available,
    /// it returns `None`.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs direct memory access and manipulation. The caller must ensure
    /// that concurrent access to the bitmap and memory pages is properly synchronized to prevent data races or
    /// undefined behavior.
    pub unsafe fn alloc_page(&mut self) -> Option<PhysAddr> {
        // Iterate over the bitmap to find a free page.
        for i in self.bitmap_index..(self.bitmap.size * 8) {
            // Check if the current bit (page) is set (meaning it's already used).
            if self.bitmap.get(i) {
                continue; // Skip if the page is already used.
            }

            // Lock the page to prevent it from being allocated again.
            self.lock_page(PhysAddr(i * PAGE_SIZE));

            // Update the index to the next bit in the bitmap to optimize subsequent searches.
            self.bitmap_index = i + 1;

            // Return the physical address of the allocated page.
            return Some(PhysAddr(i * PAGE_SIZE));
        }

        // If no free page is found, log a message and return None.
        println!("No free pages available");
        None
    }

    /// Allocates a number of contiguous physical pages based on the given `layout`.
    ///
    /// This method finds a sequence of contiguous free pages sufficient to meet the memory size
    /// specified by `layout`. The pages are locked and marked as used in the bitmap upon allocation.
    ///
    /// # Arguments
    /// * `layout` - A `Layout` instance describing the memory size and alignment requirements.
    ///
    /// # Returns
    /// * `Some(PhysAddr)` where `PhysAddr` is the starting physical address of the allocated pages
    ///   if there are enough contiguous free pages available.
    /// * `None` if there is insufficient contiguous free memory to satisfy the request.
    ///
    /// # Safety
    /// This function is unsafe as it handles raw pointers and performs no bounds checking on memory accesses.
    pub unsafe fn alloc_pages(&mut self, layout: Layout) -> Option<PhysAddr> {
        // Calculate the number of pages required to satisfy the layout.
        let num_pages = (layout.size() + PAGE_SIZE - 1) / PAGE_SIZE;

        // Iterate over the bitmap to find a sequence of contiguous free pages.
        let mut start = None;
        let mut count = 0;

        for i in self.bitmap_index..(self.bitmap.size * 8) {
            if !self.bitmap.get(i) {
                // Check if the current page is free.
                if start.is_none() {
                    start = Some(i);
                }
                count += 1;

                // Check if we have found enough contiguous pages.
                if count == num_pages {
                    // Lock and mark the pages as used.
                    for j in start.unwrap()..(start.unwrap() + num_pages) {
                        self.lock_page(PhysAddr(j * PAGE_SIZE));
                    }

                    // Update the bitmap index for future searches.
                    self.bitmap_index = start.unwrap() + num_pages;

                    // Return the physical address of the allocated pages.
                    return Some(PhysAddr(start.unwrap() * PAGE_SIZE));
                }
            } else {
                start = None;
                count = 0;
            }
        }

        // Log a message and return None if no contiguous pages are available.
        println!("No contiguous pages of required size available");
        None
    }

    /// Locks a range of memory pages to mark them as in use.
    ///
    /// This function iterates over a specified range of memory pages starting from a given physical address
    /// (`start`) and marks each page as locked or in use by calling `lock_page` on each one.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs direct memory operations. The caller must ensure that:
    /// - The range of pages being locked does not overlap with already locked or reserved memory regions.
    /// - The `start` address and subsequent addresses calculated within the range are valid physical memory addresses.
    /// - The function is not called concurrently in a way that could lead to race conditions.
    ///
    /// # Arguments
    ///
    /// * `start` - The physical address of the first page to be locked.
    /// * `size` - The number of consecutive pages to lock, starting from `start`.
    pub unsafe fn lock_pages(&mut self, start: PhysAddr, size: usize) {
        // Iterate over the range of pages and lock each one.
        for i in 0..size {
            // Calculate the physical address of the current page to lock.
            let current_page_address = start.0 + i * PAGE_SIZE;
            // Lock the current page to mark it as in use.
            self.lock_page(PhysAddr(current_page_address));
        }
        println!("Locked {} pages starting from address {:#x}", size, start.0)
    }

    /// Returns the amount of free memory in megabytes.
    pub fn free_memory_mb(&self) -> usize {
        self.free_memory / 1024 / 1024
    }

    // Initializes the bitmap based on the provided memory region and total memory.
    unsafe fn init_bitmap(&mut self, region: &Region, total_memory: usize) {
        let bitmap_size = self.calc_bitmap_size(total_memory);
        self.free_memory = total_memory;
        // Create a new bitmap at the region's starting address with the calculated size.
        self.bitmap = Bitmap::new(region.as_mut_ptr(), bitmap_size);
        // Initialize all bitmap bits to 0 (free).
        for i in 0..bitmap_size {
            self.bitmap.buffer.add(i as usize).write(0);
        }
    }

    // Calculates the required bitmap size based on the total memory size.
    fn calc_bitmap_size(&self, total_memory_size: usize) -> usize {
        (total_memory_size / PAGE_SIZE / 8 + 1) as usize
    }

    // Reserves system pages in the bitmap.
    unsafe fn reserve_sys_pages(&mut self, total_memory_size: usize) {
        // Reserve pages starting from address 0.
        self.reserve_pages(PhysAddr(0), total_memory_size / PAGE_SIZE + 1);
        // Additionally, reserve a fixed block of pages (256 pages).
        self.reserve_pages(PhysAddr(0), 0x100);
    }

    // Marks pages as usable based on the EFI memory map.
    unsafe fn mark_usable(&mut self, boot_info: &'static BootInfo) {
        // Iterate over the memory map and unreserve pages marked as conventional memory.
        iter_and_apply(boot_info, |descriptor| {
            if descriptor.r#type == EfiMemoryType::EfiConventionalMemory as u32 {
                self.unreserve_pages(
                    descriptor.physical_start.to_phys_addr(),
                    descriptor.number_of_pages as usize,
                );
            }
        });
    }

    // Locks the bitmap's memory pages.
    unsafe fn lock_bitmap(&mut self) {
        // Lock the pages occupied by the bitmap itself.
        let bitmap_size = self.calc_bitmap_size(self.free_memory);
        self.lock_pages(
            self.bitmap.buffer.to_phys_addr(),
            bitmap_size / PAGE_SIZE + 1,
        );
    }

    // Reserves a range of pages starting from a given address.
    unsafe fn reserve_pages(&mut self, start: PhysAddr, size: usize) {
        // Iterate over the range and reserve each page.
        for i in 0..size {
            self.reserve_page(start + i * PAGE_SIZE);
        }
    }

    // Unreserves a range of pages starting from a given address.
    unsafe fn unreserve_pages(&mut self, start: PhysAddr, size: usize) {
        // Iterate over the range and unreserve each page.
        for i in 0..size {
            self.unreserve_page(start + i * PAGE_SIZE);
        }
    }

    // Reserves a single page at the specified address.
    unsafe fn reserve_page(&mut self, addr: PhysAddr) {
        let index = addr.0 / PAGE_SIZE;
        if !self.bitmap.get(index) {
            self.bitmap.set(index, true);
            // Update memory counters.
            self.free_memory -= PAGE_SIZE;
            self.reserved_memory += PAGE_SIZE;
        }
    }

    // Unreserves a single page at the specified address.
    unsafe fn unreserve_page(&mut self, addr: PhysAddr) {
        let index = addr.0 / PAGE_SIZE;
        if self.bitmap.get(index) {
            self.bitmap.set(index, false);
            // Update memory counters and the bitmap index.
            self.free_memory += PAGE_SIZE;
            self.reserved_memory -= PAGE_SIZE;
            if self.bitmap_index > index {
                self.bitmap_index = index;
            }
        }
    }

    // Locks a single page at the specified address.
    unsafe fn lock_page(&mut self, addr: PhysAddr) {
        let index = addr.0 / PAGE_SIZE;
        if !self.bitmap.get(index) {
            self.bitmap.set(index, true);
            // Update memory counters.
            self.free_memory -= PAGE_SIZE;
            self.used_memory += PAGE_SIZE;
        }
    }
}
