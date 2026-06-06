## Phase 6 — Dual-Core Bootloader

### 6.1 — Workspace & Flash Partitioning (Done)
- [x] Update memory.x: CM7 gets 1 MB (0x08000000), reserve 1 MB for CM4
- [x] Create cm4/ crate with its own vector table at 0x08100000
- [x] Hold CM4 in reset via raw write to RCC_MP_C1GR1 (CM4RST bit)

### 6.2 — CM4 Firmware (Done)
- [x] CM4 blinks LD3 (PB1, red LED)
- [x] Use stm32h7-staging PAC with stm32h747cm4 feature
- [x] CM4 linker script: FLASH at 0x08100000, RAM at 0x10000000 (DTCM)
- [x] VTOR set to 0x08100000 at start of Reset

### 6.3 — CM7 Releases CM4 (Done)
- [x] Release CM4 from reset (clear CM4RST) after UART init
- [x] Print "CM4 released" via UART

### 6.4 — Firmware Validation (Future)
- [ ] CRC32 validation of CM4 firmware before release
- [ ] Fail-safe: hold CM4 + error pattern on LED if CRC mismatch

### 6.5 — Inter-Core Communication (Future)
- [ ] HSEM hardware semaphores for mutual exclusion
- [ ] Shared memory mailbox in AXI SRAM (0x24000000)
- [ ] Simple message protocol (ping/pong or shared state)
