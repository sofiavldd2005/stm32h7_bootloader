use cortex_m::asm;
use stm32h7_staging::stm32h747cm7 as device;

pub fn init() {
    let rcc = unsafe { &*device::RCC::ptr() };
    let gpiod = unsafe { &*device::GPIOD::ptr() };
    let usart3 = unsafe { &*device::USART3::ptr() };

    rcc.ahb4enr().modify(|_, w| w.gpioden().set_bit());
    rcc.apb1lenr().modify(|_, w| w.usart3en().set_bit());
    asm::dmb();

    gpiod
        .moder()
        .modify(|_, w| w.moder8().alternate().moder9().alternate());
    gpiod.afrh().modify(|_, w| w.afr8().af7().afr9().af7());

    usart3.cr1().modify(|_, w| w.ue().clear_bit());
    usart3.brr().write(|w| unsafe { w.brr().bits(0x0353) });
    usart3
        .cr1()
        .modify(|_, w| w.te().set_bit().re().set_bit().ue().set_bit());
}

pub fn putc(c: u8) {
    let usart3 = unsafe { &*device::USART3::ptr() };
    while !usart3.isr().read().txe().bit_is_set() {}
    usart3.tdr().write(|w| unsafe { w.tdr().bits(c.into()) });
}

pub fn puts(s: &str) {
    for &b in s.as_bytes() {
        if b == b'\n' {
            putc(b'\r');
        }
        putc(b);
    }
}

pub fn hex(n: u32) {
    for i in (0..8).rev() {
        let nibble = (n >> (i * 4)) & 0xF;
        let c = if nibble < 10 {
            b'0' + nibble as u8
        } else {
            b'A' + nibble as u8 - 10
        };
        putc(c);
    }
}
