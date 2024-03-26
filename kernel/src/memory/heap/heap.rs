use crate::{data_types::linked_list::LinkedList, memory::region::Region};
use core::{
    alloc::Layout,
    cmp::min,
    mem::{size_of, size_of_val},
    ptr::NonNull,
};

/// A simple heap allocator implementing a buddy system allocation strategy, with a fixed number of size classes.
///
/// The buddy system allocator divides memory into partitions to minimize fragmentation and simplify merging of adjacent free blocks.
/// This implementation uses a series of size classes, each represented by a linked list, to manage memory allocations efficiently.
///
/// Generic parameter `N` specifies the number of size classes.
pub struct Heap<const N: usize> {
    /// An array of linked lists, where each linked list corresponds to a size class in the buddy system.
    /// The index in the array represents the size class, and each linked list manages
    /// free memory blocks of a specific size range. Adjacent free blocks can be merged to form a larger block.
    free_list: [LinkedList; N],

    /// The total size of user-allocated memory.
    /// This value represents the sum of the sizes of all blocks allocated by the user,
    /// excluding the overhead introduced by the memory management structures.
    user_size: usize,

    /// The total size of memory that has been allocated from the system.
    /// This includes the memory allocated by the user as well as any additional overhead
    /// introduced by the buddy system's memory management.
    allocated: usize,

    /// The total size of the heap managed by this allocator.
    /// This includes all memory under the allocator's management, encompassing both
    /// allocated blocks and free blocks available for future allocations.
    total: usize,
}

impl<const N: usize> Heap<N> {
    pub const fn new() -> Self {
        Heap {
            free_list: [LinkedList::new(); N],
            user_size: 0,
            allocated: 0,
            total: 0,
        }
    }

    /// Adds a memory region to the heap, aligning and subdividing it into blocks managed by the buddy system.
    ///
    /// This function takes a memory region and subdivides it into blocks that are powers of two in size,
    /// which are then added to the corresponding size classes in the buddy system allocator.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it assumes that the provided memory region is valid and
    /// does not overlap with any existing regions in the heap. The caller must ensure these conditions are met.
    ///
    /// # Arguments
    ///
    /// * `region` - A memory region to be added to the heap.
    pub unsafe fn add_region(&mut self, region: Region) {
        // Align the start address to the size of usize to ensure proper alignment for allocations.
        let start = Self::align_up(region.start(), size_of::<usize>());

        // Align the end address down to ensure it's a multiple of the size of usize.
        // This step avoids partial block allocations at the end of the region.
        let end = Self::align_down(region.start() + region.size(), size_of::<usize>());

        assert!(start <= end, "Invalid heap region");

        let mut current = start;

        // Iterate over the region, subdividing it into blocks.
        while current + size_of::<usize>() <= end {
            // Calculate the largest power-of-two block size that can fit in the remaining region
            let block_size = Self::calculate_block_size(current, end);

            // Add the block to the appropriate size class in the free list
            self.free_list[block_size.trailing_zeros() as usize].push(current as *mut usize);

            current += block_size;
            self.total += block_size;
        }
    }

    /// Allocates memory based on the specified layout.
    ///
    /// This method finds a suitable memory block, potentially splitting larger blocks to fit the request.
    /// It uses a buddy system approach to find the best-fit block, minimizing fragmentation.
    ///
    /// # Arguments
    ///
    /// * `layout` - The memory layout specifying the size and alignment requirements for the allocation.
    ///
    /// # Returns
    ///
    /// * `Result<NonNull<u8>, ()>` - Returns a pointer to the allocated memory block on success,
    ///   or an error if allocation fails.
    pub fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, ()> {
        // Calculate the adjusted size, rounding up to the nearest power of two and considering alignment requirements.
        // This adjustment ensures that the allocated memory block meets the requested layout constraints.
        let size = layout
            .size()
            .next_power_of_two()
            .max(layout.align().max(size_of::<usize>()));

        // Determine the size class index based on the trailing zeros of the adjusted size,
        // which corresponds to the block size in the buddy system.
        let class = size.trailing_zeros() as usize;

        // Iterate through the free list, starting from the determined class, looking for a suitable block.
        for i in class..N {
            // Check if there's an available block in the current class.
            if !self.free_list[i].is_empty() {
                // Attempt to split larger blocks to match the requested size if necessary.
                if self.split_blocks(i, class).is_err() {
                    return Err(());
                }

                // Perform the allocation from the now appropriately sized block.
                return self.allocate_from_class(class, size, layout.size());
            }
        }

        // If no suitable block was found, return an error
        Err(())
    }

