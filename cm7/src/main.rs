#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

use cortex_m::asm;
use stm32h7_staging::stm32h747cm7 as device;

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
    let gpioe = unsafe { &*device::GPIOE::ptr() };
    let rcc = unsafe { &*device::RCC::ptr() };

    rcc.ahb4enr().modify(|_, w| w.gpioeen().set_bit());
    asm::dmb();
    gpioe.moder().modify(|_, w| w.moder1().output());

    gpioe.bsrr().write(|w| w.bs1().set_bit());
    delay(6_000_000);
    gpioe.bsrr().write(|w| w.br1().set_bit());
    delay(6_000_000);
    gpioe.bsrr().write(|w| w.bs1().set_bit());
    delay(6_000_000);
    gpioe.bsrr().write(|w| w.br1().set_bit());

    system_init();
    led_blink()
}

fn system_init() {
    let rcc = unsafe { &*device::RCC::ptr() };
    let pwr = unsafe { &*device::PWR::ptr() };
    let flash = unsafe { &*device::FLASH::ptr() };

    rcc.apb4enr().modify(|_, w| w.syscfgen().set_bit());
    asm::dmb();

    pwr.cr3().modify(|_, w| {
        w.sden().set_bit()
         .ldoen().clear_bit()
         .bypass().clear_bit()
    });
    asm::dsb();
    while !pwr.csr1().read().actvosrdy().bit_is_set() {}

    pwr.d3cr().modify(|_, w| unsafe { w.vos().bits(3) });
    asm::dsb();
    while !pwr.d3cr().read().vosrdy().bit_is_set() {}

    rcc.pllckselr().modify(|_, w| unsafe { w.divm1().bits(4) });

    rcc.pll1divr().write(|w| unsafe {
        w.divn1().bits(49)
         .divp1().bits(1)
         .divq1().bits(3)
         .divr1().bits(1)
    });

    rcc.pllcfgr().modify(|_, w| unsafe {
        w.pll1rge().bits(3)
         .pll1vcosel().wide_vco()
         .divp1en().set_bit()
         .divq1en().set_bit()
         .divr1en().set_bit()
    });

    rcc.cr().modify(|_, w| w.pll1on().set_bit());
    asm::dsb();
    while !rcc.cr().read().pll1rdy().bit_is_set() {}

    rcc.d1cfgr().modify(|_, w| unsafe {
        w.d1cpre().bits(0)
         .hpre().bits(8)
         .d1ppre().bits(4)
    });
    rcc.d2cfgr().modify(|_, w| unsafe {
        w.d2ppre1().bits(4)
         .d2ppre2().bits(4)
    });
    rcc.d3cfgr().modify(|_, w| unsafe {
        w.d3ppre().bits(4)
    });

    flash.acr().modify(|_, w| unsafe { w.latency().bits(2) });
    while flash.acr().read().latency().bits() != 2 {}

    rcc.cfgr().modify(|_, w| w.sw().pll1());
    asm::dsb();
    while rcc.cfgr().read().sws().bits() != 3 {}

    const RCC_MP_C1GR1: *mut u32 = (0x5802_4400 + 0x100) as *mut u32;
    unsafe { core::ptr::write_volatile(RCC_MP_C1GR1, 0x01); }
}

fn led_blink() -> ! {
    let rcc = unsafe { &*device::RCC::ptr() };
    let gpioe = unsafe { &*device::GPIOE::ptr() };

    rcc.ahb4enr().modify(|_, w| w.gpioeen().set_bit());
    asm::dmb();

    gpioe.moder().modify(|_, w| w.moder1().output());

    uart_init();

    loop {
        gpioe.bsrr().write(|w| w.bs1().set_bit());
        uart_puts("Hello World\r\n");
        delay(40_000_000);
        gpioe.bsrr().write(|w| w.br1().set_bit());
        delay(40_000_000);
    }
}

fn uart_init() {
    let rcc = unsafe { &*device::RCC::ptr() };
    let gpiod = unsafe { &*device::GPIOD::ptr() };
    let usart3 = unsafe { &*device::USART3::ptr() };

    rcc.ahb4enr().modify(|_, w| w.gpioden().set_bit());
    rcc.apb1lenr().modify(|_, w| w.usart3en().set_bit());
    asm::dmb();

    gpiod.moder().modify(|_, w| {
        w.moder8().alternate()
         .moder9().alternate()
    });
    gpiod.afrh().modify(|_, w| w.afr8().af7().afr9().af7());

    usart3.cr1().modify(|_, w| w.ue().clear_bit());
    usart3.brr().write(|w| unsafe { w.brr().bits(0x0353) });
    usart3.cr1().modify(|_, w| w.te().set_bit().re().set_bit().ue().set_bit());
}

fn uart_putc(c: u8) {
    let usart3 = unsafe { &*device::USART3::ptr() };
    while !usart3.isr().read().txe().bit_is_set() {}
    usart3.tdr().write(|w| unsafe { w.tdr().bits(c.into()) });
}

fn uart_puts(s: &str) {
    for &b in s.as_bytes() {
        if b == b'\n' {
            uart_putc(b'\r');
        }
        uart_putc(b);
    }
}

fn delay(cycles: u32) {
    let mut i = cycles;
    while i != 0 {
        //aw asm!("nop") instead of cortex_m::asm::nop() with options(nomem, nostack,
        //preserves_flags).
        unsafe { asm!("nop"); }
        i -= 1;
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
