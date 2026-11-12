<!-- SPDX-License-Identifier: MIT -->

# Omarchy Mobile — Samsung Galaxy S20 FE (`r8q`) Port & Integration Plan

**Status: M0–M2, M4.1 & M5.1 (Wi-Fi) verified on hardware; `omamobile-device-r8q` 0.1.4 installed, 0.1.6 queued (0.1.5 built with the Rust ELF but never installed — superseded for a clean `pacman -U` upgrade path). Desktop runs stable on `simpledrm` + `llvmpipe` with UFS, suspend-block, watchdog, `cpuidle`, and `cpufreq` (`performance`) mitigations active. Wi-Fi associates; GPU/BT/audio/battery still staged.**
- **Kernel branch:** `omamobile-r8q` (worktree `~/omamobile-kernel/wt-r8q`, output `~/omamobile-kernel/out-r8q`)
- **Reference upstream:** `derekross/linux-omarchy-mobile` (`v7.3-rc6`), `sm8250-mainline` / postmarketOS `samsung-r8q` (`qcom/sm8250-samsung-r8q.dts`)

---

## 0. Current Status & Open Work (2026-11-12)

The phone boots mainline Linux `7.3.0-rc6` from the `recovery` partition (`sda24`) while leaving LineageOS untouched on `boot` (`sda23`) + `super`. USB NCM networking (`172.16.42.1`), SSH, USB serial console (`ttyGS0`), UFS storage, touchscreen (`stmfts`), Wi-Fi (`wlp1s0` associated), and Hyprland (`simpledrm` + `llvmpipe`) are live. Bluetooth/audio/battery bring-up is staged, not live.

### Fixed in Tree / Active on Hardware

| Problem | Root Cause | Fix & Deployment State |
| --- | --- | --- |
| Power button dead; no wake source | `CONFIG_POWER_RESET_QCOM_PON=m` with no modules in early image | `CONFIG_POWER_RESET_QCOM_PON=y` built-in (`device/r8q/kernel.config`) — **live** |
| Phone wedges minutes after boot on `s2idle` | `s2idle` entry freezes userspace and never resumes on Samsung TZ | `# CONFIG_SUSPEND is not set`, logind gate, masked sleep targets, `systemd.mask=` (`0.1.4`) — **live** |
| Wedges required physical VolUp+Power hold | `CONFIG_QCOM_WDT=m`, `CONFIG_SOFT_WATCHDOG` unset | `CONFIG_QCOM_WDT=y`, `CONFIG_SOFT_WATCHDOG=y`, systemd `RuntimeWatchdogSec=30s` (`0.1.4`) — **live** |
| Disk I/O wedge after ~150 ms idle (`sshd` banner hang, `journald` stops) | UFS `auto_hibern8` exit fails; unsupported write-booster query error on runtime suspend | `omamobile-r8q-ufs.service` writes `0` to `auto_hibern8` (`0.1.4`); kernel skips unsupported WB attribute (`drivers/ufs/core/ufshcd.c`) — **live** |
| Watchdog reboots every 3–22 min, one with RCU stall on a CPU parked in `cpuidle` | Deep `cpuidle` (`state1`) power-collapse hangs cores on Samsung TZ | `omamobile-r8q-cpuidle.service` disables deep `cpuidle` states (WFI only) (`0.1.4`) — **live** |
| **Crash ~11 min into soak (`ramoops`: `rpmh timeout` in `sugov`)** | **`cpufreq` (`schedutil`) OPP transitions time out on an AMC `RPMh` write** (`qcom_cpufreq_hw` → `icc_set_bw` → BCM voter → `rpmh_write_batch` `-110`), wedging a core until the watchdog resets the phone | **All three policies pinned to `performance` at runtime — live; persisted in `0.1.5` via a second `ExecStart` in the already-enabled `omamobile-r8q-cpuidle.service`** |
| Boot hung on `multi-user.target` | Hand-installed `omamobile-bluetooth.service` hung in `btmgmt config` | Unit disabled on phone; keep out of package until QCA6390 BT is verified — **live** |
| Broken `bt-agent` user unit restart-loops every ~2 s | BlueZ ships `bt-agent.service` but `/usr/bin/bt-agent` is not installed | Masked live (`systemctl --user mask`) 2026-11-12 — **live** |
| `systemd-zram-setup@zram0` hung at boot | `CONFIG_ZRAM=m` with no modules installed | `CONFIG_ZRAM=y` built-in + 4 GiB `/swapfile` — **live** |
| Blind hangs when network/UI stops | No USB serial getty | `console=ttyGS0` + `serial-getty@ttyGS0` (`0.1.4`) — **live** |
| Panel blanks after 10 min and never wakes | `fbcon` `consoleblank=600` ignores DRM pageflips | `omamobile-r8q-panel.timer` unblanks `/sys/class/graphics/fb0/blank` every 4 min (`0.1.4`); `consoleblank=0` queued for next flash — **live** |
| Hyprland renders with `llvmpipe` (slow) | `msm` blacklisted: on-phone `a650_sqe.fw` is v93, kernel needs ≥ 0x95 | Upstream `linux-firmware` `qcom/a650_sqe.fw` (31964 bytes, v0x112, verified against `a6xx_ucode_check_version`) staged on laptop; no `zap`/TZ errors in any journal — stage + `modprobe msm` next |
| Sluggish UI on `llvmpipe` | Animations/blur/shadows enabled before Adreno GPU is active | `hyprland-r8q.conf` disables animations/blur/shadows until Adreno `renderD128` is live (`0.1.4`) — **live** |
| Missing kernel modules for running `-dirty` kernel | Tarball modules mismatched `-dirty` `utsrelease` | Rebuilt `DEVICE=r8q bin/omamobile modules` and pushed 1,818 modules (`depmod` clean) — **live** |

