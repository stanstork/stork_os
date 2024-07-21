use super::tss::TssDescriptor;
use crate::structures::DescriptorTablePointer;
use bitfield_struct::bitfield;

// External assembly function to flush the GDT.
extern "C" {
    fn gdt_flush(ptr: &DescriptorTablePointer);
}

/// Represents the CPU privilege levels in the operating system.
///
/// Privilege levels determine the permissions and access rights of code
/// executing at different levels. The lower the privilege level, the more
/// permissions the code has. Typically, there are two primary levels:
///
/// - `Kernel`: Represents the highest privilege level (0). Code running at this
///   level has full access to all hardware and memory.
/// - `User`: Represents a lower privilege level (3). Code running at this level
///   has restricted access, typically to ensure system stability and security.
///
/// The `PrivilegeLevel` enum maps these levels to their corresponding bit
/// representations used by the CPU.
#[derive(Default, Debug, Clone, Copy)]
#[repr(u16)]
pub(crate) enum PrivilegeLevel {
    #[default]
    Kernel = 0,
    User = 3,
}

impl PrivilegeLevel {
    /// Converts the `PrivilegeLevel` enum variant into its corresponding bit representation.
    pub const fn into_bits(self) -> u16 {
        self as u16
    }

    /// Converts a bit value into a `PrivilegeLevel` enum variant.
    ///
    /// # Panics
    /// Panics if the value does not correspond to a valid `PrivilegeLevel`.
    pub const fn from_bits(value: u16) -> Self {
        match value {
            0 => PrivilegeLevel::Kernel,
            3 => PrivilegeLevel::User,
            _ => panic!("Invalid privilege level"),
        }
    }
}

/// Represents a segment selector used in segment-based memory addressing.
///
/// A segment selector contains the index of a segment descriptor within the
/// Global Descriptor Table (GDT) or Local Descriptor Table (LDT), as well as
/// the privilege level of the segment. The selector is used by the CPU to
/// access segment descriptors efficiently.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct SegmentSelector(pub u16);

impl SegmentSelector {
    /// Creates a new `SegmentSelector` with the specified index and privilege level.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the segment descriptor in the GDT or LDT.
    /// * `privilege_level` - The privilege level of the segment (Kernel or User).
    ///
    /// # Returns
    ///
    /// A new `SegmentSelector` instance with the provided index and privilege level.
    pub fn new(index: u16, privilege_level: PrivilegeLevel) -> Self {
        SegmentSelector(index << 3 | privilege_level.into_bits())
    }
}

/// Represents the type of a segment in the GDT/LDT.
///
/// Segment types are used to define the nature and behavior of a segment, such
/// as whether it is a code segment, a data segment, or a task segment.
#[derive(Default, Debug, Clone, Copy)]
#[repr(u8)]
pub enum SegmentType {
    /// No segment type.
    #[default]
    None = 0,
    /// Code segment.
    Code = 26, // 0b11010
    /// Code segment that has been accessed.
    CodeAccessed = 27, // 0b11011
    /// Data segment.
    Data = 18, // 0b10010
    /// Data segment that has been accessed.
    DataAccessed = 19, // 0b10011
    /// Task segment.
    Task = 9, // 0b01001
}

impl SegmentType {
    /// Converts the `SegmentType` enum variant into its corresponding bit representation.
    pub const fn into_bits(self) -> u8 {
        self as u8
    }

    /// Converts a bit value into a `SegmentType` enum variant.
    ///
    /// # Panics
    /// Panics if the value does not correspond to a valid `SegmentType`.
    pub const fn from_bits(value: u8) -> Self {
        match value {
            0 => SegmentType::None,
            26 => SegmentType::Code,
            27 => SegmentType::CodeAccessed,
            18 => SegmentType::Data,
            19 => SegmentType::DataAccessed,
            9 => SegmentType::Task,
            _ => panic!("Invalid segment type"),
        }
    }
}
/// Represents a segment descriptor in the GDT/LDT.
///
/// A segment descriptor defines the properties of a memory segment, including
/// its base address, limit, type, privilege level, and other attributes.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SegmentDescriptor {
    pub limit_low: u16,
    pub base_low: u16,
    pub base_middle: u8,
    pub attributes: SegmentAttributes,
    pub base_high: u8,
}

impl SegmentDescriptor {
    /// Creates a null segment descriptor.
    ///
    /// A null segment descriptor is used to represent an invalid or unused segment.
    #[inline]
    const fn null() -> Self {
        Self::new(0, SegmentType::None, PrivilegeLevel::Kernel, true, false)
    }

    /// Creates a kernel code segment descriptor.
    #[inline]
    const fn kernel_code() -> Self {
        Self::new_from_type(SegmentType::Code, PrivilegeLevel::Kernel)
    }

    /// Creates a kernel data segment descriptor.
    #[inline]
    const fn kernel_data() -> Self {
        Self::new_from_type(SegmentType::Data, PrivilegeLevel::Kernel)
    }

