<!-- SPDX-License-Identifier: MIT -->

# Success 02: RPMh TCS Reclaim, XPU Page Guard & Liveness Net

- **Status:** Verified live (`omamobile-device-r8q` `0.1.16`); native DT fix built in `1324fe066`.
- **Back:** [SUCCESS.md](../SUCCESS.md) · [Board Overview](../README.md)

## What Works
- **Bounded RPMh TCS wait & timeout reclaim:** Kernel commit `e96cf40a1` bounds `rpmh-rsc.c` wait loops (`RPMH_TCS_CLAIM_TIMEOUT_MS=10000`), releases `drv->lock` cleanly on timeout, and reclaims timed-out TCS slots (`a807c8708`, `df727e3ec`, `ebeabc490`).
- **XPU/TZ/Knox DRAM protection:**
  - Live boot on `gd469bd3bf2a9`: `5,880` unreserved pages (`23,520 KiB`) across 9 Samsung regions offlined via `/sys/devices/system/memory/hard_offline_page` (`HardwareCorrupted: 23520 kB`), now automated at boot by `offline_unreserved_xpu_pages()` in `packages/omamobile-device-r8q/r8q-liveness-monitor`.
  - Clean DT fix in `wt-r8q` (`1324fe066`, `sm8250-samsung-common.dtsi`): reserves all 10 Samsung `no-map` regions natively.
- **Liveness watchdog & monitor (`0.1.16`):** `omamobile-r8q-liveness.service` runs per-CPU `SCHED_FIFO` 99 heartbeats, reads `$SYS/devices/system/cpu/cpu$c/online` directly without `SIGPIPE` subshells, backed by `RuntimeWatchdogSec=30s`.
- **Runtime-PM wake locks:** `packages/omamobile-device-r8q/80-r8q-gpu.rules` pins `9c0000.geniqup` and `980000.i2c` to `power/control="on"`, eliminating `BCM/QUP0` RPMh churn.
