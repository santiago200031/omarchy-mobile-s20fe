<!-- SPDX-License-Identifier: MIT -->

# Omarchy Mobile — Samsung Galaxy S20 FE (`r8q`) Port & Integration Plan

**Status (2026-11-12):** M0-M2, M4.1, M5.1 verified on hardware. `omamobile-device-r8q` **0.1.7 installed and verified live** (`performance` pin, no deep idle, `auto_hibern8`+`clkscale`+`clkgate=0`); **0.1.9 installed and verified live** 2026-11-13 (stub on `:18081`, RTC epoch + wifi ASPM units active, ASPM `0/0` on `phy0`). Desktop on `simpledrm` + `llvmpipe`; Wi-Fi on; crash loop ~11 min persists with all mitigations (see blocker 1); GPU/BT/audio/battery staged.
- **Kernel branch:** `omamobile-r8q` (`~/omamobile-kernel/wt-r8q`, out `~/omamobile-kernel/out-r8q`)
- **Upstream:** `derekross/linux-omarchy-mobile` (`v7.3-rc6`)

---

## 0. Current Status & Open Work

Mainline `7.3.0-rc6` boots from `recovery` (`sda24`); LineageOS untouched on `boot` (`sda23`) + `super`. Live: USB NCM (`172.16.42.1`), SSH, `ttyGS0`, UFS, touchscreen (`stmfts`), Wi-Fi (`wlp1s0`, off for bisect), Hyprland (`simpledrm` + `llvmpipe`). Staged: GPU, BT, audio, battery.

### Fixed in Tree / Active on Hardware

| Symptom | Root Cause | Fix & State |
| --- | --- | --- |
| Dead power button / no wake source | `CONFIG_POWER_RESET_QCOM_PON=m`, no early modules | Built-in `=y` — **live** |
| Freeze minutes after boot (`s2idle`) | `s2idle` never resumes on Samsung TZ | `# CONFIG_SUSPEND is not set`, logind gate, masked targets — **live** |
| Hangs needed a button hold | Watchdog drivers unset/modular | `CONFIG_QCOM_WDT=y`, `CONFIG_SOFT_WATCHDOG=y`, `RuntimeWatchdogSec=30s` — **live** |
| Disk I/O freeze after 150 ms idle | UFS `auto_hibern8` exit fails + unsupported WB query | `omamobile-r8q-ufs.service` writes `0` (`0.1.4`); kernel skips WB attr — **live** |
| RCU stalls on cores in deep `cpuidle` | `state1` power-collapse hangs cores (Samsung TZ) | Service disables `state1` (WFI only) (`0.1.4`) — **live** |
| ~11 min crash (`rpmh_write_batch -110` in `sugov`) | `schedutil` OPP transitions time out on AMC `RPMh` votes | All policies pinned `performance` (`0.1.6`) — **live** |
| ~11 min crash, Wi-Fi off (`ufshcd_ungate_work` -> same `-110`) | UFS clock ungating votes `icc_set_bw` over AMC `RPMh` | Runtime `clkscale_enable=0` + `clkgate_enable=0` — **live; queued in `0.1.7`** |
| Boot hung on `multi-user.target` | Hand-installed `omamobile-bluetooth.service` hung in `btmgmt` | Unit disabled; exclude until BT verified — **live** |
| `bt-agent` user unit restart-loop | BlueZ unit without `/usr/bin/bt-agent` | `systemctl --user mask` — **live** |
| `zram-setup` hung at boot | `CONFIG_ZRAM=m`, no modules | Built-in `CONFIG_ZRAM=y` + 4 GiB `/swapfile` — **live** |
| Blind hangs on network/UI wedge | No USB serial getty | `console=ttyGS0` + `serial-getty@ttyGS0` (`0.1.4`) — **live** |
| Panel blanks at 10 min, never wakes | `consoleblank=600` ignores DRM pageflips | `omamobile-r8q-panel.timer` unblanks `fb0` every 4 min (`0.1.4`); `consoleblank=0` in next flash — **live** |
| Sluggish UI on `llvmpipe` | Animations/blur with no GPU | Disabled in `hyprland-r8q.conf` until `renderD128` is live (`0.1.4`) — **live** |
| Missing modules for `-dirty` kernel | Tarball `utsrelease` mismatch | Rebuilt/pushed 1,818 modules (`depmod` clean) — **live** |