### Open Blockers (Prioritized Order)

1. **Install `omamobile-device-r8q` `0.1.5` + 30-min soak:** persists the `performance` governor pin and the LFM3-1.7B-only AI server. Built in the Arch Linux ARM container (`tools/build-packages --local r8q`, local key `keys/r8q`, phone trusts it). Then a 30-minute idle soak: zero `ramoops` reboots confirms the RPMh hang is mitigated.
2. **Adreno 650 GPU bring-up (`msm` blacklisted; `adreno_smmu` `-110`):** *Firmware gate* is clear (v0x112 SQE on laptop, no zap/TZ errors ever logged). Stage `a650_sqe.fw` to `/lib/firmware`, `modprobe msm`, verify `/dev/dri/renderD128` + `vulkaninfo`. *Power-domain gate:* `adreno_smmu` fails (`GPU_CX_GDSC` never comes up) — fix `GPU_CX_GDSC`/`gpucc`/`rpmhpd` in `sm8250-samsung-r8q.dts`.
3. **Staged peripherals, one per boot with `ttyGS0` armed (BT/audio/battery — Wi-Fi already associates):**
   - **Wi-Fi (`ath11k_pci`, LIVE 2026-11-12):** `wlp1s0` associates with the AP (ramoops console evidence); modules `ath11k_pci`/`qrtr_mhi` load. Remaining: connection UX/persistence, keep `usb0` unmanaged in NetworkManager.
   - **Bluetooth (`btqca` / `hci0`):** `hci0` QCA UART setup completes (ramoops). Test `bluetoothctl` scan with a timeout guard before re-enabling any system unit.
   - **Audio (`snd-soc-sm8250` + dual `CS35L41`):** modules (`snd_soc_cs35l41_*`, `lpass` macros) load; no ALSA card yet. Verify `kona-mtp-snd-card` in `aplay -l` + UCM2 playback.
   - **Battery & Charging (`MAX77705` / `PCA9468`):** one `-EINVAL` on `MAX77705` probe; expose `/sys/class/power_supply/*` to `omamobile-shell`.
4. **Lock-screen touch input (`LockGlance.qml`):** desktop Hyprland takes touch on `/dev/input/event3` (`stmfts`); the glance swipe-up does not unlock (`wakeTouch` swallows touches while `lock.displayDark` is set, nothing clears it on `r8q`). Trace the touch seat/layer focus, wire the power-key wake binding, then re-enable `idle.lock`.
5. **Next `recovery` flash (`consoleblank=0`):** covered by the panel timer today; next human-approved `recovery` flash bakes it into the cmdline. Never flash `boot`/`super`/bootloader partitions.