    /// Creates a user code segment descriptor.
    #[inline]
    const fn user_code() -> Self {
        Self::new_from_type(SegmentType::Code, PrivilegeLevel::User)
    }

    /// Creates a user data segment descriptor.
    #[inline]
    const fn user_data() -> Self {
        Self::new_from_type(SegmentType::Data, PrivilegeLevel::User)
    }

    /// Creates a new segment descriptor with the specified attributes.
    ///
    /// # Arguments
    ///
    /// * `limit_low` - The lower 16 bits of the segment limit.
    /// * `segment_type` - The type of the segment (e.g., code, data).
    /// * `dpl` - The Descriptor Privilege Level (DPL) of the segment.
    /// * `present` - Whether the segment is present in memory.
    /// * `long_mode` - Whether the segment operates in long mode (64-bit).
    #[inline]
    const fn new(
        limit_low: u16,
        segment_type: SegmentType,
        dpl: PrivilegeLevel,
        present: bool,
        long_mode: bool,
    ) -> Self {
        Self {
            limit_low,
            base_low: 0,
            base_middle: 0,
            attributes: SegmentAttributes::new()
                .with_segment_type(segment_type)
                .with_dpl(dpl)
                .with_present(present)
                .with_long_mode(long_mode),
            base_high: 0,
        }
    }

    /// Creates a new segment descriptor based on the segment type and privilege level.
    ///
    /// This method simplifies the creation of common segment descriptors by
    /// setting the appropriate flags based on the segment type.
    ///
    /// # Arguments
    ///
    /// * `segment_type` - The type of the segment (e.g., code, data).
    /// * `dpl` - The Descriptor Privilege Level (DPL) of the segment.
    #[inline]
    const fn new_from_type(segment_type: SegmentType, dpl: PrivilegeLevel) -> Self {
        let long_mode = matches!(segment_type, SegmentType::Code);
        Self::new(0, segment_type, dpl, true, long_mode)
    }
}

/// Represents the attributes of a segment descriptor in the GDT/LDT.
///
/// The segment attributes are encoded in a 16-bit value, including the segment
/// type, privilege level, and other properties. These attributes define how
/// the segment can be accessed and used by the CPU.
#[bitfield(u16)]
pub struct SegmentAttributes {
    /// Segment type (5 bits).
    #[bits(5)]
    pub segment_type: SegmentType,
    /// Descriptor Privilege Level (DPL) (2 bits).
    #[bits(2)]
    pub dpl: PrivilegeLevel,
    /// Segment present flag (1 bit).
    pub present: bool,
    /// High bits of the segment limit (4 bits).
    #[bits(4)]
    pub limit_high: u8,
    /// Available for use by system software (AVL) flag (1 bit).
    pub avl: bool,
    /// Long mode flag (1 bit).
    pub long_mode: bool,
    /// Default operation size flag (1 bit).
    pub default: bool,
    /// Granularity flag (1 bit).
    pub granularity: bool,
}

/// Represents the Global Descriptor Table (GDT).
#[derive(Debug, Clone)]
#[repr(C, packed)]
pub struct GlobalDescriptorTable {
    null: SegmentDescriptor,
    kernel_code: SegmentDescriptor,
    kernel_data: SegmentDescriptor,
    user_code: SegmentDescriptor,
    user_data: SegmentDescriptor,
    pub tss: TssDescriptor,
}

impl GlobalDescriptorTable {
    /// Creates a new Global Descriptor Table (GDT) with default segment descriptors.
    ///
    /// This GDT includes a null segment, kernel code and data segments, user code and data segments, and a TSS.
    pub const fn new() -> Self {
        Self {
            null: SegmentDescriptor::null(),
            kernel_code: SegmentDescriptor::kernel_code(),
            kernel_data: SegmentDescriptor::kernel_data(),
            user_code: SegmentDescriptor::user_code(),
            user_data: SegmentDescriptor::user_data(),
            tss: TssDescriptor::null(),
        }
    }

    /// Returns a pointer to the GDT.
    ///
    /// This pointer includes the limit (size of the GDT minus 1) and the base address of the GDT.
    pub fn get_pointer(&self) -> DescriptorTablePointer {
        DescriptorTablePointer {
            limit: (core::mem::size_of_val(self) - 1) as u16,
            base: self as *const _ as u64,
        }
    }

    /// Flushes the GDT to the CPU.
    ///
    /// This method loads the GDT into the CPU, making it the active GDT. This is necessary for the CPU
    /// to recognize and use the new GDT. The function `gdt_flush` is an external assembly function
    /// that performs the actual loading of the GDT.
    pub(super) fn flush(&self) {
        unsafe {
            let ptr = self.get_pointer();
            gdt_flush(&ptr);
        }
    }
}

/// The Global Descriptor Table (GDT) for the operating system.
pub static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();

/// Initializes the GDT by flushing it to the CPU.
pub fn init() {
    unsafe {
        GDT.flush();
    }
}
