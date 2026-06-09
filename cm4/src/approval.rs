use shared::{delay, FW_APPROVED};

pub fn wait_for_approval() -> bool {
    for _ in 0..10 {
        let fw = unsafe { core::ptr::read_volatile(FW_APPROVED) };
        if fw == 0xDEAD_BEEF {
            return true;
        }
        delay(3_000_000);
    }
    false
}
