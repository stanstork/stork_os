use crate::memory::{addr::VirtAddr, PAGE_SIZE};
use bitflags::bitflags;
use core::ops::{Index, IndexMut};

// Define flags for page table entries. These flags are used to control access and behavior of memory pages.
// For example, they can indicate whether a page is present in memory, writable, or executable.
bitflags! {
    pub struct PageEntryFlags: usize {
        const PRESENT         = 1 << 0;  // Page is present in memory.
        const WRITABLE        = 1 << 1;  // Page is writable.
        const USER_ACCESSIBLE = 1 << 2;  // Page is accessible from user mode.
        const WRITE_THROUGH   = 1 << 3;  // Write-through caching is enabled for the page.
        const CACHE_DISABLE   = 1 << 4;  // Cache is disabled for this page.
        const ACCESSED        = 1 << 5;  // Page has been accessed (read).
        const DIRTY           = 1 << 6;  // Page has been written to.
        const HUGE_PAGE       = 1 << 7;  // Page is a huge page (larger than the standard page size).
        const GLOBAL          = 1 << 8;  // Page is global (not flushed from the TLB on task switch).
        const NO_EXECUTE      = 1 << 63; // Execution is disabled for this page.
    }
}

/// Represents the level of a page table in the x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableLevel {
    PML4,
    PDP,
    PD,
    PT,
}

/// Represents a single entry in a page table.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct PageEntry(pub usize);

/// Represents a page table, which consists of 512 entries in x86_64 architecture.
/// Each page table can potentially map up to 2MB of virtual memory (512 entries * 4KB page size).
#[repr(C)]
#[repr(align(4096))]
#[derive(Debug)]
pub struct PageTable {
    pub(crate) entries: [PageEntry; 512],
}

impl PageTable {
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PageEntry> {
        let ptr = self.entries.as_mut_ptr();
        (256..512).map(move |i| unsafe { &mut *ptr.add(i) })
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &PageEntry> {
        (256..512).map(move |i| &self.entries[i])
    }

    pub fn get_entry(&self, index: usize) -> &PageEntry {
        &self.entries[index]
    }

    pub fn get_entry_for_address(&self, virt_addr: u64) -> Option<&PageEntry> {
        let pml4_index = ((virt_addr >> 39) & 0x1FF) as usize;
        let pdpt_index = ((virt_addr >> 30) & 0x1FF) as usize;
        let pd_index = ((virt_addr >> 21) & 0x1FF) as usize;
        let pt_index = ((virt_addr >> 12) & 0x1FF) as usize;

        let pml4_entry = self.get_entry(pml4_index);
        if !pml4_entry.is_present() {
            return None;
        }

        let pdpt = unsafe { &*(pml4_entry.get_frame_addr()? as *const PageTable) };
        let pdpt_entry = pdpt.get_entry(pdpt_index);
        if !pdpt_entry.is_present() {
            return None;
        }

        let pd = unsafe { &*(pdpt_entry.get_frame_addr()? as *const PageTable) };
        let pd_entry = pd.get_entry(pd_index);
        if !pd_entry.is_present() {
            return None;
        }

        let pt = unsafe { &*(pd_entry.get_frame_addr()? as *const PageTable) };
        let pt_entry = pt.get_entry(pt_index);

        if pt_entry.is_present() {
            Some(pt_entry)
        } else {
            None
        }
    }
}

// Define a wrapper around the raw pointer.
#[derive(Clone)]
pub struct PageTablePtr {
    pub ptr: *mut PageTable,
    pub(super) level: TableLevel,
}

impl PageEntry {
    /// Returns the flags of the page table entry.
    pub fn flags(&self) -> PageEntryFlags {
        PageEntryFlags::from_bits_truncate(self.0)
    }

    /// Sets the flags of the page table entry.
    pub fn set_flags(&mut self, flags: PageEntryFlags) {
        self.0 = self.0 | (self.flags() | flags).bits();
    }

    /// Sets the frame address in the page table entry, preserving the flags.
    pub fn set_frame_addr(&mut self, addr: usize) {
        let flags = self.flags();
        self.0 = addr;
        self.set_flags(flags | PageEntryFlags::PRESENT);
    }

    /// Gets the frame address if the entry is present.
    pub fn get_frame_addr(&self) -> Option<usize> {
        if self.flags().contains(PageEntryFlags::PRESENT) {
            Some(self.0 & !0xFFF)
        } else {
            None
        }
    }

