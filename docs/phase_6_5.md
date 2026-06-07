
## Phase 6.5 — Implementation Plan

### Key discovery: CRC placeholder already in CM4 linker script

`cm4/link.x:27-30` already reserves the last 4 bytes of CM4 flash for a CRC:
```ld
.crc ORIGIN(FLASH) + LENGTH(FLASH) - 4 :
{
    LONG(0x00000000);
} > FLASH
```

This changes the approach slightly — we can store the golden CRC **in the CM4 binary itself**, and CM7 reads it from `0x081F_FFFC`.

### Build flow

| Step | What | How |
|------|------|-----|
| 1 | Build CM4 binary | `make release-cm4` — produces ELF at `cm4/target/thumbv7em-none-eabihf/release/cm4` |
| 2 | **build.rs** extracts `.vector_table` + `.text` + `.crc` sections from CM4 ELF, computes CRC32 of the **first 1M−4 bytes**, writes `crc_golden.rs` | Adds `object` crate in CM7's `[build-dependencies]`; no external tool needed |
| 3 | Build CM7 with golden CRC included | `include!(concat!(env!("OUT_DIR"), "/crc_golden.rs"))` |
| 4 | Flash both | `make flash-cm4 && make flash-cm7` |

### Runtime flow (CM7, in `led_blink()` before the handshake)

```
Enable CRC clock    (AHB4ENR.CRCEN bit 19)
Reset CRC unit
Feed loop: for addr in 0x0810_0000 .. 0x081F_FFFC step 4:
    word = read_volatile(addr)
    write CRC_DR ← word
result = read CRC_DR
golden = read_volatile(0x081F_FFFC)

if result == golden:
    FW_APPROVED ← 0xDEAD_BEEF
else:
    FW_APPROVED ← 0
    fast blink LD2 at 100ms forever
→ continue to HSEM handshake
```

### Runtime flow (CM4, after VTOR init)

```
Poll FW_APPROVED in a loop with ~500ms delay (shared::delay(3_000_000))
If FW_APPROVED == 0xDEAD_BEEF → normal blink
After ~10 polls (≈5s) without approval → fast blink LD1 at 100ms forever
```

### Files to modify

| File | Change |
|------|--------|
| `shared/src/lib.rs` | Add `FW_APPROVED` const at `0x2400_0010` |
| `cm7/Cargo.toml` | Add `[build-dependencies]` with `object = "0.36"` and `crc32fast = "1.4"` |
| `cm7/build.rs` (new) | Read CM4 ELF, extract loadable bytes, compute CRC32, write `$OUT_DIR/crc_golden.rs` |
| `cm7/src/main.rs` | CRC validation loop before HSEM handshake, fast-blink on failure |
| `cm4/src/main.rs` | Poll `FW_APPROVED` with 5s timeout, fast-blink on failure |
| `Makefile` | Add comments about build order (cm4 before cm7 in release) |

### Edge cases considered

| Edge case | How it's handled |
|-----------|------------------|
| Build.rs can't find CM4 binary | Emit `CRC_GOLDEN = 0xFFFF_FFFF` + warning to stderr; runtime will fail and fast-blink |
| Unprogrammed flash between `.text` and `.crc` section | Reads as `0xFF_FF_FF_FF` from hardware — consistent with `objcopy -O binary` padding |
| CM4 binary not built yet | User must build cm4 first (`make release-cm4`), then cm7; documented in Makefile |
| `.crc` section (4 bytes `0x0000_0000`) | Excluded from CRC computation (only CRC first 1M−4 bytes) |
| Both cores running concurrent CRC | Not possible — CRC is on CM7 only, before handshake, CM4 is still in poll loop |

### Verification

After implementation:
- `make check` passes clippy on all crates
- `make cm7` compiles (may warn if CM4 binary missing; should still produce a binary)
- Flash and observe: LD2 normal, LD1 normal → CRC passed. Intentionally corrupt a byte in CM4 flash → LD2 fast-blink, LD1 fast-blink after 5s

---

Ready to proceed with implementation?

---

