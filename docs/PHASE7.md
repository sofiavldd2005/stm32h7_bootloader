# Phase 7 — Boot Decision & Firmware Upgrade Protocol

## Overview

Phase 7 adds two capabilities to the bootloader:

1. **Boot decision**: Sample the user button (B1, PA0) at power-on to choose between normal boot and update mode.
2. **Firmware upgrade protocol**: A UART-based packet protocol using Postcard + COBS framing for reliable firmware transfer.
My idea is to be able have a protocol that can also work with Ethernet. Right now the idea is to use UART in order to keep it simple. But doing the firmware transfers via Ethernet is a goal for the future.

This document covers the design decisions, hardware details, protocol specification, and implementation guide.

---

## Table of Contents

- [1. Boot Mode Selection](#1-boot-mode-selection)
- [2. Shared Memory Layout](#2-shared-memory-layout)
- [3. Protocol Design](#3-protocol-design)
- [4. Host Tool Design](#4-host-tool-design)
- [5. Flash Programming](#5-flash-programming)
- [6. Design Decisions](#6-design-decisions)
- [7. Implementation Order](#7-implementation-order)
- [8. Future Considerations](#8-future-considerations)

---

## 1. Boot Mode Selection

### Hardware

| Item | Detail |
|------|--------|
| Button | B1 on Nucleo-H755ZI-Q |
| Pin | PA0 |
| Logic | Active low (pressed = 0, released = 1) |
| Pull | External pull-up on board; internal pull-up also active by default |

No GPIO reconfiguration is needed after reset — PA0 defaults to input mode with pull-up enabled.

### Flow

```
Power-on reset
      │
      ▼
   system_init()  (PLL, clocks, SMPS)
      │
      ▼
   Sample PA0
      │
      ├── Pressed (Low) ────────────────────────────────┐
      │                                                  │
      │  Write BOOT_MODE = UPDATE to shared memory       │
      │  3 fast LD2 blinks (200 ms each)                 │
      │  "UPDATE MODE" on UART                           │
      │  Enter protocol command loop                     │
      │  (Phase 8: flash erase/write/crc/boot commands)  │
      │                                                  │
      └── Released (High) ───────────────────────────────┘
                                                         │
              Write BOOT_MODE = NORMAL to shared memory   │
              CRC validation of CM4 firmware              │
              HSEM handshake with CM4                     │
              Application loop (normal operation)         │
```

### CM4 side

CM4 reads `BOOT_MODE` from shared memory after the DIAG handshake:

- **NORMAL**: Poll `FW_APPROVED` as before (5s timeout), then normal blink loop.
- **UPDATE**: Skip `FW_APPROVED` polling, enter a companion loop (blink LD1 at normal rate, ready for coordinated reset when new firmware is flashed).

---

## 2. Shared Memory Layout

Extended from Phase 6.5:

| Address | Constant | Value (write) | Purpose |
|---------|----------|---------------|---------|
| `0x2400_0000` | `SHARED_MAGIC` | `0xCAFE_BABE` | Written by CM4 at boot to signal shared memory is ready |
| `0x2400_0004` | `PROBE` | — | Debug scratch |
| `0x2400_0008` | `DIAG` | `0xCAFE_F00D` | Handshake: CM7 writes DIAG, CM4 polls |
| `0x2400_000C` | — | `0xCAFE_F00D` | CM4 stores the received DIAG value |
| `0x2400_0010` | `FW_APPROVED` | `0xDEAD_BEEF` | CRC validation result from CM7 |
| `0x2400_0014` | `BOOT_MODE` | `0` or `1` | Boot mode selection |

### shared/src/lib.rs additions

```rust
pub const BOOT_MODE: *mut u32  = 0x2400_0014 as *mut u32;
pub const BOOT_NORMAL: u32 = 0;
pub const BOOT_UPDATE: u32  = 1;
```

---

## 3. Protocol Design

### Why Postcard + COBS?

| Layer | Library | Why |
|-------|---------|-----|
| Serialization | `postcard` | Extremely compact (varint-encoded), `#![no_std]`, serde-based, no alloc required |
| Framing | `cobs` | Byte-stuffing: removes `0x00` from payload so `0x00` can be used as frame delimiter; simple, standard, no dependency on UART line discipline |
| Transport | UART / Ethernet | Abstracted — changing transport means swapping the read/write calls, not the protocol logic |

### Packet format

```
[ COBS(postcard_bytes) ] [ 0x00 ]
```

1. Serialize the command/reply struct to bytes with Postcard
2. Apply COBS encoding (replaces `0x00` bytes with non-zero codes)
3. Append `0x00` as frame delimiter
4. Send over UART

On the receiver side:

1. Read bytes until `0x00`
2. COBS-decode the buffer
3. Deserialize with Postcard

### Protocol structs (in `shared` crate)

```rust
use serde::{Serialize, Deserialize};

/// Commands sent from host to bootloader.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HostCommand {
    /// Ping the bootloader — expects PingReply.
    Ping,
    /// Erase all CM4 flash sectors.
    EraseAll,
    /// Write data to a flash address.
    /// `addr` must be 256-byte aligned.
    /// `data` max 256 bytes per packet.
    Write {
        addr: u32,
        data: [u8; 256],
        len: u16,
    },
    /// Request CRC-32/MPEG2 of a flash region.
    Crc32 {
        addr: u32,
        len: u32,
    },
    /// Boot the validated firmware (jump to CM4).
    Boot,
}

/// Replies from bootloader to host.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HostReply {
    /// Command accepted.
    Ack,
    /// Command rejected with error code.
    Nak(u8),
    /// Response to Crc32 request.
    CrcResult(u32),
    /// Bootloader ready for next command.
    Ready,
}
```

Because these are in the `shared` crate with `#[derive(Serialize, Deserialize)]`, the **same types compile for both the embedded target and the host CLI tool**. No duplication.

### Error codes

| Code | Name | Meaning |
|------|------|---------|
| `0x01` | BadAddress | Address not aligned or out of range |
| `0x02` | BadLength | Data length exceeds maximum |
| `0x03` | FlashError | Flash erase/program operation failed |
| `0x04` | CrcMismatch | Final CRC verification failed |
| `0x05` | Busy | Bootloader is busy with prior operation |

### Protocol sequence (full firmware update)

```
HOST                          BOOTLOADER
 │                                │
 │───── Ping ────────────────────►│
 │◄──── Ack                       │
 │                                │
 │───── EraseAll ────────────────►│  (erases CM4 flash)
 │◄──── Ack                       │
 │                                │
 │───── Write{ addr, data } ─────►│  (repeated ~4000 times for 1 MB)
 │◄──── Ack                       │
 │                                │
 │───── Crc32{ 0x08100000, 1M }─►│  (bootloader computes hardware CRC)
 │◄──── CrcResult(0x12345678)     │
 │verifies CRC matches            │
 │                                │
 │───── Boot ────────────────────►│  (sets FW_APPROVED, signals CM4)
 │◄──── Ack                       │
```

### Transport abstraction

The protocol handler in CM7 should be transport-agnostic. The core loop:

```rust
fn receive_frame() -> Option<Vec<u8>> { /* COBS + transport read */ }
fn send_frame(data: &[u8])               { /* COBS + transport write */ }

loop {
    if let Some(frame) = receive_frame() {
        let cmd: HostCommand = postcard::from_bytes(&frame).ok()?;
        let reply = dispatch(cmd);
        let encoded = postcard::to_vec(&reply)?;
        send_frame(&encoded);
    }
}
```

To switch from UART to Ethernet later, only `receive_frame` and `send_frame` need to change.

---

## 4. Host Tool Design

### Workspace structure

```
bootloader/
├── shared/        # #![no_std], serde derives, protocol enums
├── cm7/           # target thumbv7em-none-eabihf
├── cm4/           # target thumbv7em-none-eabihf
└── host/          # native Rust binary
    ├── Cargo.toml
    └── src/main.rs
```

### host/Cargo.toml

```toml
[package]
name = "host"
version = "0.1.0"
edition = "2024"

[dependencies]
shared = { path = "../shared" }
postcard = { version = "1", features = ["alloc"] }
serialport = "4"
clap = { version = "4", features = ["derive"] }
```

### CLI interface (clap)

```
cargo run -p host -- flash <binary>          # CRC + flash + verify + boot
cargo run -p host -- ping                    # check bootloader alive
cargo run -p host -- boot                    # send boot command
cargo run -p host -- crc <addr> <len>        # request CRC of flash region
cargo run -p host -- erase                   # erase all
```

### Host workflow for `flash` command

```
1. Read CM4 binary, compute CRC-32/MPEG2 (same algorithm as build.rs)
2. Ping bootloader → verify alive
3. Send EraseAll
4. Chunk binary into 256-byte blocks, send Write packets
5. Request Crc32 of full CM4 region
6. Compare host-side CRC vs bootloader CRC
7. If match: send Boot
8. If mismatch: report error, do not boot
```

### Dependencies note

`serialport` is a native crate (not no_std) — fine for the `host` target. The `shared` crate must remain `#![no_std]` so it compiles for CM7/CM4. Keep serde + postcard usage in shared `default-features = false`.

---

## 5. Flash Programming

### Flash controller basics (STM32H7)

The CM7 flash controller is at `0x5802_2000` (FLASH1) and manages both banks:

| Register | Offset | Purpose |
|----------|--------|---------|
| `FLASH_CR1` | `0x00` | Control register: lock, unlock, programming, erasing |
| `FLASH_SR1` | `0x10` | Status register: busy, error flags |
| `FLASH_KEYR1` | `0x04` | Unlock key register (write KEY1, KEY2) |

### Unlock sequence

```rust
flash.keyr1().write(|w| unsafe { w.bits(0x45670123) });
flash.keyr1().write(|w| unsafe { w.bits(0xCDEF89AB) });
// Wait for ready
while !flash.sr1().read().bsy1().bit_is_set() {}
```

### Erase sequence (sector)

The CM4 flash bank (Bank 2, `0x0810_0000`) consists of sectors. On STM32H755, each sector is 128 KB for the first 8 sectors (1 MB total).

1. Unlock flash
2. Set `FLASH_CR1.SER = 1` (sector erase)
3. Set `FLASH_CR1.SNB` to target sector number
4. Set `FLASH_CR1.START = 1`
5. Wait for `FLASH_SR1.BSY1 = 0`

### Program sequence (256-bit write)

The H7 flash can write 256 bits (8 words) in a single programming operation. For simplicity, we use 32-bit writes:

1. Unlock flash
2. Set `FLASH_CR1.PG = 1` (programming mode)
3. Write data to target address in flash
4. Wait for `FLASH_SR1.BSY1 = 0`
5. Verify written value

### Flash constraints

| Constraint | Value |
|------------|-------|
| Minimum write unit | 32-bit word (aligned) |
| Sector size | 128 KB (Bank 2, sectors 8–15) |
| Total CM4 flash | 1 MB (8 sectors) |
| Erase time | ~100 ms per sector |
| Program time | ~10 µs per 32-bit word |

### Addressing

CM4 flash occupies `0x0810_0000 – 0x081F_FFFF`. From CM7's flash controller, the physical sector number for Bank 2 is:

```
sector_number = (address - 0x0810_0000) / 0x2_0000 + 8
```

Sectors 0–7 = CM4 bank, sectors 8–15 = CM7 bank. Write operations from CM7 to CM4's region must target the correct sector via the flash controller.

---

## 6. Design Decisions

### Why Postcard over raw binary?

- Postcard's varint encoding produces compact packets (smaller than JSON/CBOR)
- Serde derives let us use the same structs on host and embedded side
- Changing the protocol (adding new commands) doesn't break existing parsers
- Postcard is `#![no_std]` with zero allocations in the embedded decoder

### Why COBS over a length-prefix framing?

| Aspect | COBS | Length-prefix |
|--------|------|---------------|
| Delimiter | `0x00` (one byte) | Need escape mechanism or fixed-size buffers |
| Resynchronization | Receiver syncs on next `0x00` | Lost sync requires timeout |
| Maximum overhead | ~0.4% on 256-byte payload (1 extra byte per 254 payload bytes) | 0% overhead |
| Embedded complexity | 15 lines | 5 lines |
| Well-known | Yes (used in CAN, USB CDC, etc.) | Ad-hoc |

COBS wins for reliability: if a byte is corrupted mid-frame, the next `0x00` re-syncs immediately.

### Why `[u8; 256]` + length rather than `heapless::Vec`?

Both work. `[u8; 256]` + `len: u16` avoids adding a `heapless` dependency to `shared` just for one Vec. The fixed-size array is simpler, and 256 bytes is a natural max payload (flash write units are aligned).

### Why implement flash programming on CM7 rather than CM4 self-programming?

- CM7 already has the CRC peripheral and UART initialized
- CM7 has access to both flash banks
- CM4 flash can't execute code during erase/program (busy stalls the core)
- Centralizing bootloader logic on CM7 keeps CM4 simple (poll and blink)

### Why Boot command separate from Write?

The host must verify the CRC **before** telling CM7 to boot. If boot followed automatically after the last write, a partial/corrupted transfer could leave the system in an unbootable state. The explicit Boot command:
1. Gives the host final control
2. Allows the bootloader to do a final validation
3. Makes the boot decision atomic (no race between write verification and boot)

---

## 7. Implementation Order

### Phase 7a — Boot mode (no protocol yet)

| Step | Description | Files |
|------|-------------|-------|
| 1 | Add `BOOT_MODE`, `BOOT_NORMAL`, `BOOT_UPDATE` to shared | `shared/src/lib.rs` |
| 2 | Create `cm7/src/gpio.rs` — `init_button()`, `is_button_pressed()` | `cm7/src/gpio.rs` |
| 3 | Wire button check into `cm7/src/main.rs` before CRC validation | `cm7/src/main.rs` |
| 4 | Read `BOOT_MODE` in `cm4/src/main.rs` after DIAG handshake | `cm4/src/main.rs` |

**Testing**: Hold B1 during reset → 3 fast LD2 blinks, no CRC validation, "UPDATE MODE" on UART. Release B1 → normal boot flow unchanged.

### Phase 7b — Protocol types in shared

| Step | Description | Files |
|------|-------------|-------|
| 5 | Add serde, postcard, cobs deps to shared | `shared/Cargo.toml` |
| 6 | Write `HostCommand` and `HostReply` enums in shared | `shared/src/lib.rs` |
| 7 | Write COBS encode/decode helpers in shared | `shared/src/cobs.rs` |

### Phase 7c — Protocol handler + flash HAL

| Step | Description | Files |
|------|-------------|-------|
| 8 | Create `cm7/src/protocol.rs` — command dispatch loop | `cm7/src/protocol.rs` |
| 9 | Create `cm7/src/flash.rs` — unlock, erase, program, verify | `cm7/src/flash.rs` |
| 10 | Wire protocol loop into update mode path in main.rs | `cm7/src/main.rs` |

### Phase 7d — Host tool

| Step | Description | Files |
|------|-------------|-------|
| 11 | Create `host/` crate with clap CLI and serialport | `host/Cargo.toml`, `host/src/main.rs` |
| 12 | Implement `flash` command (chunk, send, verify, boot) | `host/src/main.rs` |

---

## 8. Future Considerations

### Ethernet transport

When switching to Ethernet, the shared protocol types remain unchanged. Only the transport layer changes:

```rust
// UART version
fn read_byte() -> u8 { /* poll USART3 */ }

// Ethernet version (LWIP or smoltcp)
fn read_byte() -> u8 { /* read from TCP socket */ }
```

The COBS framing also stays — Ethernet has its own framing (TCP segments), but COBS adds a simple application-layer delimiter that prevents partial-frame deserialization.

### Postcard schema evolution

Adding new variants to `HostCommand` or `HostReply` does not break backward compatibility — Postcard uses integer discriminants. Old bootloader simply receives an unknown variant and returns `Nak`. Forwards-compatible by design.

### Dual-bank boot

In a future design, CM7 could maintain two CM4 firmware slots (bank A / bank B) for safe rollback. The protocol only needs a `slot: u8` field added to `Write` and `Crc32` commands.

### Security

Postcard + COBS provides no encryption. For production, add:
- Challenge-response authentication before erase
- Signed firmware images (RSA/Ed25519 verify in bootloader)
- Encrypted transport (AES-GCM on top of COBS frames)