    /// Checks if the page is present in memory.
    pub fn is_present(&self) -> bool {
        self.flags().contains(PageEntryFlags::PRESENT)
    }
}

impl TableLevel {
    /// Returns the next level of the page table.
    pub fn next_level(&self) -> TableLevel {
        match self {
            TableLevel::PML4 => TableLevel::PDP,
            TableLevel::PDP => TableLevel::PD,
            TableLevel::PD => TableLevel::PT,
            TableLevel::PT => panic!("Page table is the last level"),
        }
    }

    pub fn index(&self, virt_addr: VirtAddr) -> usize {
        match self {
            TableLevel::PML4 => (virt_addr.0 >> 39) & 0x1FF,
            TableLevel::PDP => (virt_addr.0 >> 30) & 0x1FF,
            TableLevel::PD => (virt_addr.0 >> 21) & 0x1FF,
            TableLevel::PT => (virt_addr.0 >> 12) & 0x1FF,
        }
    }
}

impl PageTablePtr {
    /// Creates a new PageTablePtr with a level.
    pub fn new(ptr: *mut PageTable, level: TableLevel) -> Self {
        PageTablePtr { ptr, level }
    }

    /// Advances to the next level page table, creating it if necessary.
    ///
    /// This function navigates to the next level page table for the given virtual address.
    /// If the next level table does not exist, it creates one using the provided frame allocator.
    ///
    /// # Arguments
    /// * `virt_addr` - The virtual address for which to find the next level page table.
    /// * `frame_alloc` - A mutable reference to a closure that allocates frames.
    ///
    /// # Safety
    /// This function is unsafe because it performs raw pointer dereferencing.
    ///
    /// # Returns
    /// An `Option` containing the next level `PageTablePtr` if successful, or `None` if it fails.
    pub unsafe fn next_or_create<F: FnMut() -> *mut PageTable>(
        &mut self,
        virt_addr: VirtAddr,
        frame_alloc: &mut F,
    ) -> Option<PageTablePtr> {
        let index = self.level.index(virt_addr);
        let entry = &self[index];

        if entry.is_present() {
            let addr = entry.get_frame_addr()?;
            let level = self.level.next_level();
            Some(PageTablePtr::new(addr as *mut PageTable, level))
        } else {
            // Create the next level table if not present.
            Some(self.create_next_table(index, frame_alloc))
        }
    }

    /// Advances to the next level page table without creating it.
    ///
    /// This function navigates to the next level page table for the given virtual address.
    /// It does not create a new table if the next level table does not exist.
    ///
    /// # Arguments
    /// * `virt_addr` - The virtual address for which to find the next level page table.
    ///
    /// # Safety
    /// This function is unsafe because it performs raw pointer dereferencing.
    ///
    /// # Returns
    /// The next level `PageTablePtr`.
    pub unsafe fn next(&mut self, virt_addr: VirtAddr) -> PageTablePtr {
        let index = self.level.index(virt_addr);
        let entry = &self[index];

        let addr = entry.get_frame_addr().unwrap();
        let level = self.level.next_level();
        PageTablePtr::new(addr as *mut PageTable, level)
    }

    unsafe fn create_next_table<F: FnMut() -> *mut PageTable>(
        &mut self,
        index: usize,
        frame_alloc: &mut F,
    ) -> PageTablePtr {
        let page_table_addr = frame_alloc();
        // Zero out the new page table.
        (page_table_addr as *mut u8).write_bytes(0, PAGE_SIZE);

        // Set up the current entry to point to the new table.
        self[index].set_frame_addr(page_table_addr as usize);
        self[index].set_flags(PageEntryFlags::PRESENT | PageEntryFlags::WRITABLE);

        PageTablePtr::new(page_table_addr, self.level.next_level())
    }
}

// Allows read-only access to a page table entry by its index.
impl Index<usize> for PageTable {
    type Output = PageEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

// Allows mutable access to a page table entry by its index.
impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

// Implement the Index trait for the wrapper.
impl Index<usize> for PageTablePtr {
    type Output = PageEntry;

    fn index(&self, index: usize) -> &Self::Output {
        // Safety: The caller must ensure that the index is within bounds and the pointer is valid.
        unsafe { &(*self.ptr).entries[index] }
    }
}

// Implement the IndexMut trait for the wrapper.
impl IndexMut<usize> for PageTablePtr {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        // Safety: The caller must ensure that the index is within bounds and the pointer is valid.
        unsafe { &mut (*self.ptr).entries[index] }
    }
}
