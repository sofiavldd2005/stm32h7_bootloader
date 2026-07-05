# AGENTS.md — BootLoader

This repository is a workspace for an ARM Cortex-M bootloader project.

## Structure

- `Literature/` — reference manuals (`.pdf` and pre-converted `.txt` for grep):
  - `RM0399` — STM32H745/755/747/757 reference manual
  - `PM0214` — Cortex-M4 programming manual
  - `PM0253` — Cortex-M7 programming manual
  - `The Embedonomicon` — Rust `cortex-m-rt` vector tables and startup code
- `CODING_GUIDELINES.md` — rules for addressing RM lookups, PAC usage, table citations, and debug methodology
- `opencode.json` — OpenCode agent permission rules (must ask before: editing, destructive `bash`, git mutating writes, `sudo` is denied)

## Status

Bootloader workspace scaffolded with custom vector tables and exception
handlers from scratch (no `cortex-m-rt`). Dual-core bringup complete:

- **CM7**: PLL1 at 392 MHz, LD2 blink (PE1), UART "Hello World"
  (PD8/PD9, 115200 8N1)
- **CM4**: LD1 blink (PB0) with custom vector table at `0x08100000`,
  VTOR init, AHB4ENR self-enable

### Dual-core handshake working

- HSEM semaphore 0 used for inter-core sync:
  - **CM4** CoreID = **1** (confirmed via RLR-based probe)
  - **CM7** CoreID = **3** (from HAL)
  - CM4 writes magic → signals CM7 via sem → CM7 reads magic, writes DIAG
    → CM4 reads DIAG (result `0xCAFE_F00D` at `0x2400000C`)

### Phase 6.5 — Firmware CRC validation

- CRC-32/MPEG2 (non-reflected, `0x04C11DB7`) validates CM4 firmware at boot.
- `cm7/build.rs` computes CRC from CM4 ELF segments at build time, emits `CRC_GOLDEN`.
- Runtime uses STM32 hardware CRC peripheral byte-at-a-time via `CRC_DR8`.
- `FW_APPROVED` flag at `0x2400_0010`; `0xDEAD_BEEF` = approved.
- CM4 polls with 5s timeout (10× ~500ms), fast-blinks on failure.

### Phase 7a — Boot mode selection

- `BOOT_MODE` at `0x2400_0014`; `BOOT_NORMAL=0`, `BOOT_UPDATE=1`.
- Button B1 (PC13) sampled at startup to select mode.
- Update mode uses PB14 (LD3, red LED) blink.
- CM4 reads `BOOT_MODE` after handshake; skips `FW_APPROVED` poll in UPDATE mode.

### Module reorganization

- `cm7/src/` split from monolithic `main.rs` into `uart.rs`, `clock.rs`, `crc.rs`, `handshake.rs`, `gpio.rs`.
- `cm4/src/` split into `handshake.rs`, `approval.rs`.
- `shared/src/mem.rs` extracted for shared-memory constants.

### Critical debug notes

- **B1 button is on PC13, NOT PA0** on this board (NUCLEO-H755ZI-Q). PA0 reads HIGH always (floating/analog).
- **B1 is active-HIGH**: external pull-down on PC13, button connects to VDD. Check `!is_pin_low()` for "pressed".
- **GPIO port spacing is 0x400** (not 0x04). GPIOC = 0x5802_0800 (not 0x5802_0008). Each port maps 1024 bytes.
- **GPIO init requires DSB**: AHB4ENR clock-enable and MODER/PUPDR writes need `asm!("dsb")` to guarantee ordering on Cortex-M7 AXI bus.
- **Analog-mode bug**: STM32H7 GPIO defaults to MODER=3 (analog), IDR reads 0 → false "button pressed". Must configure MODER=00 and enable PUPDR pull-up before reading.
- CM4 must set `AHB4ENR |= GPIOBEN` itself; CM7 setting it is insufficient. See `CM4_GPIOB_ACCESS.md`.
- **Flash AXI Write Buffer (`FW` bit)**: STM32H7 flash uses 256-bit words. Partial writes (< 256 bits) sit in a buffer. The `FW` (Force Write) bit in `CR` must be set **AFTER** writing the data to the memory address to flush the buffer. Setting `FW` before the data write flushes an empty buffer and discards the actual data.
- **Flash KEYR2 Address**: When `SWAP_BANK=0`, `FLASH_KEYR2` is at offset `0x104` (not `0x108` as some RM tables imply). The `stm32h7-staging` PAC is correct (`bank2().keyr()`).

### HSEM AXI read-buffer workaround

- Reading `HSEM_R[n]` after a write may return STALE data (Cortex-M7 AXI read-buffer issue).
- **Solution**: read `HSEM_RLR[n]` instead — has **clear-on-read LOCK** flag semantics.
- `shared::hsem_take()` / `hsem_try_take()` / `hsem_release()` use RLR-based detection.
- Release writes COREID (not 0) to toggle the semaphore, per RM.

### Next

- Phase 8: Postcard+COBS UART firmware upgrade protocol, host CLI
- CI pipelines, build/test/lint configuration

## Agent constraints (from `opencode.json`)

- **Read**: allowed broadly; `.env*` and `.ssh/` files denied
- **Edit**: must ask the user before every edit
- **Bash**: most commands allowed; `rm`, `touch`, `mv`, `git commit/push/pull/reset/clean`, `npm install -g`, `brew`, `apt` require asking; `sudo` is denied
- **External directory** access requires asking
