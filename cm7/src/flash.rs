use cortex_m::asm;
use stm32h7_staging::stm32h747cm7 as device;

/// CM4 flash occupies Bank 2: 0x0810_0000 – 0x081F_FFFF.
const CM4_FLASH_START: u32 = 0x0810_0000;
/// One-past-end of CM4 flash.
const CM4_FLASH_END: u32 = 0x0820_0000;
/// Sector size (128 KB).
const SECTOR_SIZE: u32 = 0x0002_0000;

unsafe fn unlock() {
    let flash = unsafe { &*device::FLASH::ptr() };
    if flash.bank2().cr().read().lock().bit_is_clear() {
        return;
    }
    flash.bank2().keyr().write(|w| unsafe { w.bits(0x45670123) });
    flash.bank2().keyr().write(|w| unsafe { w.bits(0xCDEF89AB) });
}

/// Lock flash Bank 2 after operations.
unsafe fn lock() {
    let flash = unsafe { &*device::FLASH::ptr() };
    flash.bank2().cr().modify(|_, w| w.lock().set_bit());
}

/// Return the raw SR2 register value for diagnostics.
pub fn read_sr2() -> u32 {
    let flash = unsafe { &*device::FLASH::ptr() };
    flash.bank2().sr().read().bits()
}

unsafe fn wait_ready() {
    let flash = unsafe { &*device::FLASH::ptr() };
    loop {
        let sr = flash.bank2().sr().read();
        if !sr.bsy().bit_is_set() && !sr.qw().bit_is_set() {
            break;
        }
    }
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

    // 1. Enable programming and set PSIZE to 32-bit (2)
    flash.bank2().cr().modify(|_, w| w.pg().set_bit().psize().bits(2));
    asm::dsb();

    // 2. Write the 32-bit word to the flash address
    unsafe { core::ptr::write_volatile(addr as *mut u32, data); }
    asm::dsb();

    // 3. Force flush the partial 256-bit write buffer
    flash.bank2().cr().modify(|_, w| w.fw().set_bit());
    asm::dsb();

    // Wait for BSY to clear (and write buffer to drain)
    loop {
        let sr = flash.bank2().sr().read();
        if !sr.bsy().bit_is_set() && !sr.qw().bit_is_set() {
            break;
        }
    }

    // 4. Disable programming
    flash.bank2().cr().modify(|_, w| w.pg().clear_bit());
    flash.bank2().ccr().write(|w| w.clr_eop().set_bit());

    // Check all error flags; clear and bail on any.
    let sr = flash.bank2().sr().read();
    let any_err = sr.wrperr().bit_is_set()
        || sr.pgserr().bit_is_set()
        || sr.strberr().bit_is_set()
        || sr.incerr().bit_is_set()
        || sr.operr().bit_is_set();
    if any_err {
        flash.bank2().ccr().write(|w| {
            w.clr_wrperr().set_bit()
             .clr_pgserr().set_bit()
             .clr_strberr().set_bit()
             .clr_incerr().set_bit()
             .clr_operr().set_bit()
        });
        unsafe { lock(); }
        return Err(());
    }

    // Flush AXI read buffer before read-back verify
    asm::dsb();
    let _ = unsafe { core::ptr::read_volatile(0x2400_0000 as *const u32) };
    asm::dsb();
    let readback = unsafe { core::ptr::read_volatile(addr as *const u32) };
    if readback != data {
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
