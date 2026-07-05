# Flash Programming on STM32H7 AXI Bus

During Phase 7a/8 implementation of the host CLI and firmware update protocol, we encountered a silent failure when trying to write to the Cortex-M4 flash (Bank 2) from the Cortex-M7 core.

## The Symptoms
- The flash `erase_all()` and `sector_erase()` operations succeeded.
- Programming a single 32-bit word using `program_word()` reported success (no error flags set in `SR2`: `WRPERR`, `PGSERR`, `STRBERR`, etc. were all clear).
- However, reading back the memory address yielded `0xFFFFFFFF` (erased state). The data was silently discarded.

## Root Cause 1: PAC Address Discrepancy for KEYR2
Initially, the diagnostic output showed the flash Bank 2 `CR2` register had its `LOCK` bit set even after calling `unlock()`. 
- The `stm32h7-staging` PAC maps the `KEYR2` register to offset `0x104` (absolute address `0x5200_2104` for STM32H747/755).
- Some RM0399 tables list `FLASH_KEYR2` at offset `0x108`.
- This led to a belief that the PAC was wrong, and an attempt was made to write the unlock keys to `0x5200_2108` via raw pointers, resulting in an imprecise `BusFault`.

**Resolution**: RM0399's SWAP_BANK register table clarifies this: when `SWAP_BANK=0`, `FLASH_KEYR2` is indeed at offset `0x104`. The PAC was correct. The keys must be written to `bank2().keyr()` (0x5200_2104). The LOCK bit was actually being re-set by subsequent code (e.g., `lock()` being called after an erase), leading to a misdiagnosis of the unlock sequence failing.

## Root Cause 2: AXI 256-bit Write Buffer Flush
The actual reason the writes were silently failing was the AXI flash interface write buffer mechanism (RM0399 §4.3.9).
- On the STM32H7, the flash word size is **256 bits** (32 bytes).
- When writing a partial word (e.g., a single 32-bit value), the data enters a write buffer and is not immediately flashed.
- To force the controller to flash a partial buffer (padding the rest with zeros), the `FW` (Force Write) bit must be set in the `CR` register.

**The Bug**:
The code was setting the `FW` bit *at the same time* as enabling programming (`PG`):
```rust
// WRONG SEQUENCE
flash.bank2().cr().modify(|_, w| w.pg().set_bit().psize().bits(2).fw().set_bit());
asm::dsb();
unsafe { core::ptr::write_volatile(addr as *mut u32, data); }
```
This caused the flash controller to flush an **empty** write buffer, leaving the actual 32-bit data sitting in the buffer indefinitely until `PG` was cleared, which discarded it.

**The Fix**:
The `FW` bit must be set **after** the memory-mapped write, effectively acting as a "flush" trigger for the data already present in the write buffer:
```rust
// CORRECT SEQUENCE
// 1. Enable programming and set PSIZE to 32-bit (2)
flash.bank2().cr().modify(|_, w| w.pg().set_bit().psize().bits(2));
asm::dsb();

// 2. Write the 32-bit word to the flash address (goes into write buffer)
unsafe { core::ptr::write_volatile(addr as *mut u32, data); }
asm::dsb();

// 3. Force flush the partial 256-bit write buffer
flash.bank2().cr().modify(|_, w| w.fw().set_bit());
asm::dsb();

// 4. Wait for BSY to clear
// ...
```
After applying this sequence, flash writes succeeded, and readbacks returned the correct `0xDEADBEEF` payload.
