use super::table::{PageEntryFlags, PageTable, PageTablePtr, TableLevel};
use crate::{
    memory::{
        addr::{PhysAddr, VirtAddr},
        PAGE_SIZE,
    },
    ALLOCATOR,
};

#[derive(Clone)]
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
    pub unsafe fn map_memory<F: FnMut() -> *mut PageTable>(
        &mut self,
        virt: VirtAddr,
        phys: PhysAddr,
        frame_alloc: &mut F,
        user: bool,
    ) {
        // Traverse the page table hierarchy, creating tables as needed, and chain the calls.
        let mut pt = self
            .pml4
            .next_or_create(virt, frame_alloc)
            .and_then(|mut pdp| pdp.next_or_create(virt, frame_alloc))
            .and_then(|mut pd| pd.next_or_create(virt, frame_alloc))
            .unwrap();

        // Calculate the index for the final level based on the virtual address.
        let index = pt.level.index(virt);

        // Obtain a mutable reference to the final page table entry.
        let entry = &mut pt[index];

        // Set the frame address to the provided physical address and mark it as writable.
        entry.set_frame_addr(phys.0 as usize);

        let mut flags = PageEntryFlags::PRESENT | PageEntryFlags::WRITABLE;
        if user {
            flags |= PageEntryFlags::USER_ACCESSIBLE;
        }
        entry.set_flags(flags);
    }

    /// Maps a user-accessible page table entry.
    ///
    /// This function navigates through the page tables (PML4, PDPT, PDT) to set a specific
    /// page table entry as user accessible. It updates the page table entries along the way
    /// and finally sets the physical address for the given virtual address with the desired flags.
    ///
    /// # Arguments
    /// * `page_table` - A mutable pointer to the PML4 page table.
    /// * `virt_addr` - The virtual address to be mapped.
    /// * `phys_addr` - The physical address to be mapped to the virtual address.
    ///
    /// # Security
    /// This function ensures that kernel pages are not modified. It operates only on the
    /// user-accessible portion of the address space. Kernel pages reside in the higher half
    /// of the address space in a typical x86_64 paging scheme, and this function assumes
    /// that `virt_addr` provided is a user-space address.
    pub fn map_user_page(page_table: *mut PageTable, virt_addr: VirtAddr, phys_addr: PhysAddr) {
        // Initialize the current page table pointer to the PML4 table
        let mut current_table_ptr = PageTablePtr::new(page_table, TableLevel::PML4);

        // Traverse through the PML4, PDPT, and PDT levels
        for _ in (1..4).rev() {
            let index = current_table_ptr.level.index(virt_addr);
            let entry = unsafe { &mut (*current_table_ptr.ptr)[index] };

            // Set the entry to be user accessible
            entry.set_flags(entry.flags() | PageEntryFlags::USER_ACCESSIBLE);

            // Move to the next level page table
            current_table_ptr = unsafe { current_table_ptr.next(virt_addr) };
        }

        // Set the final page table entry
        let final_index = current_table_ptr.level.index(virt_addr);
        let final_entry = unsafe { &mut (*current_table_ptr.ptr)[final_index] };
        final_entry.set_frame_addr(phys_addr.0);
        final_entry.set_flags(
            PageEntryFlags::PRESENT | PageEntryFlags::WRITABLE | PageEntryFlags::USER_ACCESSIBLE,
        );
    }

    /// Clones the PML4 table, including all its lower-level tables.
    ///
    /// # Arguments
    /// * `src` - A pointer to the source PML4 table to be cloned.
    ///
    /// # Safety
    /// This function is unsafe because it performs raw pointer dereferencing.
    ///
    /// # Returns
    /// A pointer to the newly cloned PML4 table.
    pub unsafe fn clone_pml4(src: *mut PageTable) -> *mut PageTable {
        let page_table_manager = PageTableManager::new(src);
        let new_pml4 = page_table_manager.alloc_zeroed_page().0 as *mut PageTable;

        for i in 0..512 {
            let src_entry = &(*src)[i];
            if src_entry.get_frame_addr().is_none() {
                continue;
            }

            let origin = src_entry.get_frame_addr().unwrap() as *mut PageTable;
            let cloned_pdp = Self::clone_pdp(origin, &page_table_manager);
            (*new_pml4)[i].set_frame_addr(cloned_pdp as usize);
            (*new_pml4)[i].set_flags(PageEntryFlags::PRESENT | PageEntryFlags::WRITABLE);
        }

        new_pml4
    }

    /// Converts a virtual address to a physical address.
    ///
    /// This function navigates through the page tables to find the physical address
    /// corresponding to a given virtual address.
    ///
    /// # Arguments
    /// * `virt` - The virtual address to be translated.
    ///
    /// # Safety
    /// This function is unsafe because it performs raw pointer dereferencing.
    ///
    /// # Returns
    /// The physical address corresponding to the given virtual address.
    pub unsafe fn phys_addr(&self, virt: VirtAddr) -> PhysAddr {
        let plm4_entry = &self.pml4[TableLevel::PML4.index(virt)];
        let pdp = plm4_entry.get_frame_addr().unwrap() as *mut PageTable;

        let pdp_entry = &(*pdp)[TableLevel::PDP.index(virt)];
        let pd = pdp_entry.get_frame_addr().unwrap() as *mut PageTable;

        let pd_entry = &(*pd)[TableLevel::PD.index(virt)];
        let pt = pd_entry.get_frame_addr().unwrap() as *mut PageTable;

        let pt_entry = &(*pt)[TableLevel::PT.index(virt)];
        PhysAddr(pt_entry.get_frame_addr().unwrap())
    }

    /// Allocates a zeroed page.
    ///
    /// This function allocates a new page using the heap allocator and zeroes it out.
    ///
    /// # Safety
    /// This function is unsafe because it performs raw pointer dereferencing and assumes
    /// the heap allocator is correctly implemented.
    ///
    /// # Returns
    /// The physical address of the newly allocated zeroed page.
    pub unsafe fn alloc_zeroed_page(&self) -> PhysAddr {
        // Allocate a new page using the heap allocator
        let virtual_address = ALLOCATOR.alloc_page();

        // Zero out the allocated page
        virtual_address.write_bytes(0, PAGE_SIZE);

        // Convert the virtual address to a physical address
        self.phys_addr(VirtAddr(virtual_address as usize))
    }

    /// Clones the PDPT (Page Directory Pointer Table), including all its lower-level tables.
    ///
    /// # Arguments
    /// * `src` - A pointer to the source PDPT to be cloned.
    /// * `page_table_manager` - A reference to the page table manager.
    ///
    /// # Safety
    /// This function is unsafe because it performs raw pointer dereferencing.
    ///
    /// # Returns
    /// A pointer to the newly cloned PDPT.
    unsafe fn clone_pdp(
        src: *mut PageTable,
        page_table_manager: &PageTableManager,
    ) -> *mut PageTable {
        let new_pdp = page_table_manager.alloc_zeroed_page().0 as *mut PageTable;

        for i in 0..512 {
            let src_entry = &(*src)[i];
            if src_entry.get_frame_addr().is_none() {
                continue;
            }

            let origin = src_entry.get_frame_addr().unwrap() as *mut PageTable;
            let cloned_pd = Self::clone_pd(origin, page_table_manager);
            (*new_pdp)[i].set_frame_addr(cloned_pd as usize);
            (*new_pdp)[i].set_flags(PageEntryFlags::PRESENT | PageEntryFlags::WRITABLE);
        }

        new_pdp
    }

    /// Clones the PD (Page Directory), including all its lower-level tables.
    ///
    /// # Arguments
    /// * `src` - A pointer to the source PD to be cloned.
    /// * `page_table_manager` - A reference to the page table manager.
    ///
    /// # Safety
    /// This function is unsafe because it performs raw pointer dereferencing.
    ///
    /// # Returns
    /// A pointer to the newly cloned PD.
    unsafe fn clone_pd(
        src: *mut PageTable,
        page_table_manager: &PageTableManager,
    ) -> *mut PageTable {
        let new_pd = page_table_manager.alloc_zeroed_page().0 as *mut PageTable;

        for i in 0..512 {
            let src_entry = &(*src)[i];
            if src_entry.get_frame_addr().is_none() {
                continue;
            }

            let origin = src_entry.get_frame_addr().unwrap() as *mut PageTable;
            let cloned_pt = Self::clone_pt(origin, page_table_manager);
            (*new_pd)[i].set_frame_addr(cloned_pt as usize);
            (*new_pd)[i].set_flags(PageEntryFlags::PRESENT | PageEntryFlags::WRITABLE);
        }

        new_pd
    }

    /// Clones the PT (Page Table).
    ///
    /// # Arguments
    /// * `src` - A pointer to the source PT to be cloned.
    /// * `page_table_manager` - A reference to the page table manager.
    ///
    /// # Safety
    /// This function is unsafe because it performs raw pointer dereferencing.
    ///
    /// # Returns
    /// A pointer to the newly cloned PT.
    unsafe fn clone_pt(
        src: *mut PageTable,
        page_table_manager: &PageTableManager,
    ) -> *mut PageTable {
        let new_pt = page_table_manager.alloc_zeroed_page().0 as *mut PageTable;

        for i in 0..512 {
            let src_entry = &(*src)[i];
            if src_entry.get_frame_addr().is_none() {
                continue;
            }

            (*new_pt)[i] = *src_entry;
        }

        new_pt
    }
}
