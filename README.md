# A simple DIY bootloader for the STM32H755ZIQ MCU

So basically a year ago (summer 2025), I bought a nucleo board for this MCU, because I wanted to start exploring dual-core architectures. 
I first started programming this MCU with STM32CubeIDE, and I hated how dual-core projects STM32CubeIDE projects were organized.
The most I was able to do was to have the cores communicating with each other using SPI and have each one toggle a different LED, since then my
dual-core experiments have been on hold.
At the same time, I had been learning Rust since 2024, and started doing some embedded rust stuff in the summer of 2025. This year I came in contact with the [The Embedonomicon](https://docs.rust-embedded.org/embedonomicon/exceptions.html) book.
I've always had an interest on bootloader and wanted to do one myself just for the fun of it. So after reading the book, I just though lets do one for the nucleo I have.

I still strugle a bit with dual-core MCUs and have a talent to brick stms, 
so I started with a bootloader for the Cortex-M7 core, as a learning experience :)
Update: Now both cores run: CM7 handles UART + LD2, CM4 blinks LD1.
Update: Dual-core HSEM handshake working — CM4 writes magic, CM7 reads it and writes DIAG, CM4 reads back DIAG (result `0xCAFE_F00D`).

## Project Status

| Phase | Description | Status |
|-------|-------------|--------|
| 0 | Workspace scaffold, linker, build.rs | Done |
| 1 | Custom vector table, Reset handler | Done |
| 2 | All 14 exception vectors | Done |
| 3 | PLL1 → 392 MHz, SMPS, VOS1, LD2 (PE1) blink | Done |
| 4 | USART3 "Hello World" (115200 8N1, PD8/PD9) | Done |
| 5 | Dual-core bringup: CM4 LD1 blink (PB0) | Done |
| 6 | Dual-core bringup: CM4 blink, HSEM handshake | Done |
| 6.5 | Firmware CRC validation | Done |

### Phase 6 details

**HSEM handshake** — See [`docs/HSEM_STATE_FLOW.md`](docs/HSEM_STATE_FLOW.md) for the full state diagram.

Semaphore 0 used for inter-core sync. CM4 CoreID=1, CM7 CoreID=3. RLR-based lock detection works around the Cortex-M7 AXI read-buffer stale-data issue.

**CRC firmware validation** — See [`docs/CRC.md`](docs/CRC.md) for details.

At build time, `cm7/build.rs` reads the CM4 ELF, extracts the binary content, and computes a CRC-32/MPEG2 checksum. This golden CRC is embedded as a constant in CM7 via `include!`.

At boot, CM7 uses the hardware CRC peripheral to re-compute the CRC of the CM4 flash region (byte-at-a-time via `CRC_DR8`). If the computed CRC matches the golden value, CM7 writes `0xDEAD_BEEF` to `FW_APPROVED` in shared memory and proceeds to the HSEM handshake. On mismatch, LD2 fast-blinks and the bootloader halts.

CM4 polls `FW_APPROVED` with a 5-second timeout before entering its normal blink loop.

## AI Disclaimer

An AI assistant (OpenCode) was used to:
- Design and refine the phases in ROADMAP.md
- Generate PLL and clock tree configuration code (Phase 3)
- Write the USART3 initialization and byte-transmit logic (Phase 4)
- Troubleshoot and fix the HSEM inter-core handshake implementation
  (RLR-based lock detection, AXI read-buffer workaround, COREID probing)
- Implement the CRC-32/MPEG2 firmware validation system (build.rs + hardware CRC)
- Debug the CRC mismatch caused by reflected vs non-reflected CRC algorithm mismatch
- Refactor monolithic main.rs into modules (uart, clock, crc, handshake)

All output code by AI was reviewed by a human developer.
