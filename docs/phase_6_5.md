
## What was built:
- cm7/build.rs — computes CRC-32/MPEG2 of the CM4 binary at build time, generates CRC_GOLDEN constant
- shared::FW_APPROVED at 0x2400_0010 — shared memory flag
- CM7 validates CM4 firmware CRC before handshake, writes FW_APPROVED on success
- CM4 polls FW_APPROVED with 5s timeout, fast-blinks on failure
- Fast-blink error pattern on both cores when CRC fails
## Bugs squashed:
1. Missing linker script directives in new build.rs (CM7 ELF had no LOAD segments)
2. workspace_root was missing .parent() — path resolved to cm7/target/...cm4 instead of cm4/target/...cm4
3. include! from OUT_DIR was missing — runtime compared against 0x081F_FFFC placeholder instead of computed golden CRC
4. uart_init() after CRC check — status message never printed
5. CRC algorithm: crc32fast (reflected PKZIP) → custom crc32_mpeg2 (non-reflected, bit-by-bit)
