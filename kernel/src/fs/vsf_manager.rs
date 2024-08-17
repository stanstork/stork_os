use super::VirtualFileSystem;
use alloc::vec::Vec;

pub struct VfsManager {
    pub fss: Vec<*const VirtualFileSystem>,
}

impl VfsManager {
    pub fn new() -> Self {
        VfsManager { fss: Vec::new() }
    }

    pub fn mount(&mut self, fs: *const VirtualFileSystem) {
        self.fss.push(fs);
    }

    pub fn unmount(&mut self, fs: *const VirtualFileSystem) {
        self.fss.retain(|&x| x != fs);
    }
}
