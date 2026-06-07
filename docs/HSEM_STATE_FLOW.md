# Dual-core Handshake — State Flow

```
                     CM4                               CM7
                      │                                 │
                      │  write SHARED_MAGIC=0xCAFE_BABE │
                      │◄────────────────────────────────│
                      │                                 │
  ┌─────────────────┐ │                                 │
  │ hsem_take(0,1,0)│ │                                 │
  │ 1) read RLR[0]  ├─┤                                 │
  │    → LOCK clears│ │                                 │
  │ 2) write R[0]   │ │                                 │
  │    = 0x100      │ │  ─── HSEM: COREID=1 vs bus ──►  │
  │                 │ │      master=1 → match!          │
  │ 3) read RLR[0]  │ │      R[0] = LOCK=1, COREID=1    │
  │    → LOCK=1 ✓   │ │      RLR[0] = LOCK=1, COREID=1  │
  └───────┬─────────┘ │                                 │
          │           │                                 │
          ▼           │                                 │
  write DIAG=0xCAFE_0002│                               │
          │           │                                 │
  ┌─────────────────┐ │                                 │
  │hsem_release(0,1,0)│                                 │
  │ write R[0]=0x100├──── HSEM: COREID=1 matches        │
  │   → toggle unlocks│     → R[0] = LOCK=0, COREID=0   │
  └───────┬─────────┘ │     RLR stays 0x80000100        │
          │           │     (history preserved)         │
          ▼           │                                 │
  ┌─────────────────┐ │  ┌────────────────────────────┐ │
  │ POLL DIAG loop  │ │  │  hsem_take(0,3,0)          │ │
  │                 │ │  │  1) read RLR[0]            │ │
  │                 │ │  │     → clears LOCK          │ │
  │                 │ │  │  2) write R[0]=0x300       ├─┤
  │                 │ │  │     → COREID=3 vs master=3 │ │
  │                 │ │  │       → match!             │ │
  │                 │ │  │  3) read RLR[0]            │ │
  │                 │ │  │     → LOCK=1 ✓, COREID=3   │ │
  │                 │ │  └──────────┬─────────────────┘ │
  │                 │ │             ▼                   │
  │                 │ │  read SHARED_MAGIC → 0xCAFE_BABE│
  │                 │ │             │                   │
  │                 │ │  write DIAG = 0xCAFE_F00D ──────┤
  │                 │ │             │                   │
  │                 │ │  ┌─────────────────────────┐    │
  │◄── DIAG reads ──┤ │  │ hsem_release(0,3,0)     │    │
  │   0xCAFE_F00D   │ │  │ write R[0]=0x300        ├────┤
  │                 │ │  │ → toggle unlock         │    │
  ▼                 │ │  │ → R[0] = LOCK=0         │    │
store 0xCAFE_F00D   │ │  └─────────────────────────┘    │
  → 0x2400000C      │ │             │                   │
          │         │ │             ▼                   │
          ▼         │ │    UART: "CM4 magic: 0xCAFEBABE"│
   ┌──────────┐     │ │             │                   │
   │ BLINK    │     │ │             ▼                   │
   │ PB0 LED  │     │ │      ┌──────────┐               │
   │ LD1 ON   │     │ │      │ BLINK    │               │
   │ LD1 OFF        │ │      │ PE1 LED  │               │
   └─────────┘      │ │      │ LD2 ON   │               │
                    │ │      │ LD2 OFF  │               │
                    │ │      │ UART     │               │
                    │ │      │ "Hello"  │               │
                    │ │      └──────────┘               │
```

## Under the hood: HSEM_R vs HSEM_RLR

### `hsem_take(sem, coreid, procid)` — locking
```
┌───────────────────────────────────────────────────────┐
│  ①  read  HSEM_RLR[n]  ←  clears  RLR.LOCK  bit       │
│        RLR.LOCK was maybe 1 (history), now → 0        │
│                                                       │
│  ②  write HSEM_R[n] = (coreid<<8 | procid)            │
│        ┌──── HSEM  hardware  ──────────────────┐      │
│        │  write.COREID  ==  bus master.COREID? │      │
│        │    YES → R.LOCK=1, RLR.LOCK=1         │      │
│        │          update RLR.COREID/PROCID     │      │
│        │    NO  → write ignored (no effect)    │      │
│        └───────────────────────────────────────┘      │
│                                                       │
│  ③  read  HSEM_RLR[n]                                 │
│        RLR.LOCK=1  AND  RLR.COREID==our_coreid        │
│          →  we  own  the  semaphore                   │
│        otherwise → someone else has it, retry         │
└───────────────────────────────────────────────────────┘
```

