# Coding Guidelines — BootLoader

## Reference Manuals First

**Never guess** addresses, register field values, COREIDs, or configuration sequences. All peripherals and core behaviour are documented in the reference manuals available in `Literature/`:

| Document | Covers |
|----------|--------|
| `RM0399.txt` (available as `.pdf` too) | STM32H745/755/747/757 memory map, peripherals, clock tree, HSEM, CRC, RCC |
| `PM0214.txt` | Cortex-M4 programming manual (core registers, memory model, MPU, barriers) |
| `PM0253.txt` | Cortex-M7 programming manual (core registers, cache, MPU, AXI, barriers) |
| `The Embedonomicon.txt` | Rust `cortex-m-rt` vector table and startup patterns |

**Before writing any register access:**
1. Look up the peripheral's base address in the RM memory map tables (`grep -n "0x5802.*CRC\|0x5802.*HSEM" RM0399.txt`).
2. Look up the register offset, field layout, and reset value in the RM register map tables.
3. Confirm the clock enable bit in the RCC section of the RM.
4. Cross-check against the PAC (`stm32h7-staging`) register definitions — the SVD is the single source of truth for register types and field widths.

## PAC is Authoritative

When the PDF text extraction conflicts with the PAC (e.g. base address, field width, reset value), **trust the PAC**. The SVD files from ST are the canonical hardware description.

## Table Lookups

Many critical values are in RM tables:
- **Table 94**: Authorized AHB bus master IDs (HSEM COREID assignments)
- **Peripheral boundary tables**: Base addresses for each peripheral
- **Register map tables**: Offsets, field names, reset values
- **Clock tree tables**: PLL config, divider ratios, enable bits

Always find and cite the table number when referencing a value from the RM.

## Naming Conventions

- `#![no_std]` + `#![no_main]` for all binary crates.
- `#[unsafe(no_mangle)]` and `#[unsafe(link_section)]` (edition 2024) for vector table entries.
- All unsafe functions must have a `/// # Safety` doc section.
- PAC peripheral types via `stm32h7_staging::stm32h747cm7` / `stm32h747cm4`.

## Debug Methodology

1. **Hypothesis → RM check → test**: never skip step 2.
2. When a register read returns unexpected data, check the bus/memory model in the PM (AXI read buffers, write buffers, cache behaviour).
3. Document root causes in `AGENTS.md` so they are not rediscovered.
4. Keep probe-rs register dumps as evidence in commit messages or related `.md` files.

## Agent Behaviour

- The AI agent must cite the specific section or table when stating a hardware value.
- If the answer is not in the local Literature, the agent must say so and ask rather than guess.
- After any incorrect assumption, the agent must correct the record in `AGENTS.md`.
