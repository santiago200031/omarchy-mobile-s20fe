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

1. **UFS RPMh AMC-vote soak & `0.1.7` package:** Wi-Fi-off bisection proved crashes #3-#6 came from `ufshcd_ungate_work -> ufs_qcom_icc_update_bw -> icc_set_bw -> rpmh_write_batch (-110)` (same voter as the `sugov` crash, different trigger: any UFS clock ungate, even background journal/SSH I/O). Mitigations VERIFIED 2026-11-12: with `performance` + no deep idle + `auto_hibern8=0` + `clkscale/clkgate=0` (+ Wi-Fi off for the bisect), the phone idled past 17 min with zero `ramoops` events — first survival beyond the ~11-min wall. `0.1.7` installed on-device (UFS helper verified live via `restart`, not `start`, on an already-active oneshot). Evening 2026-11-12: crashes #7 (~22:34), #8 (~22:47) and #9 (~23:46) all landed with the full `0.1.7` mitigations live — silencing the votes does NOT hold. Crash #8's pstore (our own read): first stall at 700 s, repeated stalls past 919 s, CPU 7 NMI-dead three times while CPUs 0/2 answered theirs, zero error lines throughout — one wedged prime core. Crash #10 (`0.1.9` boot `29d4ee8d`): first stall 552 s, silent (no NMI dump captured) — 0.1.9 changes nothing about the wall, as expected. Install persists across resets (`0.1.9-1` intact, both units auto-active on the next boot). Boot `ae226d76` (`0.1.9`) ran clean to 652 s, then died at the wall — confirmed crash #11: first stall 700 s, CPU 4 NMI-dead while CPU 1 answered, zero error lines. Victims now span little (1), big (4, 6) and prime (7): the wedge is not prime-specific. CPU-7-offline soak STARTED 2026-11-13 on boot `6e38eaed` at 635 s (announced, reversible; dead-man timer armed then removed — verified no `cpu7-deadman` unit/timer exists, `cpu7=0` holds past the T+120 s fire point). RESULT 2026-11-13: `cpu7=0` survived idle to **1757 s (29.3 min) with zero stalls**, then **crashed (#13) at `1768-1780 s`** seconds after an `scp` push of `LFM3-1.7B-Q4_K_M.gguf` began writing to `sda36` (156 MiB written). `console-ramoops-0` on boot `bd4fac14` captured `[ 1768.090086] watchdog0: pretimeout event` and `[ 1780.137259] rcu: INFO: rcu_preempt detected stalls on CPUs/tasks: 1, 4, 6 (detected by 2, t=5252 jiffies, ncpus=7)`. Offlining CPU 7 delays idle failure, but does NOT cure the wedge under sustained UFS+USB writeback. Soak phase ENDS here; userspace mitigations exhausted.
2. **Adreno 650 (`M3.3`):** Firmware gate clear (`a650_sqe.fw` v0x112 on laptop; signed `a650_zap.mbn` extracted). Stage SQE to `/lib/firmware`, fix `adreno_smmu -110` (`GPU_CX_GDSC`/`gpucc`/`rpmhpd`) + `S6E3FC3` DSI nodes, unblacklist `msm`, verify `renderD128` + `vulkaninfo`. Pin GPU devfreq on first boot (same `-110` trap).
3. **Staged peripherals (one per boot, `ttyGS0` armed):** Wi-Fi UX/persistence (`usb0` unmanaged); `bluetoothctl` scan with timeout guard (`bt-agent` stays masked); ADSP + `snd-soc-sm8250` + dual `CS35L41` (`aplay -l` + UCM2); `MAX77705` `-EINVAL` + `/sys/class/power_supply/*` to shell.
4. **`omamobile-update` fetch failure:** (a) stale `/etc/systemd/network/20-usb-gadget.network` (from Nov 11 bring-up, no `Metric=`) shadows `/usr/lib/systemd/network/20-usb-gadget.network` (`Metric=900` in `0.1.6+`) — remove `/etc/systemd/network/20-usb-gadget.network` and restart `systemd-networkd` so `wlp1s0` metric 600 wins; (b) `[omamobile]` points at missing `file:///var/lib/omamobile/repo` — populate via `tools/repo-push` or switch channel (`backup=(etc/pacman.d/omamobile-mirrorlist)` added in `0.1.10`); (c) `/proc/config.gz` already has `CONFIG_SECURITY_LANDLOCK=y` and `CONFIG_SECURITY_LANDLOCK_LOG=y`, so kernel Landlock is present (add `DisableSandbox` only if pacman sandbox fails on another check). Run post-soak (flaps networking).
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
- **M6 — Battery, Thermal, Power:** [ ] **M6.1:** fix `MAX77705` `-EINVAL`, expose `/sys/class/power_supply/*` to shell. Own probe 2026-11-13: chip answers on I2C bus 0 addr `0x66`; revision reg `0x01` reads `0x02` = `MAX77705_PASS2` (decoded against our own worktree `max77705-private.h`: PASS1=1/PASS2=2/PASS3=3, mask `0x07`). Live FDT inspection confirmed both `pmic@66` (`maxim,max77705`) and `charger@69` (`maxim,max77705-charger`) set `interrupt-parent` + `pinctrl-0` to SPMI PMIC `gpio11`; per mainline `sdm845-samsung-starqltechn.dts`, only `pmic@66` owns the PMIC GPIO interrupt + `pinctrl-0`, while `charger@69` uses `interrupt-parent = <&max77705>; interrupts = <0>;` with no `pinctrl-0`. Modular probe finding 2026-11-13: building `CONFIG_MFD_MAX77705`, `CONFIG_CHARGER_MAX77705`, and `CONFIG_BATTERY_MAX17042` as `=y` with `fuel-gauge@36` enabled froze early boot at `0.95 s` (`scsi host0: ufshcd`); keeping all MAX77705/MAX17042 configs `=m` boots cleanly. Live SSH module test (`modprobe max77705`, `max77705_charger`, `max17042_battery`, plus `echo max17047 0x36 > /sys/bus/i2c/devices/i2c-0/new_device`) succeeded on `i2c-0`: `max77705-charger` reported `online=1`, `present=1`, `health=Good`, `USB`, `4400000` uV, and `/sys/class/power_supply/max170xx_battery` reported `capacity=99`, `voltage_now=4302343`, `voltage_ocv=4320859`, `temp=260` (26.0 °C), `current_now=-126718`, `health=Good` (`i2cget -y 0 0x36` reg `0x00=0x0080`, `0x06=0x63ed`, `0x09=0xd6ba`). Next: enable `fuel-gauge@36` with `=m` auto-load and read `max170xx_battery` in `ai-server`/shell. [ ] **M6.2:** RPMh AMC BCM-voter `-110` quirk + TZ `state1` fix in kernel/DTB (reference: `sm8250.dtsi` `apps_rsc` ACTIVE 2 / SLEEP 3 / WAKE 3 / CONTROL 1; live FDT confirmed `qcom,tcs-config = <2 2 0 3 1 3 3 1>` matches `sm8250.dtsi`, and `psci` runs OSI mode after `failed to set PC mode: -3`).
- **M7 — Rootfs & Shell:** [x] **M7.1:** coexistent Arch rootfs on `userdata`; device package container-built (`--local r8q`, `keys/r8q`). [ ] **M7.2:** `omamobile-shell` scale-2 + `omamobile-settings` panel (replaces `cage`/`cog`/`darkhttpd` prototype).
- **M8 — On-Device AI (LFM3-1.7B) & Tuning:** [x] **M8.1:** `zram ram/2` (`zstd`), 4 GiB `/swapfile`, MGLRU `min_ttl_ms=1000`, `swappiness=150`, `page-cluster=0` (`0.1.10` adds `dirty_background_bytes=16M` / `dirty_bytes=64M` writeback cap). [ ] **M8.2 [BLOCKED ON M3.3]:** 120 Hz animations + Adreno devfreq (staged, inactive on `simpledrm`). [x] **M8.3:** LFM3-1.7B `Q4_K_M` ~15.7 tok/s on Kryo 585 (`tools/r8q-ai`; `tools/omamobile-ai` pins inference to online A77 cores `4-7` via `taskset` and binds `127.0.0.1:8080`). [x] **M8.4:** Verified 2026-11-13 on `sda36`: `/home/alarm/models/LFM3-1.7B-Q4_K_M.gguf` (`1.0G`, `chown 1000:1000`, sha256 `3793e1e6e2266919d6e015287221b41100175a98eddaba92e111aed3b05c93c4` matching HF LFS) streamed over `omamobile.initramfs` SSH with zero errors (`0.1.10-1` installed; `omamobile-ai-server` moved to `:18081` in `0.1.9`, clearing `:8080` for `llama-server`). [ ] **M8.5:** `GGML_VULKAN` bench after M3.3 (deprioritized behind phone stability/usability). [ ] **M8.6 (pi with our LFM3 — end of priority after phone stability & hardware bring-up):** 1) `mise` on phone (`pacman -S mise` if absent), `mise use -g pi` (verify the backend serves an aarch64 asset at install time, else the registry's npm backend); 2) provider entry in `~/.pi/agent/models-store.json` (`api openai-completions`, `baseUrl http://127.0.0.1:8080/v1`, model id = live `/v1/models`) + `settings.json` default; 3) `pi -p` roundtrip. Real endpoint is our own `tools/omamobile-ai` `llama-server` (OpenAI `/v1`, on-demand + idle watchdog). Warmup: the server auto-starts on any `omamobile-ai chat` and idles off — start it with a long timeout before a pi session.

Model policy: **LFM3-1.7B-`Q4_K_M` is the sole default** (`/v1/models` lists just it; `/home/alarm/models/` empty until `tools/r8q-ai push-model`). Catalog (verified HF repos 2026-11-12): `LiquidAI/LFM3-1.7B-GGUF` (`Q4_K_M`/`QAD-Q4_0`, ~1.39 GB) - `Qwen/Qwen2.5-1.5B-Instruct-GGUF` (fallback) - `meta-llama/Llama-3.2-3B-Instruct-GGUF` (heavy) - `LiquidAI/LFM3-VL-2B-GGUF` + `mmproj` (vision/video) - `vikhyatk/moondream2` (light OCR) - `LiquidAI/LFM3-Omni-2B-GGUF` (`--reasoning off`) - `LiquidAI/LFM3-Audio-2B-GGUF` (ASR) - `ggerganov/whisper.cpp` + `hexgrad/Kokoro-82M` (modular STT/TTS) - `ggml-org/embeddinggemma-2-GGUF` (+`unsloth`, base `google/embeddinggemma-2`, ~350 MB RAG).

---

## 4. Rules, Checklist & Next Steps

- **Memory:** `zram-size = ram / 2`, 4 GiB `/swapfile` overflow, MGLRU `min_ttl_ms=1000`, `swappiness=150`, `page-cluster=0` (later: `MULTI_COMP` `lz4`+`zstd`).
- **Scheduling:** `performance` governor + UFS `clkscale/clkgate=0` until M6.2; pin GPU devfreq on M3.3 bring-up; restore `schedutil` + `UCLAMP_MIN` + `misc:vfr=true` + 120 Hz only after soak-verified fixes.
- **Packaging:** everything in `packages/omamobile-device-r8q/`, container-built (`--local r8q`); vendor firmware to `~/.cache/omamobile/firmware/r8q/` (never commit blobs); `a650_zap.mbn` under `NoUpgrade`.
- **Checklist (every commit):** `tools/pr-check`, `tests/packages`.
- **Next 4 (phone stability & usability first; AI at the end):** 1) **Boot isolation & kernel baseline findings (`2026-11-13`):** the last functional Arch boots in `journalctl` (`boots -7` and `-4`, up to ~30 min) ran `7.3.0-rc6-omamobile-push1-g534e72153193-dirty` (`fts5cu56a.ko` built-in, `aliases { serial0 = &uart6; }`, `i2c@a8c000` `okay`, `&dispcc` `protected-clocks = <0..57>`, `CONFIG_U_SERIAL_CONSOLE` unset). Reconstructed `wt-r8q` commits (`df28227c0`..`1dd099a7a`) omitted `&dispcc` `protected-clocks` and `serial0 = &uart6`, while enabling `CONFIG_U_SERIAL_CONSOLE=y` coincided with Arch `sysinit.target` stalling before `basic.target` and resetting at ~36 s (`RuntimeWatchdogSec=30s`; `omamobile.initramfs` stays stable on the same kernel, and `e2fsck -fn /dev/sda36` passed clean). A reverted `02604ea` config (`CONFIG_U_SERIAL_CONSOLE` unset, `MAX77705` `=m`) + `serial0 = &uart6` + `&dispcc protected-clocks` (`1827` modules) was packed (`recovery-omamobile.img` sha256 `aea42c0d2052d6550b508f493d96ac6f6d04f102498b46bdf4147027ca612c2e`) and flashed to `RECOVERY` (`sda24`). Next: boot Recovery into Arch, check `omamobile-boot-trace.service` / remove temporary `/etc/systemd/system/omamobile-boot-trace.service` + `00-live-flush.conf` on `sda36`, and update `/usr/lib/modules` to the `1827`-module build if needed. 2) **Enable persistent M6.1 battery (`=m`):** turn on `fuel-gauge@36` after early boot with `MAX77705`/`MAX17042` as modules and wire `max170xx_battery` into `ai-server`/shell. 3) **Hardware usability one per boot:** touchscreen (`994000.i2c` `gpi_dma` `EV ALLOCATE ret:-5` investigation + valid `stmfts`/panel touch config vs nonexistent `CONFIG_TOUCHSCREEN_FTS5CU56A`), Adreno 650 (`M3.3`) + 120 Hz AMOLED, BT, `CS35L41` audio, and SLPI sensors; optionally add a `qemu-system-aarch64 -M virt` harness to pre-test `Image`/`initramfs`/`switch_root`/`systemd` offline (cannot emulate `SM8250`/S-Boot silicon paths). 4) **Last:** fix ~11–30 min `RPMh`/RCU wedge via `rpmh_rsc_dump_state`, then verify `LFM3-1.7B-Q4_K_M.gguf` (`M8.4`/`M8.6`) under Hyprland.
- **Clean-room principle:** all userspace glue stays packaged in `omamobile-device-r8q` (MIT); kernel/DTB quirks are structured for the kernel tree (`GPL-2.0-only`). Everything here is implemented by us from our own hardware measurements; nothing is copied from anywhere.
