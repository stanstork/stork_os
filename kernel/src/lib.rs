#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(abi_x86_interrupt)] // enable x86 interrupts
#![feature(ptr_internals)] // enable pointer internals
#![feature(const_trait_impl)] // enable const trait impl
#![feature(effects)] // enable effects
#![feature(naked_functions)] // enable naked functions
#![feature(ptr_metadata)] // enable pointer metadata
#![feature(exposed_provenance)] // enable exposed provenance
#![feature(asm_const)] // enable asm const
#![feature(core_intrinsics)] // enable core intrinsics
#![feature(const_mut_refs)]
#![feature(sync_unsafe_cell)]
#![feature(const_size_of_val)]

use crate::{
    cpu::interrupts::{disable_interrupts, enable_interrupts},
    interrupts::isr::idt_init,
};
use core::{
    alloc::{GlobalAlloc, Layout},
    arch::asm,
    panic::PanicInfo,
};
use cpu::{gdt::GDTR, tss};
use drivers::screen::display::Display;
use memory::{
    addr::VirtAddr,
    global_allocator::GlobalAllocator,
    paging::{
        page_table_manager::{self, PageTableManager},
        table::PageTable,
        ROOT_PAGE_TABLE,
    },
    PAGE_FRAME_ALLOCATOR,
};
use structures::BootInfo;

extern crate alloc;

mod cpu;
mod data_types;
mod drivers;
mod interrupts;
mod memory;
mod registers;
mod structures;

// The `#[global_allocator]` attribute is used to designate a specific allocator as the global memory allocator for the Rust program.
// When this attribute is used, Rust will use the specified allocator for all dynamic memory allocations throughout the program.
#[global_allocator]
static mut ALLOCATOR: GlobalAllocator = GlobalAllocator::new();

pub(crate) static mut BOOT_INFO: Option<&'static BootInfo> = None;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    disable_interrupts();

    Display::init_display(&boot_info.framebuffer, &boot_info.font);

    cls!(); // clear the screen
    println!("Welcome to the StorkOS!"); // print a welcome message

    GDTR.load(); // initialize the Global Descriptor Table
    idt_init(); // initialize the Interrupt Descriptor Table

    // initialize the memory
    unsafe { memory::init(boot_info) };

    enable_interrupts();

    unsafe { tss::load_task_state_segment() };

    unsafe { switch_to_user_mode() };

    loop {}
}

// this function is called on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("Panic: {}", _info);
    loop {}
}

pub fn allocate_user_stack() -> u64 {
    // Define the stack size
    const STACK_SIZE: usize = 0x4000; // 16 KB

    // Allocate memory for the stack using the heap allocator
    let layout = Layout::from_size_align(STACK_SIZE, 4096).unwrap();
    let user_stack_address = unsafe {
        PAGE_FRAME_ALLOCATOR
            .as_mut()
            .unwrap()
            .alloc_pages(layout)
            .unwrap()
    };
    let virt_addr = user_stack_address.0;

    let mut page_table_manager =
        PageTableManager::new(unsafe { ROOT_PAGE_TABLE } as *mut PageTable);
    unsafe {
        page_table_manager.map_memory(
            VirtAddr(virt_addr),
            user_stack_address,
            PAGE_FRAME_ALLOCATOR.as_mut().unwrap(),
        )
    };

    // Return the top of the stack (stack grows downwards)
    user_stack_address.0 as u64 + STACK_SIZE as u64
}

pub unsafe fn switch_to_user_mode() {
    let user_stack: u64 = allocate_user_stack(); // Use the function to allocate user stack
    let user_rip: u64 = user_mode_entry as u64;

    asm!(
        "cli",                           // Disable interrupts
                                // Load the stack segment selector for user mode
        "mov rsp, {0}",                  // Load the stack pointer with the user mode stack address
        "push 0x23",                     // Push the data segment selector for user mode
        "push {0}",                      // Push the user mode stack address
        "pushf",                         // Push the flags register
        "pop rax",                       // Pop flags into rax to modify
        "or rax, 0x200",                 // Set the interrupt enable flag (IF)
        "push rax",                      // Push the modified flags
        "push 0x1B",                     // Push the code segment selector for user mode
        "push {1}",                      // Push the user mode instruction pointer
        "iretq",                         // Interrupt return to switch to user mode
        in(reg) user_stack,
        in(reg) user_rip,
        options(noreturn)
    );
}

// #[no_mangle]
extern "C" fn user_mode_entry() -> ! {
    // This is the entry point for user mode.
    // Write user mode code here.

    loop {
        // User mode code goes here.
    }
}
