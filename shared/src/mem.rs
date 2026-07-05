// Shared memory addresses (AXI SRAM at 0x2400_0000)
pub const SHARED_MAGIC: *mut u32 = 0x2400_0000 as *mut u32;
pub const PROBE: *mut u32 = 0x2400_0004 as *mut u32;
pub const DIAG: *mut u32 = 0x2400_0008 as *mut u32;
pub const FW_APPROVED: *mut u32 = 0x2400_0010 as *mut u32;
pub const BOOT_MODE: *mut u32 = 0x2400_0014 as *mut u32;

pub const BOOT_NORMAL: u32 = 0;
pub const BOOT_UPDATE: u32 = 1;

// Values necessary to use with then HSEM taken out from the RM03999
pub const COREID_CM7: u8 = 3;
///< CPU1 (from HAL: HSEM_CPU1_COREID)
pub const COREID_CM4: u8 = 1;
///< CPU2 (from HAL: HSEM_CPU2_COREID)
pub const PROCID_DEFAULT: u8 = 0;

///< HSEM base addresses (pub so binary crates can probe)
pub const HSEM_BASE: u32 = 0x5802_6400;
/// RLR offset from base
pub const RLR_BASE: u32 = HSEM_BASE + 0x80;