> [!IMPORTANT]
> Operational rules ([AGENTS.md](../../AGENTS.md) & [CONTRIBUTING.md](../../CONTRIBUTING.md)):
> - Read-only probes first; **one hardware change per boot**.
> - Ask for human approval before every reboot, flash, or partition write.
> - Build all packages in the container (`tools/build-packages --local r8q`) — never run `gcc`/Rust compiles directly on the phone.
> - Service enablement goes through `/etc` (`systemctl enable`); vendor `/usr/lib/...wants` links alone did not register as enabled on this port.
> - Never commit or paste private hardware identifiers (serials, MAC addresses, IMEIs, or NV dumps).

---

## 1. Hardware & Device Summary (Captured Read-Only)

| Field | Value | Source |
| --- | --- | --- |
| **Device Name** | Samsung Galaxy S20 FE (Snapdragon) | `SM-G780G` (4G) / `SM-G781B` (5G) |
| **Codename** | `r8q` (`r8qxx`) | `ro.product.device`, `ro.product.name` |
| **SoC** | Qualcomm Snapdragon 865 (`SM8250` / `"kona"`) | `ro.soc.model=SM8250`, `ro.board.platform=kona` |
| **CPU** | Kryo 585 Octa-core (1× 2.84 GHz + 3× 2.42 GHz A77, 4× 1.80 GHz A55) | ARMv8.2-A (FP16, DotProd) |
| **GPU** | Qualcomm Adreno 650 | Mesa Turnip / Freedreno (`msm`), Samsung-signed `a650_zap.mbn` on phone |
| **RAM** | 6 GB LPDDR5 (`~5.5 GiB` usable) | `/proc/meminfo` (`5747672 kB`) |
| **Storage** | UFS (`1d84000.ufshc`), LUNs `sda`–`sdd` | `ro.boot.boot_devices=soc/1d84000.ufshc` |
| **Partition Scheme** | **Single-slot (Non-A/B)** + Dynamic `super` | `ro.build.ab_update=false` |
| **Bootloader** | Samsung S-Boot / ABL (`G780GXXSHEYJ1`, `SVB1.0`, `orange`) | `dtbo_idx=7`, `dtb_idx=0` |
| **Display Panel** | Samsung `S6E3FC3_AMS646YD01` (1080×2400 FHD+ AMOLED, 120 Hz, DSI cmd) | `lcd_id=0x805081` |
| **USB Controller** | Synopsys DWC3 (`a600000.dwc3`) | `androidboot.usbcontroller=a600000.dwc3` |
| **Wi-Fi / BT / NFC** | Qualcomm QCA6390 (PCIe + UART) + Samsung `s3fwrn5` NFC | `androidboot.hardware.sku=s3fwrn5` |
| **PMIC / Amps** | `pm8150` / `pm8150b` / `pm8150l` + `MAX77705` + dual `CS35L41` | Live DTB / `dmesg` |

### Partition Layout & Safety Guards (`UFS_RW=""`)

| Partition | Block Device | Role & Safety Classification |
| --- | --- | --- |
| `boot` | `/dev/block/sda23` | Active LineageOS kernel + initramfs (**untouched fallback**) |
| `recovery` | `/dev/block/sda24` | **Omarchy Mobile boot image (guarded `tools/r8q-boot` only)** |
| `dtbo` | `/dev/block/sda22` | Device tree overlay partition |
| `super` | `/dev/block/sda29` | Android dynamic partitions (`system`, `vendor`, `product`, `odm`) |
| `userdata` | `/dev/block/sda36` | Hosts `/data/omarchy` rootfs image alongside Android |
| `efs`, `sec_efs`, `persist` | `sda9`, `sda12`, `sda8` | **CRITICAL NV / calibration — READ-ONLY, BACKED UP IN M0** |
| `modemst1/2`, `fsg`, `fsc` | `sda1/2`, `sdd3`, `sda3` | **Modem EFS & golden backup — READ-ONLY, BACKED UP IN M0** |
| `modem`, `dsp` | `sda20`, `sda21` | Radio & DSP firmware (read-only firmware extraction) |
| `xbl*`, `abl`, `tz`, `aop`, `hyp`, `vbmeta*` | `sdb1/2`, `sdd5/7/8/25/35`, `sda27` | **Bootloader / TrustZone / Hypervisor — NEVER TOUCH** |

