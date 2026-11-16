<!-- SPDX-License-Identifier: MIT -->

# r8q (Galaxy S20 FE SM-G780G) — File Dashboard

**Device:** Samsung Galaxy S20 FE (`r8q`, SM8250, Snapdragon 865)
**Live Kernel:** `7.3.0-rc6-omamobile-push1-g7ca90963c426` (P1 native DT + touch + BT + Gate C audio + SLPI/VolUp + HI-847 camera verified on hardware)
**Device Package:** `omamobile-device-r8q` `0.1.24` + `hexagonrpc` `0.5.0-2` + `libcamera 0.7.2-4.24` (installed & verified on phone)
**Boot Method:** Dual-boot via Samsung `recovery` (`sda24`) + microSD rootfs (`/omarchy-r8q.img`)
**Safety:** All 77 UFS partitions read-only (`ro=1`), LineageOS untouched

---

## Status Overview

| Column | Count | Last Updated |
| --- | --- | --- |
| ✅ SUCCESS | 6 | 2026-11-16 |
| ❌ FAILED (known, mitigated) | 7 | 2026-11-16 |
| 🔲 MISSING | 5 (P1/P2/SLPI+VolUp done) | 2026-11-16 |

---

## ✅ SUCCESS (6 cards — verified on hardware)

| # | Card | What Works | Evidence |
|---|------|-----------|----------|
| 01 | [Dual-Boot & UFS RO Guard](board/success/01-dual-boot-ufs-ro-guard.md) | `sda24` recovery boot + microSD rootfs; 77/77 UFS `ro=1` | `r8q-preflight --json` ufs_guard ok |
| 02 | [RPMh TCS Reclaim & Liveness Net](board/success/02-rpmh-rsc-and-liveness-net.md) | `rpmh-rsc.c` TCS timeout reclaim; XPU page guard; per-CPU liveness watchdog | No RPMh/RCU freezes; `HardwareCorrupted: 23520 kB` |
| 03 | [Display: simpledrm → MSM & S6E3FC3](board/success/03-display-mdss-s6e3fc3.md) | Zero white flash; `modprobe msm = 0`; `DSI-1 1080x2400@60`; backlight | `/dev/dri/card1`, `/dev/dri/renderD128` |
| 04 | [Adreno 650, Freedreno & Hyprland Desktop](board/success/04-gpu-adreno650-hyprland-desktop.md) | GPU at 305 MHz; Hyprland + quickshell + squeekboard + Xwayland; touch + keys | `3d00000.gpu` pinned; `stmfts-r8q.ko` live |
| 05 | [USB Gadget, Power/Battery & ZRAM](board/success/05-usb-power-battery-zram.md) | USB NCM `172.16.42.1`; battery telemetry `99%`; zram + swapfile + MGLRU | `max77705`/`max17042` as modules |
| 06 | [Wi-Fi, Bluetooth, Clock Sync & Boot GUI](board/success/06-wifi-bluetooth-and-clock.md) | `wlp1s0` UP; `hci0` Powered (BR/EDR+LE); RTC sync; `sddm` auto-start | `ath11k` + `qca6390` firmware ok |

---

## ❌ FAILED (7 cards — root causes known, permanent rules in place)

