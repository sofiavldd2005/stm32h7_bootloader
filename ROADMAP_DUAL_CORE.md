# Phase 6 — Dual-Core Workspace & Flash Partitioning (Steps 1+2+3)
A. Phase 6 Sub-ROADMAP
## Phase 6 — Dual-Core Bootloader

### 6.1 — Workspace & Flash Partitioning
- Update memory.x: CM7 gets 1 MB (0x08000000), reserve 1 MB for CM4
- Create cm4/ crate with its own vector table at 0x08100000
- Hold CM4 in reset via raw write to RCC_MP_C1GR1 (CM4RST bit)

### 6.2 — CM4 Firmware
- CM4 blinks its own LED (e.g., PB1 = LD3 red)
- Use same stm32h7-staging PAC with stm32h747cm4 feature
- CM4 linker script: FLASH at 0x08100000, RAM at 0x10000000 (DTCM)

### 6.3 — CM7 Bootloader Logic
- Validate CM4 firmware CRC before launch
- Release CM4 from reset, set CM4BOOT vector
- Fail-safe: if CRC invalid, hold CM4 and signal error via UART/LED

### 6.4 — Inter-Core Communication
- HSEM hardware semaphores for mutual exclusion
- Shared memory mailbox in AXI SRAM (0x24000000)
- Simple message protocol (ping/pong or shared state)

B. Concrete edits needed

1. Update workspace Cargo.toml — add "cm4" to members
2. Update cm7/memory.x — change LENGTH = 2M to LENGTH = 1M
3. Create cm4/ crate (new files):
- cm4/Cargo.toml — dep on stm32h7-staging with stm32h747cm4 feature
- cm4/src/main.rs — vector table at 0x08100000, Reset handler, loop {}
- cm4/link.x — FLASH: ORIGIN = 0x08100000, LENGTH = 1M; RAM: ORIGIN = 0x10000000, LENGTH = 128K
- cm4/build.rs — same as cm7's
- cm4/memory.x — empty or matching link.x
4. Add CM4RST to CM7 system_init():
// Hold CM4 in reset (register not in PAC, use raw write)
const RCC_MP_C1GR1: *mut u32 = (0x5802_4400 + 0x100) as *mut u32;
unsafe { core::ptr::write_volatile(RCC_MP_C1GR1, 0x01); }
5. Write PHASE6_ROADMAP.md with the sub-phase breakdown above
6. Update project .cargo/config.toml — add separate runner config for CM4 target