### Open Blockers (Priority Order)

1. **UFS RPMh AMC-vote soak & `0.1.7` package:** Wi-Fi-off bisection proved crashes #3-#6 came from `ufshcd_ungate_work -> ufs_qcom_icc_update_bw -> icc_set_bw -> rpmh_write_batch (-110)` (same voter as the `sugov` crash, different trigger: any UFS clock ungate, even background journal/SSH I/O). Mitigations VERIFIED 2026-11-12: with `performance` + no deep idle + `auto_hibern8=0` + `clkscale/clkgate=0` (+ Wi-Fi off for the bisect), the phone idled past 17 min with zero `ramoops` events — first survival beyond the ~11-min wall. `0.1.7` installed on-device (UFS helper verified live via `restart`, not `start`, on an already-active oneshot). Evening 2026-11-12: crashes #7 (~22:34), #8 (~22:47) and #9 (~23:46) all landed with the full `0.1.7` mitigations live — silencing the votes does NOT hold. Crash #8's pstore (our own read): first stall at 700 s, repeated stalls past 919 s, CPU 7 NMI-dead three times while CPUs 0/2 answered theirs, zero error lines throughout — one wedged prime core. Crash #10 (`0.1.9` boot `29d4ee8d`): first stall 552 s, silent (no NMI dump captured) — 0.1.9 changes nothing about the wall, as expected. Install persists across resets (`0.1.9-1` intact, both units auto-active on the next boot). Boot `ae226d76` (`0.1.9`) ran clean to 652 s, then died at the wall — confirmed crash #11: first stall 700 s, CPU 4 NMI-dead while CPU 1 answered, zero error lines. Victims now span little (1), big (4, 6) and prime (7): the wedge is not prime-specific. CPU-7-offline soak STARTED 2026-11-13 on boot `6e38eaed` at 637 s (announced, reversible; dead-man timer armed then removed — verified no `cpu7-deadman` unit/timer exists, `cpu7=0` holds past the T+120 s fire point). RESULT 2026-11-13: `cpu7=0` from 637 s, survived to **1726 s (28.8 min) with zero stalls** and stopped deliberately (not re-onlined, not provoked). Baseline first stalls were 552-1070 s, so the wall moved by 656 s past the old record — prime-core / shared PSCI OSI transition path is the quarry. Caveat: 637 s pre-test uptime, hotplug is itself a PSCI transition. Soak phase ENDS here; userspace mitigations exhausted.
2. **Adreno 650 (`M3.3`):** Firmware gate clear (`a650_sqe.fw` v0x112 on laptop; signed `a650_zap.mbn` extracted). Stage SQE to `/lib/firmware`, fix `adreno_smmu -110` (`GPU_CX_GDSC`/`gpucc`/`rpmhpd`) + `S6E3FC3` DSI nodes, unblacklist `msm`, verify `renderD128` + `vulkaninfo`. Pin GPU devfreq on first boot (same `-110` trap).
3. **Staged peripherals (one per boot, `ttyGS0` armed):** Wi-Fi UX/persistence (`usb0` unmanaged); `bluetoothctl` scan with timeout guard (`bt-agent` stays masked); ADSP + `snd-soc-sm8250` + dual `CS35L41` (`aplay -l` + UCM2); `MAX77705` `-EINVAL` + `/sys/class/power_supply/*` to shell.
4. **`omamobile-update` fetch failure:** (a) live `usb0` default route has no metric, beats `wlp1s0` — restart `systemd-networkd` to apply `0.1.6` `Metric=900`; (b) `[omamobile]` points at missing `file:///var/lib/omamobile/repo` — populate via `tools/repo-push` or comment out; (c) add `DisableSandbox` (no kernel Landlock). Bundle into `0.1.7`; run post-soak (flaps networking).
5. **Lock-screen touch (`LockGlance.qml`):** `wakeTouch` swallows touches while `lock.displayDark` is set; nothing clears it on `r8q`. Fix seat/focus + power-key wake binding before re-enabling `idle.lock`.
6. **Next `recovery` flash:** bake `consoleblank=0` into cmdline on next human-approved `recovery` write. Never touch `boot`/`super`/bootloader.

