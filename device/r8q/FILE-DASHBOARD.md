<!-- SPDX-License-Identifier: MIT -->

# r8q (Galaxy S20 FE SM-G780G) — File Dashboard

**Device:** Samsung Galaxy S20 FE (`r8q`, SM8250, Snapdragon 865)
**Live Kernel:** `7.3.0-rc6-omamobile-push1-gcaa38517520a` (P1 native DT + touch + BT verified on hardware)
**Device Package:** `omamobile-device-r8q` `0.1.19` (verified with `audio` + `hci0` retry + UCM2 fixes on `r8q-p3-device-package-bump`)
**Boot Method:** Dual-boot via Samsung `recovery` (`sda24`) + microSD rootfs (`/omarchy-r8q.img`)
**Safety:** All 77 UFS partitions read-only (`ro=1`), LineageOS untouched

---

## Status Overview

| Column | Count | Last Updated |
| --- | --- | --- |
| ✅ SUCCESS | 6 | 2026-11-15 |
| ❌ FAILED (known, mitigated) | 7 | 2026-11-15 |
| 🔲 MISSING | 5 | 2026-11-15 |

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
| Audible tone check | ⏳ | PipeWire silent playback verified zero errors; optional human-attended low-volume tone check |

**Files involved:**
- `device/r8q/overlays/remoteprocs.dtso` (exists, needs testing)
- `device/r8q/overlays/audio-amps.dtso` (exists, needs testing)
- `device/r8q/overlays/audio-sound.dtso` (exists, needs testing)
- `packages/omamobile-device-r8q/ucm2/` (exists with HiFi.conf)
- `tools/r8q-boot --overlay remoteprocs,audio-amps,audio-sound`

### P3: Cellular/Modem & Wireless Soak (MEDIUM)

| Item | Status | Details |
|------|--------|---------|
| Wi-Fi+BT soak test | 🔴 | Sustained downloads + USB NCM + BT audio |
| Modem/cellular | 🔴 | NV partitions backed up; keep `ro=1`; test after audio+soak pass |
| Native `caa385175` rebuild | ⏳ | Verify BT works without `r8q-bt-uart.ko` |

**Files involved:**
- `tools/r8q-stabilize` (exists)
- `device/r8q/kernel.config` (`CONFIG_BT_LE=y` already set)

### P4: Sensors, Cameras & DPMS (MEDIUM)

| Item | Status | Details |
|------|--------|---------|
| DPMS & Lock Screen | 🔴 | Verify `LockGlance.qml` wake; Power-key DPMS off/on |
| SLPI sensors | 🔴 | After `remoteprocs.dtso` enables SLPI |
| Camera: HI-847 I2C | 🔴 | I2C bus timeout — needs boot image flash (kexec can't change DT) |
| Camera: other 3 sensors | ⏳ | IMX555, S5K3L6, IMX616 — no mainline drivers yet |
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
| `version` | ⚠️ | `0.1.19` in repo (`0.1.14` installed on phone) |
| `test/all` | ✅ | Comprehensive config tests |
| `hyprland-r8q.conf` | ✅ | `DSI-1,1080x2400@60,0x0,2.25`; `vrr=0`; no blur/shadow/animations |
| `ucm2/` | ✅ | HiFi.conf, Samsung-Galaxy-S20-FE.conf, kona-mtp-snd-card.conf |
| `80-r8q-gpu.rules` | ✅ | Pins GPU, I2C, serial, touch `power/control="on"` |
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
| `display-clocks.dtso` | ✅ | Bisect overlay — clears `protected-clocks` alone |
| `remoteprocs.dtso` | ⏳ | Needs testing — enables ADSP, CDSP, SLPI |
| `audio-amps.dtso` | ⏳ | Needs testing — enables `gpi_dma1`, `i2c11` |
| `audio-sound.dtso` | ⏳ | Needs testing — enables sound card |
| `camera.dtso` | ⏳ | Needs testing — enables CAMSS, CCI10, HI-847 |

### Tools (`tools/r8q-*`)

| Tool | Status | Notes |
|------|--------|-------|
| `r8q-boot` | ✅ | Pack/verify recovery images; supports overlays |
| `r8q-boot-watch` | ✅ | Watch boot progress |
| `r8q-flash-recovery` | ✅ | Write recovery partition over SSH |
| `r8q-push` | ✅ | Push modules + verify |
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
| Worktree | ✅ | `~/omamobile-kernel/wt-r8q`, branch `omamobile-r8q` |
| Output | ✅ | `~/omamobile-kernel/out-r8q` |
| HEAD | ⚠️ | `caa385175` (BT UART alias clamp) — last commit |
| Key commits | ⚠️ | `1324fe066` (XPU DT), `5866f774c` (touch), `caa385175` (BT) — need sign-offs |
| Branch `camera-wip` | ✅ | Exists for camera work |
| Branch `r8q/display-panel` | ✅ | Exists |

---

## 🔑 Critical Path

```
P1 (Flash Native DT) → P2 (Audio/DSP) → P3 (Soak/Modem) → P4 (Cameras/Sensors) → P5 (Stability)
     │                                                    │
     └─ Human sign-offs ──────────────────────────────────┘
     └─ Rebuild + flash ──────────────────────────────────┘
     └─ Verify uname -r ──────────────────────────────────┘
```

**Blockers:**
1. Human maintainer `Signed-off-by:` on 3 kernel commits
2. Human approval + Download Mode for `tools/r8q-flash-recovery`
3. Audio overlay testing requires remoteprocs first

**What's working right now (no changes needed):**
- Hyprland desktop (running as `alarm` user)
- Wi-Fi (`wlp1s0` UP, 192.168.178.98)
- Bluetooth (`hci0` Powered: yes)
- Touch (`stmfts-r8q.ko` live, 16-byte Type-B)
- GPU (`3d00000.gpu` at 305 MHz, `renderD128`)
- USB gadget (`172.16.42.1`)
- Battery telemetry (99%)
- Liveness watchdog (active)
- UFS guard (77/77 `ro=1`)
