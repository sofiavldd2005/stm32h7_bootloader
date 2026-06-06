#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

use cortex_m::asm;
use stm32h7_staging::stm32h747cm4 as device;

pub union Vector {
    reserved: u32,
    handler: unsafe extern "C" fn() -> !,
}

macro_rules! exceptions {
    ($($name:ident),*) => {
        $(
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $name() -> ! { loop {} }
        )*
    }
}

exceptions!(NMI, HardFault, MemManage, BusFault, UsageFault, SVCall, PendSV, SysTick);

#[unsafe(link_section = ".vector_table.reset_vector")]
#[unsafe(no_mangle)]
pub static RESET_VECTOR: unsafe extern "C" fn() -> ! = Reset;

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Reset() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
