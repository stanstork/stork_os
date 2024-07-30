#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(abi_x86_interrupt)] // enable x86 interrupts
#![feature(naked_functions)] // enable naked functions
#![feature(core_intrinsics)] // enable core intrinsics
#![feature(const_refs_to_cell)] // enable const references to UnsafeCell

use acpi::rsdp;
use apic::{Apic, APIC};
use core::{arch::asm, panic::PanicInfo};
use drivers::screen::display::{self, DISPLAY};
use interrupts::{
    isr::{self, IDT},
    no_interrupts,
};
use memory::global_allocator::GlobalAllocator;
use structures::BootInfo;
use tasks::{
    process::Process,
    scheduler::{Scheduler, SCHEDULER},
    thread::Priority,
    KERNEL_STACK_SIZE, KERNEL_STACK_START,
};

extern crate alloc;

mod acpi;
mod apic;
mod arch;
mod cpu;
mod data_types;
mod drivers;
mod gdt;
mod interrupts;
mod memory;
mod registers;
mod structures;
mod sync;
mod tasks;
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

            rsdp::init_rsdp(boot_info);
            // apic::setup_apic();
            println!("APIC initialized");
        });

        // IDT.disable_pic_interrupt(2);
        let apic = Apic::init();
        APIC = Some(apic);
        APIC.as_ref().unwrap().ioapic.enable_irq(1);
        APIC.as_ref().unwrap().ioapic.enable_irq(0);

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
    tasks::move_stack(KERNEL_STACK_START as *mut u8, KERNEL_STACK_SIZE as u64);

    let proc1 = Process::create_kernel_process(test_thread1, Priority::Medium);
    println!("Process 1 created");
    let proc2 = Process::create_kernel_process(test_thread2, Priority::Medium);
    println!("Process 2 created");

    let mut scheduler = Scheduler::new();

    scheduler.add_thread(proc1.borrow().threads[0].borrow().clone());
    scheduler.add_thread(proc2.borrow().threads[0].borrow().clone());

    SCHEDULER = Some(scheduler);
}

extern "C" fn test_thread1() {
    let color_on: u32 = 0xFF00FF00; // Green color
    let color_off: u32 = 0xFFFFFFFF; // White color
    let size: usize = 50;
    loop {
        unsafe {
            DISPLAY.draw_square(0, 0, size, color_on);
            // Simulate some work
            for _ in 0..100_000 {
                asm!("nop");
            }
            DISPLAY.draw_square(0, 0, size, color_off);
            // Simulate some work
            for _ in 0..100_000 {
                asm!("nop");
            }
        }
    }
}

extern "C" fn test_thread2() {
    let color_on: u32 = 0xFFFF0000; // Red color
    let color_off: u32 = 0xFF0000FF; // Blue color
    let size: usize = 50;
    loop {
        unsafe {
            DISPLAY.draw_square(0, 105, size, color_on);
            // Simulate some work
            for _ in 0..100_000 {
                asm!("nop");
            }
            DISPLAY.draw_square(0, 105, size, color_off);
            // Simulate some work
            for _ in 0..100_000 {
                asm!("nop");
            }
        }
    }
}
