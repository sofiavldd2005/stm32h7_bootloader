#![no_std]
#![no_main]

use core::panic::PanicInfo;

use cortex_m::asm;
use stm32h7_staging::stm32h747cm4 as device;

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

const PROBE: *mut u32 = 0x2400_0004 as *mut u32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn NMI() -> ! { unsafe { core::ptr::write_volatile(PROBE, 0xDEAD_0001); } loop {} }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HardFault() -> ! { unsafe { core::ptr::write_volatile(PROBE, 0xDEAD_0002); } loop {} }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn MemManage() -> ! { unsafe { core::ptr::write_volatile(PROBE, 0xDEAD_0003); } loop {} }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn BusFault() -> ! { unsafe { core::ptr::write_volatile(PROBE, 0xDEAD_0004); } loop {} }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn UsageFault() -> ! { unsafe { core::ptr::write_volatile(PROBE, 0xDEAD_0005); } loop {} }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SVCall() -> ! { unsafe { core::ptr::write_volatile(PROBE, 0xDEAD_0006); } loop {} }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PendSV() -> ! { unsafe { core::ptr::write_volatile(PROBE, 0xDEAD_0007); } loop {} }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SysTick() -> ! { unsafe { core::ptr::write_volatile(PROBE, 0xDEAD_0008); } loop {} }

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

const SHARED_MAGIC: *mut u32 = 0x2400_0000 as *mut u32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Reset() -> ! {
    unsafe { core::ptr::write_volatile(SHARED_MAGIC, 0xCAFE_BABE); }

    const SCB_VTOR: *mut u32 = 0xE000ED08 as *mut u32;
    unsafe { core::ptr::write_volatile(SCB_VTOR, 0x0810_0000); }

    let gpiob = unsafe { &*device::GPIOB::ptr() };

    // Enable GPIOB clock from CM4 side
    const AHB4ENR: *mut u32 = 0x5802_44E0 as *mut u32;
    let en = unsafe { core::ptr::read_volatile(AHB4ENR) };
    unsafe { core::ptr::write_volatile(AHB4ENR, en | (1 << 1)); }
    unsafe { core::arch::asm!("dsb"); }
    unsafe { core::ptr::write_volatile(PROBE, 0xCAFE_0001); }

    asm::dmb();
    const MODER: *mut u32 = 0x5802_0400 as *mut u32;
    unsafe { core::ptr::write_volatile(MODER, 0xFFFF_FEB9); }
    unsafe { core::arch::asm!("dsb"); }
    let check = unsafe { core::ptr::read_volatile(MODER) };
    unsafe { core::ptr::write_volatile(0x2400_0008 as *mut u32, check); }
    unsafe { core::ptr::write_volatile(PROBE, 0xCAFE_0002); }

    loop {
        gpiob.bsrr().write(|w| w.bs0().set_bit());
        unsafe { core::ptr::write_volatile(PROBE, 0xCAFE_0003); }
        delay(6_000_000);
        gpiob.bsrr().write(|w| w.br0().set_bit());
        unsafe { core::ptr::write_volatile(PROBE, 0xCAFE_0004); }
        delay(6_000_000);
    }
}

fn delay(cycles: u32) {
    let mut i = cycles;
    while i != 0 {
        unsafe { core::arch::asm!("nop"); }
        i -= 1;
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
