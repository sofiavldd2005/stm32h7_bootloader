#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[unsafe(link_section = ".vector_table.reset_vector")]
#[unsafe(no_mangle)]
pub static RESET_VECTOR: unsafe extern "C" fn() -> ! = Reset;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Reset() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
