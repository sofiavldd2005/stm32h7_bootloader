#![no_std]
#![no_main]



use cortex_m::asm;
use stm32h7_staging::stm32h747cm7 as device;

#[allow(unused_imports)] //The unused imports in this case are required by the linker script
use shared::{Vector, EXCEPTIONS, DIAG, delay, FW_APPROVED};
include!(concat!(env!("OUT_DIR"), "/crc_golden.rs"));


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

    system_init();
    led_blink();
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
}

fn led_blink() -> ! {
    let rcc = unsafe { &*device::RCC::ptr() };
    let gpioe = unsafe { &*device::GPIOE::ptr() };

    rcc.ahb4enr().modify(|_, w| w.gpioeen().set_bit().gpioben().set_bit());
    asm::dmb();

    gpioe.moder().modify(|_, w| w.moder1().output());

    uart_init();

    // --- Firmware validation ---
    if validate_cm4_firmware() {
        unsafe { core::ptr::write_volatile(FW_APPROVED, 0xDEAD_BEEF); }
        uart_puts("CM4 CRC PASS\r\n");
    } else {
        unsafe { core::ptr::write_volatile(FW_APPROVED, 0); }
        uart_puts("CM4 CRC FAIL\r\n");
        // Fast blink forever
        loop {
            gpioe.bsrr().write(|w| w.bs1().set_bit());
            delay(800_000);
            gpioe.bsrr().write(|w| w.br1().set_bit());
            delay(800_000);
        }
    }
    // --- Handshake (existing) ---

    rcc.ahb4enr().modify(|_, w| w.hsemen().set_bit());
    unsafe { core::arch::asm!("dsb"); }
    // Handshake: wait for CM4, read magic, write DIAG, release
    unsafe { shared::hsem_take(0, shared::COREID_CM7, shared::PROCID_DEFAULT); }
    let magic = unsafe { core::ptr::read_volatile(shared::SHARED_MAGIC) };
    unsafe { core::ptr::write_volatile(shared::DIAG, 0xCAFE_F00D); }
    unsafe { shared::hsem_release(0, shared::COREID_CM7, shared::PROCID_DEFAULT); }
    uart_puts("CM4 magic: 0x");
    uart_hex(magic);
    uart_puts("\r\n");

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

fn uart_hex(n: u32) {
    for i in (0..8).rev() {
        let nibble = (n >> (i * 4)) & 0xF;
        let c = if nibble < 10 { b'0' + nibble as u8 } else { b'A' + nibble as u8 - 10 };
        uart_putc(c);
    }
}

fn validate_cm4_firmware() -> bool {
    let rcc    = unsafe { &*device::RCC::ptr() };
    let crc    = unsafe { &*device::CRC::ptr() };

    // Enable CRC clock
    rcc.ahb4enr().modify(|_, w| w.crcen().set_bit());
    unsafe { core::arch::asm!("dsb"); }

    // Reset CRC unit
    crc.cr().modify(|_, w| w.reset().reset());

    // Feed 32-bit words: 0x0810_0000 .. 0x081F_FFFC (exclude last 4 bytes)
    // Feed bytes one at a time — matches non-reflected CRC-32/MPEG2
    let base = 0x0810_0000 as *const u8;
    for i in 0..(1024 * 1024 - 4) {
    let byte = unsafe { core::ptr::read_volatile(base.add(i)) };
    crc.dr8().write(|w| unsafe { w.bits(byte) });
    }
    let computed = crc.dr().read().bits();

    computed == CRC_GOLDEN
}
