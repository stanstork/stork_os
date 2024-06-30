use core::arch::asm;

pub struct Cr3;

impl Cr3 {
    pub fn write(value: u64) {
        unsafe {
            asm!(
                "mov cr3, {}",
                in(reg) value,
                options(nostack)
            );
        }
    }

    pub fn read() -> usize {
        let value: u64;
        unsafe {
            asm!(
                "mov {}, cr3",
                out(reg) value,
                options(nostack)
            );
        }
        value as usize
    }
}
