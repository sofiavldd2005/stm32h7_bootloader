## Phase 6 — Dual-Core Bootloader

### 6.1 — Workspace & Flash Partitioning (Done)
- [x] Update memory.x: CM7 gets 1 MB (0x08000000), reserve 1 MB for CM4
- [x] Create cm4/ crate with its own vector table at 0x08100000
- [x] Hold CM4 in reset via raw write to RCC_MP_C1GR1 (CM4RST bit)

### 6.2 — CM4 Firmware (Planned)
- [ ] CM4 blinks its own LED (e.g., PB1 = LD3 red)
- [ ] Use stm32h7-staging PAC with stm32h747cm4 feature
- [ ] CM4 linker script: FLASH at 0x08100000, RAM at 0x10000000 (DTCM)
- [ ] Flashing: probe-rs can target either core via `--chip STM32H755ZITx` (core-select)

### 6.3 — CM7 Bootloader Logic (Planned)
- [ ] Validate CM4 firmware CRC before launch
- [ ] Release CM4 from reset, set CM4BOOT vector
- [ ] Fail-safe: if CRC invalid, hold CM4 and signal error via UART/LED

### 6.4 — Inter-Core Communication (Planned)
- [ ] HSEM hardware semaphores for mutual exclusion
- [ ] Shared memory mailbox in AXI SRAM (0x24000000)
- [ ] Simple message protocol (ping/pong or shared state)
