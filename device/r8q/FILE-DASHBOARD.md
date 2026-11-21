<!-- SPDX-License-Identifier: MIT -->

# r8q (Galaxy S20 FE SM-G780G) — File Dashboard

**Updated:** `2026-11-21` (`0.1.33-1` + `recovery-rt8547-torch.img`)
**Device:** Samsung Galaxy S20 FE (`r8q`, SM8250, Snapdragon 865)
**Live Kernel:** `7.3.0-rc6-omamobile-push1-gae195e6deaac` (`recovery-rt8547-torch.img` — native root `sda36`, `1080x2400@120` display, Adreno 650 fault quiesce + Vulkan, Wi-Fi, BT, remoteprocs, `SamsungGalaxyS2` playback + `DMIC3` capture via `dmic-va.dtso`, `camera-uw.dtso` + `omacam` `0.5x..3x` lenses, RT8547 `white:torch` via `torch.dtso`, SLPI sensors)
**Live Packages:** `omamobile-device-r8q 0.1.33-1`, `pd-mapper 1.1-2`, `omamobile-ai 0.1.0-1`, `omamobile-settings 0.1.3-1` (upgraded natively via `tools/phone-install-package`), `libcamera 0.7.2-4.25`
**Boot Method:** Samsung `recovery` (`sda24`) → native `/dev/sda36` (`LABEL=omamobile-root`); a plain power-on boots it ([NOTES §24](NOTES.md))
**Safety:** `77/77` UFS partitions read-only (`ro=1`), `userdata` (`sda36`) `rw`; NV partitions untouched
**Helper Scripts:** [`tools/r8q-progress`](../../tools/r8q-progress) (`--live`, `--next N`, `--json`) + [`tools/r8q-check-all`](../../tools/r8q-check-all) (`--quick`) backed by [`board/tasks.json`](board/tasks.json); hardware map in [`HARDWARE-DISCOVERY.md`](HARDWARE-DISCOVERY.md)

---

## Stable Baseline (Rollback Target)

[`device/r8q/stable-image.conf`](stable-image.conf) records the verified native-root image on `sda24`:

| Fact | Value |
| --- | --- |
| Release | `7.3.0-rc6-omamobile-push1-gae195e6deaac` |
| Image | `~/omamobile-kernel/out-r8q/recovery-fault-quiesce.img` |
| sha256 | `adeddb31faa4a263616ff8383b80e0316b3c1bcff2ea6f8004dd02f207d660a6` |
| Packed with | `tools/r8q-boot pack` (native `userdata` `sda36`, no `--dual-boot`) |
| Partition | `sda24` (`recovery`) |
| Modem | PCIe2 and its PHY `disabled` in the packed DTB, no `modem` overlay, no Sahara loader |

```sh
tools/r8q-check-all --quick         # fast one-shot pass: pr-check, r8q-progress, stable-check, phone-smoke, extras
tools/r8q-stable-check --image      # the recorded image still hashes
tools/r8q-stable-check --phone      # the phone runs it, PCIe2 off, 77/77 UFS guarded
tools/phone-smoke                   # read-only full system smoke check
tools/r8q-progress --live           # compute completion % + check wt-r8q & phone
```

Verified on `2026-11-21` (`NOTES §52–§53` on `recovery-rt8547-torch.img`, `sha256: 04e252eb19670b33e7ad09f92855929af59f32ae76950a483314ab5e45ef28c4`): `tools/phone-smoke` reports `storage guard ok` (`77/77`), `sound cards ok (SamsungGalaxyS2)`, `cameras ok (2 listed by libcamera)`, `battery ok (99%)`, `USB gadget ok`, `sensors ok (AmbientLight)`, plus clean `pw-record` `DMIC3` capture, verified `omacam` `0.5x <-> 3x` switching, and physical RT8547 Flashlight quick-settings tile ON/OFF. `stable-image.conf` keeps the `recovery-fault-quiesce.img` rollback pin until base-DTS fold (`[doc-04]`).

---

## Progress & Status Overview (`2026-11-21`)

