use core::arch::asm;

pub struct Rdtsc {
    pub low: u32,
    pub high: u32,
}

impl Rdtsc {
    pub fn read() -> u64 {
        let low: u32;
        let high: u32;
        unsafe {
            asm!(
                "rdtsc",
                out("eax") low,
                out("edx") high,
                options(nostack)
            );
        }
        ((high as u64) << 32) | (low as u64)
    }
}
