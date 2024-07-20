use super::id::{IdAllocator, Pid, Tid};
use crate::{
    cpu::gdt::PrivilegeLevel,
    memory::{
        addr::{PhysAddr, VirtAddr},
        paging::{
            page_table_manager::{self, PageTableManager},
            table::{PageEntry, PageEntryFlags, PageTable},
            ROOT_PAGE_TABLE,
        },
        PAGE_FRAME_ALLOCATOR, PAGE_SIZE,
    },
    print, println,
    registers::cr3::Cr3,
    ALLOCATOR,
};
use alloc::{alloc::alloc_zeroed, rc::Rc, vec::Vec};
use core::{alloc::Layout, cell::RefCell, mem::size_of, ptr::copy_nonoverlapping};

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
    pub(crate) page_table: *mut PageTable,
    pub(crate) threads: Vec<Rc<RefCell<Thread>>>,
}

pub struct Thread {
    pub tid: Tid,
    pub process: Rc<RefCell<Process>>,
    pub(crate) stack_pointer: u64,
}

extern "C" fn idle_thread() {
    println!("Idle thread running");
    loop {}
}

extern "C" {
    fn start_thread(stack_pointer: u64);
}

impl Process {
    pub fn create_kernel_process() -> Self {
        let root_page_table = unsafe { &mut *(ROOT_PAGE_TABLE as *mut PageTable) };
        let page_table = root_page_table as *mut PageTable;

        println!("Root page table: {:#x}", page_table as usize);

        let process = Process {
            pid: Pid::next(),
            page_table,
            threads: Vec::new(),
        };

        // let thread = Thread::create_idle_thread(Rc::clone(&Rc::new(RefCell::new(process))));

        Self {
            pid: Pid::next(),
            page_table,
            threads: alloc::vec![],
        }
    }

    pub fn create_user_process() -> Rc<RefCell<Process>> {
        let root_page_table = unsafe { &mut *(ROOT_PAGE_TABLE as *mut PageTable) };
        let new_page_table =
            unsafe { PageTableManager::clone_pml4(root_page_table as *mut PageTable) };

        println!("New Page Table: {:#x}", new_page_table as usize);

        let process = Process {
            pid: Pid::next(),
            page_table: new_page_table,
            threads: Vec::new(),
        };

        let proc = Rc::new(RefCell::new(process));
        let cloned_proc = Rc::clone(&proc);

        let thread = Thread::create_user_thread(cloned_proc);

        proc.borrow_mut()
            .threads
            .push(Rc::new(RefCell::new(thread)));

        proc
    }
}

impl Thread {
    pub fn create_user_thread(process: Rc<RefCell<Process>>) -> Self {
        let stack = unsafe { ALLOCATOR.alloc_page() };
        let mut stack_top = unsafe { stack.add(STACK_SIZE) } as *mut u64;

        let entry_point = unsafe {
            let code = INFINITE_LOOP.as_ptr() as *const usize;
            let size = INFINITE_LOOP.len();
            Self::map_user_memory(process.borrow().page_table, code, size)
        };

        println!("Entry point: {:#x}", entry_point);

        unsafe {
            stack_top = stack_top.offset(-1);
            *stack_top = 0x23; // ss
            stack_top = stack_top.offset(-1);
            *stack_top = stack_top as u64; // rsp (user stack pointer)
            stack_top = stack_top.offset(-1);
            *stack_top = 0x202; // rflags
            stack_top = stack_top.offset(-1);
            *stack_top = 0x1B; // cs
            stack_top = stack_top.offset(-1);
            *stack_top = entry_point as u64; // rip

            // Push general-purpose registers in reverse order
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // rax
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // rcx
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // rdx
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // rbx
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // rbp
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // rsi
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // rdi
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // r8
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // r9
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // r10
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // r11
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // r12
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // r13
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // r14
            stack_top = stack_top.offset(-1);
            *stack_top = 0; // r15

            // Align stack to 16 bytes
            stack_top = (stack_top as usize & !0xF) as *mut u64;

            println!("Stack top: {:#x}", stack_top as usize);

            print_stack(stack, STACK_SIZE);

            let thread = Thread {
                tid: Tid::next(),
                process,
                stack_pointer: stack_top as u64,
            };
            thread
        }
    }

    pub fn exec(&self) {
        let process = self.process.borrow();
        let page_table = process.page_table;
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

        println!("Virtual address: {:#x}", virt_addr as usize);
        println!("Physical address: {:#x}", phys_addr.0 as usize);

        copy_nonoverlapping(address as *const u8, virt_addr as *mut u8, size);

        PageTableManager::set_user_accessible(
            page_table,
            VirtAddr(virt_addr as usize),
            PhysAddr(phys_addr.0 as usize),
        );

        virt_addr as usize
    }
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
