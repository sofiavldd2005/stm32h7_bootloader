use stm32h7_staging::stm32h747cm7 as device;

pub fn sync_with_cm4() -> u32 {
    let rcc = unsafe { &*device::RCC::ptr() };
    rcc.ahb4enr().modify(|_, w| w.hsemen().set_bit());
    unsafe {
        core::arch::asm!("dsb");
    }

    unsafe {
        shared::hsem_take(0, shared::mem::COREID_CM7, shared::mem::PROCID_DEFAULT);
    }
    let magic = unsafe { core::ptr::read_volatile(shared::mem::SHARED_MAGIC) };
    unsafe {
        core::ptr::write_volatile(shared::mem::DIAG, 0xCAFE_F00D);
    }
    unsafe {
        shared::hsem_release(0, shared::mem::COREID_CM7, shared::mem::PROCID_DEFAULT);
    }
    magic
}