| Metric | Progress | Notes |
| --- | --- | --- |
| **Overall (Hardware + Upstream)** | **67.7% (44/65 atomic tasks)** | Full breakdown in [`board/MISSING.md`](board/MISSING.md) |
| **Hardware Bring-Up (`CONTRIBUTING.md` §5.4 Stages 1–16)** | **71.4% (40/56)** | Stages 1–3, 5, 6, 7–8, 14 achieved; 4, 9–13, 15–16 partial/researched (`HARDWARE-DISCOVERY.md`) |
| **Daily Core (Stages 1–8 + UI)** | **81.8% (18/22)** | Usable daily phone on desk (`120 Hz`, full-duplex audio, `0.5x`/`3x` cams, torch, `settings 0.1.3`, Wi-Fi, BT, sensors, `LFM3` AI) |
| **Upstream & Docs (U1–U2)** | **44.4% (4/9)** | Repo CI gates, `CONTRIBUTING.md` §3.1 & `DEVICES.md` pass; 38/83 kernel commits need human `Signed-off-by:` |

| Column | Count | Last Updated |
| --- | --- | --- |
| ✅ SUCCESS | 7 cards ([board/SUCCESS.md](board/SUCCESS.md)) | `2026-11-21` |
| ❌ FAILED (permanent rules) | 8 cards ([board/FAILED.md](board/FAILED.md)) | `2026-11-20` |
| 🔲 MISSING / STEP QUEUE | 16-stage breakdown ([board/MISSING.md](board/MISSING.md) · [`board/tasks.json`](board/tasks.json)) | `2026-11-21` |

---

## ✅ SUCCESS (7 cards — verified on hardware)

| # | Card | What Works | Evidence |
|---|------|-----------|----------|
| 01 | [Dual-Boot & UFS RO Guard](board/success/01-dual-boot-ufs-ro-guard.md) | `sda24` recovery boot -> native `/dev/sda36`; 77/77 UFS `ro=1` | `phone-smoke`: `77/77` UFS guarded, `/dev/sda36` rw |
| 02 | [RPMh TCS Reclaim & Liveness Net](board/success/02-rpmh-rsc-and-liveness-net.md) | `rpmh-rsc.c` TCS timeout reclaim; XPU `no-map` DT (`1324fe066`) | No RPMh/RCU freezes; `HardwareCorrupted: 0 kB` |
| 03 | [Display: simpledrm → MSM & S6E3FC3](board/success/03-display-mdss-s6e3fc3.md) | Zero white flash; native `1080x2400@120`; DPMS + lock wake | Zero DSI/TE errors under GPU load (`NOTES §49`) |
| 04 | [Adreno 650, Freedreno & Hyprland Desktop](board/success/04-gpu-adreno650-hyprland-desktop.md) | GPU `305–587 MHz` (`ae195e6de` fault quiesce); Turnip Vulkan; `stmfts` + keys | `pp64 30.23 / tg16 1.37 t/s`; desktop survives recovery |
| 05 | [USB Gadget, Power/Battery & ZRAM](board/success/05-usb-power-battery-zram.md) | USB NCM `172.16.42.1`; battery `100%`; `2.7G` zram + `4G` swapfile + MGLRU | `max77705`/`max17042` live; `6.8 G` tiered swap |
| 06 | [Wi-Fi, Bluetooth, Clock Sync & Boot GUI](board/success/06-wifi-bluetooth-and-clock.md) | `wlp1s0` UP; `hci0` Powered (`BR/EDR`+`LE`); RTC `sysinit`; `sddm` | `ath11k` + `qca6390` + `caa385175` native UART |
| 07 | [Stability Soak, Pseudo-NMI & On-Device AI](board/success/07-stability-soak-nmi-and-ai.md) | `1800 s` soak (`failures=0`); NMI watchdog; pi-tuned `LFM3` (`10.5 tok/s`); full-duplex `SamsungGalaxyS2` audio + SLPI sensors + `camera-uw` (`2` sensors) | `pd-mapper 1.1-2` + `0.1.30-1` `pw-play`/`pw-record` clean, `cam -l` 2 sensors, 0 dmesg errors (`NOTES §51–§52`) |

---

## ❌ FAILED (8 cards — root causes known, permanent rules in place)