> **Operational Rules ([AGENTS.md](../../AGENTS.md), [CONTRIBUTING.md](../../CONTRIBUTING.md)):** read-only probes first; **one hardware change per boot**; human approval before any reboot, flash, or partition write. Build in container (`tools/build-packages --local r8q`), never compile on the phone. Enable units via `/etc` (`systemctl enable`). Never commit identifiers (serials, MACs, IMEIs, NV dumps).

---

## 1. Hardware & Partition Reference

| Subsystem | Spec | Source |
| --- | --- | --- |
| **Device** | Galaxy S20 FE Snapdragon (`SM-G780G`/`SM-G781B`), `r8q` | `ro.product.device` |
| **SoC/CPU** | Snapdragon 865 (`SM8250`/`kona`), Kryo 585 (1x2.84 + 3x2.42 A77, 4x1.80 A55) | `ro.soc.model=SM8250` |
| **GPU/RAM** | Adreno 650 (Turnip/Freedreno) / 6 GB LPDDR5 (~5.5 GiB) | `MemTotal: 5747672 kB` |
| **Storage** | UFS (`1d84000.ufshc`, `sda`-`sdd`), **single-slot Non-A/B** | `ro.build.ab_update=false` |
| **Bootloader** | Samsung S-Boot/ABL (`G780GXXSHEYJ1`, `SVB1.0`, `orange`) | `dtbo_idx=7`, `dtb_idx=0` |
| **Display** | `S6E3FC3_AMS646YD01` 1080x2400 AMOLED 120 Hz DSI-cmd | `lcd_id=0x805081` |
| **USB/Radio** | DWC3 (`a600000.dwc3`), QCA6390 (PCIe+UART), `s3fwrn5` NFC | `androidboot.hardware.sku` |
| **PMIC/Audio** | `pm8150/b/l`, `MAX77705`, dual `CS35L41` | Live DTB / `dmesg` |

| Partition | Block Device | Rule |
| --- | --- | --- |
| `boot` | `sda23` | LineageOS — **untouched fallback** |
| `recovery` | `sda24` | Omarchy boot image (`tools/r8q-boot` guarded writes only) |
| `dtbo` / `super` | `sda22` / `sda29` | Android DTBO + dynamic partitions |
| `userdata` | `sda36` | Coexistent Arch rootfs alongside Android |
| `efs`, `sec_efs`, `persist` | `sda9`, `sda12`, `sda8` | **NV/calibration — READ-ONLY (M0 backup)** |
| `modemst1/2`, `fsg`, `fsc` | `sda1/2`, `sdd3`, `sda3` | **Modem EFS — READ-ONLY (M0 backup)** |
| `modem`, `dsp` | `sda20`, `sda21` | Radio/DSP firmware (read-only extraction) |
| `xbl*`, `abl`, `tz`, `aop`, `hyp`, `vbmeta*` | `sdb`, `sdd` range, `sda27` | **Bootloader/TZ — NEVER TOUCH** |

`device/r8q/device.conf`: `BOOT_METHOD=ram-only`, `UFS_RW=""` — Pixel tools cannot touch `boot`/`super`. Normal reboot lands LineageOS (`sda23`); `adb reboot recovery`/VolUp+Power boots Omarchy (`sda24`). **Scope:** recovery-slot boot only; no other boot flows or third-party kernel params apply.