---

## 2. Architectural Comparison: Pixel (`gs201`) vs. S20 FE (`sm8250`)

| Subsystem | `cheetah` (Pixel 7 Pro / `gs201`) | `r8q` (Galaxy S20 FE / `sm8250`) |
| --- | --- | --- |
| **Slot Layout** | A/B (`boot_a` Android, `boot_b` Omarchy) | **Single-slot**: `boot` (`sda23`) Android, `recovery` (`sda24`) Omarchy |
| **Boot Image** | Android Boot Header v4 (`fastboot`) | **Android Boot Header v2** + `recovery_dtbo` + `sm8250-samsung-r8q.dtb` + AVB RSA4096 footer |
| **Display Engine** | Exynos DECON + DSIM | `simpledrm` (current) → Qualcomm **MDSS / DPU1** (`msm_drm`) + `S6E3FC3` DSI (target) |
| **GPU Driver** | Mali-G710 (`panthor`) | Adreno 650 (**Mesa Turnip / Freedreno** + Samsung-signed `a650_zap.mbn` + `a650_sqe.fw` ≥ 0x95) |
| **Power / Clocks** | Exynos ACPM | Qualcomm **RPMh** (`qcom-cpufreq-hw`, `rpmhpd`) — `performance` governor until the AMC-vote hang is fixed |
| **Audio / Sensors** | Google AoC | Qualcomm **QDSP6 / ADSP** (`snd-soc-sm8250` + `CS35L41`) & **SLPI** (`fastrpc`) |

---

## 3. Non-A/B Boot & Rootfs Architecture

1. **`device/r8q/device.conf` Guard:** `BOOT_METHOD=ram-only` + `UFS_RW=""` so generic Pixel scripts (`tools/install-phone`) refuse to overwrite `boot` or `super`; only guarded `tools/r8q-boot` touches `recovery`.
2. **Instant Android Fallback:** normal reboot lands on `sda23` (LineageOS); `adb reboot recovery` (or VolUp+Power) boots Omarchy on `sda24`.
3. **Coexistent Rootfs:** Arch Linux ARM `aarch64` lives on `userdata`; managed via `omamobile-device-r8q` (0.1.4 installed, 0.1.5 built) and `tools/r8q-stabilize`.

---

## 4. Complete Gated Milestone Plan (M0 – M8)

### M0: Read-Only Capture & NV Backup
- [x] **M0.1 — Full NV & Recovery Backup (2026-11-11):** all 11 critical partitions backed up with double-read `SHA256SUMS` (off-repo `~/android/backups/r8q-nv/`).
- [x] **M0.2 — Stock Boot Header & Live FDT (2026-11-11):** Header v2 offsets + live FDT (`qcom,msm-id = <0x0164 0x020001>`, `qcom,board-id = <0x08 0x06>`) saved to `~/android/reference/r8q/live.dtb`.

### M1: Kernel & Device Tree Build
- [x] **M1.1 — Mainline Kernel & `r8q` DTB (2026-11-11):** `7.3.0-rc6-omamobile-push1` + `sm8250-samsung-r8q.dtb` + 1,818 modules in the `omamobile-build:24.04` Docker container (`CONFIG_POWER_RESET_QCOM_PON=y`, `CONFIG_QCOM_WDT=y`, `CONFIG_SOFT_WATCHDOG=y`, `CONFIG_ZRAM=y`, `# CONFIG_SUSPEND is not set`, UFS write-booster fix).