| # | Card | Failure | Permanent Rule |
|---|------|---------|---------------|
| 01 | [Unreserved XPU/TZ/Knox DRAM](board/failed/01-unreserved-xpu-tz-knox-dram.md) | T+12s freeze from XPU violation | DT fix `1324fe066` + `offline_unreserved_xpu_pages()` fallback |
| 02 | [Suspend/cpuidle/DVFS/UFS PM](board/failed/02-suspend-cpuidle-dvfs-ufspm.md) | RPMh -110 timeouts, RCU stalls | `SUSPEND` off, `WFI`-only cpuidle, CPU `performance`, UFS PM disabled |
| 03 | [max77705 built-in & unbind](board/failed/03-max77705-builtin-and-unbind.md) | `=y` freezes boot; `rmmod` kills battery | Keep `=m`, load post-boot, pin I2C `power/control="on"` |
| 04 | [Protected DISPCC clocks & white flash](board/failed/04-display-protected-clocks-and-white-flash.md) | `ESC0_CLK=0` → `-22`; unclocked FB flash | Use `display.dtso` (not `display-mdss.dtso`); clear `protected-clocks` in DT |
| 05 | `[omamobile]/mesa, misc.vfr, @120` | Mali Mesa lacks `msm_dri.so`; `vfr` error | Use `extra/mesa` + `vulkan-freedreno`; `vrr=0`; `@60` |
| 06 | [Boot & session hazards](board/failed/06-boot-and-session-hazards.md) | Serial alias `-19`, `btmgmt` hang, RTC `1975`, `SIGPIPE` loop | `caa385175` (kernel), `--timeout` on btmgmt, `sysinit.target` RTC, direct sysfs |
| 07 | [stmfts5 8-byte vs 16-byte](board/failed/07-stmfts5-8byte-vs-samsung-16byte-events.md) | `unknown FTS5 event` errors | 16-byte parser `5866f774c`; pin `994000.i2c` `on` |

---

## 🔲 MISSING (5 cards — next work queue)

### P1: Flash Native DT + Touch + BT Recovery (VERIFIED ON HARDWARE; SIGNOFF PENDING)

| Item | Status | Details |
|------|--------|---------|
| Kernel sign-offs | 🔴 | All 20 commits on `omamobile-r8q-p1-signoff` (`caa385175`) need human `Signed-off-by:` before upstream PRs (`tools/kernel-pr-check`) |
| Rebuild recovery image | ✅ | Built `~/omamobile-kernel/out-r8q-p1` (`7.3.0-rc6-omamobile-push1-gcaa38517520a`, 1821 modules) |
| Flash to `sda24` | ✅ | Flashed & verified over USB SSH (`tools/r8q-flash-recovery`) |
| Verify `uname -r` | ✅ | Verified `7.3.0-rc6-omamobile-push1-gcaa38517520a`, `HardwareCorrupted: 0 kB`, 37/37 `ro=1` |
| Verify native drivers | ✅ | Native `stmfts` + `qcom_geni_serial` + `hci0` working without `stmfts-r8q.ko`/`r8q-bt-uart.ko` |

**Files involved:**
- `~/omamobile-kernel/wt-r8q` (branch `omamobile-r8q-p1-signoff`, HEAD `caa385175`)
- `tools/r8q-boot pack` → `out-r8q-p1/recovery-omamobile.img`
- `tools/r8q-flash-recovery` → `sda24`
- `tools/r8q-push` → modules tarball

### P2: Gate C — Remoteprocs & Audio (VERIFIED END-TO-END ON HARDWARE)

| Item | Status | Details |
|------|--------|---------|
| `remoteprocs.dtso` | ✅ | `slpi`, `cdsp`, `adsp` all `running` |
| `audio-amps.dtso` | ✅ | `gpi_dma1` (`a00000.dma-controller`) + `i2c11` (`a8c000.i2c`) probed dual `cs35l41` (`11-0040`, `11-0041`, Rev A0) |
| `audio-sound.dtso` | ✅ | Routed `PRIMARY_TDM_RX_0` (`0x18`) on `MI2S0` (`gpio138..141`), `4x32-bit` slots (`6.144 MHz` BCLK); PipeWire → `MultiMedia1` → `cs35l41` `PUP_DONE=1`, `PLL_LOCK=1` |
| Firmware staging | ✅ | `adsp.mbn`, `cdsp.mbn`, `slpi.mbn` loaded cleanly |
| Audible playback | ✅ | Verified on bottom speaker (`Analog=18`, `Digital=817`); quieter than full stock (CS35L41 tuning/boost follow-up) |

