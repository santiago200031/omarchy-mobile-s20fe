<!-- SPDX-License-Identifier: MIT -->

# r8q (Galaxy S20 FE SM-G780G) — File Dashboard

**Device:** Samsung Galaxy S20 FE (`r8q`, SM8250, Snapdragon 865)
**Live Kernel:** `7.3.0-rc6-omamobile-push1-g3667ab368ef9` (`camera-uw-wip` — display, GPU, BT, Wi-Fi, remoteprocs, audio, `cs35l41` `r8q`, SLPI sensors, VolUp, pseudo-NMI, HI-847 tele, and S5K3L6 ultrawide verified on hardware; restored 2026-11-17 after the SDX55M Stage 3A experiment and pinned in `stable-image.conf`)
**Kernel Branch:** `camera-uw-wip` (`3667ab368` on `omamobile-r8q-signed`; booted & verified on `sda24` with `--overlay camera-uw`)
**Device Package:** `omamobile-device-r8q` `0.1.25` in the repo (adds the `omamobile-ai` drop-in, `zram-generator` and the parked `modem.dtso`); `0.1.24` on the phone, with the same AI listener set by hand in `/etc/omamobile-ai.conf.d/20-usb.conf`. `omamobile-ai` `0.1.0-1` + `libcamera 0.7.2-4.25` installed & verified
**Boot Method:** Dual-boot via Samsung `recovery` (`sda24`) + microSD rootfs (`/omarchy-r8q.img`)
**Safety:** All 77 UFS partitions read-only (`ro=1`), LineageOS untouched

---

## Stable Baseline (Rollback Target)

`device/r8q/stable-image.conf` records the last image verified end-to-end on the phone. Every modem experiment is packed from its own output tree; afterwards the phone goes back to these exact bytes, never to a rebuild.

| Fact | Value |
| --- | --- |
| Release | `7.3.0-rc6-omamobile-push1-g3667ab368ef9` |
| Image | `~/omamobile-kernel/out-r8q-uw/recovery-cam-uw.img` |
| sha256 | `66fe127ffd9345e7913aec5eac8a02de0da44bfdbe15987abff1cb61b26d8503` |
| Packed with | `tools/r8q-boot pack --dual-boot --overlay camera-uw` |
| Partition | `sda24` (`recovery`) |
| Modem | PCIe2 and its PHY `disabled` in the packed DTB, no `modem` overlay, no Sahara loader or blacklist |

```sh
tools/r8q-stable-check --image      # the recorded image still hashes
tools/r8q-stable-check --phone      # the phone runs it, PCIe2 off, no bring-up leftovers
tools/r8q-flash-recovery --stable --dry-run
tools/r8q-flash-recovery --stable   # write it; the human's go
```

Verified on 2026-11-17 after restoring from the Stage 3A kernel: `uname -r` matched the record, `sda24` read back the same sha256, 77/77 UFS partitions `ro=1`, root `rw`, PCIe2 and its PHY `disabled`, no SDX55M on `mhi_pci_generic`, and the modules for the release present. `tools/phone-smoke` reports only the two r8q differences a generic test cannot know: the root is on `/dev/loop0` (the SD-card image, not `userdata`), and boot-time messages (`ufshcd` WriteBooster query, DSI PLL retries) with the storage mounted and the display working.

---

## Status Overview

| Column | Count | Last Updated |
| --- | --- | --- |
| ✅ SUCCESS | 7 | 2026-11-17 |
| ❌ FAILED (known, mitigated) | 7 | 2026-11-17 |
| 🔲 MISSING | 3 open (main/front cameras; modem userspace SIM/QMI; Freedreno/Vulkan optional) + kernel sign-offs upstream | 2026-11-18 |

---

## ✅ SUCCESS (7 cards — verified on hardware)