### M2: Storage Guard, Recovery Boot & Stability Mitigations
- [x] **M2.1 — `tools/r8q-boot` & Header-v2 AVB Image (2026-11-11):** uncompressed `Image` + storage guard + `recovery_dtbo` + DTB with RSA4096 AVB footer (`recovery-omamobile.img`, `sda24`).
- [x] **M2.2 — Bare-Metal First Boot & USB Networking (2026-11-11):** UFS LUNs, DWC3 `cdc_ncm` gadget (`172.16.42.1`), `ttyGS0` serial console, `sshd`. ([SUPERSEDED BRING-UP SCAFFOLD] Early bring-up used an Alpine/dropbear prototype before the persistent Arch rootfs in M7.)
- [x] **M2.3 — Stability Mitigations (`omamobile-device-r8q` 0.1.4, 2026-11-12):** container-built, installed on phone. UFS `auto_hibern8=0`, 30 s watchdog, sleep targets masked, deep `cpuidle` disabled, panel timer, serial getty, `hyprland-r8q.conf` performance baseline.
- [ ] **M2.4 — Persist `cpufreq` `performance` Governor (`omamobile-device-r8q` 0.1.6):** `cpufreq-no-dvfs` as second `ExecStart` in the already-enabled `omamobile-r8q-cpuidle.service`. Rust-only LFM3-1.7B AI server included (aarch64 ELF verified). Install + 30-min soak next. Root-causes the `ramoops` RPMh-timeout reboots.

### M3: Display & Adreno 650 GPU Acceleration
- [x] **M3.1 — `simpledrm` Framebuffer Baseline (2026-11-11):** bootloader-initialized DSI scanout claimed as continuous splash on `card0` / `/dev/fb0` (1080×2400 `a8r8g8b8`); stable Hyprland desktop on `llvmpipe`. No native `msm_drm`/DSI driver — that is M3.3.
- [x] **M3.2 — Extract GPU & Subsystem Microcode (2026-11-11):** `a650_gmu.bin`, Samsung-signed `a650_zap.mbn`, `adsp/cdsp/slpi.mbn`, `cs35l40`/`cs35l41`, `qca6390` blobs via `tools/r8q-extract-firmware`; upstream `linux-firmware` `qcom/a650_sqe.fw` v0x112 on laptop (satisfies the ≥ 0x95 gate per `a6xx_ucode_check_version`).
- [ ] **M3.3 — Native `msm_drm` DSI Panel (`S6E3FC3` 120 Hz) & Adreno 650 (`renderD128`):**
  *Gate:* stage v0x112 `a650_sqe.fw` to `/lib/firmware`, fix `adreno_smmu` `-110` (`GPU_CX_GDSC` power domain) + native MDSS/DSI `S6E3FC3` nodes in `sm8250-samsung-r8q.dts`.
  *DTS & rules staged:* `&gmu`, `&gpu`, `&gpu_zap_shader` (`qcom/a650_zap.mbn`) and `80-r8q-gpu.rules` (`simple_ondemand` devfreq, 150–587 MHz) — inactive while `msm` is blacklisted.
  *Pass criteria:* unblacklist `msm`, `/dev/dri/renderD128` initializes, `vulkaninfo`/`glmark2-es2-wayland` run on Adreno 650, 120 Hz Hyprland animations re-enabled.

### M4: Touchscreen, Buttons & Sensors
- [x] **M4.1 — Touchscreen (`st,stmfts5`) & Hardware Keys (2026-11-11):** `CONFIG_TOUCHSCREEN_STMFTS=y` + `CONFIG_I2C_QCOM_GENI=y`; multitouch on `/dev/input/event3`, `gpio_keys` on `event2`, `qpnp_pon` power key on `event0`.
- [ ] **M4.2 — Lock-Screen Touch Input (`LockGlance.qml`):** desktop touch works; the glance swipe-up does not unlock. Trace the touch seat/layer focus, wire the power-key wake binding, then re-enable `idle.lock`.
- [ ] **M4.3 — SLPI Sensors (`fastrpc` + `iio-sensor-proxy`):** two `/sys/bus/iio` devices exist but no `/sys/class/remoteproc` and no verified readings. Bring up SLPI with staged `slpi.mbn`, verify accel/gyro/light via `monitor-sensor`.