**Files involved:**
- `device/r8q/overlays/remoteprocs.dtso` (verified)
- `device/r8q/overlays/audio-amps.dtso` (verified)
- `device/r8q/overlays/audio-sound.dtso` (verified with `PRIMARY_TDM_RX_0`)
- `packages/omamobile-device-r8q/ucm2/` (verified `HiFi.conf`)
- `tools/r8q-boot --dual-boot --overlay display,remoteprocs,audio-amps,audio-sound`

### P3: Cellular/Modem & Wireless Soak (MEDIUM)

| Item | Status | Details |
|------|--------|---------|
| Wi-Fi+BT soak test | 🔴 | Sustained downloads + USB NCM + BT audio |
| Modem/cellular | 🔴 | Take off-repo RO backup of `efs*`/`persist`/`modem*` first (`AGENTS.md`); keep `ro=1` |

**Files involved:**
- `tools/r8q-stabilize` (exists)
- `device/r8q/kernel.config` (`CONFIG_BT_LE=y` already set)

### P4: Sensors, Cameras & DPMS (IN PROGRESS — SLPI Sensors & VolUp DONE)

| Item | Status | Details |
|------|--------|---------|
| DPMS & Lock Screen | 🔴 | Verify `LockGlance.qml` wake; Power-key DPMS off/on |
| SLPI sensors | ✅ | `slpi-sensors.dtso` (`L8C` `1.8 V`) + `hexagonrpc 0.5.0-2` + `iio-sensor-proxy` (`accel`, `light`, `compass` live; `proximity` `UNKNOWN`) |
| Volume Up key (`vol-up.dtso`) | ✅ | Wired to `&pm8150l_gpios 3 GPIO_ACTIVE_LOW` (IRQ `179`); tested working on hardware |
| Camera: HI-847 telephoto | ✅ | `vreg_l7c_1p8`, `MCLK3`, `CSIPHY2`, `.llp`/crop/props, `orientation=1`/`rotation=270` (`7ca90963c`) + `libcamera 0.7.2-4.24` (`0058`); `omacam` live @ 30.02 fps |
| Camera: other 3 sensors & FD650 EGL | ⏳ | IMX555, S5K3L6, IMX616 need drivers; `0.1.23` pins `LIBCAMERA_SOFTISP_MODE=cpu` (`FD650` `debayer_egl` hangcheck follow-up) |
| `tools/r8q-install` alignment | ✅ | `monitors.lua` @60, `vrr=0`, scale 2.25 |

**Files involved:**
- `device/r8q/CAMERA-PLAN.md` (exists)
- `device/r8q/overlays/camera.dtso` (exists)
- `device/r8q/board/missing/r8q-camera-upstream-reference.dtsi` (exists)
- `device/r8q/board/missing/r8q-aperture-config.xml` (exists)
- `packages/omamobile-device-r8q/ucm2/` (exists)

### P5: Stability Soak, Pseudo-NMI & AI (LOW)

| Item | Status | Details |
|------|--------|---------|
| 30-min GUI + writeback soak | 🔴 | Zero RCU stalls, RPMh hangs, watchdogs |
| `CONFIG_ARM64_PSEUDO_NMI` | 🔴 | One experimental boot to test hardlockup detection |
| Freedreno perf tuning | 🔴 | GPU devfreq above 305 MHz once stable |
| On-device AI benchmarks | 🔴 | `GGML_VULKAN` + `LFM3-1.7B` |

**Files involved:**
- `packages/omamobile-device-r8q/rust/ai-server/` (exists)
- `tools/r8q-ai` (exists)
- `tools/r8q-optimize` (exists)

---

## 📁 File Inventory

### Device Package (`packages/omamobile-device-r8q/`)

