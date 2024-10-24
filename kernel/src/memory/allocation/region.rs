use crate::memory::addr::PhysAddr;

#[derive(Clone, Debug)]
pub struct Region {
    start: PhysAddr,
    size: usize,
}

impl Region {
    pub fn new(start: PhysAddr, size: usize) -> Self {
        Self { start, size }
    }

    pub fn start(&self) -> PhysAddr {
        self.start
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn as_mut_ptr(&self) -> *mut u8 {
        self.start.as_mut_ptr()
    }
}
