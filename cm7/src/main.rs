#![no_std]
#![no_main]

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
    let gpiob = &*device::GPIOB::ptr();
    let rcc = &*device::RCC::ptr();

    rcc.ahb4enr().modify(|_, w| w.gpioben().set_bit());
    asm::dmb();
    gpiob.moder().modify(|_, w| w.moder0().output());

    gpiob.bsrr().write(|w| w.bs0().set_bit());
    delay(2_000_000);
    gpiob.bsrr().write(|w| w.br0().set_bit());
    delay(2_000_000);
    gpiob.bsrr().write(|w| w.bs0().set_bit());
    delay(2_000_000);
    gpiob.bsrr().write(|w| w.br0().set_bit());

    system_init();
    led_blink()
}

fn system_init() {
    let rcc = unsafe { &*device::RCC::ptr() };
    let pwr = unsafe { &*device::PWR::ptr() };
    let flash = unsafe { &*device::FLASH::ptr() };

    rcc.apb4enr().modify(|_, w| w.syscfgen().set_bit());
    asm::dmb();

    pwr.cr3().modify(|_, w| unsafe {
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
}

fn led_blink() -> ! {
    let rcc = unsafe { &*device::RCC::ptr() };
    let gpiob = unsafe { &*device::GPIOB::ptr() };

    rcc.ahb4enr().modify(|_, w| w.gpioben().set_bit());
    asm::dmb();

    gpiob.moder().modify(|_, w| w.moder0().output());

    loop {
        gpiob.bsrr().write(|w| w.bs0().set_bit());
        delay(400_000_000);
        gpiob.bsrr().write(|w| w.br0().set_bit());
        delay(400_000_000);
    }
}

fn delay(cycles: u32) {
    for _ in 0..cycles {
        asm::nop();
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
