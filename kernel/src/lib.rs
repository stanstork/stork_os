#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(abi_x86_interrupt)] // enable x86 interrupts
#![feature(naked_functions)] // enable naked functions
#![feature(core_intrinsics)] // enable core intrinsics

use core::{arch::asm, panic::PanicInfo};
use drivers::screen::display::{self};
use interrupts::{isr, no_interrupts};
use memory::global_allocator::GlobalAllocator;
use process::{
    process::{Priority, Process},
    schedule,
    scheduler::{Scheduler, SCHEDULER},
    KERNEL_STACK_SIZE, KERNEL_STACK_START,
};
use structures::BootInfo;

extern crate alloc;

mod cpu;
mod data_types;
mod drivers;
mod gdt;
mod interrupts;
mod memory;
mod process;
mod registers;
mod structures;
mod tss;

// The `#[global_allocator]` attribute is used to designate a specific allocator as the global memory allocator for the Rust program.
// When this attribute is used, Rust will use the specified allocator for all dynamic memory allocations throughout the program.
#[global_allocator]
static mut ALLOCATOR: GlobalAllocator = GlobalAllocator::new();

pub const STACK_SIZE: usize = 0x4000; // 16 KB
pub static mut INITIAL_RSP: u64 = 0;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    unsafe {
        asm!("mov {}, rsp", out(reg) INITIAL_RSP);

        no_interrupts(|| {
            display::init(&boot_info.framebuffer, &boot_info.font);

            cls!(); // clear the screen
            println!("Welcome to the StorkOS!"); // print a welcome message

            gdt::init(); // initialize the Global Descriptor Table
            isr::init(); // initialize the Interrupt Descriptor Table

            // initialize the memory
            memory::init(boot_info);
            tss::load_tss();

            // test_proc();
        });

        test_proc();
    }

    loop {}
}

// this function is called on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("Panic: {}", _info);
    loop {}
}

pub unsafe fn test_proc() {
    process::move_stack(KERNEL_STACK_START as *mut u8, KERNEL_STACK_SIZE as u64);

    let proc1 = Process::create_kernel_process(test_thread1, Priority::Medium);
    println!("Process 1 created");
    let proc2 = Process::create_kernel_process(test_thread2, Priority::Medium);
    println!("Process 2 created");

    let mut scheduler = Scheduler::new();

    scheduler.add_thread(proc1.borrow().threads[0].clone());
    scheduler.add_thread(proc2.borrow().threads[0].clone());

    SCHEDULER = Some(scheduler);

    proc2.borrow().threads[0].borrow().exec();
    // schedule();
}

extern "C" fn test_thread1() {
    loop {
        println!("Thread 111111 running");
    }
}

extern "C" fn test_thread2() {
    loop {
        println!("Thread 222222 running");
    }
}