| # | Card | Failure | Permanent Rule |
|---|------|---------|---------------|
| 01 | [Unreserved XPU/TZ/Knox DRAM](board/failed/01-unreserved-xpu-tz-knox-dram.md) | T+12s freeze from XPU violation | DT fix `1324fe066` + `offline_unreserved_xpu_pages()` fallback |
| 02 | [Suspend/cpuidle/DVFS/UFS PM](board/failed/02-suspend-cpuidle-dvfs-ufspm.md) | RPMh -110 timeouts, RCU stalls | `SUSPEND` off, `WFI`-only cpuidle, CPU `performance`, UFS PM disabled |
| 03 | [max77705 built-in & unbind](board/failed/03-max77705-builtin-and-unbind.md) | `=y` freezes boot; `rmmod` kills battery | Keep `=m`, load post-boot, pin I2C `power/control="on"` |
| 04 | [Protected DISPCC clocks & white flash](board/failed/04-display-protected-clocks-and-white-flash.md) | `ESC0_CLK=0` → `-22`; unclocked FB flash | Use `display.dtso`; clear `protected-clocks` in DT |
| 05 | `[omamobile]/mesa, misc.vfr, @120` | Mali Mesa lacked `msm_dri.so`; `vfr` error | Use `extra/mesa` + `vulkan-freedreno`; `vrr=0`; native `@120` verified `2026-11-20` |
| 06 | [Boot & session hazards](board/failed/06-boot-and-session-hazards.md) | Serial alias `-19`, `btmgmt` hang, RTC `1975`, `SIGPIPE` loop | `caa385175` (kernel), `--timeout` on btmgmt, `sysinit.target` RTC, direct sysfs |
| 07 | [stmfts5 8-byte vs 16-byte](board/failed/07-stmfts5-8byte-vs-samsung-16byte-events.md) | `unknown FTS5 event` errors | 16-byte parser `5866f774c`; pin `994000.i2c` `on` |
| 08 | [Vulkan `llama.cpp` Adreno GPU Wedge](board/failed/08-vulkan-llm-adreno-gpu-wedge.md) | Vulkan LLM load wedges GPU -> `cx gdsc` collapse failure | Fixed `cx collapse` quiesce (`ae195e6de`); `-p 512` stall open -> keep `GPU_LAYERS=0` |

---

## 🔲 MISSING — Atomic Queue ([board/MISSING.md](board/MISSING.md) · `tools/r8q-progress`)

> **Human-action rule (`AGENTS.md` §2):** AI agents must never add human `Signed-off-by:`. Currently **38 of 83** commits in `~/omamobile-kernel/wt-r8q` (`omamobile..HEAD`) await Santiago's sign-off (`git -C ~/omamobile-kernel/wt-r8q rebase --signoff ...`).

1. **Immediate Software / Offline Tasks (`3` open):** `[net-03]` BT A2DP audio + Wi-Fi concurrency; `[gpu-03]` bisect `-p 512` compute stall (attended; triggers blanking recovery -> `[gpu-05]`); `[fp-02]` write `goodix,gw9558x` SPI/TZ feasibility note.
2. **Kernel DT Fixes (`human-gate` reboot, `7` open):** `[cam-03..04]` fix `camss` `port@1` (`s5k3l6_out`) async deadlock -> `omacam` tele/uw verify; `[aud-04..05]` `va_macro` `qcom,dmic-sample-rate = <600000>` + LPI DMIC capture; `[hap-04]` `&pm8150l_flash` (`leds-qcom-flash.ko` ready); `[hap-02]` haptics DT (`LINEAR_0832RC` / `max77705`); `[gpu-04]` DPU scanout recovery restore; `[doc-04]` re-pin `stable-image.conf`.
3. **High-Risk Hardware Bring-Up (`8` open):** `[usb-03..04]` Type-C replug/PD & OTG (`usbc-deadman`), `[mdm-03..05]` native `sda36` `modem.dtso` boot + NV verify + SIM/QMI/MBIM, `[pwr-03..04]` / `[slp-02..03]` `s2idle`/DVFS/`cpuidle`/UFS PM w/ watchdog, `[cam-05]` IMX555/IMX616 drivers.
4. **Kernel Upstream (`4` open):** `[krn-02..05]` Human `Signed-off-by:` on 38 commits -> split topic branches -> `tools/kernel-pr-check --build --merge origin/omamobile` -> fork PRs.

---

## 📁 File Inventory

### Device Package (`packages/omamobile-device-r8q/`)

