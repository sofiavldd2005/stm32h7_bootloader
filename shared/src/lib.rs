#![no_std]

use core::panic::PanicInfo;
use core::arch::asm;
pub union Vector {
    reserved: u32,
    handler: unsafe extern "C" fn() -> !,
}


unsafe extern "C" {
    fn NMI() -> !;
    fn HardFault() -> !;
    fn MemManage() -> !;
    fn BusFault() -> !;
    fn UsageFault() -> !;
    fn SVCall() -> !;
    fn PendSV() -> !;
    fn SysTick() -> !;
}


#[unsafe(link_section = ".vector_table.exceptions")]
#[unsafe(no_mangle)]
pub static EXCEPTIONS: [Vector; 14] = [
    Vector { handler: NMI },
    Vector { handler: HardFault },
    Vector { handler: MemManage },
    Vector { handler: BusFault },
    Vector { handler: UsageFault },
    Vector { reserved: 0 },
    Vector { reserved: 0 },
    Vector { reserved: 0 },
    Vector { reserved: 0 },
    Vector { handler: SVCall },
    Vector { reserved: 0 },
    Vector { reserved: 0 },
    Vector { handler: PendSV },
    Vector { handler: SysTick },
];

#[panic_handler]
pub fn panic(_: &PanicInfo) -> ! {
    loop {}
}

/// My own HalDelay() implementation
pub fn delay(cycles: u32) {
    let mut i = cycles;
    while i != 0 {
        //raw asm!("nop") instead of cortex_m::asm::nop() with options(nomem, nostack,
        //preserves_flags).
        unsafe { asm!("nop"); }
        i -= 1;
    }
}
