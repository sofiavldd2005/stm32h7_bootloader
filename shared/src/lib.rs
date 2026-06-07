#![no_std]

use core::panic::PanicInfo;
use core::arch::asm;
pub union Vector {
    reserved: u32,
    handler: unsafe extern "C" fn() -> !,
}


unsafe extern "C" {
    fn NMI() -> !;
    fn HardFault() -> !;
    fn MemManage() -> !;
    fn BusFault() -> !;
    fn UsageFault() -> !;
    fn SVCall() -> !;
    fn PendSV() -> !;
    fn SysTick() -> !;
}
// Shared-memory layout (already there)
pub const SHARED_MAGIC: *mut u32 = 0x2400_0000 as *mut u32;
pub const PROBE:       *mut u32 = 0x2400_0004 as *mut u32;
pub const DIAG:        *mut u32 = 0x2400_0008 as *mut u32;
pub const FW_APPROVED: *mut u32 = 0x2400_0010 as *mut u32; ///< TO use with the CRC Check of CM4 flash
// HSEM stuff
pub const HSEM_BASE: u32 = 0x5802_6400; ///< HSEM base addresses (pub so binary crates can probe)
pub const RLR_BASE: u32 = HSEM_BASE + 0x80; ///< RLR offset from base
pub const COREID_CM7: u8 = 3; ///< CPU1 (from HAL: HSEM_CPU1_COREID)
pub const COREID_CM4: u8 = 1; ///< CPU2 (from HAL: HSEM_CPU2_COREID)
pub const PROCID_DEFAULT: u8 = 0;


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

#[panic_handler]
pub fn panic(_: &PanicInfo) -> ! {
    loop {}
}

/// My own HalDelay() implementation
pub fn delay(cycles: u32) {
    let mut i = cycles;
    while i != 0 {
        //raw asm!("nop") instead of cortex_m::asm::nop() with options(nomem, nostack, preserves_flags).
        unsafe { asm!("nop"); }
        i -= 1;
    }
}

const LOCK_BIT: u32 = 1 << 31;
const COREID_MASK: u32 = 0x0F;
const COREID_SHIFT: u32 = 8;

/// Try to take a semaphore (non-blocking).
///
/// Returns true if acquired, false if already held by another master.
///
/// # Safety
///
/// Same as `hsem_take`.
pub unsafe fn hsem_try_take(sem: usize, coreid: u8, procid: u8) -> bool {
    let r_reg   = (HSEM_BASE + sem as u32 * 4) as *mut u32;
    let rlr_reg = (RLR_BASE + sem as u32 * 4) as *mut u32;
    let val = (coreid as u32) << COREID_SHIFT | procid as u32;
    unsafe { core::ptr::read_volatile(rlr_reg); }
    unsafe { core::ptr::write_volatile(r_reg, val); }
    unsafe { core::arch::asm!("dmb"); }
    let status = unsafe { core::ptr::read_volatile(rlr_reg) };
    (status & LOCK_BIT) != 0 && ((status >> COREID_SHIFT) & COREID_MASK) as u8 == coreid
}

/// Spin until semaphore acquired.
///
/// Uses RLR-based detection to work around AXI read-buffer stale-data issue
/// (reading R[n] after writing it may return stale data on STM32H7 AXI fabric).
///
/// # Safety
///
/// - `coreid` must match the calling CPU.
/// - `sem` must be 0..31.
pub unsafe fn hsem_take(sem: usize, coreid: u8, procid: u8) {
    let r_reg   = (HSEM_BASE + sem as u32 * 4) as *mut u32;
    let rlr_reg = (RLR_BASE + sem as u32 * 4) as *mut u32;
    let val = (coreid as u32) << COREID_SHIFT | procid as u32;
    loop {
        // 1: Clear RLR.LOCK flag by reading it (clear-on-read)
        unsafe { core::ptr::read_volatile(rlr_reg); }
        // 2: Attempt lock
        unsafe { core::ptr::write_volatile(r_reg, val); }
        unsafe { core::arch::asm!("dmb"); }
        // 3: RLR.LOCK=1 + COREID matches → we own it.
        let status = unsafe { core::ptr::read_volatile(rlr_reg) };
        if (status & LOCK_BIT) != 0
            && ((status >> COREID_SHIFT) & COREID_MASK) as u8 == coreid
        {
            break;
        }
    }
}

/// Release a previously-taken semaphore.
///
/// Per the RM, writing the same COREID + PROCID toggles the semaphore.
///
/// # Safety
///
/// Must be called from the same core that took it.
pub unsafe fn hsem_release(sem: usize, coreid: u8, procid: u8) {
    let r_reg = (HSEM_BASE + sem as u32 * 4) as *mut u32;
    let val = (coreid as u32) << COREID_SHIFT | procid as u32;
    unsafe { core::ptr::write_volatile(r_reg, val); }
    unsafe { core::arch::asm!("dmb"); }
}
