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
    rsi: u64,
    rdi: u64,
    rdx: u64,
    rcx: u64,
    rbx: u64,
    rax: u64,
    rbp: u64,
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

pub struct Thread {
    pub tid: Tid,
    pub process: Rc<RefCell<Process>>,
    pub stack_pointer: u64,
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

pub extern "C" fn idle_thread() {
    println!("Idle thread running");
    loop {}
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

    pub fn create_kernel_process(func: extern "C" fn()) -> Rc<RefCell<Process>> {
        let process = Rc::new(RefCell::new(Process::new()));
        let thread = Thread::create_thread(
            func as *const usize,
            process.clone(),
            PrivilegeLevel::Kernel,
        );

        process
            .borrow_mut()
            .threads
            .push(Rc::new(RefCell::new(thread)));

        process
    }

    pub fn create_user_process() -> Rc<RefCell<Process>> {
        let process = Rc::new(RefCell::new(Process::new()));
        let thread = Thread::create_user_thread(process.clone());

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
            }
        }
    }

    pub fn create_user_thread(process: Rc<RefCell<Process>>) -> Self {
        let entry_point = unsafe {
            let code = INFINITE_LOOP.as_ptr() as *const usize;
            let size = INFINITE_LOOP.len();
            Self::map_user_memory(process.borrow().page_table, code, size)
        };

        println!("Entry point: {:#x}", entry_point);

        Self::create_thread(entry_point as *const usize, process, PrivilegeLevel::User)
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

    unsafe fn create_stack_frame(cs: u64, ss: u64, rip: u64) -> *mut Registers {
        let stack = Stack::new();
        let mut stack_top = stack.top();

        *stack_top = 0xDEADBEEFu64; // Dummy value to help with debugging
        stack_top = stack_top.offset(-1);

        let registers = (stack_top as usize - size_of::<Registers>()) as *mut Registers;

        (*registers).rip = rip;
        (*registers).rsp = stack_top as u64;
        (*registers).cs = cs;
        (*registers).ss = ss;
        (*registers).rflags = 0x202;

        print_stack(stack.stack_ptr, STACK_SIZE);

        registers
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