| File | Status | Notes |
|------|--------|-------|
| `PKGBUILD` | ✅ | Builds `reboot-to`, `r8q-liveness-heartbeat`, AI server, preflight |
| `version` | ✅ | `0.1.24` (installed & verified on phone with `hexagonrpc 0.5.0-2` and `libcamera 0.7.2-4.24`) |
| `70-omamobile-dma-heap.rules` | ✅ | Grants `video` / `uaccess` access to `/dev/dma_heap` |
| `70-r8q-libcamera.conf` | ✅ | Defaults `LIBCAMERA_SOFTISP_MODE=cpu` to avoid `FD650` EGL hang |
| `test/all` | ✅ | Comprehensive config tests |
| `hyprland-r8q.conf` | ✅ | `DSI-1,1080x2400@60,0x0,2.25`; `vrr=0`; no blur/shadow/animations |
| `ucm2/` | ✅ | HiFi.conf, Samsung-Galaxy-S20-FE.conf, kona-mtp-snd-card.conf |
| `80-r8q-gpu.rules` | ✅ | Pins GPU, I2C, serial, touch `power/control="on"` |
| `79-r8q-iio-sensor-proxy.rules` | ✅ | Tags `fastrpc-sdsp*` with `ssc-accel ssc-light ssc-compass` and `ACCEL_MOUNT_MATRIX="-1, 0, 0; 0, -1, 0; 0, 0, 1"` |
| `10-r8q-hexagonrpcd.conf` | ✅ | Drop-in for `hexagonrpcd-sdsp.service` (`-R .../r8q` + `ConditionPathExists`) |
| `packages/hexagonrpc/` | ✅ | `hexagonrpc 0.5.0-2` with `sns_reg_version` + `ssc_hw_rev` patch |
| `r8q-liveness-monitor` | ✅ | Per-CPU heartbeats, XPU guard, clock sync, touch/BT/desktop helpers |
| `omamobile-r8q-liveness.service` | ✅ | `RuntimeWatchdogSec=30s`, `Restart=always` |
| `omamobile-r8q-battery.service` | ✅ | Post-boot module load |
| `omamobile-r8q-wifi.service` | ✅ | `r8q-wifi-aspm` |
| `omamobile-r8q-rtc.service` | ✅ | `sysinit.target`, `DefaultDependencies=no` |
| `omamobile-r8q-panel.service` + `.timer` | ✅ | Backlight unblank guard |
| `omamobile-r8q-ufs.service` | ✅ | `auto_hibern8=0` |
| `omamobile-r8q-cpuidle.service` | ✅ | `WFI`-only |
| `10-r8q-no-suspend.conf` | ✅ | `AllowSuspend=no`, `AllowHibernation=no` |
| `10-r8q-no-idle.conf` | ✅ | `IdleAction=ignore` |
| `10-r8q-watchdog.conf` | ✅ | `RuntimeWatchdogSec=30s` |
| `10-omamobile-r8q-bringup.conf` | ✅ | Blacklists `icc_bwmon`, `camcc_sm8250` |
| `50-r8q-gpu-blacklist.conf` | ✅ | GPU not blacklisted |
| `cpufreq-no-dvfs` | ✅ | CPU `performance` |
| `cpuidle-no-deep-sleep` | ✅ | `WFI`-only |
| `ufs-no-autohibern8` | ✅ | `auto_hibern8=0`, `clkscale_enable=0`, `clkgate_enable=0` |
| `r8q-wifi-aspm` | ✅ | Masks PCIe0 `l0s_aspm` |
| `r8q-rtc-epoch` | ✅ | RTC fallback to `2026-11-15` |
| `r8q-battery-modprobe` | ✅ | Loads `max77705_charger`, `max17042_battery` |
| `90-r8q-vm.conf` | ✅ | Swappiness, dirty bytes |
| `91-r8q-zram.conf` | ✅ | `zram-size = ram / 2` |
| `80-r8q-mglru.conf` | ✅ | `min_ttl_ms` |
| `rust/ai-server/` | ✅ | Rust on-device AI server |
| `rust/r8q-preflight/` | ✅ | Rust preflight checker |
| `reboot-to.c` | ✅ | Freestanding reboot syscall |
| `reboot` | ✅ | Reboot wrapper |
| `omamobile-device-r8q.install` | ✅ | Post-install hooks |
| `sshd-omamobile.conf` + `sshd.sysusers` | ✅ | SSH config |
| `omamobile-mirrorlist` | ✅ | `[omamobile]` mirror |
| `omamobile-r8q-panel.timer` | ✅ | `OnUnitActiveSec=4min` |