---

## 2. Architecture (`gs201` vs `sm8250`)

| Layer | `cheetah` (Pixel 7 Pro) | `r8q` (S20 FE) |
| --- | --- | --- |
| **Slots/Image** | A/B, Header v4 (`fastboot`) | Single-slot, Header v2 + `recovery_dtbo` + DTB + AVB RSA4096 |
| **Display/GPU** | Exynos DECON + Mali-G710 | `simpledrm` -> MDSS/DPU1 + `S6E3FC3`; Adreno 650 + signed `a650_zap.mbn` |
| **Power** | Exynos ACPM | Qualcomm `RPMh` (`performance` + UFS `clkscale/clkgate=0` until AMC fix) |
| **Audio/Sensors** | Google AoC | QDSP6/ADSP + `CS35L41` & SLPI (`fastrpc`) |

---

## 3. Gated Milestones (M0-M8)

- **M0 — Capture & NV Backup:** [x] **M0.1:** 11 critical partitions, double-read `SHA256SUMS` (`~/android/backups/r8q-nv/`). [x] **M0.2:** Header v2 offsets + live FDT (`msm-id=<0x0164 0x020001>`, `board-id=<0x08 0x06>`) to live DTB reference.
- **M1 — Kernel & DTB:** [x] **M1.1:** `7.3.0-rc6-omamobile-push1` + `sm8250-samsung-r8q.dtb` + 1,818 modules in `omamobile-build:24.04` (`PON=y`, `QCOM_WDT=y`, `SOFT_WATCHDOG=y`, `ZRAM=y`, no `SUSPEND`, UFS WB fix).
- **M2 — Guard, Recovery Boot & Stability:** [x] **M2.1:** `tools/r8q-boot` packs `Image` + guard + `recovery_dtbo` + DTB + AVB footer (`sda24`). [x] **M2.2:** first bare-metal boot (USB NCM `172.16.42.1`, `ttyGS0`, SSH; early Alpine prototype superseded by Arch rootfs). [x] **M2.3:** `0.1.4` installed (UFS `auto_hibern8=0`, 30 s watchdog, sleep masked, `state1` off, panel timer, serial getty, flat Hyprland). [x] **M2.4:** `0.1.6` installed (auto-pins `performance`; Rust-only LFM3-1.7B server). [ ] **M2.5:** `0.1.7` installed + verified live; soak NOT clean (crashes #7-#9 with all mitigations) — but a clean boot passed 907 s with zero stalls (watch `W+6min`), then the cycle resumed; boot `c8a2e7a5` on `0.1.9` then ran clean to a first stall at 1070 s (crash #12, victims CPUs 5/4, detector CPU 7 alive) — longest clean stretch yet, then the wall took it too (fresh 39 s boot, dead again minutes later — wedge is intermittent, not clockwork); `0.1.9` installed + verified live on boot `29d4ee8d`; next: CPU-7-offline soak + 30-min zero-`ramoops` soak.
- **M3 — Display & GPU:** [x] **M3.1:** `simpledrm` 1080x2400 FB + stable `llvmpipe` Hyprland. [x] **M3.2:** extracted `a650_gmu.bin`, signed `a650_zap.mbn`, `adsp/cdsp/slpi.mbn`, `cs35l40/41`, `qca6390` blobs; upstream `a650_sqe.fw` v0x112 (at least 0x95) staged. [ ] **M3.3:** stage SQE, fix `adreno_smmu -110` (`GPU_CX_GDSC`/`gpucc`/`rpmhpd`) + `S6E3FC3` DSI nodes, unblacklist `msm`, verify `renderD128` + `vulkaninfo` (pin GPU devfreq first — same `-110` trap).
- **M4 — Touch, Keys & Sensors:** [x] **M4.1:** `stmfts` on `/dev/input/event3`, `gpio_keys` `event2`, `qpnp_pon` `event0`. [ ] **M4.2:** fix `LockGlance.qml` touch/wake + power-key binding, re-enable `idle.lock`. [ ] **M4.3:** bring up SLPI (`slpi.mbn` + `fastrpc`), verify via `iio-sensor-proxy` (2 `/sys/bus/iio` devices exist, no `remoteproc` yet).
- **M5 — Wi-Fi, BT & Audio:** [x] **M5.1:** `wlp1s0` associates (off for UFS-bisect); keep `usb0` unmanaged. [ ] **M5.2:** `bluetoothctl` scan with timeout guard (`bt-agent` stays masked). [ ] **M5.3:** `kona-mtp-snd-card` in `aplay -l` + UCM2 (`HiFi.conf`) playback.
- **M6 — Battery, Thermal, Power:** [ ] **M6.1:** fix `MAX77705` `-EINVAL`, expose `/sys/class/power_supply/*` to shell. Own probe 2026-11-13: chip answers on I2C bus 0 addr `0x66`; revision reg `0x01` reads `0x02` = `MAX77705_PASS2` (decoded against our own worktree `max77705-private.h`: PASS1=1/PASS2=2/PASS3=3, mask `0x07`). But the live probe fails `-EINVAL` (regulator-settings flavor at 5.7 s), NOT the `-ENODEV` our worktree's PASS3-only gate would print — running driver differs from worktree or fails later at regulator/DT settings. Resolved 2026-11-13 from live `dmesg`: `pin gpio11 already requested by 0-0069; cannot claim for 0-0066` — a pinctrl conflict between the two MAX77705 sub-functions over SPMI PMIC gpio11, i.e. a DT pin-assignment bug. Not the silicon revision (chip is PASS2, irrelevant) and not a regulator value. Fix belongs in the DTB (kernel tree): one owner for the pin. Live bindings: `charger@69` has `max77705-charger` bound and working (it holds gpio11); `pmic@66` has no driver (MFD probe died on the conflict). Telemetry win 2026-11-13: `/sys/class/power_supply/max77705-charger` is live (Charging/Fast/Good/online, 4.4 V, 900 mA) — the shell-readable half of M6.1 works pre-DT-fix; fuel-gauge `%` still needs the MFD/DT fix. [ ] **M6.2:** RPMh AMC BCM-voter `-110` quirk + TZ `state1` fix in kernel/DTB (reference: `sm8250.dtsi` `apps_rsc` ACTIVE 2 / SLEEP 3 / WAKE 3 / CONTROL 1; live-DTB comparison pending — file lives on the other laptop).
- **M7 — Rootfs & Shell:** [x] **M7.1:** coexistent Arch rootfs on `userdata`; device package container-built (`--local r8q`, `keys/r8q`). [ ] **M7.2:** `omamobile-shell` scale-2 + `omamobile-settings` panel (replaces `cage`/`cog`/`darkhttpd` prototype).
- **M8 — On-Device AI (LFM3-1.7B) & Tuning:** [x] **M8.1:** `zram ram/2` (`zstd`), 4 GiB `/swapfile`, MGLRU `min_ttl_ms=1000`, `swappiness=150`, `page-cluster=0`. [ ] **M8.2 [BLOCKED ON M3.3]:** 120 Hz animations + Adreno devfreq (staged, inactive on `simpledrm`). [x] **M8.3:** LFM3-1.7B `Q4_K_M` ~15.7 tok/s on Kryo 585 (`tools/r8q-ai`). [ ] **M8.4:** Model gap closed 2026-11-13: LFM3-1.7B `Q4_K_M` fetched on the laptop and verified (1,099,627,808 bytes, GGUF v3, 287 tensors, sha256 `3793e1e6…05c93c4` = HF LFS digest); phone had runtime (`llama-server` build 12292 arm64) but an EMPTY `/home/alarm/models/` — pushing resumably (rsync absent on phone, so byte-offset `cat >>` resume across resets). `omamobile-ai-server` on `:8080` serves the `/v1/models` listing but answers chat from canned templates (verified in our own source — no inference); real chat must proxy to the on-demand inference server or replace the stub. Port collision: `llama-server` (real inference, our own `tools/omamobile-ai`) also wants `127.0.0.1:8080` — real inference wins the port; the stub moves or retires (one-line `PORT` change, batched with the next rebuild). [ ] **M8.5:** `GGML_VULKAN` bench after M3.3. [ ] **M8.6 (pi with our LFM3):** 1) `mise` on phone (`pacman -S mise` if absent), `mise use -g pi` (verify the backend serves an aarch64 asset at install time, else the registry's npm backend); 2) provider entry in `~/.pi/agent/models-store.json` (`api openai-completions`, `baseUrl http://127.0.0.1:8080/v1`, model id = live `/v1/models`) + `settings.json` default; 3) `pi -p` roundtrip. Real endpoint is our own `tools/omamobile-ai` `llama-server` (OpenAI `/v1`, on-demand + idle watchdog). Warmup: the server auto-starts on any `omamobile-ai chat` and idles off — start it with a long timeout before a pi session.

Model policy: **LFM3-1.7B-`Q4_K_M` is the sole default** (`/v1/models` lists just it; `/home/alarm/models/` empty until `tools/r8q-ai push-model`). Catalog (verified HF repos 2026-11-12): `LiquidAI/LFM3-1.7B-GGUF` (`Q4_K_M`/`QAD-Q4_0`, ~1.39 GB) - `Qwen/Qwen2.5-1.5B-Instruct-GGUF` (fallback) - `meta-llama/Llama-3.2-3B-Instruct-GGUF` (heavy) - `LiquidAI/LFM3-VL-2B-GGUF` + `mmproj` (vision/video) - `vikhyatk/moondream2` (light OCR) - `LiquidAI/LFM3-Omni-2B-GGUF` (`--reasoning off`) - `LiquidAI/LFM3-Audio-2B-GGUF` (ASR) - `ggerganov/whisper.cpp` + `hexgrad/Kokoro-82M` (modular STT/TTS) - `ggml-org/embeddinggemma-2-GGUF` (+`unsloth`, base `google/embeddinggemma-2`, ~350 MB RAG).

---

## 4. Rules, Checklist & Next Steps

- **Memory:** `zram-size = ram / 2`, 4 GiB `/swapfile` overflow, MGLRU `min_ttl_ms=1000`, `swappiness=150`, `page-cluster=0` (later: `MULTI_COMP` `lz4`+`zstd`).
- **Scheduling:** `performance` governor + UFS `clkscale/clkgate=0` until M6.2; pin GPU devfreq on M3.3 bring-up; restore `schedutil` + `UCLAMP_MIN` + `misc:vfr=true` + 120 Hz only after soak-verified fixes.
- **Packaging:** everything in `packages/omamobile-device-r8q/`, container-built (`--local r8q`); vendor firmware to `~/.cache/omamobile/firmware/r8q/` (never commit blobs); `a650_zap.mbn` under `NoUpgrade`.
- **Checklist (every commit):** `tools/pr-check`, `tests/packages`.
- **Next 4 (one change per boot):** 1) observe boot `29d4ee8d` (`0.1.9` is this boot's change — install verified, units active); 2) CPU-7-offline single-change soak — **STARTED 2026-11-13 on boot `6e38eaed`** at 637 s: dead-man timer armed then stopped, CPU 7 offlined cleanly (hotplug survived, `online=0`), zero stalls at start; record first-stall is 1070 s, so survival well past that is the signal; 3) kernel handoff dossier (prime-core power path) if the CPU7 test confirms; 4) everything else (Adreno M3.3, peripherals, lock touch, 120 Hz) stays staged behind a clean soak.
- **Clean-room principle:** all userspace glue stays packaged in `omamobile-device-r8q` (MIT); kernel/DTB quirks are structured for the kernel tree (`GPL-2.0-only`). Everything here is implemented by us from our own hardware measurements; nothing is copied from anywhere.
