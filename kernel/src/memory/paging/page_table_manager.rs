use super::table::{PageEntryFlags, PageTable, PageTablePtr, TableLevel};
use crate::memory::{
    addr::{PhysAddr, VirtAddr},
    PAGE_FRAME_ALLOCATOR, PAGE_SIZE,
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

    pub unsafe fn clone_pml4(src: *mut PageTable, kernel: *mut PageTable) -> *mut PageTable {
        let new_page = PAGE_FRAME_ALLOCATOR
            .as_mut()
            .unwrap()
            .alloc_page()
            .unwrap()
            .0 as *mut PageTable;
        new_page.write_bytes(0, PAGE_SIZE);

        for i in 0..512 {
            let src_entry = &mut (*src)[i];
            if src_entry.get_frame_addr().is_none() {
                continue;
            }

            let entry = src_entry.0;
            let kernel_entry = (*kernel)[i].0;

            if entry == kernel_entry {
                (*new_page)[i] = *src_entry;
            } else {
                let origin = src_entry.get_frame_addr().unwrap() as *mut PageTable;
                let kern = (*kernel)[i].get_frame_addr().unwrap() as *mut PageTable;
                let pt = Self::clone_pdp(origin, kern);
                (*new_page)[i].set_frame_addr(pt as usize);
                (*new_page)[i].set_flags(
                    PageEntryFlags::PRESENT
                        | PageEntryFlags::WRITABLE
                        | PageEntryFlags::USER_ACCESSIBLE,
                );
            }
        }

        new_page
    }

    unsafe fn clone_pdp(origin: *mut PageTable, kern: *mut PageTable) -> *mut PageTable {
        let new_page = PAGE_FRAME_ALLOCATOR
            .as_mut()
            .unwrap()
            .alloc_page()
            .unwrap()
            .0 as *mut PageTable;
        new_page.write_bytes(0, PAGE_SIZE);

        for i in 0..512 {
            let src_entry = &mut (*origin)[i];
            if src_entry.get_frame_addr().is_none() {
                continue;
            }

            let entry = src_entry.0;
            let kernel_entry = (*kern)[i].0;

            if entry == kernel_entry {
                (*new_page)[i] = *src_entry;
            } else {
                let origin = src_entry.get_frame_addr().unwrap() as *mut PageTable;
                let kern = (*kern)[i].get_frame_addr().unwrap() as *mut PageTable;
                let pt = Self::clone_pd(origin, kern);
                (*new_page)[i].set_frame_addr(pt as usize);
                (*new_page)[i].set_flags(
                    PageEntryFlags::PRESENT
                        | PageEntryFlags::WRITABLE
                        | PageEntryFlags::USER_ACCESSIBLE,
                );
            }
        }

        new_page
    }

    unsafe fn clone_pd(origin: *mut PageTable, kern: *mut PageTable) -> *mut PageTable {
        let new_page = PAGE_FRAME_ALLOCATOR
            .as_mut()
            .unwrap()
            .alloc_page()
            .unwrap()
            .0 as *mut PageTable;
        new_page.write_bytes(0, PAGE_SIZE);

        for i in 0..512 {
            let src_entry = &mut (*origin)[i];
            if src_entry.get_frame_addr().is_none() {
                continue;
            }

            let entry = src_entry.0;
            let kernel_entry = (*kern)[i].0;

            if entry == kernel_entry {
                (*new_page)[i] = *src_entry;
            } else {
                let origin = src_entry.get_frame_addr().unwrap() as *mut PageTable;
                let kern = (*kern)[i].get_frame_addr().unwrap() as *mut PageTable;
                let pt = Self::clone_pt(origin, kern);
                (*new_page)[i].set_frame_addr(pt as usize);
                (*new_page)[i].set_flags(
                    PageEntryFlags::PRESENT
                        | PageEntryFlags::WRITABLE
                        | PageEntryFlags::USER_ACCESSIBLE,
                );
            }
        }

        new_page
    }

    unsafe fn clone_pt(origin: *mut PageTable, kern: *mut PageTable) -> *mut PageTable {
        let new_page = PAGE_FRAME_ALLOCATOR
            .as_mut()
            .unwrap()
            .alloc_page()
            .unwrap()
            .0 as *mut PageTable;
        new_page.write_bytes(0, PAGE_SIZE);

        for i in 0..512 {
            let src_entry = &mut (*origin)[i];
            if src_entry.get_frame_addr().is_none() {
                continue;
            }

            let entry = src_entry.0;
            let kernel_entry = (*kern)[i].0;

            if entry == kernel_entry {
                (*new_page)[i] = *src_entry;
            } else {
                let origin = src_entry.get_frame_addr().unwrap() as *mut PageTable;
                let kern = (*kern)[i].get_frame_addr().unwrap() as *mut PageTable;
                let pt = Self::clone_pt(origin, kern);
                (*new_page)[i].set_frame_addr(pt as usize);
                (*new_page)[i].set_flags(
                    PageEntryFlags::PRESENT
                        | PageEntryFlags::WRITABLE
                        | PageEntryFlags::USER_ACCESSIBLE,
                );
            }
        }

        new_page
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
}
