use crate::{println, structures::DescriptorTablePointer};
use core::{mem::size_of, u32};

// External function to flush the old GDT and load the new one.
extern "C" {
    fn gdt_flush(gdt_ptr: &DescriptorTablePointer);
}

/// The GdtDescriptor struct represents a single entry in the Global Descriptor Table (GDT).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct GdtDescriptor {
    limit: u16,      // The lower 16 bits of the limit
    base_low: u16,   // The lower 16 bits of the base
    base_middle: u8, // The next 8 bits of the base
    flags: u8,       // Access flags
    granularity: u8, // Granularity flags
    base_high: u8,   // The last 8 bits of the base
}

impl GdtDescriptor {
    /// Creates a new GdtDescriptor struct with all fields set to 0.
    pub const fn new() -> GdtDescriptor {
        GdtDescriptor {
            limit: 0,
            base_low: 0,
            base_middle: 0,
            flags: 0,
            granularity: 0,
            base_high: 0,
        }
    }

    /// Sets the values of the GDT descriptor.
    pub fn set(&mut self, base: u64, limit: u32, flags: u8, granularity: u8) {
        self.base_low = (base & 0xffff) as u16;
        self.base_middle = ((base >> 16) & 0xff) as u8;
        self.base_high = ((base >> 24) & 0xff) as u8;
        self.limit = (limit & 0xffff) as u16;

        self.flags = flags;
        self.granularity = ((limit >> 16) & 0x0f) as u8;
        self.granularity |= granularity & 0xf0;
    }
}

/// The GlobalDescriptorTable struct represents the Global Descriptor Table (GDT).
#[repr(C)]
#[repr(align(0x1000))]
pub struct GlobalDescriptorTable {
    pub null: GdtDescriptor,
    pub kernel_code: GdtDescriptor,
    pub kernel_data: GdtDescriptor,
    pub user_null: GdtDescriptor,
    pub user_code: GdtDescriptor,
    pub user_data: GdtDescriptor,
}

impl GlobalDescriptorTable {
    pub const fn new() -> GlobalDescriptorTable {
        GlobalDescriptorTable {
            null: GdtDescriptor::new(),
            kernel_code: GdtDescriptor::new(),
            kernel_data: GdtDescriptor::new(),
            user_null: GdtDescriptor::new(),
            user_code: GdtDescriptor::new(),
            user_data: GdtDescriptor::new(),
        }
    }

    /// Returns a pointer to the GDT.
    pub fn get_pointer(&self) -> DescriptorTablePointer {
        DescriptorTablePointer {
            limit: size_of::<Self>() as u16 - 1,
            base: self as *const _ as u64,
        }
    }
}

/// The Global Descriptor Table (GDT).
static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();

/// Initializes the Global Descriptor Table (GDT).
pub fn gdt_init() {
    println!("Initializing GDT");

    unsafe {
        // Set up the GDT with the null segment, kernel code and data segments, and user code and data segments.
        GDT.null.set(0, 0, 0, 0); // Null segment
        GDT.kernel_code.set(0, 0, 0x9A, 0xA0); // Kernel code segment
        GDT.kernel_data.set(0, 0, 0x92, 0x00); // Kernel data segment
        GDT.user_null.set(0, 0, 0, 0); // Null segment
        GDT.user_code.set(0, 0, 0xFA, 0xA0); // User code segment
        GDT.user_data.set(0, 0, 0xF2, 0x00); // User data segment

        // Flush the old GDT and load the new one
        gdt_flush(&GDT.get_pointer());
    }

    println!("GDT initialized");
}
