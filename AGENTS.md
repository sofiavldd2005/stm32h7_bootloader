# AGENTS.md — BootLoader

This repository is a workspace for an ARM Cortex-M bootloader project.

## Structure

- `Literature/The Embedonomicon.pdf` — reference document on `cortex-m-rt` vector tables and startup code (Rust embedded)
- `opencode.json` — OpenCode agent permission rules (must ask before: editing, destructive `bash`, git mutating writes, `sudo` is denied)

## Status

Bootloader workspace scaffolded with custom vector tables and exception
handlers from scratch (no `cortex-m-rt`). Dual-core bringup complete:

- **CM7**: PLL1 at 392 MHz, LD2 blink (PE1), UART "Hello World"
  (PD8/PD9, 115200 8N1)
- **CM4**: LD1 blink (PB0) with custom vector table at `0x08100000`,
  VTOR init, AHB4ENR self-enable

### Critical debug notes

- CM4 must set `AHB4ENR |= GPIOBEN` itself; CM7 setting it is
  insufficient. See `CM4_GPIOB_ACCESS.md`.

### Next

- Build/test/lint configuration
- CI workflows

## Agent constraints (from `opencode.json`)

- **Read**: allowed broadly; `.env*` and `.ssh/` files denied
- **Edit**: must ask the user before every edit
- **Bash**: most commands allowed; `rm`, `touch`, `mv`, `git commit/push/pull/reset/clean`, `npm install -g`, `brew`, `apt` require asking; `sudo` is denied
- **External directory** access requires asking
