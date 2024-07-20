use super::{
    table::{PageEntryFlags, PageTable, PageTablePtr, TableLevel},
    ROOT_PAGE_TABLE,
};
use crate::{
    memory::{
        addr::{PhysAddr, VirtAddr},
        KERNEL_PHYS_START, PAGE_SIZE,
    },
    println, ALLOCATOR,
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
            .next(virt, frame_alloc)
            .and_then(|mut pdp| pdp.next(virt, frame_alloc))
            .and_then(|mut pd| pd.next(virt, frame_alloc))
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

    pub unsafe fn map_page(&mut self, virt: VirtAddr, phys: PhysAddr, user: bool) {
        // Traverse the page table hierarchy, creating tables as needed, and chain the calls.
        let mut pt = self
            .pml4
            .next_table(virt)
            .and_then(|mut pdp| pdp.next_table(virt))
            .and_then(|mut pd| pd.next_table(virt))
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

        println!("Mapped page: {:#x} -> {:#x}", virt.0, phys.0);
        println!(
            "Page {:?} -> {:#x}",
            entry.flags(),
            entry.get_frame_addr().unwrap()
        );
    }

    pub fn set_user_accessible(
        page_table: *mut PageTable,
        virt_addr: VirtAddr,
        phys_addr: PhysAddr,
    ) {
        let mut current_table = PageTablePtr::new(page_table, TableLevel::PML4);
        let current_virt_addr = virt_addr.0;

        for _level in (1..4).rev() {
            let index = current_table.level.index(VirtAddr(current_virt_addr));
            let entry = unsafe { &mut (*current_table.ptr)[index] };

            entry.set_flags(entry.flags() | PageEntryFlags::USER_ACCESSIBLE);

            current_table = PageTablePtr::new(
                unsafe { &mut *(entry.get_frame_addr().unwrap() as *mut PageTable) },
                current_table.level.next_level(),
            )
        }

        // Set the final page entry
        let final_index = current_table.level.index(VirtAddr(current_virt_addr));
        let final_entry = unsafe { &mut (*current_table.ptr)[final_index] };
        final_entry.set_frame_addr(phys_addr.0);
        final_entry.set_flags(
            PageEntryFlags::PRESENT | PageEntryFlags::WRITABLE | PageEntryFlags::USER_ACCESSIBLE,
        );
    }

    pub unsafe fn create_address_space() -> *mut PageTable {
        let pml4 = ALLOCATOR.alloc_page() as *mut PageTable;
        let kernel_pml4 = ROOT_PAGE_TABLE as *mut PageTable;

        let pages = (*pml4).iter_mut().zip((*kernel_pml4).iter());
        for (new_entry, kernel_entry) in pages {
            *new_entry = kernel_entry.clone();
        }

        pml4
    }

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

    unsafe fn alloc_zeroed_page(&self) -> PhysAddr {
        // Allocate a new page using the heap allocator
        let virtual_address = ALLOCATOR.alloc_page();

        // Zero out the allocated page
        virtual_address.write_bytes(0, PAGE_SIZE);

        // Convert the virtual address to a physical address
        let physical_address = self.phys_addr(VirtAddr(virtual_address as usize));

        // Return the physical address
        physical_address
    }

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

    pub unsafe fn create_page_table() -> *mut PageTable {
        let root_page_table = ROOT_PAGE_TABLE as *mut PageTable;
        let mut page_table_manager = PageTableManager::new(root_page_table);
        let new_page_table = ALLOCATOR.alloc_page();

        let virt_addr = VirtAddr(new_page_table as usize);
        let phys_addr = page_table_manager.phys_addr(virt_addr);

        page_table_manager.map_page(virt_addr, phys_addr, true);

        new_page_table as *mut PageTable
    }
}

fn check_reserved_bits(pte: u64) -> bool {
    // Reserved bits 52-58
    const RESERVED_MASK: u64 = 0x007F_0000_0000_0000;

    // Extract reserved bits
    let reserved_bits = pte & RESERVED_MASK;

    // Check if any reserved bits are set
    let r = reserved_bits != 0;

    if r {
        println!("Reserved bits set: {:#x}", reserved_bits);
    }

    r
}

fn check_page_table_entries(page_table: &PageTable) {
    for i in 0..512 {
        let pte = page_table.entries[i];
        if check_reserved_bits(pte.0 as u64) {
            println!(
                "Reserved bits set in PTE at index {}: {:#x}",
                i, pte.0 as u64
            );
        }
    }
}
pub fn check_all_page_tables(pml4: &PageTable) {
    // Check PML4 entries
    check_page_table_entries(pml4);

    // Check PDPT entries
    for i in 0..512 {
        let pdpt_entry = pml4.entries[i];
        if pdpt_entry.0 & 0x1 != 0 {
            // Check if the entry is present
            let pdpt = unsafe { &*((pdpt_entry.0 & 0x000F_FFFF_FFFF_F000) as *const PageTable) };
            check_page_table_entries(pdpt);

            // Check PD entries
            for j in 0..512 {
                let pd_entry = pdpt.entries[j];
                if pd_entry.0 & 0x1 != 0 {
                    // Check if the entry is present
                    let pd =
                        unsafe { &*((pd_entry.0 & 0x000F_FFFF_FFFF_F000) as *const PageTable) };
                    check_page_table_entries(pd);

                    // Check PT entries
                    for k in 0..512 {
                        let pt_entry = pd.entries[k];
                        if pt_entry.0 & 0x1 != 0 {
                            // Check if the entry is present
                            let pt = unsafe {
                                &*((pt_entry.0 & 0x000F_FFFF_FFFF_F000) as *const PageTable)
                            };
                            check_page_table_entries(pt);
                        }
                    }
                }
            }
        }
    }
}
