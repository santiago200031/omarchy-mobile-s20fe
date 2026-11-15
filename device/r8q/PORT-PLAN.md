<!-- SPDX-License-Identifier: MIT -->

# Omarchy Mobile — Samsung Galaxy S20 FE (`r8q`, SM-G780G) Port Plan

- **Status (2026-11-15):** Kernel `7.3.0-rc6-omamobile-push1` (`omamobile-r8q` worktree `~/omamobile-kernel/wt-r8q`, out `~/omamobile-kernel/out-r8q`, build with `DEVICE=r8q`). Dual-boot layout: Omarchy recovery boots from `recovery` (`sda24`, `82,694,144 B`) with rootfs `/omarchy-r8q.img` on microSD `mmcblk0p1` (`8804000.sdhci`); **all 77 UFS `/dev/sda*` partitions stay `ro=1`**, leaving LineageOS 23.2 untouched on `boot` (`sda23`) + `super` (`sda29`) + `userdata` (`sda36`). Detailed diagnostic logs and crash postmortems live in [NOTES.md](NOTES.md).

---

## 1. What Is Working (Hardware-Verified on SM-G780G)

| Subsystem | Verified Implementation | Notes |
| --- | --- | --- |
| **Dual-Boot & RO Guard** | `tools/r8q-boot pack --dual-boot`, `omamobile.loop=/omarchy-r8q.img` | `77/77` UFS partitions `ro=1`; LineageOS intact on `sda23`/`sda29`/`sda36` |
| **Kernel Stability & Liveness** | `rpmh-rsc.c` bounded TCS claim + timeout reclaim (`e96cf40a1`), `CONFIG_SOFTLOCKUP_DETECTOR=y`, `omamobile-r8q-liveness.service`, `30s` watchdog | Idle record `69 min` on old kernel; RPMh/RCU fix + FIFO-99 liveness net live |
| **USB & Recovery Console** | `dwc3` USB 2.0 NCM gadget (`172.16.42.1`), `console=ttyGS0`, `serial-getty@ttyGS0`, OpenSSH | NetworkManager keeps `usb0` unmanaged (`Metric=2000`, `DNS=9.9.9.9`) |
| **Power & Battery Telemetry** | `max77705` + `max17042_battery` as `=m`, loaded by `r8q-battery-modprobe` | Live `capacity`, `voltage_now`, `current_now`, `temp`, `health`, `USB/Charging` |
| **Early FB -> MSM Display** | `framebuffer@9c000000` (9 clocks + `MDSS_GDSC`), `display.dtso` (`&dispcc` unprotected), `panel-samsung-s6e3fc3`, `ae94000.dsi.0` backlight | Zero white flash; `modprobe msm` = `0`, DPU `0x60000000`, `DSI-1` `1080x2400@120` |
| **Touch & Keys** | `stmfts` (`i2c5@49`), `gpio_keys` (`VolUp`/`VolDown`), `qpnp_pon` (Power built-in) | Touchscreen + hardware volume/power keys verified on live system |
| **Adreno 650 & Desktop Launch** | `a650_sqe.fw` (`0x112 >= 0x095`) + `a650_gmu.bin` (v2.1.8) -> `/dev/dri/card1` + `renderD128`; `extra/mesa` + `extra/vulkan-freedreno` (`1:26.2.7-1`) | `seatd` + `sddm` + `Hyprland` + `quickshell` (`omamobile-shell`) + `squeekboard` launched under `alarm`; hit `t+14s` hang under 3D load (offline fixes staged, §3) |
| **Memory & Swap** | `CONFIG_ZRAM=y` (`ram/2`, `zstd`), 4 GiB `/swapfile`, MGLRU, writeback caps | `omamobile-device-r8q` `0.1.14` VM tuning |

---

## 2. What Failed & Must Be Skipped (Do Not Repeat)