### Overlays (`device/r8q/overlays/`)

| File | Status | Notes |
|------|--------|-------|
| `display.dtso` | ✅ | Enables MDSS, DSI0, PHY, panel; clears `protected-clocks` |
| `display-mdss.dtso` | ⚠️ | DO NOT USE — leaves `protected-clocks` intact |
| `display-clocks.dtso` | ⚠️ | DO NOT USE — 0-byte `protected-clocks` triggers `of_prop_next_u32()` OOB drop of `DISP_CC_MDSS_BYTE0_CLK_SRC` |
| `remoteprocs.dtso` | ✅ | Verified — enables ADSP, CDSP, SLPI |
| `audio-amps.dtso` | ✅ | Verified — enables `gpi_dma1`, `i2c11` (`cs35l41` `@40`/`@41`) |
| `audio-sound.dtso` | ✅ | Verified — `PRIMARY_TDM_RX_0` (`0x18`) on `MI2S0` + `MultiMedia1` |
| `slpi-sensors.dtso` | ✅ | Verified — enables `vreg_l8c_1p8` (`L8C` `1.8 V`) for SLPI sensors |
| `vol-up.dtso` | ✅ | Verified — wires `KEY_VOLUMEUP` to `&pm8150l_gpios 3` |
| `camera.dtso` | ✅ | Verified — enables CAMSS, CCI10, HI-847 (`7ca90963c`) |

### Tools (`tools/r8q-*`)

| Tool | Status | Notes |
|------|--------|-------|
| `r8q-boot` | ✅ | Pack/verify recovery images; validates overlay deps |
| `r8q-boot-watch` | ✅ | Watch boot progress |
| `r8q-flash-recovery` | ✅ | Write recovery partition over SSH |
| `r8q-push` | ✅ | Push modules + verify |
| `r8q-audio-bringup` | ✅ | 9-step gated audio bring-up sequence |
| `r8q-camera-bringup` | ⏳ | 5-step gated camera bring-up sequence |
| `r8q-install` | ✅ | Full installer (swap, repos, packages, UI) |
| `r8q-rescue` | ✅ | Rescue operations |
| `r8q-stabilize` | ✅ | Stability testing |
| `r8q-optimize` | ✅ | System optimization |
| `r8q-extract-firmware` | ✅ | Extract vendor firmware |
| `r8q-harvest-phone` | ✅ | Harvest phone data |
| `r8q-web` | ✅ | Web interface |
| `r8q-ai` | ✅ | AI benchmark runner |

### Device Config

| File | Status | Notes |
|------|--------|-------|
| `device.conf` | ✅ | `BOOT_METHOD=ram-only`, single-slot |
| `kernel.config` | ✅ | SM8250 essentials, BT LE, camera, battery |
| `PORT-PLAN.md` | ✅ | Main porting plan |
| `NOTES.md` | ✅ | Chronological findings |
| `RUNBOOK-DISPLAY-GPU.md` | ✅ | Staged Gate A-D runbook |
| `CRASH-DOSSIER.md` | ✅ | Crash analysis |
| `CAMERA-PLAN.md` | ✅ | Camera bring-up plan |
| `board/README.md` | ✅ | Board overview |
| `board/SUCCESS.md` | ✅ | 6 success cards |
| `board/FAILED.md` | ✅ | 7 failed cards |
| `board/MISSING.md` | ✅ | 5 missing cards |
| `board/success/01-06.md` | ✅ | All 6 success detail files |
| `board/failed/01-07.md` | ✅ | All 7 failed detail files |
| `board/missing/01-05.md` | ✅ | All 5 missing detail files |
| `board/missing/r8q-camera-upstream-reference.dtsi` | ✅ | Vendor DT reference |
| `board/missing/r8q-aperture-config.xml` | ✅ | Camera aperture config |

