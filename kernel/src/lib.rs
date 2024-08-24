#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(abi_x86_interrupt)] // enable x86 interrupts
#![feature(naked_functions)] // enable naked functions
#![feature(core_intrinsics)] // enable core intrinsics
#![feature(const_refs_to_cell)] // enable const references to UnsafeCell
#![feature(str_from_raw_parts)] // enable str::from_raw_parts

use acpi::rsdp;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use apic::APIC;
use core::{arch::asm, mem::size_of, panic::PanicInfo};
use drivers::screen::display::{self, DISPLAY};
use fs::fat32::{fat32_driver::CLUSTER_LAST, DirectoryEntry};
use interrupts::{
    isr::{self, KEYBOARD_IRQ},
    no_interrupts,
};
use memory::global_allocator::GlobalAllocator;
use storage::ahci::{self, AHCI_DEVICES};
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
mod fs;
mod gdt;
mod interrupts;
mod memory;
mod pci;
mod registers;
mod storage;
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

        apic::enable_apic_mode(); // enable the APIC mode
        APIC.lock().enable_irq(KEYBOARD_IRQ as u8); // enable the keyboard interrupt

        // test_proc();

        pci::PCI::scan();
        ahci::init();

        let ahci_device = &AHCI_DEVICES.lock()[0];
        let mut vfs = fs::VirtualFileSystem::new();
        vfs.mount(
            ahci_device.clone(),
            String::from("/"),
            String::from("FAT32"),
        );

        let fat_driver = vfs.get_driver("/").unwrap();

        fat_driver.create_dir("test_dir");
        fat_driver.create_file("test_dir/test_file.txt");

        let content = "Hello, World!";
        fat_driver.write_file(
            "test_dir/test_file.txt",
            content.as_ptr() as *mut u8,
            content.len(),
        );

        let entries = fat_driver.get_dir_entries(fat_driver.fs.root_dir_cluster);
        for entry in entries {
            let size = entry.entry.size;
            println!("Name: {}, size: {}", entry.name, size);
        }
    }

    loop {}
}

pub fn print_buffer_text(buffer: *mut u8, length: usize) {
    // Convert the raw pointer to a slice
    let buffer_slice = unsafe { core::slice::from_raw_parts(buffer, length) };

    // Try to convert the slice to a String
    match core::str::from_utf8(buffer_slice) {
        Ok(text) => {
            println!("{}", text); // Print valid UTF-8 text
        }
        Err(_) => {
            // Handle non-UTF-8 data by printing a replacement character for invalid sequences
            let valid_text = buffer_slice
                .iter()
                .map(|&b| if b.is_ascii() { b as char } else { '?' }) // Replace non-ASCII with '�'
                .collect::<String>();
            println!("{}", valid_text);
        }
    }
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
