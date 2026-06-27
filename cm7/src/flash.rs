use cortex_m::asm;
use stm32h7_staging::stm32h747cm7 as device;

/// CM4 flash occupies Bank 2: 0x0810_0000 – 0x081F_FFFF.
const CM4_FLASH_START: u32 = 0x0810_0000;
/// One-past-end of CM4 flash.
const CM4_FLASH_END: u32 = 0x0820_0000;
/// Sector size (128 KB).
const SECTOR_SIZE: u32 = 0x0002_0000;

/// Unlock flash Bank 2 for write/erase.
unsafe fn unlock() {
    let flash = unsafe { &*device::FLASH::ptr() };
    flash.bank2().keyr().write(|w| unsafe { w.bits(0x45670123) });
    flash.bank2().keyr().write(|w| unsafe { w.bits(0xCDEF89AB) });
}

/// Lock flash Bank 2 after operations.
unsafe fn lock() {
    let flash = unsafe { &*device::FLASH::ptr() };
    flash.bank2().cr().modify(|_, w| w.lock().set_bit());
}

/// Spin until flash Bank 2 reports ready (BSY = 0).
unsafe fn wait_ready() {
    let flash = unsafe { &*device::FLASH::ptr() };
    while flash.bank2().sr().read().bsy().bit_is_set() {}
}

/// Erase one sector in Bank 2.
///
/// `sector` is a bank-local sector number 0..7, corresponding to
/// physical sectors 8..15 (the CM4 flash region).
pub unsafe fn sector_erase(sector: u8) -> Result<(), ()> {
    if sector > 7 {
        return Err(());
    }

    let flash = unsafe { &*device::FLASH::ptr() };
    unsafe { unlock(); }
    unsafe { wait_ready(); }

    unsafe {
        flash.bank2().cr().modify(|_, w| {
            w.ser().set_bit().snb().bits(sector).start().set_bit()
        });
    }

    unsafe { wait_ready(); }

    flash.bank2().cr().modify(|_, w| w.ser().clear_bit());
    flash.bank2().ccr().write(|w| w.clr_eop().set_bit());

    let sr = flash.bank2().sr().read();
    if sr.wrperr().bit_is_set() {
        flash.bank2().ccr().write(|w| w.clr_wrperr().set_bit());
        unsafe { lock(); }
        return Err(());
    }
    if sr.pgserr().bit_is_set() {
        flash.bank2().ccr().write(|w| w.clr_pgserr().set_bit());
        unsafe { lock(); }
        return Err(());
    }

    unsafe { lock(); }
    Ok(())
}

/// Program a 32-bit word into Bank 2 flash.
///
/// `addr` must be 4-byte aligned and within `0x0810_0000..0x0820_0000`.
pub unsafe fn program_word(addr: u32, data: u32) -> Result<(), ()> {
    if addr < CM4_FLASH_START || addr >= CM4_FLASH_END || addr & 3 != 0 {
        return Err(());
    }

    let flash = unsafe { &*device::FLASH::ptr() };
    unsafe { unlock(); }
    unsafe { wait_ready(); }

    flash.bank2().cr().modify(|_, w| w.pg().set_bit());
    asm::dmb();

    unsafe { core::ptr::write_volatile(addr as *mut u32, data); }
    asm::dmb();

    unsafe { wait_ready(); }

    flash.bank2().cr().modify(|_, w| w.pg().clear_bit());
    flash.bank2().ccr().write(|w| w.clr_eop().set_bit());

    let sr = flash.bank2().sr().read();
    if sr.wrperr().bit_is_set() {
        flash.bank2().ccr().write(|w| w.clr_wrperr().set_bit());
        unsafe { lock(); }
        return Err(());
    }

    if unsafe { core::ptr::read_volatile(addr as *const u32) } != data {
        unsafe { lock(); }
        return Err(());
    }

    unsafe { lock(); }
    Ok(())
}

/// Erase all Bank 2 sectors (physical sectors 8..15, covering the CM4 region).
pub unsafe fn erase_all() -> Result<(), ()> {
    for sector in 0..8 {
        unsafe { sector_erase(sector)?; }
    }
    Ok(())
}