| File | Status | Notes |
|------|--------|-------|
| `PKGBUILD` | ✅ | Builds `reboot-to`, `r8q-liveness-heartbeat`, preflight |
| `version` | ✅ | `0.1.29` in repo and live (`0.1.29-1` adds `pd-mapper.service` and `-9.5 dB` `HiFi.conf` safe gains; `pd-mapper 1.1-2` fixes `modemr.jsn` empty array crash) |
| `70-omamobile-dma-heap.rules` | ✅ | Grants `video` / `uaccess` access to `/dev/dma_heap` |
| `70-r8q-libcamera.conf` | ✅ | Defaults `LIBCAMERA_SOFTISP_MODE=cpu` to avoid `FD650` EGL hang |
| `test/all` | ✅ | Comprehensive config tests |
| `hyprland-r8q.conf` | ✅ | `DSI-1,1080x2400@120,0x0,2.25`; `vrr=0` |
| `ucm2/` | ✅ | `HiFi.conf` (`-9.5 dB` safe gains `Analog 10` / `Digital 657`), `Samsung-Galaxy-S20-FE.conf`, `SamsungGalaxyS2.conf` |
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
| `r8q-progress` | ✅ | Computes overall / per-stage completion % & step queue from `board/tasks.json` (`--live`, `--next`, `--json`) |
| `r8q-check-all` | ✅ | One-shot read-only check of repo gates, `r8q-progress --live`, `r8q-stable-check --phone`, `phone-smoke`, & extras |
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
| `board/README.md` | ✅ | Board overview & progress % metrics |
| `board/tasks.json` | ✅ | Single-source-of-truth atomic task tracker (`tools/r8q-progress`) |
| `board/SUCCESS.md` | ✅ | 7 success cards (synced `2026-11-21`) |
| `board/FAILED.md` | ✅ | 8 failed cards / permanent avoidance rules |
| `board/MISSING.md` | ✅ | 16-stage milestone % table + step-by-step atomic backlog |
| `board/success/01-07.md` | ✅ | All 7 success detail files |
| `board/failed/01-08.md` | ✅ | All 8 failed detail files |
| `board/missing/01-05.md` | ✅ | Archived gate logs |
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
| Worktree | ⚠️ | `~/omamobile-kernel/wt-r8q`, branch `r8q-gpu-fault-quiesce` (`83` commits ahead of `omamobile`; `38` missing human `Signed-off-by:`) |
| Output | ✅ | `~/omamobile-kernel/out-r8q` (`7.3.0-rc6-omamobile-push1-gae195e6deaac`) |
| HEAD | ✅ | `ae195e6de` (`drm/msm/a6xx: quiesce the adreno smmu across the CX collapse window`) |
| Upstream prep | 🔴 | Run human `git rebase --signoff`, split topic branches, then pass `tools/kernel-pr-check --build --merge origin/omamobile` |

---

## 🔑 Critical Path

```
Native root sda36 + 120 Hz + GPU quiesce + Full-duplex Audio + 0.5x/3x Cams + RT8547 Torch (✅) → Kernel DCO Sign-off & Upstream PRs → Stages 9–16 polish
```

**What's working right now on hardware (`7.3.0-rc6-omamobile-push1-gae195e6deaac` + `recovery-rt8547-torch.img` + `omamobile-device-r8q 0.1.33-1`):**
- Native UFS root `/dev/sda36` (`77/77` UFS partitions locked `ro=1`), USB CDC NCM + ACM gadget (`172.16.42.1`), clean boot on `tools/phone-smoke`
- Hyprland desktop (`sddm`, `quickshell`, `squeekboard`, `1080x2400@120`), DPMS & Power-key wake + lock screen, `stmfts` 16-byte Type-B touch, Volume Up/Down/Power
- Wi-Fi (`wlp1s0` UP via `ath11k_pci`) & Bluetooth (`hci0` Powered via `998000.serial`)
- Adreno 650 GPU (`3d00000.gpu`, `renderD128`, `305–587 MHz`, Turnip Vulkan `1.3.354`, fault quiesce recovery) + pseudo-NMI watchdog
- Remoteprocs (`slpi`, `cdsp`, `adsp` via `pd-mapper 1.1-2`), SLPI Sensors (`Accelerometer`, `AmbientLight`), & Full-Duplex Audio (`SamsungGalaxyS2` -> dual `cs35l41` at `-9.5 dB` UCM2 + `DMIC3` via `dmic-va.dtso` & `50-r8q-audio.conf`)
- Cameras & Torch: `camera-uw.dtso` + `omacam-lenses.toml` (`4200x3120` `0.5x` Ultrawide `s5k3l6xx` <-> `3256x2448` `3x` Telephoto `hi847` switching verified in `omacam`), and `torch.dtso` (`gpio-leds` RT8547 `GPIO 117` + `GPIO 126` `output-high`) + `20-r8q-torch.rules` (`664 root:video`) toggling Flashlight tile physically ON/OFF
- Battery telemetry (`max77705`/`max17042`, `100%`, `27 °C`, `health Good`), `6.8G` tiered swap (`2.7G` zram + `4G` swapfile + MGLRU), & pi-tuned `LFM3-1.7B` AI (`10.5 tok/s`)
