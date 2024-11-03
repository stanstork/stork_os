use core::fmt::Debug;

use crate::{
    fs::vfs::FS,
    memory::{self, addr::VirtAddr},
    println,
};

const ELF_NIDENT: usize = 16; // Size of e_ident array in Elf64Ehdr

/// ELF header structure for 64-bit systems.
/// Located at the start of an ELF file, this header provides essential details
/// for identifying the file as an ELF executable and indicates its architecture,
/// entry point, and other structural information.
/// The operating system loader uses this header to determine how to interpret,
/// load, and execute the file.
pub struct Elf64Ehdr {
    pub e_ident: [u8; ELF_NIDENT], // Identification bytes marking this as an ELF file
    pub e_type: u16,               // File type (e.g., executable, shared object)
    pub e_machine: u16,            // Target machine architecture
    pub e_version: u32,            // ELF format version
    pub e_entry: u64,              // Entry point address for execution
    pub e_phoff: u64,              // Offset to the program header table
    pub e_shoff: u64,              // Offset to the section header table
    pub e_flags: u32,              // Processor-specific flags
    pub e_ehsize: u16,             // Size of this ELF header
    pub e_phentsize: u16,          // Size of each program header entry
    pub e_phnum: u16,              // Number of entries in the program header table
    pub e_shentsize: u16,          // Size of each section header entry
    pub e_shnum: u16,              // Number of entries in the section header table
    pub e_shstrndx: u16,           // Index of the section name string table
}

/// Program header structure for 64-bit ELF files.
/// Each program header defines a segment in the ELF file, specifying how and where
/// it should be loaded into memory during execution. Segments may contain code,
/// data, or dynamic linking information and are essential for the runtime setup.
/// Program headers are located at the offset specified by `e_phoff` in the ELF header,
/// and the size of each entry is defined by `e_phentsize` in the ELF header.
pub struct Elf64Phdr {
    pub p_type: u32,   // Segment type (e.g., loadable, dynamic linking)
    pub p_flags: u32,  // Segment-specific flags (e.g., readable, writable)
    pub p_offset: u64, // Offset of the segment within the file
    pub p_vaddr: u64,  // Virtual address where the segment is loaded in memory
    pub p_paddr: u64,  // Physical address (not commonly used)
    pub p_filesz: u64, // Size of the segment within the file
    pub p_memsz: u64,  // Size of the segment in memory after loading
    pub p_align: u64,  // Alignment required for the segment
}

impl Debug for Elf64Ehdr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Elf64Ehdr")
            .field("e_ident", &self.e_ident)
            .field("e_type", &self.e_type)
            .field("e_machine", &self.e_machine)
            .field("e_version", &self.e_version)
            .field("e_entry", &self.e_entry)
            .field("e_phoff", &self.e_phoff)
            .field("e_shoff", &self.e_shoff)
            .field("e_flags", &self.e_flags)
            .field("e_ehsize", &self.e_ehsize)
            .field("e_phentsize", &self.e_phentsize)
            .field("e_phnum", &self.e_phnum)
            .field("e_shentsize", &self.e_shentsize)
            .field("e_shnum", &self.e_shnum)
            .field("e_shstrndx", &self.e_shstrndx)
            .finish()
    }
}

pub struct ElfLoader {}

impl ElfLoader {
    pub fn load_file(file_name: &str) -> VirtAddr {
        let vfs = unsafe { FS.lock() };
        let size = vfs.size(file_name).unwrap();

        println!("Loading ELF file: {} (size: {} bytes)", file_name, size);

        let buffer = memory::allocate_dma_buffer(size) as *mut u8;

        vfs.read_file(file_name, buffer);

        let elf_header = unsafe { &*(buffer as *const Elf64Ehdr) };
        println!("ELF header: {:#?}", elf_header);

        todo!("Implement ELF file loading")
    }
}
