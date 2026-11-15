<!-- SPDX-License-Identifier: MIT -->

# Success 06: Wi-Fi (`ath11k`), Bluetooth (`QCA6390` `hci0`), Clock Sync & Boot GUI

- **Status:** Verified on live SM-G780G (`2026-11-15`) under `7.3.0-rc6-omamobile-push1-gd469bd3bf2a9` + `omamobile-device-r8q` `0.1.18`.
- **Back:** [SUCCESS.md](../SUCCESS.md) · [Board Overview](../README.md)

## What Works
- **Wi-Fi (`wlp1s0`):** `ath11k_pci 0000:01:00.0` (`QCA6390 hw2.0`, `amss.bin` firmware `0x101f0625`) is up and online alongside `omamobile-r8q-wifi.service` (`r8q-wifi-aspm` masking PCIe0 `l0s_aspm` correctable errors).
- **Bluetooth (`hci0`, QCA6390 on `998000.serial` / `ttyHS0`):**
  - Root cause #1 (`qcom_geni_serial` `-19` / `-ENOSPC`): Without `/aliases/serial*` in DT, `of_alias_get_highest_id("serial")` returned `-19`, wrapping `min = -18` to unsigned int inside `ida_alloc_range()`. Fixed permanently in `wt-r8q` commit `caa385175` (`max(of_alias_get_highest_id("serial"), -1)`) and live via `r8q-bt-uart.ko` (registers runtime `serial0` alias and calls `device_links_supplier_sync_state_pause()` so probing `998000.serial` never flushes `gem_noc`/`qup_virt` `sync_state()`).
  - Root cause #2 (`CONFIG_BT_LE` & profiles): Added `CONFIG_BT_LE=y`, `CONFIG_BT_RFCOMM=m`, `CONFIG_BT_RFCOMM_TTY=y`, `CONFIG_BT_BNEP=m`, `CONFIG_BT_BNEP_MC_FILTER=y`, `CONFIG_BT_BNEP_PROTO_FILTER=y` to `device/r8q/kernel.config`, and built/staged matching `bluetooth.ko`, `btqca.ko`, `hci_uart.ko`, `rfcomm.ko`, `bnep.ko`, and `hidp.ko` in `extra/`.
  - Root cause #3 (`public-addr`): `hci_qca` downloads `qca/htbtfw20.tlv` and `qca/htnv20.bin`, then requires a non-default BD_ADDR (`HCI_QUIRK_USE_BDADDR_PROPERTY`). `ensure_r8q_bt_uart()` in `r8q-liveness-monitor` generates a locally administered `02:XX:XX:XX:XX:XX` address (`0600`) in `/var/lib/omamobile/bt-public-addr`, sets it with `btmgmt --timeout 3 --index 0 public-addr`, and powers `hci0` on (`Powered: yes`, BR/EDR + LE, `bluetooth.service` active).
  - `80-r8q-gpu.rules` pins `998000.serial` `ATTR{power/control}="on"` so runtime PM does not suspend HCI UART transfers.
- **Wall-clock & TLS / `mise` / `pacman-key`:** Fixed the `sysinit.target` ordering cycle in `omamobile-r8q-rtc.service` (`DefaultDependencies=no`, `WantedBy=sysinit.target`), added `/var/lib/systemd/timesync/clock` + `2026-11-15` fallback in `r8q-rtc-epoch`, and added `ensure_r8q_clock()` + `systemd-timesyncd` kick in `r8q-liveness-monitor` so `mise` HTTPS TLS checks and `pacman` key timestamps validate without manual clock stepping.
- **Boot-to-Desktop (`multi-user.target` cmdline):** Linked `sddm.service` under `multi-user.target.wants` and added `ensure_r8q_desktop()` in `r8q-liveness-monitor` (`0.1.18`) and `tools/r8q-install`.
