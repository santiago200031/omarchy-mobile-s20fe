<!-- SPDX-License-Identifier: MIT -->

# Success 05: USB NCM Gadget, Power/Battery Telemetry & Memory Tuning

- **Status:** Verified on live phone (`7.3.0-rc6-omamobile-push1-gd469bd3bf2a9` + `omamobile-device-r8q` `0.1.16`).
- **Back:** [SUCCESS.md](../SUCCESS.md) · [Board Overview](../README.md)

## What Works
- **USB & Rescue Console:** `dwc3` USB 2.0 NCM gadget (`172.16.42.1`), `systemd-networkd` (`Metric=2000`, `DNS=9.9.9.9`), `NetworkManager` keeping `usb0` unmanaged, OpenSSH, and `console=ttyGS0` + `serial-getty@ttyGS0` rescue shell.
- **Power & Battery Telemetry:** `max77705` charger (`0-0066`, `0-0069`) + `max17042_battery` (`0-0036`) built as `=m` and loaded after early boot by `omamobile-r8q-battery.service` (`r8q-battery-modprobe`), keeping `max77705_charger` loaded with I2C1 pinned `on`.
  - Live readouts: `capacity=99%`, `voltage_now`, `current_now`, `temp`, `health`, `USB/Charging` status.
- **Memory, ZRAM & VM:** `CONFIG_ZRAM=y` (`ram/2`, `zstd` via `91-r8q-zram.conf`), `4 GiB` `/swapfile` on loop rootfs, MGLRU (`80-r8q-mglru.conf`), and writeback throttling (`90-r8q-vm.conf`).
