use stm32h7_staging::stm32h747cm7 as device;

/// Enable clock for a GPIO port by setting its bit in AHB4ENR.
///
/// `en_bit` is the bit position in `RCC_AHB4ENR` for the port:
/// - GPIOA = 0
/// - GPIOB = 1
/// - GPIOC = 2
/// - GPIOD = 3
/// - GPIOE = 4
/// - etc.
pub fn init_port(en_bit: u8) {
    let rcc = unsafe { &*device::RCC::ptr() };
    unsafe {
        rcc.ahb4enr().modify(|_, w| w.bits(rcc.ahb4enr().read().bits() | (1 << en_bit)));
    }
    unsafe { core::arch::asm!("dsb"); }
}

/// Configure a GPIO pin as digital input.
///
/// On STM32H7 all pins default to analog mode after reset —
/// IDR reads as 0 unless the pin is set to input mode first.
///
/// `base` is the GPIO port base address (e.g. `0x5802_0000` for GPIOA,
/// `0x5802_0800` for GPIOC). Each port occupies 0x400 bytes.
/// `pin` is the pin number 0..15.
pub fn set_pin_as_input(base: u32, pin: u8) {
    let moder_addr = base as *mut u32;
    unsafe {
        let moder = core::ptr::read_volatile(moder_addr);
        core::ptr::write_volatile(moder_addr, moder & !(3 << (pin * 2)));
    }
    unsafe { core::arch::asm!("dsb"); }
}

pub fn set_pin_pull_up(base: u32, pin: u8) {
    const PUPDR_OFFSET: u32 = 0x0C;
    let pupdr_addr = (base + PUPDR_OFFSET) as *mut u32;
    unsafe {
        let pupdr = core::ptr::read_volatile(pupdr_addr);
        core::ptr::write_volatile(pupdr_addr, (pupdr & !(3 << (pin * 2))) | (1 << (pin * 2)));
    }
    unsafe { core::arch::asm!("dsb"); }
}

/// Read a GPIO pin and return true if it is low.
///
/// `base` is the GPIO port base address (e.g. `0x5802_0000` for GPIOA,
/// `0x5802_0800` for GPIOC). Each port occupies 0x400 bytes.
/// `pin` is the pin number 0..15.
///
/// Returns `true` when the pin reads low, `false` when high.
pub fn is_pin_low(base: u32, pin: u8) -> bool {
    const IDR_OFFSET: u32 = 0x10;
    let idr_addr = (base + IDR_OFFSET) as *const u32;
    let idr = unsafe { core::ptr::read_volatile(idr_addr) };
    ((idr >> pin) & 1) == 0
}
