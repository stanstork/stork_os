pub type PhysAddr = usize;
pub type VirtAddr = usize;

pub trait ToPhysAddr {
    fn to_phys_addr(&self) -> PhysAddr;
}

pub trait ToVirtAddr {
    fn to_virt_addr(&self) -> VirtAddr;
}

impl ToPhysAddr for VirtAddr {
    fn to_phys_addr(&self) -> PhysAddr {
        *self
    }
}

impl ToPhysAddr for *mut u8 {
    fn to_phys_addr(&self) -> PhysAddr {
        *self as PhysAddr
    }
}

impl ToPhysAddr for u64 {
    fn to_phys_addr(&self) -> PhysAddr {
        *self as PhysAddr
    }
}

impl ToPhysAddr for *mut u32 {
    fn to_phys_addr(&self) -> PhysAddr {
        *self as PhysAddr
    }
}

impl ToVirtAddr for u64 {
    fn to_virt_addr(&self) -> VirtAddr {
        *self as VirtAddr
    }
}
