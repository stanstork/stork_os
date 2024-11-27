use crate::{
    fs::vfs::FS,
    memory::{
        self,
        addr::VirtAddr,
        paging::{manager::PageTableManager, table::PageTable},
        PAGE_SIZE,
    },
    print, println,
};
use core::fmt::Debug;

const ELF_NIDENT: usize = 16; // Size of e_ident array in Elf64Ehdr

/// ELF header structure for 64-bit systems.
/// Located at the start of an ELF file, this header provides essential details
/// for identifying the file as an ELF executable and indicates its architecture,
/// entry point, and other structural information.
/// The operating system loader uses this header to determine how to interpret,
/// load, and execute the file.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Elf64Ehdr {
    pub e_ident: [u8; ELF_NIDENT],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

/// Program header structure for 64-bit ELF files.
/// Each program header defines a segment in the ELF file, specifying how and where
/// it should be loaded into memory during execution. Segments may contain code,
/// data, or dynamic linking information and are essential for the runtime setup.
/// Program headers are located at the offset specified by `e_phoff` in the ELF header,
/// and the size of each entry is defined by `e_phentsize` in the ELF header.
#[repr(C)]
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

enum ElfIdent {
    EI_MAG0 = 0,       // File identification byte 0, must be 0x7F
    EI_MAG1 = 1,       // File identification byte 1, must be 'E'
    EI_MAG2 = 2,       // File identification byte 2, must be 'L'
    EI_MAG3 = 3,       // File identification byte 3, must be 'F'
    EI_CLASS = 4,      // File class (32- or 64-bit)
    EI_DATA = 5,       // Data encoding (little or big endian)
    EI_VERSION = 6,    // ELF format version
    EI_OSABI = 7,      // Operating system/ABI identification
    EI_ABIVERSION = 8, // ABI version
    EI_PAD = 9,        // Start of padding bytes
}

enum PhdrType {
    PT_NULL = 0,                  // Unused entry
    PT_LOAD = 1,                  // Loadable segment
    PT_DYNAMIC = 2,               // Dynamic linking tables
    PT_INTERP = 3,                // Program interpreter path
    PT_NOTE = 4,                  // Note sections
    PT_SHLIB = 5,                 // Reserved
    PT_PHDR = 6,                  // Program header table
    PT_TLS = 7,                   // Thread-local storage segment
    PT_GNU_EH_FRAME = 0x6474E550, // GCC .eh_frame_hdr segment
    PT_GNU_STACK = 0x6474E551,    // Indicates stack executability
    PT_GNU_RELRO = 0x6474E552,    // Read-only after relocation
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

impl Debug for Elf64Phdr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Elf64Phdr")
            .field("p_type", &self.p_type)
            .field("p_flags", &self.p_flags)
            .field("p_offset", &self.p_offset)
            .field("p_vaddr", &self.p_vaddr)
            .field("p_paddr", &self.p_paddr)
            .field("p_filesz", &self.p_filesz)
            .field("p_memsz", &self.p_memsz)
            .field("p_align", &self.p_align)
            .finish()
    }
}

pub struct ElfLoader {}

impl ElfLoader {
    /// Loads an ELF file into memory and maps its segments to the provided page table.
    /// Returns the virtual address of the entry point if successful.
    pub fn load_elf(elf_file: &str, page_table: *mut PageTable) -> Option<(VirtAddr, usize)> {
        // Access the Virtual File System (VFS)
        let vfs = unsafe { FS.lock() };
        let size = vfs.size(elf_file).expect("Failed to get file size");

        // Allocate a buffer to store the ELF file contents
        let buffer_ptr = memory::allocate_dma_buffer(size);
        vfs.read_file(elf_file, buffer_ptr as *mut u8);

        println!("Loading ELF file: {} (size: {} bytes)", elf_file, size);

        // Parse the ELF header
        let elf_header = unsafe { &*(buffer_ptr as *const Elf64Ehdr) };

        // Verify the ELF header
        if !Self::validate_elf_header(elf_header) {
            println!("Invalid ELF header");
            return None;
        }

        let pt_manager = PageTableManager::new(page_table);

        // Iterate over the program headers to find loadable segments
        for i in 0..elf_header.e_phnum as usize {
            // Access the program header at the current index
            let phdr = unsafe {
                &*((buffer_ptr as usize
                    + elf_header.e_phoff as usize
                    + i * elf_header.e_phentsize as usize) as *const Elf64Phdr)
            };

            // Check if the segment is loadable
            if phdr.p_type == PhdrType::PT_LOAD as u32 {
                println!("Found loadable segment:");
                println!(
                    "  p_vaddr: {:#x}, p_memsz: {:#x}, p_filesz: {:#x}",
                    phdr.p_vaddr, phdr.p_memsz, phdr.p_filesz
                );

                // Calculate the number of pages required for the segment
                let pages = (phdr.p_memsz as usize + PAGE_SIZE - 1) / PAGE_SIZE;

                // Map each page of the segment to physical memory
                for i in 0..pages {
                    // Calculate the virtual and physical addresses for the current page
                    let virt_addr = phdr.p_paddr as usize + i * PAGE_SIZE;
                    let phys_addr = unsafe { pt_manager.alloc_zeroed_page() };

                    // Map the page to the virtual address
                    PageTableManager::map_user_page(page_table, virt_addr.into(), phys_addr);

                    // Copy the segment data to the physical address
                    let segment_offset = virt_addr - phdr.p_paddr as usize;
                    if segment_offset < phdr.p_filesz as usize {
                        let offset = phdr.p_offset as usize + segment_offset;
                        let size =
                            core::cmp::min(PAGE_SIZE, phdr.p_filesz as usize - segment_offset);

                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                (buffer_ptr as usize + offset) as *const u8,
                                phys_addr.as_mut_ptr(),
                                size,
                            );
                        }
                    }
                }
            }
        }

        // Return entry point address
        Some((elf_header.e_entry.into(), size))
    }

    fn validate_elf_header(elf_header: &Elf64Ehdr) -> bool {
        // Check the ELF magic number
        elf_header.e_ident[0..4] == [0x7F, b'E', b'L', b'F']
    }
}

// Dump the memory contents starting from the specified address
fn dump_memory(address: usize, size: usize) {
    let mut address = address;
    let mut i = 0;
    while i < size {
        let byte = unsafe { *(address as *const u8) };
        print!("{:02x} ", byte);
        address += 1;
        i += 1;
    }
}
