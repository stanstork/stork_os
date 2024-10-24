use super::thread::Thread;
use super::{
    id::{IdAllocator, Pid},
    thread::Priority,
};
use crate::arch::x86_64::gdt::PrivilegeLevel;
use crate::{
    memory::paging::{manager::PageTableManager, table::PageTable, ROOT_PAGE_TABLE},
    println,
};
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;

/// Represents a process in the system, containing a PID, a page table, and a list of threads.
#[derive(Clone)]
pub struct Process {
    pub pid: Pid,                          // Process ID
    pub page_table: *mut PageTable,        // Pointer to the process's page table
    pub threads: Vec<Rc<RefCell<Thread>>>, // List of threads belonging to the process
}

impl Process {
    /// Creates a new process with a unique PID and a cloned root page table.
    pub fn new() -> Self {
        Process {
            pid: Pid::next(),                               // Get the next available PID
            page_table: unsafe { clone_root_page_table() }, // Clone the root page table
            threads: Vec::new(),                            // Initialize an empty list of threads
        }
    }

    /// Creates a new kernel process with the given function and priority.
    ///
    /// # Arguments
    ///
    /// * `func` - Entry point function for the kernel thread.
    /// * `priority` - Priority of the kernel thread.
    ///
    /// # Returns
    ///
    /// A reference-counted pointer to the new process.
    pub fn create_kernel_process(
        func: extern "C" fn(),
        priority: Priority,
    ) -> Rc<RefCell<Process>> {
        let process = Rc::new(RefCell::new(Process::new()));
        let thread = Thread::new(
            func as *const usize,   // Set the entry point to the function
            process.clone(),        // Reference to the process this thread belongs to
            PrivilegeLevel::Kernel, // Kernel privilege level
            priority,               // Thread priority
        );

        println!(
            "Thread created with stack pointer: {:#x}",
            thread.stack_pointer
        );

        // Add the new thread to the process's list of threads
        process
            .borrow_mut()
            .threads
            .push(Rc::new(RefCell::new(thread)));

        process
    }

    /// Creates a new user process with the given priority.
    ///
    /// # Arguments
    ///
    /// * `priority` - Priority of the user thread.
    ///
    /// # Returns
    ///
    /// A reference-counted pointer to the new process.
    pub fn create_user_process(priority: Priority) -> Rc<RefCell<Process>> {
        let process = Rc::new(RefCell::new(Process::new()));
        let thread = Thread::new_user(process.clone(), priority);

        process
            .borrow_mut()
            .threads
            .push(Rc::new(RefCell::new(thread)));

        process
    }
}

/// Clones the root page table.
///
/// # Safety
///
/// This function is unsafe because it involves direct manipulation of the page tables and raw pointers.
///
/// # Returns
///
/// A pointer to the cloned page table.
unsafe fn clone_root_page_table() -> *mut PageTable {
    let root_page_table = &mut *(ROOT_PAGE_TABLE as *mut PageTable);
    PageTableManager::clone_pml4(root_page_table as *mut PageTable)
}