### M5: Wi-Fi, Bluetooth & Audio
- [x] **M5.1 — Wi-Fi (`QCA6390` / `ath11k_pci`) LIVE (2026-11-12):** `wlp1s0` associates with the AP (ramoops console evidence); `hci0` QCA UART setup completes. Remaining: connection UX/persistence with `usb0` kept unmanaged.
- [ ] **M5.2 — Bluetooth (`QCA6390` / `btqca`):** test `bluetoothctl` scan with a strict timeout guard before enabling any system unit; broken `bt-agent` user unit stays masked until `/usr/bin/bt-agent` exists.
- [ ] **M5.3 — Audio (`snd-soc-sm8250` + Dual `CS35L41`):** LPASS/CS35L41 modules load, both amps probe, no ALSA card yet. Verify `kona-mtp-snd-card` in `aplay -l` + UCM2 (`HiFi.conf`) stereo playback via PipeWire.

### M6: Battery, Charging & Thermal Management
- [ ] **M6.1 — Fuel Gauge & Charger (`MAX77705` / `PCA9468`):** fix the single `-EINVAL` on `MAX77705` probe; expose `/sys/class/power_supply/*` (voltage, current, temp, SoC) in `omamobile-shell`.
- [ ] **M6.2 — RPMh OPP & `cpuidle` Power Optimization:** debug the AMC `RPMh` timeout on DVFS transitions and the Samsung-TZ `cpuidle state1` hang. *Pass criteria:* `schedutil` + safe idle states re-enabled with zero `ramoops` panics and screen-off draw well below 1.4 W.

### M7: Rootfs & Upstream `omarchy-mobile` Device Integration
- [x] **M7.1 — Arch Linux ARM Rootfs & `omamobile-device-r8q` (2026-11-11 – 2026-11-12):** coexistent rootfs on `userdata`; package container-built (`tools/build-packages --local r8q`, local key `keys/r8q`, phone trusts it); 0.1.4 installed, 0.1.5 built.
- [ ] **M7.2 — `omamobile-shell` & Native Settings Integration:** Quickshell mobile bar, quick settings, app drawer, OSK run today (`Hyprland` + `quickshell -p /usr/share/omarchy/shell` under `alarm`); wire `1080x2400` scale-2 + `omamobile-settings` device panel for `r8q`. ([SUPERSEDED BRING-UP SCAFFOLD] The early `cage`/`cog`/`darkhttpd` :3000 prototype validated touch; production is Hyprland + `omamobile-shell`.)

### M8: On-Device Local AI (LFM3-1.7B Default) & Performance Tuning
- [x] **M8.1 — RAM Compression (`zram`) & MGLRU Tuning (2026-11-11):** `CONFIG_ZRAM=y` + 4 GiB `/swapfile` + `min_ttl_ms=1000` + `swappiness=150`, `page-cluster=0`.
- [ ] **M8.2 — 120 Hz Fluid Motion & Adreno 650 DVFS Profile [STAGED, BLOCKED ON M3.3]:** `80-r8q-gpu.rules` (150–587 MHz `simple_ondemand`) + `mobileSpring`/`mobileQuick` curves staged in the package, inactive on `simpledrm` + `llvmpipe` (animations/blur disabled in `0.1.4`). *Pass criteria:* re-enable 120 Hz Hyprland animations + GPU devfreq once M3.3 brings up `renderD128` and native `1080x2400@120` DSI.
- [x] **M8.3 — Baseline CPU Inference Benchmark (2026-11-11):** LFM3-1.7B `Q4_K_M` at ~15.7 tok/s on Kryo 585 CPU cores (`tools/r8q-ai`).
- [ ] **M8.4 — Rust-Only Edge Subagent Daemon:** `omamobile-ai-server` keeps **only the Rust implementation** (`rust/ai-server`, LFM3-1.7B default in `/v1/models`); the Python prototype is removed. The container image installs `rust` (`tools/lib/alarm-setup`), so the aarch64 binary builds natively in-container; the PKGBUILD fails loudly without `cargo` and the phone never compiles.
- [ ] **M8.5 — Adreno 650 Vulkan (`llama.cpp` `GGML_VULKAN`) Acceleration:** after M3.3 (`renderD128` + Turnip) is live, benchmark LFM3-1.7B prompt-processing speedup on Adreno 650 Vulkan.