| # | Card | What Works | Evidence |
|---|------|-----------|----------|
| 01 | [Dual-Boot & UFS RO Guard](board/success/01-dual-boot-ufs-ro-guard.md) | `sda24` recovery boot + microSD rootfs; 77/77 UFS `ro=1` | `r8q-preflight --json` ufs_guard ok |
| 02 | [RPMh TCS Reclaim & Liveness Net](board/success/02-rpmh-rsc-and-liveness-net.md) | `rpmh-rsc.c` TCS timeout reclaim; XPU page guard; per-CPU liveness watchdog | No RPMh/RCU freezes; `HardwareCorrupted: 23520 kB` |
| 03 | [Display: simpledrm → MSM & S6E3FC3](board/success/03-display-mdss-s6e3fc3.md) | Zero white flash; `modprobe msm = 0`; `DSI-1 1080x2400@60`; backlight | `/dev/dri/card1`, `/dev/dri/renderD128` |
| 04 | [Adreno 650, Freedreno & Hyprland Desktop](board/success/04-gpu-adreno650-hyprland-desktop.md) | GPU at 305 MHz; Hyprland + quickshell + squeekboard + Xwayland; touch + keys | `3d00000.gpu` pinned; `stmfts-r8q.ko` live |
| 05 | [USB Gadget, Power/Battery & ZRAM](board/success/05-usb-power-battery-zram.md) | USB NCM `172.16.42.1`; battery telemetry `99%`; zram + swapfile + MGLRU | `max77705`/`max17042` as modules |
| 06 | [Wi-Fi, Bluetooth, Clock Sync & Boot GUI](board/success/06-wifi-bluetooth-and-clock.md) | `wlp1s0` UP; `hci0` Powered (BR/EDR+LE); RTC sync; `sddm` auto-start | `ath11k` + `qca6390` firmware ok |
| 07 | [Stability Soak, Pseudo-NMI & On-Device AI](board/success/07-stability-soak-nmi-and-ai.md) | `1800 s` soak `failures=0`; `NMI watchdog: Enabled`; `omamobile-ai` + `LFM3-1.7B` on `:8080`; `omamobile-llama` packaged | 15.3 generation tok/s / 35.2 pp512 tok/s (2026-11-18), old 10.6-11.0 generation |

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

### P3: Cellular/Modem & Wireless Soak (IN PROGRESS — Wireless Concurrency, NV RO Backup & Modem Plan DONE)

| Item | Status | Details |
|------|--------|---------|
| Wi-Fi+BT concurrency check | ✅ | `wlp1s0` (`15/15` ping 0% loss, HTTPS `200`, `576–720 MBit/s`) + concurrent `btmgmt find -l` LE discovery + USB NCM; 0 dmesg errors (BT A2DP audio still to test) |
| Modem/cellular | ✅ AMSS reached | **The modem boots to AMSS on mainline (2026-11-18)**: full 14-image SAHARA set over one open MHI channel, modem jumps to mission mode itself (`CHDBOFF=0x300 ERDBOFF=0x700`, `EE: MISSION MODE`), `mhi0_QMI/MBIM/IPCR/IP_HW0/IP_SW0` and `/dev/wwan0{qmi,mbim,qcdm,at}0` present. Fixes: `qcom,pci-msi` doorbell (GIC hwirq data), ported MHI SBL/AMSS support, AP2MDM_STATUS high + single-channel serving. Next: userspace SIM/QMI on the Arch root; kernel commits need the human Signed-off-by. `MODEM-PLAN.md` §10. |

**Files involved:**
- `tools/r8q-stabilize` (exists)
- `device/r8q/kernel.config` (`CONFIG_BT_LE=y` already set)
- `device/r8q/MODEM-PLAN.md` (exists)

### P4: Sensors, Cameras & DPMS (IN PROGRESS — SLPI Sensors, VolUp & DPMS DONE)

