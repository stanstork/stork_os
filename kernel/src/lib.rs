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
use alloc::boxed::Box;
use core::{
    alloc::{GlobalAlloc, Layout},
    arch::asm,
    mem::size_of,
    panic::PanicInfo,
    ptr::{self, write_bytes},
};
use cpu::{gdt::GDTR, tss};
use drivers::screen::display::Display;
use memory::{
    addr::{PhysAddr, VirtAddr},
    global_allocator::GlobalAllocator,
    paging::{
        page_table_manager::{self, PageTableManager},
        table::PageTable,
        ROOT_PAGE_TABLE,
    },
    PAGE_FRAME_ALLOCATOR, PAGE_SIZE,
};
use registers::cr3::Cr3;
use structures::BootInfo;
use task::{create_kernel_task, create_user_task, switch_task};

extern crate alloc;

mod cpu;
mod data_types;
mod drivers;
mod interrupts;
mod memory;
mod registers;
mod structures;
mod task;

// The `#[global_allocator]` attribute is used to designate a specific allocator as the global memory allocator for the Rust program.
// When this attribute is used, Rust will use the specified allocator for all dynamic memory allocations throughout the program.
#[global_allocator]
static mut ALLOCATOR: GlobalAllocator = GlobalAllocator::new();

pub(crate) static mut BOOT_INFO: Option<&'static BootInfo> = None;

pub const KERNEL_STACK_SIZE: usize = 0x2000; // 8 KB
pub const KERNEL_STACK_START: u64 = 0x000700000000000; // 128 TB
pub const STACK_SIZE: usize = 0x4000; // 16 KB

pub static mut INITIAL_RSP: u64 = 0;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    unsafe {
        asm!("mov {}, rsp", out(reg) INITIAL_RSP);
    }

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

    unsafe { BOOT_INFO = Some(boot_info) };

    unsafe { proc_prep() };

    loop {}
}

// this function is called on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("Panic: {}", _info);
    loop {}
}

pub unsafe fn proc_prep() {
    let mut kernel_task = create_kernel_task(kernel_mode_entry as u64);
    let user_task = create_user_task(user_mode_entry as u64);

    switch_task(&mut kernel_task, &user_task);
}

// Moves the stack to a new location.
// https://web.archive.org/web/20160326122214/http://jamesmolloy.co.uk/tutorial_html/9.-Multitasking.html
fn move_stack(new_stack_start: *mut u8, size: u64) {
    let root_page_table = unsafe { &mut *(ROOT_PAGE_TABLE as *mut PageTable) };
    let mut page_table_manager = PageTableManager::new(root_page_table);
    let mut frame_alloc = || unsafe {
        PAGE_FRAME_ALLOCATOR
            .as_mut()
            .unwrap()
            .alloc_page()
            .unwrap()
            .0
    } as *mut PageTable;

    unsafe {
        let mut i = new_stack_start as u64;
        while i >= (new_stack_start as u64 - size) {
            let addr = PAGE_FRAME_ALLOCATOR
                .as_mut()
                .unwrap()
                .alloc_page()
                .unwrap()
                .0;
            page_table_manager.map_memory(
                VirtAddr(i as usize),
                PhysAddr(addr),
                &mut frame_alloc,
                true,
            );
            i = i.wrapping_sub(0x1000);
        }

        // Flush the TLB by reading and writing the page directory address again.
        let cr3 = Cr3::read();
        Cr3::write(cr3 as u64);

        let old_stack_pointer: u64;
        asm!("mov {}, rsp", out(reg) old_stack_pointer);

        let old_base_pointer: u64;
        asm!("mov {}, rbp", out(reg) old_base_pointer);

        let offset = new_stack_start as u64 - INITIAL_RSP;
        let new_stack_pointer = old_stack_pointer + offset;
        let new_base_pointer = old_base_pointer + offset;

        copy_nonoverlapping(
            old_stack_pointer as *const u8,
            new_stack_pointer as *mut u8,
            (INITIAL_RSP - old_stack_pointer) as usize,
        );

        // Backtrace through the original stack, copying new values into the new stack.
        let mut i = new_stack_start as u64;
        while i > new_stack_start as u64 - size {
            let tmp = *(i as *const u64);

            if old_stack_pointer < tmp && tmp < INITIAL_RSP {
                let new_tmp = tmp + offset;
                *(i as *mut u64) = new_tmp;
            }

            i = i.wrapping_sub(8);
        }

        // Change stacks.
        asm!("mov rsp, {}", in(reg) new_stack_pointer);
        asm!("mov rbp, {}", in(reg) new_base_pointer);
    }
}

fn copy_nonoverlapping(src: *const u8, dst: *mut u8, len: usize) {
    unsafe {
        let mut i = 0;
        while i < len {
            *dst.add(i) = *src.add(i);
            i += 1;
        }
    }
}

extern "C" fn user_mode_entry() -> ! {
    println!("Switched to user mode!");
    loop {}
}

extern "C" fn kernel_mode_entry() -> ! {
    println!("Switched to kernel mode!");
    loop {}
}
