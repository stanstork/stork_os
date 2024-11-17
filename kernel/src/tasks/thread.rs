use super::{
    elf::ElfLoader,
    id::{IdAllocator, Tid},
    process::Process,
};
use crate::{
    arch::x86_64::gdt::PrivilegeLevel,
    memory::{
        addr::{PhysAddr, VirtAddr},
        paging::{manager::PageTableManager, table::PageTable},
    },
    print, println,
    registers::cr3::Cr3,
    tasks::switch::start_thread,
    ALLOCATOR,
};
use alloc::rc::Rc;
use core::{cell::RefCell, mem::size_of, ptr::copy_nonoverlapping};

/// Represents the CPU state for a thread, to be saved and restored during context switches.
#[derive(Default)]
#[repr(C)]
pub struct State {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rbp: u64,
    rdi: u64,
    rsi: u64,
    rdx: u64,
    rcx: u64,
    rbx: u64,
    pub(crate) rax: u64,
    rip: u64,    // Instruction pointer
    cs: u64,     // Code segment
    rflags: u64, // RFLAGS register
    rsp: u64,    // Stack pointer
    ss: u64,     // Stack segment
}

/// Enum representing the priority levels of a thread. Lower values indicate higher priority.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Priority {
    High = 0,
    Medium = 1,
    Low = 2,
    Idle = 3,
}

/// Enum representing the possible statuses of a thread.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Status {
    Ready,      // Ready to run
    Running,    // Currently running
    Blocked,    // Blocked and waiting for an event
    Terminated, // Finished execution
}

/// Represents a thread in the system, including its ID, process, stack pointer, priority, and status.
#[derive(Clone)]
pub struct Thread {
    pub tid: Tid,                      // Thread ID
    pub process: Rc<RefCell<Process>>, // Process the thread belongs to
    pub stack_pointer: u64,            // Stack pointer
    pub priority: Priority,            // Thread priority
    pub status: Status,                // Current status of the thread
}

// Define the opcode for an infinite loop instruction.
// This instruction is used to halt the CPU when a thread finishes execution.
const INFINITE_LOOP: [u8; 2] = [0xeb, 0xfe];

/// The size of the stack for each thread.
pub const STACK_SIZE: usize = 4096; // 4 KB

/// Segment selectors for kernel and user modes.
pub const KERNEL_CODE_SEGMENT: u16 = 1;
pub const KERNEL_DATA_SEGMENT: u16 = 2;
pub const USER_CODE_SEGMENT: u16 = 3;
pub const USER_DATA_SEGMENT: u16 = 4;

impl Thread {
    /// Creates a new thread with the given entry point, process, privilege level, and priority.
    ///
    /// # Arguments
    ///
    /// * `entry_point` - The entry point of the thread (function to start execution).
    /// * `process` - Reference to the process this thread belongs to.
    /// * `privilege_level` - Privilege level of the thread (Kernel/User).
    /// * `priority` - Priority of the thread.
    pub fn new(
        entry_point: *const usize,
        process: Rc<RefCell<Process>>,
        privilege_level: PrivilegeLevel,
        priority: Priority,
    ) -> Self {
        // Determine the code and stack segment selectors based on the privilege level
        let (cs, ss) = match privilege_level {
            PrivilegeLevel::Kernel => (KERNEL_CODE_SEGMENT, KERNEL_DATA_SEGMENT),
            PrivilegeLevel::User => (USER_CODE_SEGMENT, USER_DATA_SEGMENT),
        };

        // Combine the segment selector with the privilege level
        let cs = cs << 3 | privilege_level as u16;
        let ss = ss << 3 | privilege_level as u16;

        unsafe {
            // Initialize the stack for the thread
            let stack = Self::init_stack(cs as u64, ss as u64, entry_point as u64);
            Thread {
                tid: Tid::next(), // Get the next available thread ID
                process,
                stack_pointer: stack as u64,
                priority,
                status: Status::Ready, // Set the initial status to Ready
            }
        }
    }

