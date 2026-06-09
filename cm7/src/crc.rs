use stm32h7_staging::stm32h747cm7 as device;

include!(concat!(env!("OUT_DIR"), "/crc_golden.rs"));

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
