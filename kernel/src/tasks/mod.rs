use crate::{
    memory::{
        addr::VirtAddr,
        paging::{page_table_manager::PageTableManager, table::PageTable, ROOT_PAGE_TABLE},
    },
    registers::cr3::Cr3,
    ALLOCATOR, INITIAL_RSP,
};
use core::{arch::asm, ptr::copy_nonoverlapping};
use scheduler::SCHEDULER;

pub(crate) mod id;
pub(crate) mod process;
pub(crate) mod scheduler;
pub(crate) mod switch;
pub(crate) mod thread;

pub const KERNEL_STACK_SIZE: usize = 0x2000; // 8 KB
pub const KERNEL_STACK_START: u64 = 0x000700000000000; // 128 TB

// By default stack is in lower memory (somewhere around 0x7000 physical),
// which causes us problems as it'll be 'linked' instead of 'copied'
// when a page directory is changed (because the area from 0x0 - approx 0x150000 is mapped in the kernel_directory).
// So, we really need to move the stack.

/// Moves the stack to a new location.
///
/// This function allocates a new stack, copies the old stack contents to the new stack,
/// and updates the stack pointers accordingly.
///
/// # Arguments
/// * `new_stack_start` - A pointer to the new stack's start address.
/// * `size` - The size of the new stack.
///
/// # Safety
/// This function is unsafe because it performs raw pointer dereferencing and
/// inline assembly for stack manipulation.
/// https://web.archive.org/web/20160326122214/http://jamesmolloy.co.uk/tutorial_html/9.-Multitasking.html
pub unsafe fn move_stack(new_stack_start: *mut u8, size: u64) {
    // Initialize the root page table and page table manager.
    let root_page_table = unsafe { &mut *(ROOT_PAGE_TABLE as *mut PageTable) };
    let mut page_table_manager = PageTableManager::new(root_page_table);

    // Frame allocator closure.
    let page_table_manager_clone = page_table_manager.clone();
    let mut frame_alloc = || page_table_manager_clone.alloc_zeroed_page().0 as *mut PageTable;

    unsafe {
        // Allocate new stack pages and map them.
        let mut addr = new_stack_start as u64;
        while addr >= (new_stack_start as u64 - size) {
            let page = ALLOCATOR.alloc_page();
            let phys_addr = page_table_manager.phys_addr(VirtAddr(page as usize));

            page_table_manager.map_memory(
                VirtAddr(addr as usize),
                phys_addr,
                &mut frame_alloc,
                true,
            );
            addr = addr.wrapping_sub(0x1000);
        }

        // Flush the TLB by reloading CR3.
        let cr3 = Cr3::read();
        Cr3::write(cr3 as u64);

        // Save the old stack and base pointers.
        let old_stack_pointer: u64;
        asm!("mov {}, rsp", out(reg) old_stack_pointer);

        let old_base_pointer: u64;
        asm!("mov {}, rbp", out(reg) old_base_pointer);

        // Calculate the new stack and base pointers.
        let offset = new_stack_start as u64 - INITIAL_RSP;
        let new_stack_pointer = old_stack_pointer + offset;
        let new_base_pointer = old_base_pointer + offset;

        // Copy the old stack contents to the new stack.
        copy_nonoverlapping(
            old_stack_pointer as *const u8,
            new_stack_pointer as *mut u8,
            (INITIAL_RSP - old_stack_pointer) as usize,
        );

        // Backtrace through the original stack, updating the frame pointers.
        let mut addr = new_stack_start as u64;
        while addr > new_stack_start as u64 - size {
            let tmp = *(addr as *const u64);
            if old_stack_pointer < tmp && tmp < INITIAL_RSP {
                let new_tmp = tmp + offset;
                *(addr as *mut u64) = new_tmp;
            }
            addr = addr.wrapping_sub(8);
        }

        // Switch to the new stack.
        asm!("mov rsp, {}", in(reg) new_stack_pointer);
        asm!("mov rbp, {}", in(reg) new_base_pointer);
    }
}

/// Schedules the next thread to run by invoking the scheduler's `schedule` method.
///
/// This function checks if the global `SCHEDULER` is initialized. If it is, it
/// calls the `schedule` method on the scheduler to perform a context switch to the next thread.
///
/// # Safety
///
/// This function is unsafe because it directly manipulates global state and
/// performs a context switch, which can have side effects on the entire system.
pub fn schedule() {
    unsafe {
        // Check if the global SCHEDULER is initialized
        if let Some(scheduler) = SCHEDULER.as_mut() {
            // Call the schedule method to perform a context switch
            scheduler.schedule();
        }
    }
}

/// Retrieves the current page table pointer for the running thread.
///
/// This function checks if the global `SCHEDULER` is initialized. If it is, it
/// retrieves the page table pointer for the currently running thread and returns it as an `Option<u64>`.
///
/// # Returns
///
/// An `Option<u64>` containing the current page table pointer if the scheduler is initialized, otherwise `None`.
///
/// # Safety
///
/// This function is unsafe because it directly manipulates global state and
/// accesses the page table pointer of the currently running thread.
pub fn get_current_page_table() -> Option<u64> {
    unsafe {
        // Check if the global SCHEDULER is initialized
        if let Some(scheduler) = SCHEDULER.as_mut() {
            // Retrieve the current thread's page table pointer
            let page_table = scheduler
                .get_current_thread() // Get the current thread
                .lock() // Lock the thread for safe access
                .process // Access the associated process
                .borrow() // Borrow the process for access
                .page_table as u64; // Get the page table pointer as u64
            return Some(page_table);
        }
        None
    }
}