    /// Creates a new user thread for the given process with the specified priority.
    ///
    /// # Arguments
    ///
    /// * `process` - Reference to the process this thread belongs to.
    /// * `priority` - Priority of the thread.
    pub fn new_user(process: Rc<RefCell<Process>>, priority: Priority) -> Self {
        // Set the entry point to an infinite loop (halts the CPU when thread finishes)
        let entry_point = unsafe {
            let code = INFINITE_LOOP.as_ptr() as *const usize;
            let size = INFINITE_LOOP.len();
            Self::map_user_memory(process.borrow().page_table, code, size)
        };

        // println!("Entry point: {:#x}", entry_point);

        Self::new(
            entry_point as *const usize,
            process,
            PrivilegeLevel::User,
            priority,
        )
    }

    pub fn new_user2(process: Rc<RefCell<Process>>, priority: Priority, elf_file: &str) -> Self {
        let page_table = process.borrow().page_table;
        let entry = ElfLoader::load_elf(elf_file, page_table);

        println!("Entry point: {:#x}", entry.unwrap());

        Self::new(
            entry.unwrap().0 as *const usize,
            process,
            PrivilegeLevel::User,
            priority,
        )

        // todo!()
    }

    /// Executes the thread by setting up the page table and stack pointer, and then starting the thread.
    pub fn run(&self) {
        let page_table = self.process.borrow().page_table;
        // Load the page table for this thread's process
        Cr3::write(page_table as u64);

        println!(
            "Executing thread with stack pointer: {:#x}",
            self.stack_pointer
        );
        // Start the thread by setting up the stack pointer and jumping to the entry point
        start_thread(self.stack_pointer);
    }

    /// Maps user memory for the thread, setting up the virtual and physical addresses.
    ///
    /// # Arguments
    ///
    /// * `page_table` - The page table for the thread's process.
    /// * `address` - The address of the code to be mapped.
    /// * `size` - The size of the code to be mapped.
    ///
    /// # Returns
    ///
    /// The virtual address where the code is mapped.
    pub unsafe fn map_user_memory(
        page_table: *mut PageTable,
        address: *const usize,
        size: usize,
    ) -> usize {
        let page_table_manager = PageTableManager::new(page_table);
        let virt_addr = ALLOCATOR.alloc_page();
        let phys_addr = page_table_manager.phys_addr(VirtAddr(virt_addr as usize));

        // Copy the code into the allocated virtual page
        copy_nonoverlapping(address as *const u8, virt_addr as *mut u8, size);

        // Map the virtual page to the physical page in the page table
        PageTableManager::map_user_page(
            page_table,
            VirtAddr(virt_addr as usize),
            PhysAddr(phys_addr.0 as usize),
        );

        virt_addr as usize
    }

    /// Initializes the stack frame for the thread, setting up the initial CPU state.
    ///
    /// # Arguments
    ///
    /// * `cs` - Code segment selector.
    /// * `ss` - Stack segment selector.
    /// * `rip` - Instruction pointer (entry point) of the thread.
    ///
    /// # Returns
    ///
    /// The top of the stack.
    unsafe fn init_stack(cs: u64, ss: u64, rip: u64) -> *mut u64 {
        let stack = ALLOCATOR.alloc_page(); // Allocate a new page for the stack
        let stack_top = (stack.add(STACK_SIZE)) as *mut u64; // Calculate the top of the stack
        let stack_top = stack_top.sub(size_of::<State>()); // Make room for the State struct

        let state = stack_top as *mut State;

        unsafe {
            // Initialize the CPU state for the thread
            (*state).rip = rip; // Set the instruction pointer to the entry point
            (*state).cs = cs; // Set the code segment selector
            (*state).rflags = 0x202; // Set the RFLAGS register to enable interrupts
            (*state).rsp = stack_top as u64; // Set the stack pointer to the top of the stack
            (*state).ss = ss; // Set the stack segment selector

            // print_stack(stack_top as *mut u8, 256);
        }

        stack_top // Return the top of the stack
    }
}

/// Prints the contents of the stack for debugging purposes.
///
/// # Arguments
///
/// * `stack` - Pointer to the stack.
/// * `stack_size` - Size of the stack.
fn print_stack(stack: *mut u8, stack_size: usize) {
    unsafe {
        let stack_top = stack.add(stack_size) as *const u64;

        println!("Stack content from top to bottom:");
        for i in (0..stack_size / 8).rev() {
            let value = *stack_top.offset(-(i as isize));
            print!("0x{:x} ", value);
        }
        println!();
    }
}
