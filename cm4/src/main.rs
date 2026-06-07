#![no_std]
#![no_main]



use stm32h7_staging::stm32h747cm4 as device;

#[allow(unused_imports)]//These imports are required by the linker
use shared::{Vector, EXCEPTIONS, DIAG, delay};


#[unsafe(no_mangle)]
/// # Safety
///
/// NMI called directly by the CPU on NMI event.
/// Must be installed at the correct vector table entry.
pub unsafe extern "C" fn NMI() -> ! { unsafe { core::ptr::write_volatile(DIAG, 0xDEAD_0001); } loop {} }
#[unsafe(no_mangle)]
/// # Safety
///
/// HardFault called directly by the CPU on HardFault event
/// Must be installed at the correct vector table entry.
pub unsafe extern "C" fn HardFault() -> ! { unsafe { core::ptr::write_volatile(DIAG, 0xDEAD_0002); } loop {} }
#[unsafe(no_mangle)]
/// # Safety
///
///MemManage called directly by the CPU on MemManage fault evnet.
/// Must be installed at the correct vector table entry.
pub unsafe extern "C" fn MemManage() -> ! { unsafe { core::ptr::write_volatile(DIAG, 0xDEAD_0003); } loop {} }
#[unsafe(no_mangle)]
/// # Safety
///
/// BusFault called directly by the CPU on event Bus fault  event.
/// Must be installed at the correct vector table entry.
pub unsafe extern "C" fn BusFault() -> ! { unsafe { core::ptr::write_volatile( DIAG, 0xDEAD_0004); } loop {} }
#[unsafe(no_mangle)]
/// # Safety
///
/// UsageFault called directly by the CPU on Usage fault event.
/// Must be installed at the correct vector table entry.
pub unsafe extern "C" fn UsageFault() -> ! { unsafe { core::ptr::write_volatile(DIAG, 0xDEAD_0005); } loop {} }
#[unsafe(no_mangle)]
/// # Safety
///
/// SVCall called directly by the CPU on SVCall event.
/// Must be installed at the correct vector table entry.
pub unsafe extern "C" fn SVCall() -> ! { unsafe { core::ptr::write_volatile(DIAG, 0xDEAD_0006); } loop {} }
#[unsafe(no_mangle)]
/// # Safety
///
/// PendSV called directly by the CPU on PendSV.
/// Must be installed at the correct vector table entry.
pub unsafe extern "C" fn PendSV() -> ! { unsafe { core::ptr::write_volatile(DIAG, 0xDEAD_0007); } loop {} }
#[unsafe(no_mangle)]
/// # Safety
///
/// SysTick called directly by the CPU on SysTick event.
/// Must be installed at the correct vector table entry.
pub unsafe extern "C" fn SysTick() -> ! { unsafe { core::ptr::write_volatile(DIAG, 0xDEAD_0008); } loop {} }

#[unsafe(link_section = ".vector_table.reset_vector")]
#[unsafe(no_mangle)]
pub static RESET_VECTOR: unsafe extern "C" fn() -> ! = Reset;



#[unsafe(no_mangle)]
/// # Safety
///
/// CPU reset entry point. Must be the second word in the vector table.
pub unsafe extern "C" fn Reset() -> ! {
    unsafe { core::ptr::write_volatile(shared::SHARED_MAGIC, 0xCAFE_BABE); }

    const SCB_VTOR: *mut u32 = 0xE000ED08 as *mut u32;
    unsafe { core::ptr::write_volatile(SCB_VTOR, 0x0810_0000); }

    let gpiob = unsafe { &*device::GPIOB::ptr() };

    // Enable GPIOB clock from CM4 side
    const AHB4ENR: *mut u32 = 0x5802_44E0 as *mut u32;
    

    let en = unsafe { core::ptr::read_volatile(AHB4ENR) };
    unsafe { core::ptr::write_volatile(AHB4ENR, en | (1 << 1)); }
    let en = unsafe { core::ptr::read_volatile(AHB4ENR) };
    

    unsafe { core::ptr::write_volatile(AHB4ENR, en | (1 << 25)); }
    unsafe { core::arch::asm!("dsb"); }
    // Signal CM7 that shared memory is ready, then wait for DIAG
    unsafe { shared::hsem_take(0, shared::COREID_CM4, shared::PROCID_DEFAULT); }
    unsafe { core::ptr::write_volatile(shared::DIAG, 0xCAFE_0002); }
    unsafe { shared::hsem_release(0, shared::COREID_CM4, shared::PROCID_DEFAULT); }
    // Poll until CM7 writes 0xCAFE_F00D
    loop {
        let d = unsafe { core::ptr::read_volatile(shared::DIAG) };
        if d == 0xCAFE_F00D {
            unsafe { core::ptr::write_volatile(0x2400_000C as *mut u32, d); }
            break;
        }
        delay(1_000);
    }
    const MODER: *mut u32 = 0x5802_0400 as *mut u32;
    unsafe { core::ptr::write_volatile(MODER, 0xFFFF_FEB9); }
    unsafe { core::arch::asm!("dsb"); }
    let check = unsafe { core::ptr::read_volatile(MODER) };
    unsafe { core::ptr::write_volatile(0x2400_0008 as *mut u32, check); }
    unsafe { core::ptr::write_volatile(DIAG, 0xCAFE_0002); }

    loop {
        gpiob.bsrr().write(|w| w.bs0().set_bit());
        unsafe { core::ptr::write_volatile(DIAG, 0xCAFE_0003); }
        delay(6_000_000);
        gpiob.bsrr().write(|w| w.br0().set_bit());
        unsafe { core::ptr::write_volatile(DIAG, 0xCAFE_0004); }
        delay(6_000_000);
    }
}




