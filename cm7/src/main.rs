#![no_std]
#![no_main]



use cortex_m::asm;
use stm32h7_staging::stm32h747cm7 as device;

#[allow(unused_imports)] //The unused imports in this case are required by the linker script
use shared::{Vector, EXCEPTIONS, DIAG, delay, FW_APPROVED};
mod uart;
mod clock;
mod crc;
mod handshake;



#[unsafe(link_section = ".vector_table.reset_vector")]
#[unsafe(no_mangle)]
pub static RESET_VECTOR: unsafe extern "C" fn() -> ! = Reset;

macro_rules! exceptions {
    ($($name:ident),*) => {
        $(
            #[unsafe(no_mangle)]
            /// # Safety
            ///
            /// Exception handler called directly by the CPU on fault/event.
            /// Must be installed at the correct vector table entry.

            pub unsafe extern "C" fn $name() -> ! { loop {} }
        )*
    }
}

exceptions!(NMI, HardFault, MemManage, BusFault, UsageFault, SVCall, PendSV, SysTick);

/// # Safety
///
/// CPU reset entry point. Must be the second word in the vector table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Reset() -> ! {
    let gpioe = unsafe { &*device::GPIOE::ptr() };
    let rcc = unsafe { &*device::RCC::ptr() };

    rcc.ahb4enr().modify(|_, w| w.gpioeen().set_bit().gpioben().set_bit());
    asm::dmb();
    gpioe.moder().modify(|_, w| w.moder1().output());

    gpioe.bsrr().write(|w| w.bs1().set_bit());
    delay(6_000_000);
    gpioe.bsrr().write(|w| w.br1().set_bit());
    delay(6_000_000);
    gpioe.bsrr().write(|w| w.bs1().set_bit());
    delay(6_000_000);
    gpioe.bsrr().write(|w| w.br1().set_bit());

    clock::system_init();
    led_blink();
}





fn led_blink() -> ! {
    let rcc   = unsafe { &*device::RCC::ptr() };
    let gpioe = unsafe { &*device::GPIOE::ptr() };

    rcc.ahb4enr().modify(|_, w| w.gpioeen().set_bit().gpioben().set_bit());
    asm::dmb();
    gpioe.moder().modify(|_, w| w.moder1().output());

    uart::init();

    if crc::validate_cm4_firmware() {
        unsafe { core::ptr::write_volatile(FW_APPROVED, 0xDEAD_BEEF); }
        uart::puts("CM4 CRC PASS\r\n");
    } else {
        unsafe { core::ptr::write_volatile(FW_APPROVED, 0); }
        uart::puts("CM4 CRC FAIL\r\n");
        loop {
            gpioe.bsrr().write(|w| w.bs1().set_bit());
            delay(800_000);
            gpioe.bsrr().write(|w| w.br1().set_bit());
            delay(800_000);
        }
    }

    let magic = handshake::sync_with_cm4();
    uart::puts("CM4 magic: 0x");
    uart::hex(magic);
    uart::puts("\r\n");

    loop {
        gpioe.bsrr().write(|w| w.bs1().set_bit());
        uart::puts("Hello World\r\n");
        delay(40_000_000);
        gpioe.bsrr().write(|w| w.br1().set_bit());
        delay(40_000_000);
    }
}
