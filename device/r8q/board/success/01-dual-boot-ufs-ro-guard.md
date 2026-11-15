<!-- SPDX-License-Identifier: MIT -->

# Success 01: Dual-Boot Recovery & Read-Only UFS Guard

- **Status:** Verified on `SM-G780G` (`r8q`) across repeated boots (`7.3.0-rc6-omamobile-push1-gd469bd3bf2a9`).
- **Back:** [SUCCESS.md](../SUCCESS.md) · [Board Overview](../README.md)

## What Works
- Omarchy boots from Samsung `recovery` (`sda24`, header v2 packed with `tools/r8q-boot pack --dual-boot`) via `VolUp + Power`.
- Root filesystem mounts cleanly from `/omarchy-r8q.img` on microSD `mmcblk0p1` (`8804000.sdhci`, `omamobile.loop=/omarchy-r8q.img`) using `/dev/loop0`.
- All `77/77` `/dev/sda*` UFS partitions stay hardware/kernel-locked read-only (`ro=1`).
- Primary Android (LineageOS 23.2 on `boot` `sda23`, `super` `sda29`, `userdata` `sda36`) and all calibration/NV partitions (`efs`, `sec_efs`, `persist`, `modemst*`, `fsg`, `fsc`) remain completely untouched.

## Evidence
- `r8q-preflight --json`: `"ufs_guard":{"ok":true,"detail":"77 UFS partitions read-only"}`.
- `grep -H . /sys/block/sda/sda*/ro /sys/block/sda/ro | grep -v ":1$"` prints nothing on live system.
