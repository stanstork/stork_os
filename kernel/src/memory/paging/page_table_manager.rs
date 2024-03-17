use super::table::{PageEntryFlags, PageTable, PageTablePtr, TableLevel};
use crate::memory::{
    addr::{PhysAddr, VirtAddr},
    physical_page_allocator::PhysicalPageAllocator,
};

pub struct PageTableManager {
    pub pml4: PageTablePtr,
}

impl PageTableManager {
    pub fn new(pml4: *mut PageTable) -> PageTableManager {
        PageTableManager {
            pml4: PageTablePtr::new(pml4, TableLevel::PML4),
        }
    }

    /// Maps a virtual address to a physical address in the page table.
    ///
    /// This function traverses the page table hierarchy, creating new tables as necessary,
    /// and sets the final entry to point to the provided physical address.
    /// It sets the page as writable.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it:
    /// - Operates on raw pointers.
    /// - Modifies page table entries which can affect the entire memory access pattern of the system.
    ///
    /// The caller must ensure that:
    /// - The provided virtual and physical addresses are valid.
    /// - The `page_frame_alloc` is correctly initialized and can safely allocate new frames.
    ///
    /// # Arguments
    ///
    /// * `virt`: The virtual address to map.
    /// * `phys`: The physical address to map to.
    /// * `page_frame_alloc`: A reference to a physical page frame allocator.
    pub unsafe fn map_memory(
        &mut self,
        virt: VirtAddr,
        phys: PhysAddr,
        page_frame_alloc: &mut PhysicalPageAllocator,
    ) {
        // Traverse the page table hierarchy, creating tables as needed, and chain the calls.
        let mut pt = self
            .pml4
            .next(virt, page_frame_alloc)
            .and_then(|mut pdp| pdp.next(virt, page_frame_alloc))
            .and_then(|mut pd| pd.next(virt, page_frame_alloc))
            .unwrap();

        // Calculate the index for the final level based on the virtual address.
        let index = pt.level.index(virt);

        // Obtain a mutable reference to the final page table entry.
        let entry = &mut pt[index];

        // Set the frame address to the provided physical address and mark it as writable.
        entry.set_frame_addr(phys as usize);
        entry.set_flags(PageEntryFlags::WRITABLE);
    }
}
