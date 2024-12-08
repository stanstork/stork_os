# stork_os

Hobby operating system written in Rust. <br>
This OS aims to do everything your existing OS already does - but slower and buggier.<br>
Designed for the x86_64 architecture.

## Current features
<b>Multitasking</b>: Processes, threads, and context switching.<br>
<b>Memory Management</b>: Physical and virtual memory allocators.<br>
<b>Interrupt Handling</b>: GDT, IDT, TSS, and timer interrupts.<br>
<b>Syscalls</b>: Basic syscall support.<br>
<b>Filesystem</b>: VFS and FAT32 support.<br>
<b>Storage</b>: AHCI driver for SATA and PCI device scanning.<br>
<b>ELF Loader</b>: Loads and executes user applications.<br>
<b>Custom std Library</b>: Minimal library for OS functionality.<br>
<b>Hardware Abstraction</b>: ACPI, APIC, and IOAPIC configuration.<br>

## Future Development
1. Networking (of course the hobby OS needs a browser).
2. Improved user-mode applications and multitasking.