| Item | Status | Details |
|------|--------|---------|
| DPMS & Lock Screen | ✅ | `hl.dsp.dpms` enable/disable, physical Power-key wake, and `secure: true` lock screen verified on hardware (`gf3a5609d121b`, 0 dmesg errors) |
| SLPI sensors | ✅ | `slpi-sensors.dtso` (`L8C` `1.8 V`) + `hexagonrpc 0.5.0-2` + `iio-sensor-proxy` (`accel`, `light`, `compass` live; `proximity` `UNKNOWN`) |
| Volume Up key (`vol-up.dtso`) | ✅ | Wired to `&pm8150l_gpios 3 GPIO_ACTIVE_LOW` (IRQ `179`); tested working on hardware |
| Camera: HI-847 tele & S5K3L6 uw | ✅ | Both back cameras live in `libcamera 0.7.2-4.25` (`hi847` `0058` + `s5k3l6xx` `0059`): `cam -l` lists both; raw + JPEG capture verified |
| Camera: IMX555 (wide), IMX616 (front) & FD650 EGL | ⏳ | Main wide & front need drivers/tuning; `70-r8q-libcamera.conf` pins `LIBCAMERA_SOFTISP_MODE=cpu` for desktop sessions |
| `tools/r8q-install` alignment | ✅ | `monitors.lua` @60, `vrr=0`, scale 2.25 |

**Files involved:**
- `device/r8q/CAMERA-PLAN.md` (exists)
- `device/r8q/overlays/camera.dtso` (exists)
- `device/r8q/board/missing/r8q-camera-upstream-reference.dtsi` (exists)
- `device/r8q/board/missing/r8q-aperture-config.xml` (exists)
- `packages/omamobile-device-r8q/ucm2/` (exists)

### P5: Stability Soak, Pseudo-NMI & AI (DONE — soak, pseudo-NMI and AI verified)

| Item | Status | Details |
|------|--------|---------|
| 30-min GUI + writeback soak | ✅ | `tools/r8q-stability-soak` passed live on `gf3a5609d121b` (`1800s`, 6 iterations, `failures=0`) |
| `CONFIG_ARM64_PSEUDO_NMI` | ✅ | Verified live with `irqchip.gicv3_pseudo_nmi=1` and `NMI watchdog: Enabled` (0 regressions) |
| `omamobile-ai` supervisor | ✅ | `omamobile-ai.service` live on `:8080`: lazy `llama-server`, OpenAI + llama.cpp router surfaces, `IDLE_TIMEOUT=180`, pinned to cores 4-6; `tools/r8q-ai` drives it (SSH fallback) |
| On-device AI benchmarks | ✅ | `AI-BENCHMARKS.md` §2: `omamobile-llama` `98d67c8` (packaged), **15.26-15.29 generation tok/s / pp512 35.18 tok/s** on 2026-11-18; old hand-built baseline 10.6-11.0 generation; 47.5 → 68.7 °C |
| Freedreno perf tuning | 🔴 | GPU devfreq above 305 MHz once stable; 587 MHz measured 1.92× at 74 °C, still opt-in |

**Files involved:**
- `packages/omamobile-ai/` (supervisor package; replaces the removed BSP `rust/ai-server/` stub)
- `packages/omamobile-device-r8q/etc/omamobile-ai.conf.d/10-sm8250.conf` (core pinning)
- `tools/r8q-ai` (API client, SSH fallback)
- `tools/r8q-optimize` (exists)
- `tools/r8q-stability-soak` (exists)
- `device/r8q/AI-BENCHMARKS.md` (verified numbers and reproduction)

---

## 📁 File Inventory

### Device Package (`packages/omamobile-device-r8q/`)

