use core::{
    fmt::{Formatter, LowerHex},
    ops::{Add, AddAssign},
};

use super::HEAP_START;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysAddr(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtAddr(pub usize);

pub trait ToPhysAddr {
    fn to_phys_addr(&self) -> PhysAddr;
}

pub trait ToVirtAddr {
    fn to_virt_addr(&self) -> VirtAddr;
}

impl ToPhysAddr for VirtAddr {
    fn to_phys_addr(&self) -> PhysAddr {
        PhysAddr(self.0)
    }
}

impl ToPhysAddr for *mut u8 {
    fn to_phys_addr(&self) -> PhysAddr {
        PhysAddr(*self as usize)
    }
}

impl ToPhysAddr for u64 {
    fn to_phys_addr(&self) -> PhysAddr {
        PhysAddr(*self as usize)
    }
}

impl ToPhysAddr for *mut u32 {
    fn to_phys_addr(&self) -> PhysAddr {
        PhysAddr(*self as usize)
    }
}

impl ToVirtAddr for u64 {
    fn to_virt_addr(&self) -> VirtAddr {
        VirtAddr(*self as usize)
    }
}

impl VirtAddr {
    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    pub fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    #[inline]
    pub fn from_ptr<T: ?Sized>(ptr: *const T) -> Self {
        Self(ptr as *const () as usize)
    }
}

impl PhysAddr {
    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    pub fn to_virt_addr(self) -> VirtAddr {
        VirtAddr(self.0 + HEAP_START.0)
    }
}

impl Add<usize> for PhysAddr {
    type Output = PhysAddr;

    fn add(self, rhs: usize) -> PhysAddr {
        PhysAddr(self.0 + rhs)
    }
}

impl AddAssign<usize> for PhysAddr {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Add<usize> for VirtAddr {
    type Output = VirtAddr;

    fn add(self, rhs: usize) -> VirtAddr {
        VirtAddr(self.0 + rhs)
    }
}

impl AddAssign<usize> for VirtAddr {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Add<u64> for VirtAddr {
    type Output = VirtAddr;

    fn add(self, rhs: u64) -> VirtAddr {
        VirtAddr(self.0 + rhs as usize)
    }
}

impl From<*mut u8> for VirtAddr {
    fn from(ptr: *mut u8) -> Self {
        VirtAddr(ptr as usize)
    }
}

impl From<u64> for VirtAddr {
    fn from(addr: u64) -> Self {
        VirtAddr(addr as usize)
    }
}

impl From<usize> for VirtAddr {
    fn from(addr: usize) -> Self {
        VirtAddr(addr)
    }
}

impl LowerHex for VirtAddr {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}
