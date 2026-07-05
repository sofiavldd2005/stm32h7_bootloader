use shared::{
    delay,
    mem::{COREID_CM4, DIAG, PROCID_DEFAULT},
};
pub fn handshake_with_cm7() {
    unsafe {
        shared::hsem_take(0, COREID_CM4, PROCID_DEFAULT);
    }
    unsafe {
        core::ptr::write_volatile(DIAG, 0xCAFE_0002);
    }
    unsafe {
        shared::hsem_release(0, COREID_CM4, PROCID_DEFAULT);
    }
    // Poll until CM7 writes 0xCAFE_F00D
    loop {
        let d = unsafe { core::ptr::read_volatile(DIAG) };
        if d == 0xCAFE_F00D {
            unsafe {
                core::ptr::write_volatile(0x2400_000C as *mut u32, d);
            }
            break;
        }
        delay(1_000);
    }
}
