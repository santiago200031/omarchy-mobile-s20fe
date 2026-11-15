<!-- SPDX-License-Identifier: MIT -->

# Missing 03: Connectivity — Wi-Fi (`ath11k`), Bluetooth & Modem

- **Priority:** P3 (`M4.2`, `M5.1`, `M5.2`).
- **Back:** [MISSING.md](../MISSING.md) · [Board Overview](../README.md)

## Remaining Work
1. **Wi-Fi (`M4.2`):** Verify `wlp1s0` (`QCA6390` `ath11k` + `amss.bin` staged) and `omamobile-r8q-wifi.service` (`r8q-wifi-aspm` silencing `l0s_aspm` correctable errors); test sustained Wi-Fi transfer alongside USB NCM.
2. **Bluetooth (`M5.2`):** Test guarded `uart6` + `qca6390` (`htbtfw20.tlv` staged) under `bluetoothctl`, then re-evaluate unmasking `bt-agent` cleanly without blocking boot.
3. **Modem / Cellular:** Keep modem disabled until Wi-Fi + audio + GUI pass soak stability; keep all stock NV partitions (`efs`, `sec_efs`, `persist`, `modemst*`, `fsg`, `fsc`) strictly `ro=1` and backed up.
