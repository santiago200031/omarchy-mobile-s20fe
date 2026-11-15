<!-- SPDX-License-Identifier: MIT -->

# Column: Success / Hardware-Verified on SM-G780G (`r8q`)

Subsystems verified working on hardware as of `2026-11-15` (branch `r8q-wifi-bt`, `omamobile-device-r8q` `0.1.18`).

| Card | Scope | Hardware Proof |
| --- | --- | --- |
| [01 Dual-Boot & UFS RO Guard](success/01-dual-boot-ufs-ro-guard.md) | `sda24` recovery boot + microSD `/omarchy-r8q.img` loop rootfs | `77/77` `/dev/sda*` partitions locked `ro=1`; LineageOS untouched |
| [02 RPMh TCS Reclaim & Liveness Net](success/02-rpmh-rsc-and-liveness-net.md) | `rpmh-rsc.c` (`e96cf40a1`), `omamobile-r8q-liveness.service`, XPU page guard | Zero RPMh/RCU freezes; `HardwareCorrupted: 23520 kB`; DT fixed in `1324fe066` |
| [03 Display: `simpledrm` -> MSM & S6E3FC3](success/03-display-mdss-s6e3fc3.md) | `framebuffer@9c000000` clocks + `display.dtso` + `ae94000.dsi.0` | Zero white flash; `modprobe msm = 0`; `DSI-1` `1080x2400@60` active |
| [04 Adreno 650, Freedreno & Hyprland Desktop](success/04-gpu-adreno650-hyprland-desktop.md) | `a650_sqe.fw` + `a650_gmu.bin`, `extra/mesa`, `sddm` + `Hyprland` + `quickshell` + `squeekboard`, `stmfts-r8q` + keys | `3d00000.gpu` at `305 MHz` (`power/control=on`), `5-0049` bound to `stmfts-r8q` (`5866f774c` in kernel) |
| [05 USB Gadget, Power/Battery & Memory/ZRAM](success/05-usb-power-battery-zram.md) | `dwc3` USB NCM + `ttyGS0`, `max77705`/`max17042`, `zram` + swapfile + MGLRU | SSH at `172.16.42.1`; `max170xx_battery/capacity=99`; `980000.i2c` awake |
| [06 Wi-Fi, Bluetooth, Clock Sync & Boot GUI](success/06-wifi-bluetooth-and-clock.md) | `ath11k` `wlp1s0`, QCA6390 `hci0` (`998000.serial` + `CONFIG_BT_LE=y`), `omamobile-r8q-rtc.service` + `timesyncd`, `sddm` auto-start | `wlp1s0` UP; `hci0` `Powered: yes` (`BR/EDR`+`LE`); `mise` TLS & `pacman-key` pass |

Back to [Board Overview](README.md).