| Failed Approach | Why It Failed | Permanent Rule |
| --- | --- | --- |
| **`s2idle` / kernel suspend** | Samsung TZ never resumes from `s2idle`; freezes minutes after boot | Keep `# CONFIG_SUSPEND is not set`, `AllowSuspend=no`, and sleep targets masked |
| **`cpuidle` `state1`** | Power-collapse hangs cores under Samsung TZ, causing RCU stalls | Keep `cpuidle-no-deep-sleep` disabling all states except `WFI` |
| **`schedutil` & UFS PM** | `sugov` & UFS clock ungating (`clkscale`/`auto_hibern8`) trigger RPMh AMC `-110` stalls | Pin CPU `performance` & UFS `auto_hibern8=0`/`clkscale=0`/`clkgate=0` until M6.2 |
| **Built-in `MAX77705` / `MAX17042`** | `=y` freezes early boot at `0.95 s` | Keep `MAX77705`/`MAX17042` as `=m`, loaded after boot via `r8q-battery-modprobe` |
| **`aliases { serial0 = &uart6; }`** | Froze early boot at `ttyHS0` / `a600000.usb` (`uart6` is BT HSUART) | Never alias `serial0` to `&uart6` |
| **`display-mdss.dtso`** | Kept `&dispcc` `protected-clocks`, returning `ESC0_CLK = 0` and `-22` in `dsi_7nm_phy_enable` | Always clear `&dispcc` `protected-clocks` when enabling `&mdss` (`display.dtso`) |
| **Unclocked `simple-framebuffer`** | Clock/genpd cleanup gated ABL's live scanout clocks -> bright white panel flash | Keep 9 display clocks + `MDSS_GDSC` on `framebuffer@9c000000` |
| **`[omamobile]/mesa` on Adreno** | Pixel Mali build omits `freedreno` / `msm_dri.so` -> `Hyprland` `SIGABRT` (`EGL_NOT_INITIALIZED`) | Install `extra/mesa`, `extra/vulkan-mesa-implicit-layers`, `extra/vulkan-freedreno` |
| **`scp` 12 GB rootfs in one transfer** | Died mid-writeback (crash #14) | Use chunked resumable `tools/r8q-push` |
| **`bt-agent` / unguarded BT at boot** | Hung boot in `multi-user.target` | Keep `bt-agent` masked until M5.2 |

---

## 3. What Still Needs Fixing & Staged Gates (Next Steps)

1. **Immediate Next Boot (Inspect & Verify Offline GPU/Mem Fixes — see [RUNBOOK-DISPLAY-GPU.md](RUNBOOK-DISPLAY-GPU.md)):**
   - Read `/sys/fs/pstore/console-ramoops-0` on the next `multi-user.target` recovery boot to capture the exact cause of the `t+14s` Gate D hang (`Hyprland` + `quickshell`).
   - Staged offline fixes ready to test on `RECOVERY`:
     - **DT reserved-memory fix (`sm8250-samsung-common.dtsi`):** eliminated `OF: reserved mem: OVERLAP DETECTED!` where upstream `cdsp_secure_heap` (`0x8bf00000..0x90500000`) overlapped `gpu_mem` ZAP (`0x9031a000..0x9031c000`).
     - **GPU OPP pin (`80-r8q-gpu.rules`):** added `ATTR{max_freq}="305000000"` alongside `min_freq` to block GPU devfreq scaling / RPMh votes during 3D bring-up.
   - Test `sddm` / `Hyprland` with `max_freq=305000000` and, if needed for UBWC/GBM triage, `AQ_NO_MODIFIERS=1`; verify physical single-screen scanout (vs DSI DSC/LM check) before switching `tools/r8q-boot` off `systemd.unit=multi-user.target`.
2. **Gate C — Audio & Remoteprocs (`M5.3` / `M4.3`):**
   - Test one overlay per boot in order: `remoteprocs.dtso` (`adsp`/`cdsp`/`slpi` + `qrtr-ns`/`pd-mapper`/`tqftpserv`) -> `audio-amps.dtso` (`cs35l41@40`/`@41` on `i2c11`) -> `audio-sound.dtso` (`TERTIARY_TDM_RX_0`, `Samsung-Galaxy-S20-FE` UCM2).
3. **Connectivity & UX (`M4.2`, `M5.1`, `M5.2`):**
   - Verify `wlp1s0` Wi-Fi (`ath11k` + `r8q-wifi-aspm`), guarded `bluetoothctl` (`qca6390`), `LockGlance.qml` backlight wake + power key DPMS before re-enabling `idle.lock`. Modem stays disabled until Wi-Fi/audio pass soak.
4. **Validation, Pseudo-NMI & On-Device AI (`M2.7`, `M2.8`, `M8`):**
   - Run 30-min active GUI + writeback soak (`M2.7`), test `CONFIG_ARM64_PSEUDO_NMI=y` alone (`M2.8`), then run Vulkan (`GGML_VULKAN`) & `LFM3-1.7B-Q4_K_M` CPU (`~15.7 tok/s`) benchmarks on the stable desktop.
5. **Minor Polish Backlog:** Silence benign `dsi0_phy_pll` rcg2 warn in `dispcc`, unify container-built Freedreno Mesa or keep `extra/mesa` pin in `r8q-install`.

---

## 4. Hardware & Partition Reference

| Item | Detail |
| --- | --- |
| **Device / SoC** | Samsung Galaxy S20 FE (`SM-G780G`, `r8q`), Qualcomm SM8250 (`msm-id=<0x164 0x20001>`, `board-id=<0x8 0x6>`), ~5.5 GiB LPDDR5 |
| **Display / GPU** | `ss_dsi_panel_S6E3FC3_AMS646YD01_FHD` (`1080x2400` 120/60 Hz DSI CMD, `te-gpio=66`), `Adreno650v3` (`a650_zap` at `0x9031a000`, `a650_sqe.fw` `0x112`) |
| **Touch / PMIC / Audio** | `stmfts` (`i2c5@49`, `irq=gpio15`), `max77705@66` (`i2c0`), dual `cs35l41@40`/`@41` (`i2c11@a8c000`), `sdhc_2` (`cd-gpio=77`, no SMMU) |
| **Partitions** | `boot` (`sda23`) LineageOS; `recovery` (`sda24`, `82,694,144 B`) Omarchy selector; `userdata` (`sda36`) Android FBE **RO**; `mmcblk0p1` `/omarchy-r8q.img`; NV (`efs`, `sec_efs`, `persist`, `modemst*`, `fsg`, `fsc`) **RO & backed up** |

> **Safety Rules ([AGENTS.md](../../AGENTS.md)):** Never touch `boot`, `super`, `userdata`, bootloader, `vbmeta`, or NV partitions; ask before every flash or reboot; build in container (`tools/build-packages --local r8q`); mask all serials/MACs/IMEIs; run `tools/pr-check` and `tests/packages` before every push.
