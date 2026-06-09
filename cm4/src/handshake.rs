use shared::{delay, DIAG};

pub fn handshake_with_cm7() {
    unsafe {
        shared::hsem_take(0, shared::COREID_CM4, shared::PROCID_DEFAULT);
    }
    unsafe {
        core::ptr::write_volatile(shared::DIAG, 0xCAFE_0002);
    }
    unsafe {
        shared::hsem_release(0, shared::COREID_CM4, shared::PROCID_DEFAULT);
    }
    // Poll until CM7 writes 0xCAFE_F00D
    loop {
        let d = unsafe { core::ptr::read_volatile(shared::DIAG) };
        if d == 0xCAFE_F00D {
            unsafe {
                core::ptr::write_volatile(0x2400_000C as *mut u32, d);
            }
            break;
        }
        delay(1_000);
    }
}
