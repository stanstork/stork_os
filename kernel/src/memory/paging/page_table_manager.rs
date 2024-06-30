use core::ptr;

use super::{
    table::{PageEntryFlags, PageTable, PageTablePtr, TableLevel},
    ROOT_PAGE_TABLE,
};
use crate::memory::{
    addr::{PhysAddr, VirtAddr},
    physical_page_allocator::PhysicalPageAllocator,
    PAGE_SIZE,
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
        entry.set_frame_addr(phys.0 as usize);
        entry.set_flags(
            PageEntryFlags::WRITABLE | PageEntryFlags::PRESENT | PageEntryFlags::ACCESSED,
        );
    }

    pub unsafe fn create_page_table(
        page_table_ptr: &mut PageTablePtr,
        virt_addr: VirtAddr,
        page_frame_alloc: &mut PhysicalPageAllocator,
        flags: PageEntryFlags,
    ) -> bool {
        let index = page_table_ptr.level.index(virt_addr);
        let entry = &mut page_table_ptr[index];

        if entry.is_present() {
            true
        } else {
            match page_frame_alloc.alloc_page() {
                Some(frame) => {
                    let page_table_addr = frame.0 as *mut PageTable;
                    (page_table_addr as *mut u8).write_bytes(0, PAGE_SIZE);

                    entry.set_frame_addr(page_table_addr as usize);
                    entry.set_flags(flags);

                    true
                }
                None => false,
            }
        }
    }

    pub fn get_physical_address(
        page_table_ptr: &mut PageTablePtr,
        virt: VirtAddr,
    ) -> Option<PhysAddr> {
        let index = page_table_ptr.level.index(virt);
        let entry = &page_table_ptr[index];

        if !entry.is_present() {
            None
        } else {
            entry.get_frame_addr().map(|addr| {
                let offset = virt.0 & 0xFFF;
                PhysAddr(addr + offset)
            })
        }
    }

    pub fn create_address_space(
        page_frame_alloc: &mut PhysicalPageAllocator,
    ) -> Option<*mut PageTable> {
        let space = unsafe {
            page_frame_alloc
                .alloc_page()
                .map(|frame| frame.0 as *mut PageTable)
        };

        if let Some(page_table) = space {
            unsafe {
                let kernel_page_table = ROOT_PAGE_TABLE as *const PageTable;

                Self::clear_page_directory(page_table);
                Self::clone_kernel_space(page_table, kernel_page_table);
            }
            Some(page_table)
        } else {
            None
        }
    }

    unsafe fn clone_kernel_space(dst: *mut PageTable, src: *const PageTable) {
        assert!(
            !dst.is_null() && !src.is_null(),
            "Source or destination cannot be null"
        );
        ptr::copy_nonoverlapping(
            src as *const u8,
            dst as *mut u8,
            512 * core::mem::size_of::<usize>(),
        );
    }

    unsafe fn clear_page_directory(page_table: *mut PageTable) {
        ptr::write_bytes(page_table as *mut u8, 0, PAGE_SIZE);
    }
}
