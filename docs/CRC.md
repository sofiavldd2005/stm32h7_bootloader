# CRC-32/MPEG2 Firmware Validation

## Overview

Before the dual-core handshake, CM7 validates the integrity of CM4's firmware by computing a CRC-32 checksum over the entire 1 MB CM4 flash region (`0x0810_0000` – `0x081F_FFFB`). The golden CRC is computed **at build time** from the CM4 ELF binary and embedded in the CM7 image. If the checksums match, CM7 sets a shared-memory flag (`FW_APPROVED`) that CM4 polls before entering its application loop.

## CRC Algorithm

The STM32H7 CRC peripheral uses **CRC-32/MPEG2** (non-reflected) with the Ethernet polynomial `0x04C11DB7`:

| Property | Value |
|----------|-------|
| Polynomial | `0x04C11DB7` (x³² + x²⁶ + x²³ + x²² + x¹⁶ + x¹² + x¹¹ + x¹⁰ + x⁸ + x⁷ + x⁵ + x⁴ + x² + x + 1) |
| Initial value | `0xFFFF_FFFF` |
| Input reflection (REFIN) | No |
| Output reflection (REFOUT) | No |
| Final XOR | `0x0000_0000` (none) |
| Also known as | CRC-32/MPEG2, CRC-32/BZIP2 |

This is **not** the same as the common PKZIP CRC-32 (`crc32fast`, `binascii.crc32`) which uses reflected input/output with a final XOR of `0xFFFF_FFFF`.

### Bit-by-bit implementation

The software implementation used in `cm7/build.rs` processes one byte at a time:

```
crc = 0xFFFF_FFFF
for each byte b in data:
    crc = crc XOR (b << 24)
    for 8 iterations:
        if crc & 0x8000_0000:
            crc = (crc << 1) XOR 0x04C11DB7
        else:
            crc = crc << 1
result = crc
```

The byte enters at the **MSB** of the 32-bit CRC shift register (bit 31), which matches how the STM32 CRC peripheral processes byte writes in non-reflected mode.

### Test vector

| Input | Expected CRC-32/MPEG2 |
|-------|----------------------|
| `""` (empty) | `0xFFFF_FFFF` |
| `"123456789"` | `0x0376_E6E7` |

## Build-time CRC (`cm7/build.rs`)

```
┌─────────────────────────────────────────────────────────────────────┐
│ cm7/build.rs                                                        │
│                                                                     │
│ 1. Read CM4 ELF: cm4/target/.../<profile>/cm4                      │
│ 2. Parse ELF segments via `object` crate                            │
│ 3. Build 1 MB buffer of CM4 flash region:                           │
│    - Copy segment data at correct offsets                           │
│    - Fill gaps with 0xFF (same as unprogrammed flash)               │
│    - Exclude last 4 bytes (CRC placeholder at 0x081F_FFFC)          │
│ 4. Compute CRC-32/MPEG2 via crc32_mpeg2()                          │
│ 5. Write CRC_GOLDEN to $OUT_DIR/crc_golden.rs                      │
│                                                                     │
│ If CM4 ELF not found: writes CRC_GOLDEN = 0xFFFF_FFFF (fail-safe)  │
└─────────────────────────────────────────────────────────────────────┘
```

The generated file is included in `cm7/src/crc.rs` via:

```rust
include!(concat!(env!("OUT_DIR"), "/crc_golden.rs"));
```

## Runtime CRC (`validate_cm4_firmware`)

```
┌─────────────────────────────────────────────────────────────────────┐
│ CM7 at boot                                                         │
│                                                                     │
│ 1. Enable CRC peripheral clock  (AHB4ENR.CRCEN)                    │
│ 2. Reset CRC unit  (CRC_CR.RESET = 1)                              │
│    → reloads INIT = 0xFFFF_FFFF                                    │
│ 3. For each byte at 0x0810_0000 .. 0x081F_FFFC (exclusive):        │
│      CRC_DR8 ← read_volatile(byte_addr)                             │
│ 4. Read CRC_DR → computed CRC                                       │
│ 5. Compare against CRC_GOLDEN                                       │
│                                                                     │
│ Match:    write FW_APPROVED = 0xDEAD_BEEF                           │
│ Mismatch: write FW_APPROVED = 0, fast-blink LD2 forever            │
└─────────────────────────────────────────────────────────────────────┘
```

## CM4 side

CM4 polls `FW_APPROVED` at `0x2400_0010` after the HSEM handshake:

- **10 polls × ~500ms** = 5 second timeout
- `FW_APPROVED == 0xDEAD_BEEF` → normal LD1 blink loop
- Timeout → fast-blink LD1 forever

## Why byte-at-a-time?

When a **byte** is written to `CRC_DR8` on the STM32, the peripheral places it at bits [31:24] of the internal processing register, then runs 8 CRC iterations (MSB first). This matches the software implementation in `build.rs` exactly. A 32-bit word write would process 4 bytes simultaneously — byte-at-a-time was chosen for direct correspondence with the bit-by-bit software CRC.

## Relevant files

| File | Role |
|------|------|
| `cm7/build.rs` | Computes golden CRC from CM4 ELF at build time |
| `cm7/src/crc.rs` | Runtime CRC validation using hardware CRC peripheral |
| `cm7/src/main.rs` | Top-level boot flow |
| `cm4/src/approval.rs` | CM4 polls `FW_APPROVED` with timeout |
| `shared/src/lib.rs` | Shared memory constants (`FW_APPROVED` at `0x2400_0010`) |
| `cm4/link.x` | Reserves last 4 bytes of CM4 flash for CRC placeholder |

## Debugging history

| Issue | Root cause | Fix |
|-------|------------|-----|
| ELF had no LOAD segments | New build.rs didn't pass `-Tlink.x` to linker | Added linker-script directives to build.rs |
| CRC always mismatched | build.rs looked for CM4 ELF at `cm7/target/...cm4` (wrong path) | Fixed path: go up 2 directories from cm7 to workspace root |
| CRC always mismatched (2) | Runtime read golden from `0x081F_FFFC` (placeholder `0x00000000`) instead of build.rs constant | Added `include!` from `$OUT_DIR`; compare against `CRC_GOLDEN` |
| Status message not printed | `uart_init()` called after CRC check | Moved `uart_init()` before validation |
| CRC algorithm mismatch | Used `crc32fast` (reflected PKZIP) | Implemented `crc32_mpeg2()` non-reflected bit-by-bit algorithm |