| File | Status | Notes |
|------|--------|-------|
| `PKGBUILD` | ✅ | Builds `reboot-to`, `r8q-liveness-heartbeat`, preflight |
| `version` | ✅ | `0.1.25` in the repo; `0.1.24` on the phone (`0.1.25` adds the AI drop-in, `zram-generator` and the parked `modem.dtso`) |
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
| `etc/omamobile-ai.conf.d/10-sm8250.conf` | ✅ | Pins inference to cores 4-6, 3 threads, listener on `0.0.0.0:8080` |
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
| `display.dtso` | ✅ | Enables MDSS, DSI0, PHY, panel; cleared `protected-clocks` (folded into base DTS) |
| `display-mdss.dtso` | ⚠️ | DO NOT USE — leaves `protected-clocks` intact |
| `display-clocks.dtso` | ⚠️ | DO NOT USE — 0-byte `protected-clocks` triggers `of_prop_next_u32()` OOB drop of `DISP_CC_MDSS_BYTE0_CLK_SRC` |
| `remoteprocs.dtso` | ✅ | Verified — enables ADSP, CDSP, SLPI (folded into base DTS) |
| `audio-amps.dtso` | ✅ | Verified `gpi_dma1`, `i2c11`, and `cirrus,subsystem-id = "r8q"` (`f3a5609d1` + `6d8948e` symlinks: `cs35l41` DSP1 preloaded `r8q-{left,right}` `spk-prot` fw live) |
| `audio-sound.dtso` | ✅ | Verified — `PRIMARY_TDM_RX_0` (`0x18`) on `MI2S0` + `MultiMedia1` (folded into base DTS on `fold-overlay-dt`) |
| `slpi-sensors.dtso` | ✅ | Verified — enables `vreg_l8c_1p8` (`L8C` `1.8 V`) for SLPI sensors (folded into base DTS on `fold-overlay-dt`) |
| `vol-up.dtso` | ✅ | Verified — wires `KEY_VOLUMEUP` to `&pm8150l_gpios 3` (folded into base DTS on `fold-overlay-dt`) |
| `camera.dtso` | ✅ | Verified — enables CAMSS, QUP I2C10, HI-847 (`7ca90963c`) (folded into base DTS on `omamobile-r8q-signed`) |
| `camera-uw.dtso` | ✅ | Verified — enables CAMSS `port@1` (`CSIPHY1`), QUP I2C2 (`988000.i2c`), and Samsung S5K3L6 (`camera@2d`) |
| `modem.dtso` | ✅ | Verified Stage 1 — enables `&pcie2` + `&pcie2_phy` + `mdm2ap`/`ap2mdm` pinctrl (PCIe Gen.3 x2 link up, `SDX55` `mhi0` BHI ready) |

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
| `r8q-stability-soak` | ✅ | Passed 30-min GUI/storage/wireless soak (`1800s`, 6 iters, `failures=0`) |
| `r8q-optimize` | ✅ | System optimization |
| `r8q-extract-firmware` | ✅ | Extract vendor firmware (`6d8948e` `spk-prot` symlinks verified live via `DSP1 Preload Switch`) |
| `r8q-harvest-phone` | ✅ | Harvest phone data |
| `r8q-web` | ✅ | Web interface |
| `r8q-ai` | ✅ | Drives the `omamobile-ai` supervisor API (`status`, `models`, `ask`, `unload`, `stop`, `start`, `push-model`); SSH fallback |

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
| `CAMERA-UW-PLAN.md` | ✅ | Ultrawide (S5K3L6) driver, DT, and `libcamera 0.7.2-4.25` verified end-to-end |
| `DPMS-PLAN.md` | ✅ | DPMS & physical Power-key wake plan (verified) |
| `MODEM-PLAN.md` | ⏳ | Modem & cellular plan: Stage 1 PASS, Stage 2/3A results (§8), and the offline design (§9: stock `qcom,pci-msi` memory doorbell, `msi-parent` on `&pcie2`) |
| `AI-BENCHMARKS.md` | ✅ | Supervisor design, packaged `omamobile-llama` `98d67c8` (15.3 generation / 35.2 pp512 tok/s), old baseline and reproduction |
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
| Worktree | ✅ | `~/omamobile-kernel/wt-r8q`, branch `omamobile-r8q-signed` (37 commits cleanly signed off on top of `2db2bc35b`) |
| Output | ✅ | `~/omamobile-kernel/out-r8q-uw` (`7.3.0-rc6-omamobile-push1-g3667ab368ef9`, 1822 modules) |
| HEAD | ✅ | `3667ab368` (S5K3L6 ultrawide driver, binding, and disabled DT node) |
| Upstream prep | ✅ | `tools/kernel-pr-check 2db2bc35b omamobile-r8q-signed` passes (37 author sign-offs, 0 errors); push target needs a user kernel fork remote (`origin` is `derekross/linux-omarchy-mobile`) |

