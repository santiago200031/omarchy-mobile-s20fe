<!-- SPDX-License-Identifier: MIT -->

# Omarchy Mobile — Samsung Galaxy S20 FE (`r8q`, SM-G780G) Port Plan

- **Status (2026-11-15):** Kernel `7.3.0-rc6-omamobile-push1` (`omamobile-r8q` worktree `~/omamobile-kernel/wt-r8q`, out `~/omamobile-kernel/out-r8q`, build with `DEVICE=r8q`). Dual-boot layout: Omarchy recovery boots from `recovery` (`sda24`) with rootfs `/omarchy-r8q.img` on microSD `mmcblk0p1` (`8804000.sdhci`); **all 77 UFS `/dev/sda*` partitions stay `ro=1`**, leaving LineageOS untouched on `boot` (`sda23`) + `super` (`sda29`) + `userdata` (`sda36`). Omarchy's full **Wayland desktop (`sddm` + `Hyprland` + `quickshell` + `squeekboard`) is live on the phone AMOLED via Adreno 650 / MSM KMS (`omamobile-device-r8q` `0.1.16`)**, with clean native DT `no-map` fixes built in `1324fe066` (`7.3.0-rc6-omamobile-push1-g1324fe066cbb`, modules pre-staged on phone). Detailed logs and crash postmortems live in [NOTES.md](NOTES.md).

---

## 1. What Is Working (Hardware-Verified on SM-G780G)

| Subsystem | Verified Implementation | Notes |
| --- | --- | --- |
| **Dual-Boot & RO Guard** | `tools/r8q-boot pack --dual-boot`, `omamobile.loop=/omarchy-r8q.img` | `77/77` UFS partitions `ro=1`; LineageOS intact on `sda23`/`sda29`/`sda36` |
| **Kernel Stability & Liveness** | `rpmh-rsc.c` bounded TCS claim + timeout reclaim (`e96cf40a1`), `omamobile-r8q-liveness.service`, `80-r8q-gpu.rules` (`0.1.16`), `30s` watchdog | Fixed `SIGPIPE` monitor loop; `9c0000.geniqup` + `980000.i2c` + `3d00000.gpu` + `3d6a000.gmu` pinned `power/control=on`; `offline_unreserved_xpu_pages` DT fallback (`23520 kB`) active on live phone & all 10 `no-map` XPU/TZ/Knox nodes added in DT `1324fe066` |
| **USB & Recovery Console** | `dwc3` USB 2.0 NCM gadget (`172.16.42.1`), `console=ttyGS0`, `serial-getty@ttyGS0`, OpenSSH | NetworkManager keeps `usb0` unmanaged (`Metric=2000`, `DNS=9.9.9.9`) |
| **Power & Battery Telemetry** | `max77705` + `max17042_battery` as `=m`, loaded by `r8q-battery-modprobe` | `max77705_charger` kept loaded (`fw_devlink` dependency of `max170xx_battery`) with I2C1 runtime-PM pinned `on` |
| **Early FB -> MSM Display** | `framebuffer@9c000000` (9 clocks + `MDSS_GDSC`), `display.dtso` (`&dispcc` unprotected), `panel-samsung-s6e3fc3`, `ae94000.dsi.0` backlight | Zero white flash; `modprobe msm` = `0`, DPU `0x60000000`, `DSI-1` `1080x2400@60` (`dpmsStatus: 1`) |
| **Touch & Keys** | `stmfts` (`i2c5@49`), `gpio_keys` (`VolUp`/`VolDown`), `qpnp_pon` (Power built-in) | Touchscreen (`stmfts_irq`) + hardware volume/power keys verified under live desktop |
| **Adreno 650 & Graphical Desktop** | `a650_sqe.fw` (`0x112`) + `a650_gmu.bin` (v2.1.8) -> `/dev/dri/card1` + `renderD128`; `extra/mesa` + `extra/vulkan-freedreno` (`1:26.2.7-1`); `0.1.16` rules & `hyprland-r8q.conf` | **Gate D PASSED (2026-11-15):** `sddm`, `Hyprland --watchdog-fd 4`, `quickshell` (`omarchy-shell`), `squeekboard`, and `Xwayland` running stably on screen (`305000000` Hz `active`, `1080x2400@60`, `scale 2.4`) |
| **Memory & Swap** | `CONFIG_ZRAM=y` (`ram/2`, `zstd`), 4 GiB `/swapfile`, MGLRU, writeback caps | `omamobile-device-r8q` `0.1.16` VM + XPU memory guard |

