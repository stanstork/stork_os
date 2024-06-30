use bitfield_struct::bitfield;
use core::{arch::asm, cell::SyncUnsafeCell};

#[derive(Default, Debug, Clone, Copy)]
#[repr(u16)]
pub enum PrivilegeLevel {
    #[default]
    Supervisor = 0,
    User = 3,
}

impl PrivilegeLevel {
    pub const fn into_bits(self) -> u16 {
        self as u16
    }

    pub const fn from_bits(value: u16) -> Self {
        match value {
            0 => PrivilegeLevel::Supervisor,
            3 => PrivilegeLevel::User,
            _ => panic!("Invalid privilege level"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct SegmentSelector(pub u16);

impl SegmentSelector {
    pub fn new(index: u16, privilege_level: PrivilegeLevel) -> Self {
        SegmentSelector(index << 3 | privilege_level.into_bits())
    }

    pub fn index(self) -> u16 {
        self.0 >> 3
    }

    pub fn privilege_level(self) -> PrivilegeLevel {
        PrivilegeLevel::from_bits(self.0 & 0b11)
    }
}

impl From<SegmentSelector> for u64 {
    #[inline]
    fn from(value: SegmentSelector) -> Self {
        value.0 as _
    }
}

#[derive(Default, Debug, Clone, Copy)]
#[repr(u8)]
pub enum SegmentType {
    #[default]
    None = 0b0,
    Code = 0b11010,
    CodeAccessed = 0b11011,
    Data = 0b10010,
    DataAccessed = 0b10011,
    Task = 0b01001,
}

impl SegmentType {
    pub const fn into_bits(self) -> u8 {
        self as u8
    }

    pub const fn from_bits(value: u8) -> Self {
        match value {
            0b0 => SegmentType::None,
            0b11010 => SegmentType::Code,
            0b11011 => SegmentType::CodeAccessed,
            0b10010 => SegmentType::Data,
            0b10011 => SegmentType::DataAccessed,
            0b01001 => SegmentType::Task,
            _ => panic!("Invalid segment type"),
        }
    }
}

#[bitfield(u16)]
pub struct SegmentAttributes {
    #[bits(5)]
    pub segment_type: SegmentType,
    #[bits(2)]
    pub dpl: PrivilegeLevel,
    pub present: bool,
    #[bits(4)]
    pub limit_high: u8,
    pub avl: bool,
    pub long_mode: bool,
    pub default: bool,
    pub granularity: bool,
}

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
    #[inline]
    const fn null() -> Self {
        Self::new(
            0,
            SegmentType::None,
            PrivilegeLevel::Supervisor,
            true,
            false,
        )
    }

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

    #[inline]
    const fn new_from_type(segment_type: SegmentType, dpl: PrivilegeLevel) -> Self {
        match segment_type {
            SegmentType::Code => Self::new(0, segment_type, dpl, true, true),
            _ => Self::new(0, segment_type, dpl, true, false),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct TssDescriptor {
    pub length: u16,
    pub base_low: u16,
    pub base_middle: u8,
    pub attributes: SegmentAttributes,
    pub base_high: u8,
    pub base_upper: u32,
    __: u32,
}

impl TssDescriptor {
    #[inline]
    pub const fn null() -> Self {
        Self {
            length: 104,
            base_low: 0,
            base_middle: 0,
            attributes: SegmentAttributes::new()
                .with_segment_type(SegmentType::Task)
                .with_long_mode(true),
            base_high: 0,
            base_upper: 0,
            __: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
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
    pub const fn new() -> Self {
        Self {
            null: SegmentDescriptor::null(),
            kernel_code: SegmentDescriptor::new_from_type(
                SegmentType::Code,
                PrivilegeLevel::Supervisor,
            ),
            kernel_data: SegmentDescriptor::new_from_type(
                SegmentType::Data,
                PrivilegeLevel::Supervisor,
            ),
            user_code: SegmentDescriptor::new_from_type(SegmentType::Code, PrivilegeLevel::User),
            user_data: SegmentDescriptor::new_from_type(SegmentType::Data, PrivilegeLevel::User),
            tss: TssDescriptor::null(),
        }
    }
}

#[repr(C, packed)]
pub struct GlobalDescriptorTableRegister {
    pub limit: u16,
    pub addr: *const GlobalDescriptorTable,
}

unsafe impl Sync for GlobalDescriptorTableRegister {}

pub static GDT: SyncUnsafeCell<GlobalDescriptorTable> =
    SyncUnsafeCell::new(GlobalDescriptorTable::new());

pub static GDTR: GlobalDescriptorTableRegister = GlobalDescriptorTableRegister {
    limit: (core::mem::size_of_val(&GDT) - 1) as u16,
    addr: GDT.get(),
};

impl GlobalDescriptorTableRegister {
    pub fn load(&self) {
        unsafe {
            asm!(
                "lgdt [{}]",
                "push {}",
                "lea {2}, [1f + rip]",
                "push {2}",
                "retfq",
                "1:",
                "mov ds, {3}",
                "mov es, {3}",
                "mov ss, {3}",
                in(reg) self,
                in(reg) u64::from(SegmentSelector::new(1, PrivilegeLevel::Supervisor)),
                lateout(reg) _,
                in(reg) u64::from(SegmentSelector::new(2, PrivilegeLevel::Supervisor)),
                options(preserves_flags)
            );
        }
    }
}
