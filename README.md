# A simple DIY bootloader for the STM32H755ZIQ MCU

So basically a year ago (summer 2025), I bought a nucleo board for this MCU, because I wanted to start exploring dual-core architectures. 
I first started programming this MCU with STM32CubeIDE, and I hated how dual-core projects STM32CubeIDE projects were organized.
The most I was able to do was to have the cores communicating with each other using SPI and have each one toggle a different LED, since then my
dual-core experiments have been on hold.
At the same time, I had been learning Rust since 2024, and started doing some embedded rust stuff in the summer of 2025. This year I came in contact with the [The Embedonomicon](https://docs.rust-embedded.org/embedonomicon/exceptions.html) book.
I've always had an interest on bootloader and wanted to do one myself just for the fun of it. So after reading the book, I just though lets do one for the nucleo I have.

I still strugle a bit with dual-core MCUs and have a talent to brick stms, 
so for now lets just do a bootloader for the Cortex-M7 core, as a learning experience. In the future I hope to have a bootloader that targets both cores :)

## Project Status

| Phase | Description | Status |
|-------|-------------|--------|
| 0 | Workspace scaffold, linker script, memory.x, build.rs | [Done] |
| 1 | Custom vector table, Reset handler (no cortex-m-rt) | [Done] |
| 2 | All 14 exception vectors via exceptions! macro | [Done] |
| 3 | PLL1 -> 400 MHz sys_ck, SMPS Direct, VOS1, LD2 (PE1) blink | [Done] |
| 4 | USART3 "Hello World" via ST-Link VCP (115200 8N1, PD8/PD9) | [Done] |
| 5 | Disable CM4 via RCC CM4RST | [Planned] |
| 6 | Dual-core bootloader: flash partitioning, CM4 firmware load, HSEM | [Future] |

## AI Disclaimer

An AI assistant (OpenCode) was used to:
- Design and refine the phases in ROADMAP.md
- Generate PLL and clock tree configuration code (Phase 3)
- Write the USART3 initialization and byte-transmit logic (Phase 4)

All outputed code by AI was reviewed by a human developer.