### Desktop/UI Config

| File | Status | Notes |
|------|--------|-------|
| `desktop/shell.toml` | ✅ | Bar size 48, spacing huge 20 |
| `desktop/setup-ui` | ✅ | UI setup script (scale 2.25, icon links) |
| `display/display-try` | ✅ | Display hand-test with dead-man |
| `initramfs/reboot-to.c` | ✅ | Freestanding reboot-to syscall |

### Kernel State

| Item | Status | Notes |
|------|--------|-------|
| Worktree | ✅ | `~/omamobile-kernel/wt-r8q`, branch `camera-wip` (`10` commits above `caa385175`) |
| Output | ✅ | `~/omamobile-kernel/out-r8q` (`7.3.0-rc6-omamobile-push1-g7ca90963c426`) |
| HEAD | ✅ | `7ca90963c` (HI-847 orientation/rotation DT) |
| Upstream prep | ⚠️ | Kernel commits need human `Signed-off-by:`; fold `PRIMARY_TDM_RX_0`, `vreg_l8c_1p8`, `pm8150l_gpios 3`, and display/camera overlays into base DTS |
| Branch `camera-wip` | ✅ | Verified live on hardware (`g7ca90963c426`, `omacam` 30 fps) |
| Branch `r8q/display-panel` | ✅ | Exists |

---

## 🔑 Critical Path

```
P1 (Native DT ✅) → P2 (Audio/DSP ✅) → P4 Sensors/VolUp (✅) → P3 (Soak/Modem) & P4 (DPMS/Camera) → P5 (Soak/Perf) → Upstream PRs
```

**Remaining blockers:**
1. Human `Signed-off-by:` on the `20` kernel commits + folding verified overlay DT fixes (`PRIMARY_TDM_RX_0`, `vreg_l8c_1p8`, `pm8150l_gpios 3`) into `sm8250-samsung-r8q.dts` before upstream PRs (`tools/kernel-pr-check`).
2. Off-repo read-only backup + SHA256 of `efs*`, `persist`, and `modem*` before any P3 modem (`remoteproc_mpss`) testing.
3. DPMS Power-key check, `r8q-audio-tuning` (CS35L41 protection firmware), remaining 3 camera sensors (`IMX555`/`S5K3L6`/`IMX616`), and Adreno `FD650` `debayer_egl`.

**What's working right now on hardware (`7.3.0-rc6-omamobile-push1-g7ca90963c426`):**
- Hyprland desktop (`sddm`, `quickshell`, `squeekboard`, `Xwayland` on `DSI-1 1080x2400@60`)
- Touch (`stmfts` natively bound, 16-byte Type-B) & Volume Up / Volume Down hardware keys
- Wi-Fi (`wlp1s0` UP via `ath11k_pci`) & Bluetooth (`hci0` Powered via native `qcom_geni_serial`)
- GPU (`3d00000.gpu` at 305 MHz, `renderD128`)
- Remoteprocs (`slpi`, `cdsp`, `adsp` running) & Audio (`PRIMARY_TDM_RX_0` → dual `cs35l41` + PipeWire)
- SLPI Sensors (`hexagonrpc` + `iio-sensor-proxy` with `ACCEL_MOUNT_MATRIX`: accelerometer, ambient light, magnetometer, compass)
- Camera (`hi847` rear telephoto streaming `3256x2448` @ 30.02 fps in `omacam` via `libcamera 0.7.2-4.24` CPU SoftISP)
- USB gadget (`172.16.42.1`), Battery telemetry (`99–100%`), Liveness watchdog (`active`), Dual-boot `--dual-boot` UFS RO guard (`ro=1`)