---

## 🔑 Critical Path

```
P1 (Native DT ✅) → P2 (Audio/DSP ✅) → Sensors/VolUp/DPMS/Pseudo-NMI/S5K3L6/AI (✅) → Modem (parked) & Main/Front Cameras → Upstream PRs
```

**Remaining open items:**
1. **Cameras (`IMX555` rear wide & `IMX616` front):** Stock topology (`live.dts`), drivers, bindings, and `libcamera` helpers; plus Adreno `FD650` `debayer_egl` hangcheck follow-up so `LIBCAMERA_SOFTISP_MODE=cpu` can be lifted if safe.
2. **Modem ([MODEM-PLAN.md](MODEM-PLAN.md) §4, §8, §9):** **parked, design done.** Done offline — 15-partition RO backup, `pci_generic.c` `samsung,r8q` BHI quirk, FW staging, Sahara loader (`tools/r8q-sahara`), Stage 2 (`mhi0` BHI ready) and Stage 3A (`PCIe2` MSI). Stage 3A **resets the phone** when `mhi_pci_generic` binds (`5be4efd5e`); §9 shows it programmed an IOVA into a DWC address-matching doorbell and adopts the stock topology instead: a `qcom,pci-msi` memory doorbell at `0x17a10040`, `msi-parent` on `&pcie2`, 32 SPIs from `0x340`, and iMSI-RX left to PCIe0. Next: the driver, binding, r8q DT node and a gated runbook (all laptop work), then an attended test with the human's go for every flash/reboot.
3. **Optional perf / UX / polish:** `omacam` live visual test switching tele ↔ ultrawide (verify `s5k3l6xx` orientation/rotation in preview), `cs35l41` speaker protection tuning (louder/balanced top+bottom speakers), Vulkan `llama.cpp` on the 587 MHz GPU (CPU baseline is verified; CDSP needs a Hexagon v66 userspace runtime), and setting up user fork remote for `linux-omarchy-mobile` push.

**What's working right now on hardware (`7.3.0-rc6-omamobile-push1-g3667ab368ef9`):**
- Hyprland desktop (`sddm`, `quickshell`, `squeekboard`, `Xwayland` on `DSI-1 1080x2400@60`), DPMS & Power-key wake + lock screen
- Touch (`stmfts`, 16-byte Type-B), Volume Up (`pm8150l_gpios 3`) & Volume Down keys
- Wi-Fi (`wlp1s0` UP via `ath11k_pci`) & Bluetooth (`hci0` Powered via `qcom_geni_serial`)
- GPU (`3d00000.gpu`, `renderD128`, Turnip Vulkan `1.3.354`) & M2.8 pseudo-NMI hardlockup watchdog (`irqchip.gicv3_pseudo_nmi=1`)
- Remoteprocs (`slpi`, `cdsp`, `adsp` running) & Audio (`PRIMARY_TDM_RX_0` → dual `cs35l41` + `r8q` `spk-prot` fw + PipeWire)
- SLPI Sensors (`hexagonrpc 0.5.0-2` + `iio-sensor-proxy`: accelerometer, ambient light, magnetometer, compass)
- Rear Cameras (`hi847` telephoto + `s5k3l6xx` ultrawide in `libcamera 0.7.2-4.25`: `cam -l` lists both, raw CSI-2 + JPEG capture verified)
- USB gadget (`172.16.42.1`), Battery telemetry (`max77705`/`max17042`), `zram0` (`2.7G zstd`), Liveness watchdog, Dual-boot UFS RO guard (`ro=1`)
