# CM4 GPIOB access on STM32H755

## Problem

The CM4 core could write to GPIOB_BSRR (ODR changed) but writes to
GPIOB_MODER were silently ignored — the register stayed at its reset
value `0xFFFFFEBF` (PB0 = analog).

### WTF is the MODER
GPIO port mode register at offset 0x00 from a GPIO base address. Each 2-bit field configures one pin's mode:
- 00 = Input
- 01 = Output
- 10 = Alternate function
- 11 = Analog (reset default)
- MODER at 0x5802_0400 (GPIOB base). Each pin uses 2 bits:

| Pin | Bits | Reset | Target 0xFFFFFEB9 |
| :--- | :--- | :--- | :--- |
| PB0 | 01:00 | 11 (an) | 01 (output) |
| PB1 | 03:02 | 11 (an) | 10 (alt fn) |
| PB2+ | ..31 | 11 (an) | 11 (an, unchanged) |
The reset value 0xFFFFFEBF = ...1111 1111 1111 1110 1011 1111. Bits 01:00 are 11 (analog), so writing to BSRR changes ODR but the pin driver is disconnected — no current to the LED. Changing bits 01:00 to 01 (output) connects the pin driver so BSRR writes actually drive the LED.
The write 0xFFFFFEB9 = ...1111 1111 1111 1110 1011 1001 — only PB0 mode changed, plus PB1 (bits 03:02) changed from 11 to 10 as collateral.

### WTF is BSRR

**ODR (Output Data Register, offset 0x14)**: Write 1 to drive a pin HIGH, 0 for LOW (in push-pull output mode). Read to see the current output state.
**BSRR (Bit Set/Reset Register, offset 0x18)**: Write-only. Lower 16 bits = set pins HIGH (BS0–BS15), upper 16 bits = set pins LOW (BR0–BR15). Writing 1 to BS0 sets ODR bit 0; 1 to BR0 clears it. Atomic — no read-modify-write needed.
So the CM4 loop does:
1. BS0=1 → PB0 HIGH → LED on
2. BR0=1 → PB0 LOW → LED off

## Symptoms

- `write_volatile(0x5802_0400, 0xFFFF_FEB9)` had no effect on MODER
- Readback from CM4 and probe-rs both showed `0xFFFFFEBF`
- BSRR writes worked: ODR changed, LED pin toggled (but no visible
  blink because MODE stayed analog)
- DSB + DMB barriers didn't help
- The shared AHB4ENR (`0x5802_44E0`) showed `GPIOBEN = 1` set by CM7
## Root cause

The CM7 enabling GPIOB clock via AHB4ENR only enables it for the D1
domain (CM7's bus fabric). The CM4, which sits on the D2 domain, needs
to request the clock through **its own write** to AHB4ENR. Even though
AHB4ENR is a shared register at the same address for both CPUs, the bus
bridge between the D2 and D1 domains apparently requires the access
request to originate from the D2-side master.

## Fix

The CM4 now writes `AHB4ENR |= GPIOBEN` (bit 1) itself, followed by a
DSB, before touching any GPIOB registers.

```rust
const AHB4ENR: *mut u32 = 0x5802_44E0 as *mut u32;
let en = unsafe { core::ptr::read_volatile(AHB4ENR) };
unsafe { core::ptr::write_volatile(AHB4ENR, en | (1 << 1)); }
unsafe { core::arch::asm!("dsb"); }
```

After this change, MODER accepts writes and the LED blinks correctly.

## Key lesson

On dual-core STM32H7, setting a peripheral clock enable in AHB4ENR from
one core does **not** guarantee the other core can access that peripheral.
Each core that needs the clock must either set AHB4ENR itself or the
clock must be pre-enabled by the boot code before either core starts
accessing peripherals.
