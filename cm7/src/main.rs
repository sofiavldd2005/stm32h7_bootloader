#![no_std]
#![no_main]



use cortex_m::asm;
use stm32h7_staging::stm32h747cm7 as device;

#[allow(unused_imports)] //The unused imports in this case are required by the linker script
use shared::{Vector, EXCEPTIONS, delay};
use shared::mem::{FW_APPROVED, BOOT_MODE, BOOT_NORMAL, BOOT_UPDATE};
mod uart;
mod clock;
mod crc;
mod handshake;
mod gpio;
mod flash;
mod protocol;



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

    // gpio::init_port(2);
    // gpio::set_pin_as_input(0x5802_0800, 13);
    // if !gpio::is_pin_low(0x5802_0800, 13) {
    if true {
        let gpiob = unsafe { &*device::GPIOB::ptr() };
        gpiob.moder().modify(|_, w| w.moder14().output());

        unsafe { core::ptr::write_volatile(BOOT_MODE, BOOT_UPDATE); }
        uart::puts("UPDATE MODE\r\n");
        for _ in 0..3 {
            gpiob.bsrr().write(|w| w.bs14().set_bit());
            delay(200_000);
            gpiob.bsrr().write(|w| w.br14().set_bit());
            delay(200_000);
        }
        // Direct flash test: erase bank 2 and program one word
        let test_addr: u32 = 0x0810_0000;
        let test_data: u32 = 0xDEAD_BEEF;

        uart::puts("FLASH TEST: erasing...\r\n");
        let erase_ok = unsafe { flash::erase_all() };
        match erase_ok {
            Ok(()) => uart::puts("FLASH TEST: erase OK\r\n"),
            Err(()) => uart::puts("FLASH TEST: erase FAIL\r\n"),
        }

        let crc_before = unsafe { crc::compute_region(test_addr, 256) };
        uart::puts("FLASH TEST: CRC before=0x");
        uart::hex(crc_before.unwrap_or(0));
        uart::puts("\r\n");

        let prog_ok = unsafe { flash::program_word(test_addr, test_data) };
        match prog_ok {
            Ok(()) => uart::puts("FLASH TEST: program OK\r\n"),
            Err(()) => {
                uart::puts("FLASH TEST: program FAIL SR2=0x");
                uart::hex(flash::read_sr2());
                uart::puts("\r\n");
            }
        }

        // Use CRC to verify (avoids AXI read-buffer stale-data issue)
        let crc_after = unsafe { crc::compute_region(test_addr, 256) };
        uart::puts("FLASH TEST: CRC after=0x");
        uart::hex(crc_after.unwrap_or(0));
        uart::puts("\r\n");
        if crc_before != crc_after {
            uart::puts("FLASH TEST: CRC changed - write succeeded\r\n");
        } else {
            uart::puts("FLASH TEST: CRC unchanged - write failed\r\n");
        }

        // Also try double read_volatile
        asm::dsb();
        let _ = unsafe { core::ptr::read_volatile(0x2400_0000 as *const u32) };
        asm::dsb();
        let read1 = unsafe { core::ptr::read_volatile(test_addr as *const u32) };
        let read2 = unsafe { core::ptr::read_volatile(test_addr as *const u32) };
        uart::puts("FLASH TEST: read1=0x");
        uart::hex(read1);
        uart::puts(" read2=0x");
        uart::hex(read2);
        uart::puts("\r\n");

        protocol::protocol_loop(&mut protocol::UartTransport);
    }
    unsafe { core::ptr::write_volatile(BOOT_MODE, BOOT_NORMAL); }

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
        delay(4_000_000);
        gpioe.bsrr().write(|w| w.br1().set_bit());
        delay(4_000_000);
    }
}
