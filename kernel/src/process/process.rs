use super::id::{IdAllocator, Pid, Tid};
use crate::{
    gdt::PrivilegeLevel,
    memory::{
        addr::{PhysAddr, VirtAddr},
        paging::{page_table_manager::PageTableManager, table::PageTable, ROOT_PAGE_TABLE},
        PAGE_SIZE,
    },
    print, println,
    registers::cr3::Cr3,
    ALLOCATOR,
};
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::{
    alloc::Layout,
    cell::RefCell,
    intrinsics::size_of,
    ptr::{self, copy_nonoverlapping},
};

pub const STACK_SIZE: usize = 4096;
pub const KERNEL_CODE_SEGMENT: u16 = 1;
pub const KERNEL_DATA_SEGMENT: u16 = 2;
pub const USER_CODE_SEGMENT: u16 = 3;
pub const USER_DATA_SEGMENT: u16 = 4;

// Define the opcode for an infinite loop instruction.
// This instruction is used to halt the CPU when a thread finishes execution.
const INFINITE_LOOP: [u8; 2] = [0xeb, 0xfe];

#[derive(Default)]
#[repr(C)]
pub struct Registers {
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
    rax: u64,
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

#[derive(Clone)]
pub struct Process {
    pub pid: Pid,
    pub page_table: *mut PageTable,
    pub threads: Vec<Rc<RefCell<Thread>>>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Priority {
    High = 0,
    Medium = 1,
    Low = 2,
    Idle = 3,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ThreadStatus {
    Ready,
    Running,
    Blocked,
    Terminated,
}

#[derive(Clone)]
pub struct Thread {
    pub tid: Tid,
    pub process: Rc<RefCell<Process>>,
    pub stack_pointer: u64,
    pub priority: Priority,
    pub status: ThreadStatus,
}

pub struct Stack {
    pub stack_ptr: *mut u8,
}

impl Stack {
    pub fn new() -> Self {
        let num_pages = (STACK_SIZE + PAGE_SIZE - 1) / PAGE_SIZE; // Calculate the number of pages needed
        let mut stack_ptr = ptr::null_mut::<u8>();

        for _ in 0..num_pages {
            let page = unsafe { ALLOCATOR.alloc_page() };
            if stack_ptr.is_null() {
                stack_ptr = page;
            }
        }

        Stack { stack_ptr }
    }

    pub fn top(&self) -> *mut u64 {
        unsafe { self.stack_ptr.add(STACK_SIZE) as *mut u64 }
    }
}

extern "C" {
    fn start_thread(stack_pointer: u64);
}

impl Process {
    pub fn new() -> Self {
        Process {
            pid: Pid::next(),
            page_table: unsafe { clone_root_page_table() },
            threads: Vec::new(),
        }
    }

    pub fn create_kernel_process(
        func: extern "C" fn(),
        priority: Priority,
    ) -> Rc<RefCell<Process>> {
        let process = Rc::new(RefCell::new(Process::new()));
        let thread = Thread::create_thread(
            func as *const usize,
            process.clone(),
            PrivilegeLevel::Kernel,
            priority,
        );

        println!(
            "Thread created with stack pointer: {:#x}",
            thread.stack_pointer
        );

        process
            .borrow_mut()
            .threads
            .push(Rc::new(RefCell::new(thread)));

        process
    }

    pub fn create_user_process(priority: Priority) -> Rc<RefCell<Process>> {
        let process = Rc::new(RefCell::new(Process::new()));
        let thread = Thread::create_user_thread(process.clone(), priority);

        process
            .borrow_mut()
            .threads
            .push(Rc::new(RefCell::new(thread)));

        process
    }
}

impl Thread {
    fn create_thread(
        entry_point: *const usize,
        process: Rc<RefCell<Process>>,
        privilege_level: PrivilegeLevel,
        priority: Priority,
    ) -> Self {
        let (cs, ss) = match privilege_level {
            PrivilegeLevel::Kernel => (KERNEL_CODE_SEGMENT, KERNEL_DATA_SEGMENT),
            PrivilegeLevel::User => (USER_CODE_SEGMENT, USER_DATA_SEGMENT),
        };

        let cs = cs << 3 | privilege_level as u16;
        let ss = ss << 3 | privilege_level as u16;

        unsafe {
            let registers = Self::create_stack_frame(cs as u64, ss as u64, entry_point as u64);
            Thread {
                tid: Tid::next(),
                process,
                stack_pointer: registers as u64,
                priority,
                status: ThreadStatus::Ready,
            }
        }
    }

    pub fn create_user_thread(process: Rc<RefCell<Process>>, priority: Priority) -> Self {
        let entry_point = unsafe {
            let code = INFINITE_LOOP.as_ptr() as *const usize;
            let size = INFINITE_LOOP.len();
            Self::map_user_memory(process.borrow().page_table, code, size)
        };

        println!("Entry point: {:#x}", entry_point);

        Self::create_thread(
            entry_point as *const usize,
            process,
            PrivilegeLevel::User,
            priority,
        )
    }

    pub fn exec(&self) {
        let page_table = self.process.borrow().page_table;
        Cr3::write(page_table as u64);

        unsafe {
            println!(
                "Executing thread with stack pointer: {:#x}",
                self.stack_pointer
            );
            start_thread(self.stack_pointer);
        }
    }

    unsafe fn map_user_memory(
        page_table: *mut PageTable,
        address: *const usize,
        size: usize,
    ) -> usize {
        let page_table_manager = PageTableManager::new(page_table);
        let virt_addr = ALLOCATOR.alloc_page();
        let phys_addr = page_table_manager.phys_addr(VirtAddr(virt_addr as usize));

        copy_nonoverlapping(address as *const u8, virt_addr as *mut u8, size);

        PageTableManager::map_user_page(
            page_table,
            VirtAddr(virt_addr as usize),
            PhysAddr(phys_addr.0 as usize),
        );

        virt_addr as usize
    }

    unsafe fn create_stack_frame(cs: u64, ss: u64, rip: u64) -> *mut u64 {
        let stack = ALLOCATOR.alloc_page();
        let stack_top = (stack.add(STACK_SIZE)) as *mut u64;
        let stack_top = stack_top.sub(size_of::<Registers>());

        let cpu_state = stack_top as *mut Registers;

        unsafe {
            (*cpu_state).rip = rip;
            (*cpu_state).cs = cs;
            (*cpu_state).rflags = 0x202;
            (*cpu_state).rsp = stack_top as u64;
            (*cpu_state).ss = ss;

            print_stack(stack_top as *mut u8, STACK_SIZE);
        }

        stack_top
    }
}

unsafe fn clone_root_page_table() -> *mut PageTable {
    let root_page_table = &mut *(ROOT_PAGE_TABLE as *mut PageTable);
    PageTableManager::clone_pml4(root_page_table as *mut PageTable)
}

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

//EXAMPLE TASKS
pub extern "C" fn task_a() {
    let mut a: u32 = 0;
    let mut b: u8 = 0;
    loop {
        if a == 100_000_000 {
            println!("Process A running. {}% complete.", b);
            a = 0;
            b += 1;

            if b == 100 {
                println!("Process A complete.");
                break;
            }
        }
        a += 1;
    }
    loop {}
}

pub extern "C" fn task_b() {
    let mut a: u32 = 0;
    let mut b: u8 = 0;
    loop {
        if a == 100_000_000 {
            println!("Process B running. {}% complete.", b);
            a = 0;
            b += 1;

            if b == 100 {
                println!("Process B complete.");
                break;
            }
        }
        a += 1;
    }
    loop {}
}