---

## 2. What Failed & Must Be Skipped (Do Not Repeat)

| Failed Approach | Why It Failed | Permanent Rule |
| --- | --- | --- |
| **`s2idle` / kernel suspend** | Samsung TZ never resumes from `s2idle`; freezes minutes after boot | Keep `# CONFIG_SUSPEND is not set`, `AllowSuspend=no`, and sleep targets masked |
| **`cpuidle` `state1`** | Power-collapse hangs cores under Samsung TZ, causing RCU stalls | Keep `cpuidle-no-deep-sleep` disabling all states except `WFI` |
| **`schedutil` & UFS PM** | `sugov` & UFS clock ungating trigger RPMh AMC `-110` stalls | Pin CPU `performance` & UFS `auto_hibern8=0`/`clkscale=0`/`clkgate=0` until M6.2 |
| **Built-in `MAX77705` / `MAX17042`** | `=y` freezes early boot at `0.95 s` | Keep `MAX77705`/`MAX17042` as `=m`, loaded via `r8q-battery-modprobe` |
| **Unloading `max77705_charger`** | `fw_devlink` (`supplier:i2c:0-0069--i2c:0-0036`) unbinds `max17042_battery`, losing `max170xx_battery` | Keep `max77705_charger` loaded; pin `9c0000.geniqup` + `980000.i2c` `power/control="on"` |
| **Unreserved Samsung XPU/TZ/Knox DRAM** | `mmc0` ADMA (`0x90800210` in `cdsp_secure_heap`) / DMA faulted TZ XPU, wedging CPU 0 -> RCU + RPMh `-110` freeze at `t+12..14s` | Reserve all 10 Samsung `no-map` regions in `sm8250-samsung-common.dtsi` (`1324fe066`); keep `offline_unreserved_xpu_pages()` fallback in `r8q-liveness-monitor` |
| **`aliases { serial0 = &uart6; }`** | Froze early boot at `ttyHS0` / `a600000.usb` (`uart6` is BT HSUART) | Never alias `serial0` to `&uart6` |
| **`display-mdss.dtso` / unclocked FB** | Kept `&dispcc` `protected-clocks` (`ESC0_CLK = 0` -> `-22`) or gated ABL scanout (white flash) | Clear `&dispcc` `protected-clocks` (`display.dtso`) and hold 9 clocks + `MDSS_GDSC` on `framebuffer@9c000000` |
| **`[omamobile]/mesa` on Adreno** | Pixel Mali build omits `msm_dri.so` -> `Hyprland` `SIGABRT` | Install `extra/mesa`, `extra/vulkan-mesa-implicit-layers`, `extra/vulkan-freedreno` |
| **`misc.vfr = true` & `@120` init mode** | `misc.vfr` does not exist in `hl.meta.lua` (`looknfeel.lua:1` red bar); `@120` stressed TE/compositor | Use `misc.vrr = 0` (or `debug.vfr`) and `1080x2400@60` (`hyprland-r8q.conf`; also align `tools/r8q-install`) |
| **`r8q-liveness-monitor` `online_cpus | grep`** | `SIGPIPE` / subshell storm inside monitor loop burned `1h49m` CPU over `8.5h` | Read `$SYS/devices/system/cpu/cpu$c/online` directly in shell |
| **`udisks2`, `voxtype`, `icc_bwmon`, `camcc_sm8250`, `bt-agent`** | Probing 77 UFS parts (`udisks2`), 147 MB Whisper autoload (`voxtype`), unpowered bus/camera clocks, and unguarded BT hung boot/session | Keep `icc_bwmon` + `camcc_sm8250` blacklisted, `udisks2`/`bt-agent` masked, and `voxtype` disabled until their subsystems are brought up |
| **`scp` 12 GB rootfs in one transfer** | Died mid-writeback (crash #14) | Use chunked resumable `tools/r8q-push` |

---

## 3. What Is Still Missing & Next Gates

1. **Optional Recovery Flash for Native DT (`1324fe066`) & Clean Boot:**
   - Live hardware (`gd469bd3bf2a9`) is working using the `23,520 KiB` runtime `hard_offline_page` guard (`HardwareCorrupted: 23520 kB`); **optional next step (human-approved):** add maintainer `Signed-off-by:` to `1324fe066` in `wt-r8q`, flash `recovery-omamobile.img` (SHA256 `8dec9656073dd663769eff7a587fc74e2321068abda825feb45a7636d47c2cef`, `7.3.0-rc6-omamobile-push1-g1324fe066cbb` modules already staged on phone) to `sda24`, verify `/proc/iomem` splits `90500000-`, `9e300000-`, and `9fb00000-bcbfffff` cleanly, then test booting straight into `graphical.target`.
2. **Gate C — Audio & Remoteprocs (`M5.3` / `M4.3`):**
   - Bring up one overlay per boot: `remoteprocs.dtso` (`adsp`/`cdsp`/`slpi` + `qrtr-ns`/`pd-mapper`/`tqftpserv`) -> `audio-amps.dtso` (`cs35l41@40`/`@41` on `i2c11`) -> `audio-sound.dtso` (`TERTIARY_TDM_RX_0`, `Samsung-Galaxy-S20-FE` UCM2).
3. **Connectivity, Cameras, Sensors & Installer Alignment (`M4.2`, `M5.1`, `M5.2`):**
   - Verify `wlp1s0` Wi-Fi (`ath11k` + `r8q-wifi-aspm`), Bluetooth (`uart6` / `qca6390` + unmask `bt-agent`), cellular/modem (only after Wi-Fi + audio soak), SLPI sensors & cameras (`camcc_sm8250`), `LockGlance.qml` backlight wake + Power key DPMS, and update `tools/r8q-install` (`1080x2400@60`, `misc.vrr = 0`, `extra/mesa`).
4. **Validation, Pseudo-NMI & On-Device AI (`M2.7`, `M2.8`, `M8`):**
   - Run 30-min active GUI + writeback soak (`M2.7`), evaluate `CONFIG_ARM64_PSEUDO_NMI=y` (`M2.8`), then run Vulkan (`GGML_VULKAN`) & `LFM3-1.7B-Q4_K_M` CPU (`~15.7 tok/s`) benchmarks.
5. **Minor Polish Backlog:** Silence benign `dsi0_phy_pll` rcg2 warn in `dispcc`, and unify container-built Freedreno Mesa or pin `extra/mesa` cleanly in installer scripts.

---

## 4. Hardware & Partition Reference

| Item | Detail |
| --- | --- |
| **Device / SoC** | Samsung Galaxy S20 FE (`SM-G780G`, `r8q`), Qualcomm SM8250 (`msm-id=<0x164 0x20001>`, `board-id=<0x8 0x6>`), ~5.5 GiB LPDDR5 |
| **Display / GPU** | `ss_dsi_panel_S6E3FC3_AMS646YD01_FHD` (`1080x2400` 120/60 Hz DSI CMD, `te-gpio=66`), `Adreno650v3` (`a650_zap` at `0x9031a000`, `a650_sqe.fw` `0x112`) |
| **Touch / PMIC / Audio** | `stmfts` (`i2c5@49`, `irq=gpio15`), `max77705@66` (`i2c0`), dual `cs35l41@40`/`@41` (`i2c11@a8c000`), `sdhc_2` (`cd-gpio=77`, no SMMU) |
| **Partitions** | `boot` (`sda23`) LineageOS; `recovery` (`sda24`, `82,694,144 B`) Omarchy selector; `userdata` (`sda36`) Android FBE **RO**; `mmcblk0p1` `/omarchy-r8q.img`; NV (`efs`, `sec_efs`, `persist`, `modemst*`, `fsg`, `fsc`) **RO & backed up** |

> **Safety Rules ([AGENTS.md](../../AGENTS.md)):** Never touch `boot`, `super`, `userdata`, bootloader, `vbmeta`, or NV partitions; ask before every flash or reboot; build in container (`tools/build-packages --local r8q`); mask all serials/MACs/IMEIs; run `tools/pr-check` and `tests/packages` before every push.
