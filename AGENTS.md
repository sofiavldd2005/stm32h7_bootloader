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
  - **CM4** CoreID = **1** (confirmed via RLR-based probe; HAL was correct)
  - **CM7** CoreID = **3** (from HAL)
  - CM4 writes magic → signals CM7 via sem → CM7 reads magic, writes DIAG
    → CM4 reads DIAG (result `0xCAFE_F00D` at `0x2400000C`)

### Critical debug notes

- CM4 must set `AHB4ENR |= GPIOBEN` itself; CM7 setting it is
  insufficient. See `CM4_GPIOB_ACCESS.md`.

### HSEM AXI read-buffer workaround

- Reading `HSEM_R[n]` after a write to the same register may return STALE
  data (Cortex-M7 AXI read-buffer issue on STM32H7).
- **Solution**: read `HSEM_RLR[n]` instead — it has **clear-on-read LOCK**
  flag semantics and always returns correct data.
- `shared::hsem_take()` / `shared::hsem_try_take()` / `shared::hsem_release()` all
  use RLR-based detection.
- Release writes COREID (not 0) to toggle the semaphore, per RM.

### Next

- Build/test/lint configuration
- CI workflows

## Agent constraints (from `opencode.json`)

- **Read**: allowed broadly; `.env*` and `.ssh/` files denied
- **Edit**: must ask the user before every edit
- **Bash**: most commands allowed; `rm`, `touch`, `mv`, `git commit/push/pull/reset/clean`, `npm install -g`, `brew`, `apt` require asking; `sudo` is denied
- **External directory** access requires asking