Model policy: **LFM3-1.7B-`Q4_K_M` is the only default model** (`/v1/models` lists just it). No GGUFs are preinstalled (`/home/alarm/models/` empty 2026-11-12); extras arrive via `tools/r8q-ai push-model` and then appear in the list.

### Model Catalog by Modality (Hugging Face Repos Verified 2026-11-12)

| Category | Model Name | Hugging Face Repository | Quant / Size | RAM (MB) | Role on S20 FE (`r8q`) |
| --- | --- | --- | --- | --- | --- |
| **Text (Agent Core)** | **`LFM3-1.7B`** | `LiquidAI/LFM3-1.7B-GGUF` | `Q4_K_M` (~1.10 GB) / `QAD-Q4_0` | **~1,390 MB** | **Default agent: MCP tool calls, 256K context (~15.7 tok/s on `r8q`)** |
| **Text (Alternative)** | **`Qwen2.5-1.5B-Instruct`** | `Qwen/Qwen2.5-1.5B-Instruct-GGUF` | `Q4_K_M` (~1.05 GB) | ~1,150 MB | Multilingual fallback (via `push-model` only) |
| **Text (Heavy)** | **`Llama-3.2-3B-Instruct`** | `meta-llama/Llama-3.2-3B-Instruct-GGUF` | `Q4_K_M` (~2.02 GB) | ~2,200 MB | Offline synthesis (uses `zram` headroom) |
| **Vision & Video** | **`LFM3-VL-2B`** | `LiquidAI/LFM3-VL-2B-GGUF` | `Q4_K_M` + `mmproj` (~1.35 GB) | ~1,550 MB | Image + multi-frame video understanding, OCR (`llama.cpp` ≥ `v0.8.0`) |
| **Vision (Light)** | **`Moondream2`** | `vikhyatk/moondream2` | `Q4_K_M` + `mmproj` (~1.28 GB) | ~1,400 MB | Lightweight OCR / document transcription |
| **Omni (Audio+Vision)** | **`LFM3-Omni-2B`** | `LiquidAI/LFM3-Omni-2B-GGUF` | `Q4_K_M` (~1.48 GB) | ~1,650 MB | Text/image/video/audio in, text/speech out (run with `--reasoning off`) |
| **Audio (Speech)** | **`LFM3-Audio-2B`** | `LiquidAI/LFM3-Audio-2B-GGUF` | `Q4_K_M` (~1.40 GB) | ~1,550 MB | End-to-end local ASR and speech-to-speech |
| **Audio (Modular)** | **`Whisper Tiny` + `Kokoro-82M`** | `ggerganov/whisper.cpp` + `hexgrad/Kokoro-82M` | `FP16` + `ONNX` (~164 MB) | ~230 MB | Ultra-low-footprint modular STT + TTS |
| **Embeddings** | **`EmbeddingGemma 2`** | `ggml-org/embeddinggemma-2-GGUF` (also `unsloth/embeddinggemma-2-GGUF`; base weights `google/embeddinggemma-2`) | `Q8_0` (~296 MB) | ~350 MB | Local RAG over notes, docs, MCP schemas |

---

## 5. Deep Optimization Guide: RAM Compression & Frame Pacing on `r8q`

### 5.1 Maximal RAM & Compression Architecture (6 GB LPDDR5)

1. **`zram-size = ram / 2` + UFS `/swapfile` overflow:** `zram` capped at `ram / 2` (never 100%: over-commit stalls starve DMA for `ath11k_pci`/`msm`). 4 GiB `/swapfile` at lower priority behind `zram0`.
2. **Compression strategy (`zstd`):** `zstd` ~2.8–3.3:1 on anonymous pages, negligible latency on A77 cores. Optional later: `CONFIG_ZRAM_MULTI_COMP=y` (`lz4` hot + `zstd` recompress) and writeback for ice-cold LLM pages.
3. **MGLRU + VM sysctls (`90-r8q-vm.conf` / `80-r8q-mglru.conf`):** `min_ttl_ms=1000` shields Wayland surfaces + touch handlers; `swappiness=150`, `page-cluster=0`.

### 5.2 Display & CPU Scheduling Pipeline (Current vs. Post-M3.3)