    /// Deallocates a memory block, potentially merging it with its buddy block.
    ///
    /// This method frees the memory at the specified pointer and tries to merge the freed block
    /// with its buddy block if the buddy is also free. This process helps reduce fragmentation
    /// in the buddy system allocator.
    ///
    /// # Arguments
    ///
    /// * `ptr` - A non-null pointer to the memory block to be deallocated.
    /// * `layout` - The memory layout that was used for the allocation. This includes the size
    ///   and alignment of the memory block.
    pub fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        // Calculate the adjusted size, which is the next power of two greater than
        // the maximum of the layout's size and alignment, and the size of usize.
        // This ensures the block size is consistent with how it was allocated.
        let size = layout
            .size()
            .next_power_of_two()
            .max(layout.align().max(size_of::<usize>()));

        // Determine the class based on the block size. This is where the block will be
        // returned to in the free list.
        let class = size.trailing_zeros() as usize;

        // Add the block back to the appropriate free list, making it available for future allocations.
        self.free_list[class].push(ptr.as_ptr() as *mut usize);

        // Attempt to merge the block with its buddy. If the buddy is also free, the two blocks
        // will be combined into a larger block and moved to a higher size class, reducing fragmentation.
        self.merge_buddies(ptr.as_ptr() as usize, class);
    }

    // Aligns the given address up to the nearest multiple of the alignment
    fn align_up(address: usize, alignment: usize) -> usize {
        (address + alignment - 1) & !(alignment - 1)
    }

    // Aligns the given address down to the nearest multiple of the alignment
    fn align_down(address: usize, alignment: usize) -> usize {
        address & !(alignment - 1)
    }

    // Calculate the maximum block size for the current position in the region
    fn calculate_block_size(current: usize, end: usize) -> usize {
        let low_bit = current & (!current + 1);
        min(low_bit, Self::prev_power_of_two(end - current))
    }

    // Compute the previous power of two for a given number
    fn prev_power_of_two(x: usize) -> usize {
        1 << (size_of_val(&x) * 8 - x.leading_zeros() as usize - 1)
    }

    /// Splits blocks in the free list to achieve a block of the desired size class.
    ///
    /// This function iterates through the free list, starting from a larger block size (start_class)
    /// and progressively splits blocks until it reaches the target size class (target_class).
    ///
    /// # Arguments
    ///
    /// * `start_class` - The size class from which to start splitting blocks.
    /// * `target_class` - The target size class we want to achieve through splitting.
    ///
    /// # Returns
    ///
    /// * `Result<(), ()>` - Returns `Ok(())` if the splitting was successful, or `Err(())` if it failed.
    fn split_blocks(&mut self, start_class: usize, target_class: usize) -> Result<(), ()> {
        // Iterate from the larger block size class down to the target block size class.
        for j in (target_class + 1..=start_class).rev() {
            // Attempt to pop a block from the current size class.
            if let Some(block) = self.free_list[j].pop() {
                // Calculate the size of the smaller block to be created by splitting.
                let half_block_size = 1 << (j - 1);
                // Calculate the address of the new block created as a result of the split.
                let new_block = (block as usize + half_block_size) as *mut usize;

                // Push the two newly created blocks back into the free list of the smaller size class.
                self.free_list[j - 1].push(new_block);
                self.free_list[j - 1].push(block);
            } else {
                // If no block is available to split, return an error indicating failure.
                return Err(());
            }
        }

        // If splitting was successful down to the target class, return Ok.
        Ok(())
    }

    /// Allocates a block of memory from a specified size class within the heap.
    ///
    /// This function removes a block from the free list of the given size class and updates the heap's allocation statistics.
    /// It ensures that the memory allocation is reflected in the heap's bookkeeping, tracking both the user-requested size
    /// and the actual allocated block size.
    ///
    /// # Arguments
    ///
    /// * `class` - The size class from which to allocate the block.
    /// * `size` - The actual size of the block being allocated (including any overhead or alignment adjustments).
    /// * `user_size` - The size requested by the user, which may be smaller than the `size` due to alignment and metadata.
    ///
    /// # Returns
    ///
    /// * `Result<NonNull<u8>, ()>` - Returns a pointer to the allocated block on success, or an error if allocation fails.
    fn allocate_from_class(
        &mut self,
        class: usize,
        size: usize,
        user_size: usize,
    ) -> Result<NonNull<u8>, ()> {
        // Attempt to pop a block from the specified size class's free list.
        let block = self.free_list[class]
            .pop()
            .expect("Block should be available");

        // Attempt to create a NonNull pointer from the block address.
        let result = NonNull::new(block as *mut u8);

        // If the NonNull pointer is successfully created, update the heap's allocation statistics.
        if let Some(non_null) = result {
            // Update the total user-allocated size.
            self.user_size += user_size;
            // Update the total size allocated by the heap (including overhead).
            self.allocated += size;
            // Return the pointer to the allocated block.
            Ok(non_null)
        } else {
            // If creating the NonNull pointer failed, return an error.
            Err(())
        }
    }

    /// Attempts to merge a freed block with its buddy block.
    ///
    /// This function is called during deallocation to try and merge the freed block with its corresponding buddy block.
    /// If the buddy block is also free, the two blocks are merged into a larger block and moved to the next size class.
    /// This process is repeated recursively until no more merges are possible or the largest size class is reached.
    ///
    /// # Arguments
    ///
    /// * `current_ptr` - The starting address of the current block to be merged.
    /// * `current_class` - The size class of the current block.
    fn merge_buddies(&mut self, mut current_ptr: usize, mut current_class: usize) {
        // Continue trying to merge until the largest size class is reached.
        while current_class < self.free_list.len() - 1 {
            // Calculate the address of the buddy block.
            // The buddy's address is determined by XORing the current block's address with the size of the block.
            let buddy = current_ptr ^ (1 << current_class);

            // Check if the buddy block is free and can be merged.
            if self.find_buddy(current_class, buddy) {
                // If the buddy is found, remove it from the current size class's free list.
                self.free_list[current_class].pop();

                // Determine the starting address of the merged block (which is the minimum of the two buddies' addresses).
                current_ptr = min(current_ptr, buddy);

                // Move to the next size class as the blocks are merged into a larger block.
                current_class += 1;

                // Add the merged block to the free list of the next size class.
                self.free_list[current_class].push(current_ptr as *mut usize);
            } else {
                // If the buddy is not free, stop trying to merge.
                break;
            }
        }
    }

    /// Checks if the buddy of a block is free and removes it from the free list if it is.
    ///
    /// This function iterates over the free list of a given size class to find the buddy block. If the buddy is found,
    /// it is removed from the free list, indicating that it is available for merging.
    ///
    /// # Arguments
    ///
    /// * `class` - The size class of the block for which the buddy is being searched.
    /// * `buddy` - The address of the buddy block to find.
    ///
    /// # Returns
    ///
    /// * `bool` - Returns `true` if the buddy block was found and removed from the free list; otherwise, `false`.
    fn find_buddy(&mut self, class: usize, buddy: usize) -> bool {
        // Assume the buddy is not found initially.
        let mut found_buddy = false;

        // Iterate over the blocks in the free list of the given size class.
        for block in self.free_list[class].iter_mut() {
            // Check if the current block is the buddy we're looking for.
            if block.value() as usize == buddy {
                // If so, remove the buddy from the free list.
                block.pop();
                // Mark that we've found the buddy.
                found_buddy = true;
                // Exit the loop since we've found what we were looking for.
                break;
            }
        }

        // Return whether the buddy was found (and thus removed from the free list).
        found_buddy
    }
}
