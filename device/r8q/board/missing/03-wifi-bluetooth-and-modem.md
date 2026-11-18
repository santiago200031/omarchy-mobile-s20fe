<!-- SPDX-License-Identifier: MIT -->

# Missing 03: Cellular / Modem & Sustained Wireless Soak

- **Priority:** P3 (`M4.2` soak, `M5.1` modem).
- **Status (`2026-11-18`):** Wi-Fi/BT and the sustained soak are **done**, and **the modem boots to AMSS on mainline** ([MODEM-PLAN](../../MODEM-PLAN.md) §10): full SAHARA set over one MHI channel, mission mode entered by the modem itself, QMI/MBIM/DIAG/AT ports up. The first normal boot (2026-11-18) also showed that the modem power-on removes the SD card: the root goes read-only and `wwan.ko` cannot load, so the SD-less rescue image is still the only working modem configuration. What remains is a root that survives the modem power-on (a `userdata` root or a modem 3.3 V rail) and the userspace (SIM/QMI), tracked in [MISSING.md](../MISSING.md) and [MODEM-PLAN.md](../../MODEM-PLAN.md) §12.
- **Done & Moved:** Wi-Fi (`wlp1s0`, `ath11k` + `r8q-wifi-aspm`) and Bluetooth (`hci0`, `qca6390` on `998000.serial`) are now verified working in [Success 06](../success/06-wifi-bluetooth-and-clock.md).
- **Back:** [MISSING.md](../MISSING.md) · [Board Overview](../README.md)

## Remaining Work
1. **Wi-Fi + BT sustained concurrency (`M4.2`/`M5.2`):** Test sustained Wi-Fi downloads alongside USB NCM and active Bluetooth pairing/audio once Gate C audio is enabled; validate native `caa385175` + `CONFIG_BT_LE=y` kernel build without `r8q-bt-uart.ko`.
2. **Modem / Cellular (`M5.1`):** Keep modem disabled until Wi-Fi + audio + GUI pass soak stability; keep all stock NV partitions (`efs`, `sec_efs`, `persist`, `modemst*`, `fsg`, `fsc`) strictly `ro=1` and backed up.
