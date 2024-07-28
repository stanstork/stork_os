use crate::println;

#[repr(C)]
pub struct Lapic {
    registers: [u32; 1024],
}

impl Lapic {
    /// Create a new instance of the LAPIC at the specified virtual address.
    pub fn new(base_address: usize) -> &'static mut Self {
        unsafe { &mut *(base_address as *mut Lapic) }
    }

    pub unsafe fn write_register(&mut self, offset: usize, value: u32) {
        self.registers[offset / 4] = value;
    }

    unsafe fn read_register(&self, offset: usize) -> u32 {
        self.registers[offset / 4]
    }

    pub fn enable(&mut self) {
        unsafe {
            self.write_register(0xF0, self.read_register(0x0F0) | 0x100);
        }
    }
}
