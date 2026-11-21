<!-- SPDX-License-Identifier: MIT -->

# Column: Success / Hardware-Verified on SM-G780G (`r8q`)

Subsystems verified working on real hardware as of **`2026-11-21`** (`7.3.0-rc6-omamobile-push1-gae195e6deaac`, native root `/dev/sda36`, `omamobile-device-r8q 0.1.33-1`, `pd-mapper 1.1-2`, `omamobile-ai 0.1.0-1`). Run `tools/r8q-progress --all` to inspect every completed atomic task with verification evidence.

| Card | Scope | Verified Hardware Proof (`2026-11-21`) |
| --- | --- | --- |
| [01 Dual-Boot & UFS RO Guard](success/01-dual-boot-ufs-ro-guard.md) | `sda24` recovery boot -> native `userdata` (`/dev/sda36`) root | `77/77` `/dev/sda*` partitions locked `ro=1`; LineageOS partitions untouched (`NOTES §40-43`) |
| [02 RPMh TCS Reclaim & Liveness Net](success/02-rpmh-rsc-and-liveness-net.md) | `rpmh-rsc.c` (`e96cf40a1`), XPU `no-map` (`1324fe066`), watchdog | Zero RPMh/RCU freezes; `HardwareCorrupted: 0 kB` |
| [03 Display: `simpledrm` -> MSM & S6E3FC3](success/03-display-mdss-s6e3fc3.md) | `msm_dpu` + `DSI0` + `ae94000.dsi.0`, cleared `protected-clocks` | Zero white flash; native `1080x2400@120`; zero DSI/TE errors; DPMS + lock wake ok |
| [04 Adreno 650, Freedreno & Hyprland Desktop](success/04-gpu-adreno650-hyprland-desktop.md) | `a650_sqe.fw` + `a650_gmu.bin`, fault quiesce (`ae195e6de`), `stmfts`, keys | Turnip Vulkan `pp64 30.23 / tg16 1.37 t/s`; desktop survives GPU fault recovery; VolUp/Down/Power + `5-0049` live |
| [05 USB Gadget, Power/Battery & Memory/ZRAM](success/05-usb-power-battery-zram.md) | `dwc3` USB NCM + ACM, `max77705`/`max17042`, `2.7G` zram + `4G` swapfile + MGLRU | SSH at `172.16.42.1`; `100%`, `27 °C`, `health Good`; `6.8G` tiered swap |
| [06 Wi-Fi, Bluetooth, Clock Sync & Boot GUI](success/06-wifi-bluetooth-and-clock.md) | `ath11k_pci` `wlp1s0`, QCA6390 `hci0` (`998000.serial`, `caa385175`), RTC `sysinit`, `sddm` | `wlp1s0` UP; `hci0` `Powered: yes` (`BR/EDR`+`LE`); auto-login Hyprland + `quickshell` |
| [07 Stability Soak, Pseudo-NMI & On-Device AI](success/07-stability-soak-nmi-and-ai.md) | `tools/r8q-stability-soak`, `CONFIG_ARM64_PSEUDO_NMI`, pi-tuned `LFM3-1.7B` | `1800 s` soak (`failures=0`); NMI watchdog on; CPU Q8 + harbor-pi LoRA at `10.5 tok/s`; full-duplex `SamsungGalaxyS2` audio (`-9.5 dB` + `DMIC3`), `omacam` `0.5x <-> 3x` cams, RT8547 `white:torch` & SLPI sensors live (`NOTES §51–§53`) |

Back to [Board Overview](README.md) · [Remaining Queue](MISSING.md).
