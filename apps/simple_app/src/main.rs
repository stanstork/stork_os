#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn main() -> i32 {
    42 // Returning a constant to test
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