### `hsem_release(sem, coreid, procid)` — releasing
```
┌───────────────────────────────────────────────────────┐
│  write HSEM_R[n] = (coreid<<8 | procid)               │
│        ┌──── HSEM  hardware  ──────────────────┐      │
│        │  Already locked by THIS COREID?       │      │
│        │    YES → toggle: R.LOCK=0 (release)   │      │
│        │    NO  → write ignored                │      │
│        │  RLR is NOT touched on release        │      │
│        └───────────────────────────────────────┘      │
└───────────────────────────────────────────────────────┘
```

## HSEM_R vs HSEM_RLR — register details

Both registers exist per semaphore `n` (0..31).

|                        | `HSEM_R[n]`                                 | `HSEM_RLR[n]`                               |
|------------------------|---------------------------------------------|---------------------------------------------|
| **Address**            | `BASE + 0x000 + 4*n`                       | `BASE + 0x080 + 4*n`                       |
| **Access**             | Read / Write                               | Read-only                                   |
| **Purpose**            | Lock & release the semaphore               | Record of the last successful lock          |
| **Bit 31 (LOCK)**      | Persistent: `1` while locked, `0` when free | **Clear-on-read**: set to `1` by any successful lock, **cleared when any master reads this register** |
| **Bits [11:8] (COREID)** | Written by master to attempt lock; read back shows current owner | Read-only, shows the COREID that last locked |
| **Bits [7:0] (PROCID)**  | Written by master as process identifier     | Read-only, shows the PROCID from last lock  |
| **Updated on...**      | Lock → `LOCK=1`; Release → `LOCK=0`         | Only on **successful lock** (not on release) |

### Why RLR avoids the stale-read problem

Reading `HSEM_R[n]` after writing it may return stale data (Cortex-M7 AXI read buffer). RLR is at a **different physical address**, so the read-buffer entry polluted by the write to `R[n]` is not reused — the read of `RLR[n]` always goes to the bus and returns fresh data.

### Clear-on-read semantics

The RLR.LOCK bit works like a **one-shot flag**:

```
Initial state: RLR.LOCK = 0

  Master A locks semaphore → RLR.LOCK = 1
  Any master reads RLR     → returns 1, then RLR.LOCK is cleared back to 0
  Any master reads RLR     → returns 0 (flag is now cleared)
  Master B locks semaphore → RLR.LOCK = 1
  ...
```

This is what makes the RLR-based `hsem_take` algorithm work:

```
step ①  read RLR[n]   → clears any stale LOCK flag
step ②  write R[n]    → attempt lock (if we succeed, RLR.LOCK becomes 1)
step ③  read RLR[n]   → LOCK=1 + COREID matches ours → we locked it
                        LOCK=0 → someone else held it, retry
```

Because step ① guarantees the flag starts at 0, a `LOCK=1` in step ③ can only have been set by **our** write in step ② (assuming no other master with our COREID is racing — which holds in a dual-core system where CM7=3 and CM4=1).

### Shared memory read (DIAG polling)
```
┌───────────────────────────────────────────────────────┐
│  CM4 reads  0x24000008  (DIAG)                        │
│    ┌─── AXI  SRAM ────────────────────────────┐       │
│    │  Normal SRAM read, NO read-buffer issue  │       │
│    │  (AXI read buffer only affects           │       │
│    │   peripheral address range like HSEM)    │       │
│    └──────────────────────────────────────────┘       │
│                                                       │
│  Bus vs HSEM reads:                                   │
│    HSEM_R[n] → stale data on first read after write   │
│    HSEM_RLR[n] → always correct (different address)   │
│    SRAM (like DIAG) → always correct                  │
└───────────────────────────────────────────────────────┘
```