* **Current baseline (`simpledrm` + `llvmpipe` + RPMh workaround):** `hyprland-r8q.conf` without blur/shadows/animations (`0.1.4`); all `cpufreq` policies pinned to `performance` (`0.1.5` `cpufreq-no-dvfs`) so `RPMh` DVFS transitions cannot wedge the phone.
* **CPU governor rule (current pin vs. post-fix target):** TODAY `performance` is mandatory — `schedutil` (`sugov:0`) DVFS transitions (`qcom_cpufreq_hw_target_index` → `icc_set_bw` → BCM voter → `Error sending AMC RPMH requests (-110)`) stall an RCU core and trip the 30 s watchdog within 3–12 min. `schedutil` returns only after the kernel maintainer lands an RPMh AMC-vote quirk/retry for `r8q` (M6.2), with `Hyprland`/`pipewire` under `UCLAMP_MIN` (~15–20%) so touch frames preempt background inference threads.
* **Post-M3.3/M6.2 target:** native `1080x2400@120` command mode with TE sync + `misc:vfr=true`; `mobileSpring`/`mobileQuick` curves; `simple_ondemand` GPU devfreq (150–587 MHz); `schedutil` + `UCLAMP_MIN` re-enabled only after zero `rpmh` timeouts under load.

---

## 6. Upstream `omarchy-mobile` Integration & Device Conventions

* **Package structure (`packages/omamobile-device-r8q/`):** every hardware workaround ships versioned via `pacman`, built in the `aarch64` container (`tools/build-packages --local r8q`), never hand-edited on the phone or compiled on it:
  * `omamobile-r8q-ufs.service` (`auto_hibern8=0`), `omamobile-r8q-cpuidle.service` (deep-idle disable + `performance` pin), `omamobile-r8q-panel.timer` (`fb0` unblank guard)
  * `20-usb-gadget.network` (`172.16.42.1`) + NetworkManager `usb0`-unmanaged rule, `sshd` config, serial getty
  * ALSA UCM2 (`ucm2/Samsung/r8q/HiFi.conf`, `kona-mtp-snd-card.conf`)
* **Firmware extraction (`tools/r8q-extract-firmware`):** never commit Qualcomm/Samsung blobs (`a650_zap.mbn`, `slpi/adsp/cdsp.mbn`, `qca6390`); extract locally to `~/.cache/omamobile/firmware/r8q/`; protect `a650_zap.mbn` via `NoUpgrade` in `pacman.conf`.
* **Device descriptor (`device/r8q/device.conf`):** keep `BOOT_METHOD=ram-only`, `UFS_RW=""` so Pixel-oriented scripts and test suites (`tests/boot-method`, `tests/device-overlays`) enforce Samsung non-A/B safety.
* **AI server (Rust-only):** `rust/ai-server` is the single implementation; the Python prototype is deleted and the PKGBUILD fails loudly without `cargo`. The container image installs `rust` via `tools/lib/alarm-setup` (part of the image hash), so the aarch64 binary builds natively in-container.

---

## 7. Pre-Push Checklist (Every Commit)

```sh
tools/pr-check
tests/packages
```

---

## 8. Recommended Next 4 Steps (Strict One-Change-Per-Boot Order)

1. **Install `omamobile-device-r8q` 0.1.6 + 30-min soak (isolates the RPMh fix):** install, confirm all three policies stay `performance` after reboot, verify `/v1/models` lists only LFM3-1.7B (Rust ELF), then idle-soak 30 min with zero `ramoops` reboots. (One change this boot — no GPU/peripheral probes until the soak passes.)
2. **Adreno 650 (`M3.3`):** stage v0x112 `a650_sqe.fw`, `modprobe msm`, verify `renderD128` + `vulkaninfo` (serial console armed).
3. **Peripherals (`M5.2/M5.3/M6.1`), one per boot:** `bluetoothctl` scan → `kona-mtp-snd-card` playback → `MAX77705` power-supply telemetry.
4. **Lock touch + native 120 Hz DSI (`M4.2/M3.3`):** fix `LockGlance.qml` wake/touch, transition off `simpledrm`, flash `consoleblank=0` recovery image once (human-approved).
