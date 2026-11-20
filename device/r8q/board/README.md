<!-- SPDX-License-Identifier: MIT -->

# Samsung Galaxy S20 FE (`r8q`, SM-G780G) Porting Board

- **Hardware / Kernel:** Qualcomm Snapdragon 865 (`SM8250`), stable kernel `7.3.0-rc6-omamobile-push1-g3667ab368ef9` (pinned in [`stable-image.conf`](../stable-image.conf)) with native DT + touch, `CONFIG_BT_LE=y` modules, `omamobile-device-r8q` `0.1.24` on the phone (`0.1.26` in the repo), `omamobile-ai` `0.1.0-1` and the packaged `omamobile-llama` `1.0.0.20261118.98d67c8-1` serving `LFM3-1.7B`.
- **Boot & Safety Model:** `recovery` (`sda24`) is the phone's power-on default — a plain power-on boots it, there is no dual-boot fallback ([NOTES §24](../NOTES.md)). All `77/77` `/dev/sda*` UFS partitions remain read-only (`ro=1`).
- **Detailed Archive & Runbooks:** [PORT-PLAN.md](../PORT-PLAN.md) · [NOTES.md](../NOTES.md) · [RUNBOOK-DISPLAY-GPU.md](../RUNBOOK-DISPLAY-GPU.md) · [CRASH-DOSSIER.md](../CRASH-DOSSIER.md)

---

## Board Columns

| [Success (`SUCCESS.md`)](SUCCESS.md) | [Failed / Do Not Repeat (`FAILED.md`)](FAILED.md) | [Missing / Next (`MISSING.md`)](MISSING.md) |
| --- | --- | --- |
| [01 Dual-Boot & UFS RO Guard](success/01-dual-boot-ufs-ro-guard.md) | [01 Unreserved XPU/TZ/Knox DRAM](failed/01-unreserved-xpu-tz-knox-dram.md) | [01 Flash Native DT `1324fe066` Recovery](missing/01-flash-native-dt-recovery-1324fe066.md) |
| [02 RPMh TCS Reclaim & Liveness Net](success/02-rpmh-rsc-and-liveness-net.md) | [02 Suspend, `cpuidle`, `schedutil` & UFS PM](failed/02-suspend-cpuidle-dvfs-ufspm.md) | [02 Gate C: Remoteprocs & Audio](missing/02-gate-c-remoteprocs-and-audio.md) |
| [03 Display: `simpledrm` -> MSM & S6E3FC3](success/03-display-mdss-s6e3fc3.md) | [03 `max77705` Built-in & Unbind](failed/03-max77705-builtin-and-unbind.md) | [03 Cellular / Modem & Wireless Soak](missing/03-wifi-bluetooth-and-modem.md) |
| [04 Adreno 650, Freedreno & Hyprland Desktop](success/04-gpu-adreno650-hyprland-desktop.md) | [04 Protected DISPCC Clocks & White Flash](failed/04-display-protected-clocks-and-white-flash.md) | [04 Sensors, Cameras & DPMS Wake](missing/04-sensors-cameras-dpms-and-installer.md) |
| [05 USB Gadget, Power/Battery & Memory/ZRAM](success/05-usb-power-battery-zram.md) | [05 `[omamobile]/mesa`, `misc.vfr` & `@120`](failed/05-omamobile-mesa-and-hyprland-vfr.md) | [05 Stability Soak, Pseudo-NMI & AI Bench](missing/05-soak-pseudonmi-and-ai-benchmarks.md) |
| [06 Wi-Fi, Bluetooth, Clock Sync & Boot GUI](success/06-wifi-bluetooth-and-clock.md) | [06 Boot & Session Hazards](failed/06-boot-and-session-hazards.md) | |
| [07 Stability Soak, Pseudo-NMI & On-Device AI](success/07-stability-soak-nmi-and-ai.md) | [07 `stmfts5` 8-Byte vs `fts5cu56a` 16-Byte Events](failed/07-stmfts5-8byte-vs-samsung-16byte-events.md) | |
| [08 Vulkan `llama.cpp` Adreno GPU Wedge](failed/08-vulkan-llm-adreno-gpu-wedge.md) | |
