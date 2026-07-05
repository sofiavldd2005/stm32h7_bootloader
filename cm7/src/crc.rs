use stm32h7_staging::stm32h747cm7 as device;

include!(concat!(env!("OUT_DIR"), "/crc_golden.rs"));

/// Compute CRC-32/MPEG2 over a flash region using the hardware CRC peripheral.
///
/// Returns `None` if the region is empty or out of range.
pub unsafe fn compute_region(addr: u32, len: u32) -> Option<u32> {
    if len == 0 || addr >= 0x0820_0000 || addr + len > 0x0820_0000 {
        return None;
    }
    let rcc = unsafe { &*device::RCC::ptr() };
    let crc = unsafe { &*device::CRC::ptr() };

    rcc.ahb4enr().modify(|_, w| w.crcen().set_bit());
    unsafe { core::arch::asm!("dsb"); }

    crc.cr().modify(|_, w| w.reset().reset());

    let base = addr as *const u8;
    for i in 0..len as usize {
        let byte = unsafe { core::ptr::read_volatile(base.add(i)) };
        crc.dr8().write(|w| unsafe { w.bits(byte) });
    }
    Some(crc.dr().read().bits())
}

pub fn validate_cm4_firmware() -> bool {
    let rcc = unsafe { &*device::RCC::ptr() };
    let crc = unsafe { &*device::CRC::ptr() };

    rcc.ahb4enr().modify(|_, w| w.crcen().set_bit());
    unsafe {
        core::arch::asm!("dsb");
    }

    crc.cr().modify(|_, w| w.reset().reset());

    let base = 0x0810_0000 as *const u8;
    for i in 0..(1024 * 1024 - 4) {
        let byte = unsafe { core::ptr::read_volatile(base.add(i)) };
        crc.dr8().write(|w| unsafe { w.bits(byte) });
    }
    let computed = crc.dr().read().bits();

    computed == CRC_GOLDEN
}
