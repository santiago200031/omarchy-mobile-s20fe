<!-- SPDX-License-Identifier: MIT -->

# Omarchy Mobile — Samsung Galaxy S20 FE (`r8q`) Port & Integration Plan

**Status: pre-M0 (captured baseline facts, guarded descriptor ready).**
- **Kernel branch (planned):** `omamobile-r8q` (worktree `~/omamobile-kernel/wt-r8q`, output `~/omamobile-kernel/out-r8q`)
- **Reference upstream:** `sm8250-mainline` / postmarketOS `samsung-r8q` (`qcom/sm8250-samsung-r8q.dts`)

> [!IMPORTANT]
> Every step below follows [AGENTS.md](../../AGENTS.md) and [CONTRIBUTING.md](../../CONTRIBUTING.md).
> - Read-only probes first; one change per boot.
> - Ask the human before every reboot, flash, or partition write.
> - Never commit or paste private hardware identifiers (`<SERIAL>`, MAC addresses, IMEIs, or NV dumps).
> - Keep it simple: reuse existing upstream `sm8250` drivers and device trees rather than reinventing custom glue.

---

## 1. Hardware & Device Summary (Captured Read-Only via ADB)

| Field | Value | Source |
| --- | --- | --- |
| **Device Name** | Samsung Galaxy S20 FE (Snapdragon) | `ro.product.model` (`SM-G780G` 4G / `SM-G781B` 5G) |
| **Codename** | `r8q` (`r8qxx`) | `ro.product.device`, `ro.product.name` |
| **SoC** | Qualcomm Snapdragon 865 (`SM8250` / `"kona"`) | `ro.soc.model=SM8250`, `ro.board.platform=kona` |
| **CPU** | Kryo 585 Octa-core (1x 2.84 GHz + 3x 2.42 GHz Cortex-A77, 4x 1.80 GHz Cortex-A55) | ARMv8.2-A (FP16, DotProd) |
| **GPU** | Qualcomm Adreno 650 | `ro.hardware.vulkan=adreno` (Mesa Turnip / Freedreno) |
| **RAM** | 6 GB LPDDR5 (`~5.5 GiB` usable by Linux kernel) | `/proc/meminfo` (`MemTotal: 5747672 kB`), `dram_info=6G` |
| **Storage** | UFS (`1d84000.ufshc`), LUNs `sda`, `sdb`, `sdc`, `sdd` | `ro.boot.boot_devices=soc/1d84000.ufshc` |
| **Partition Scheme** | **Single-slot (Non-A/B)** + Dynamic `super` | `ro.build.ab_update=false` |
| **Bootloader** | Samsung S-Boot / ABL (`G780GXXSHEYJ1`, `SVB1.0`, `orange`) | `/proc/cmdline` (`dtbo_idx=7`, `dtb_idx=0`) |
| **Running Baseline** | LineageOS (Android 16), kernel `4.19.325-cip136-st20-perf` | `uname -r` |
| **Display Panel** | Samsung `S6E3FC3_AMS646YD01` (1080x2400 FHD+ Super AMOLED, 120 Hz, DSI cmd mode) | `/proc/cmdline` (`lcd_id=0x805081`) |
| **USB Controller** | Synopsys DWC3 (`a600000.dwc3`) | `androidboot.usbcontroller=a600000.dwc3` |
| **Modem / Baseband** | Qualcomm `mdm` (PCIe + QRTR/MHI) | `androidboot.baseband=mdm` |
| **Wi-Fi / BT / NFC** | Qualcomm/Broadcom combo + Samsung `s3fwrn5` NFC | `androidboot.hardware.sku=s3fwrn5` |

### Partition Layout (Single-Slot / Non-A/B on UFS)

Unlike Google Pixels, Samsung `r8q` does **not** have `_a` / `_b` slots (`ro.build.ab_update=false`).

| Partition | Block Device | Purpose / Safety Classification |
| --- | --- | --- |
| `boot` | `/dev/block/sda23` | Active kernel + initramfs (LineageOS resides here) |
| `recovery` | `/dev/block/sda24` | Recovery boot image (candidate for non-destructive test boot) |
| `dtbo` | `/dev/block/sda22` | Device tree overlay partition |
| `super` | `/dev/block/sda29` | Dynamic partitions (`system`, `vendor`, `product`, `odm`) |
| `userdata` | `/dev/block/sda36` | User data / target for Omarchy rootfs image or loop |
| `efs`, `sec_efs` | `sda9`, `sda12` | **CRITICAL NV (IMEI, MAC, calibration) — READ-ONLY, BACK UP FIRST** |
| `persist` | `/dev/block/sda8` | **Sensor / calibration persist — READ-ONLY, BACK UP FIRST** |
| `modemst1`, `modemst2` | `sda1`, `sda2` | **Modem EFS runtime state — READ-ONLY, BACK UP FIRST** |
| `fsg`, `fsc` | `sdd3`, `sda3` | **Modem golden backup / cookie — READ-ONLY, BACK UP FIRST** |
| `modem`, `dsp` | `sda20`, `sda21` | Radio & DSP firmware (mount read-only for firmware extraction) |
| `param`, `steady` | `sda10`, `sda17` | Bootloader parameters & boot splash |
| `xbl`, `xbl_config` | `sdb1`, `sdb2` | Primary bootloader stage — **NEVER TOUCH** |
| `abl`, `tz`, `aop`, `hyp` | `sdd8`, `sdd5`, `sdd7`, `sdd35` | Bootloader / TrustZone / hypervisor — **NEVER TOUCH** |
| `vbmeta`, `vbmeta_samsung` | `sdd25`, `sda27` | Verified boot metadata — **NEVER TOUCH** |

### Sanitized Boot Command Line Reference

```text
cgroup_disable=pressure rcupdate.rcu_expedited=1 rcu_nocbs=0-7 kpti=off
androidboot.hardware=qcom androidboot.usbcontroller=a600000.dwc3 console=null
firmware_class.path=/vendor/firmware_mnt/image swiotlb=2048
androidboot.verifiedbootstate=orange androidboot.bootdevice=1d84000.ufshc
androidboot.serialno=<SERIAL> androidboot.baseband=mdm
msm_drm.dsi_display0=ss_dsi_panel_S6E3FC3_AMS646YD01_FHD: lcd_id=0x805081
androidboot.dtbo_idx=7 androidboot.dtb_idx=0 androidboot.revision=6
androidboot.ap_serial=<HEX> androidboot.em.did=<HEX>
androidboot.em.model=SM-G780G androidboot.bootloader=G780GXXSHEYJ1
androidboot.dram_info=01,07,00,6G androidboot.hardware.sku=s3fwrn5
```

---

## 2. Key Architectural Differences: `gs201` (Pixel) vs `sm8250` (`r8q`)

| Subsystem | `cheetah` (Pixel 7 Pro / `gs201`) | `r8q` (Galaxy S20 FE / `sm8250`) |
| --- | --- | --- |
| **SoC Family** | Google Tensor G2 (`samsung,gs201`) | Qualcomm Snapdragon 865 (`qcom,sm8250`) |
| **Slot Layout** | A/B (`boot_a` stock, `boot_b` Omarchy) | **Single-slot (Non-A/B)**: `boot` + `recovery` |
| **Boot Protocol** | Fastboot (`BOOT_METHOD=android-v4`) | Samsung S-Boot / Odin / Recovery / `kexec` (`BOOT_METHOD=ram-only` guard) |
| **Display Engine** | Exynos DECON + DSIM (`gs-decon`) | Qualcomm **MDSS / DPU1** (`msm_drm`) + DSI (`samsung,s6e3fc3`) |
| **GPU Driver** | Mali-G710 (`panthor`) | Adreno 650 (**Mesa `freedreno` / `turnip`**) |
| **PMIC / Power** | Exynos ACPM mailbox + S2MPG12 | Qualcomm **SPMI** (`pm8150`, `pm8150b`, `pm8150l`) + RPMh |
| **Audio / DSP** | Google AoC | Qualcomm **QDSP6 / ADSP** (`q6afe`, `q6asm`, `snd-soc-sm8250`) |
| **Sensors** | AoC USF | Qualcomm **SLPI** via `fastrpc` + `iio-sensor-proxy` / `ssc` |

### Why Keep It Simple: Upstream Reuse
We do **not** need to write custom display, GPU, clock, or pinctrl drivers from scratch:
1. **`sm8250` is mature in mainline Linux**: RPMh, GCC/DISPCC/GPUCC clocks, Adreno 650 (`a650`), UFS, DWC3 USB, and DPU1 display are already upstream.
2. **`sm8250-samsung-r8q.dts` already exists** in the `sm8250-mainline` / postmarketOS community trees with working display (`S6E3FC3`), touchscreen, Wi-Fi, Bluetooth, and GPU.
3. Our job is strictly **integrating that DTB + config fragment into `linux-omarchy-mobile`** and adapting the initramfs storage guard for a single-slot Samsung layout.

---

## 3. Non-A/B Safety & Boot Strategy

Because `r8q` has no Slot B:
1. **`device/r8q/device.conf` sets `BOOT_METHOD=ram-only` and `UFS_RW=""`**. This guarantees that `bin/omamobile flash`, `push`, and `tools/install-phone` refuse to overwrite partitions on the phone.
2. **Keep LineageOS intact on `boot` + `super`** during bring-up.
3. **Two safe ways to boot test kernels without losing LineageOS:**
   - **Option A (`recovery` partition slot):** Back up `/dev/block/by-name/recovery` (`sda24`), flash the test mainline `boot.img` to `recovery` only, and boot into it via `adb reboot recovery` (or Vol Up + Power). Simple reboot returns straight to LineageOS on `boot` (`sda23`)!
   - **Option B (`kexec` from running LineageOS):** Load mainline kernel + initramfs + DTB into RAM via `kexec`. (If S-Boot/TZ blocks `kexec`, use Option A).
4. **Rootfs storage:** Store the Omarchy rootfs as an image file inside `/data/omarchy-root.img` on `userdata` (`sda36`) during bring-up so LineageOS and Omarchy can coexist without wiping the phone.

---

## 4. Complete Gated TODO & Integration Plan (M0 – M8)

Each milestone is gated. Complete them in order, one change per boot, recording the date and `uname -r` on completion.

### M0: Read-Only Capture & NV Backup (Pre-Bringup Gate)
- [x] Verify ADB connection and capture sanitized SoC, model, cmdline, and `/dev/block/by-name` map.
- [x] Create guarded `device/r8q/device.conf` (`BOOT_METHOD=ram-only`).
- [x] **M0.1 — Full NV & Recovery Backup (Read-Only):**
  - **Completed (2026-11-11):** Backed up `efs` (`sda9`), `sec_efs` (`sda12`), `persist` (`sda8`), `modemst1` (`sda1`), `modemst2` (`sda2`), `fsg` (`sdd3`), `fsc` (`sda3`), `param` (`sda10`), `boot` (`sda23`), `recovery` (`sda24`), and `dtbo` (`sda22`) into `~/android/backups/r8q-nv/` (mode `0700`, files `0600`).
  - **Verification:** All 11 partitions verified via double-read SHA256 on the phone before and after transfer; `SHA256SUMS` recorded off-repo.
- [x] **M0.2 — Extract Stock/LineageOS Boot Header & Firmware:**
  - **Completed (2026-11-11):** Analyzed backed-up `boot.img` and live device tree via root ADB.
  - **Boot Header v2:** `page_size=4096`, `base=0x00000000`, `kernel_offset=0x00008000`, `ramdisk_offset=0x01000000`, `tags_offset=0x00000100`.
  - **Live FDT Model:** `Samsung R8Q EUR PROJECT - R8 PV2 (board-id,06)`.
  - **Compatible:** `qcom,kona-mtp`, `qcom,kona`, `qcom,mtp`.
  - **Qualcomm IDs:** `qcom,msm-id = <0x0164 0x020001>` (SM8250 rev 2.1), `qcom,board-id = <0x08 0x06>` (MTP rev 6). Live FDT saved to `~/android/reference/r8q/live.dtb`.

### M1: Kernel & Device Tree Build (Offline on Laptop)
- [x] **M1.1 — Kernel & SM8250 Device Tree Build (`linux-omarchy-mobile`):**
  - **Status:** Complete (2026-11-11).
  - **Kernel Release:** `7.3.0-rc6-omamobile-push1-gfc7086a68fca`
  - **Toolchain:** Android Clang 22.0.2 (r596125) + lld in `omamobile-build:24.04` Docker container.
  - **Artifacts:**
    - `arch/arm64/boot/Image`: 43,051,520 bytes
    - `arch/arm64/boot/Image.gz`: 15,534,511 bytes
    - `arch/arm64/boot/dts/qcom/sm8250-samsung-r8q.dtb`: 112,499 bytes (verified with `qcom,kona-mtp`, `qcom,kona`, `qcom,board-id = <0x08 0x06>`)
    - `modules-7.3.0-rc6-omamobile-push1-gfc7086a68fca.tar.zst`: 17 MiB (1,827 stripped modules)
  - **Pass:** Build cleanly completed in Docker container without errors. Modules cleanly stripped and archived.

### M2: Initramfs Storage Guard & Non-Destructive First Boot
- [x] **M2.1 — Adapt Initramfs Storage Guard for `r8q` & Pack Header-v2 Image:**
  - **Status:** Complete (2026-11-11).
  - **Tool:** Created `tools/r8q-boot` supporting `pack`, `verify`, `flash-recovery`, and `restore-recovery`.
  - **Storage Guard:** Enforces 100% read-only lock (`/etc/omamobile-ufs-rw` empty), USB network gadget (`172.16.42.1`), ACM console, Dropbear SSH keys, and panic/watchdog parameters.
  - **Boot Header v2 & DTBO:** Header configured with `os_version = 16.0.0` (0x200001aa), `recovery_dtbo` (5,062,656 bytes embedded at offset 0x0143f000) extracted from device backup, and DTB at 0x01f00000.
  - **AVB Footer:** Signed with `avbtool add_hash_footer` using `SHA256_RSA4096`, key `testkey_rsa4096.pem` matching device `vbmeta` public key hash `2597c218aae470a130f61162feaae70afd97f011`, rollback index `1790812800`, salt `c87c8f874194bd7fde350e71426ac253760df3146460905df3fc630afc2a0322`, and prop `com.android.build.recovery.fingerprint`.
  - **Artifact:** `$KERNEL_OUT/recovery-omamobile.img` (82,694,144 bytes, verified `ANDROID!` header v2 magic, page size 4096, kernel `Image.gz` + Alpine rootfs + dropbear + `recovery_dtbo` + `sm8250-samsung-r8q.dtb` + AVB footer, sha256 `acd51fe0e9baa7d707ee8b871d4d21c7d7c17c60be90cdc3466e4a9489e4ca9a`).
- [x] **M2.1b — Create Device Support Package (`packages/omamobile-device-r8q`):**
  - **Status:** Complete (2026-11-11).
  - **Package:** `packages/omamobile-device-r8q` provides freestanding `reboot-to` syscall helper, `reboot` wrapper for download/recovery, `20-usb-gadget.network` (`172.16.42.1`), `sshd` privilege separation, and pacman mirrorlist.
  - **Verification:** `packages/omamobile-device-r8q/test/all` passed 100%; `tools/pr-check` verified clean.
- [ ] **M2.2 — First Boot via `recovery` Slot (`sda24`):**
  - **Pre-gate:** Ask human for approval before writing `sda24`. Verify `boot` (`sda23`) is untouched.
  - **Commands:** Pack test `recovery-omamobile.img` (with `/chosen/bootargs` including `console=tty0 panic=10`), write to `/dev/block/by-name/recovery` (`sda24`), verify sha256, and run `adb reboot recovery`.
  - **Expect/Pass:** Laptop detects USB network gadget (`172.16.42.1`), `tools/phone-ssh --fresh uname -r` returns the mainline kernel version, and all UFS partitions report `ro=1`.
  - **Stop condition:** No USB enumeration within 90 s (phone auto-reboots back to LineageOS on `sda23`).
  - **Rollback:** Reboot normally into LineageOS (`sda23`), then `dd` backed-up `recovery.img` back to `sda24`.
  - **Risk:** Low (primary `boot` slot and `super` are never touched).

### M3: Display & GPU (Hyprland First Light)
- [ ] **M3.1 — Framebuffer & KMS (`msm_drm` + `s6e3fc3`):**
  - **Expect/Pass:** `/sys/class/drm/card0-DSI-1/status` reports `connected`; 1080x2400@120Hz mode active; backlight control works via `/sys/class/backlight/`.
- [ ] **M3.2 — Adreno 650 GPU (`msm` + `a650_zap.mbn` + Mesa Turnip/Freedreno):**
  - **Expect/Pass:** `/dev/dri/renderD128` initializes with `a650_zap.mbn`; `glmark2-es2-wayland` and `vulkaninfo` identify Adreno 650; Hyprland and `omamobile-shell` launch at 120 Hz.

### M4: Touchscreen, Buttons & Sensors
- [ ] **M4.1 — Touchscreen (`sec_ts` I2C) & GPIO Keys:**
  - **Expect/Pass:** `libinput debug-events` reports multitouch coordinates, Volume Up/Down (`gpio-keys`), and Power key (`qcom-pon`).
- [ ] **M4.2 — Sensors (SLPI / `fastrpc`):**
  - **Expect/Pass:** Accelerometer, gyroscope, and ambient light report via `iio-sensor-proxy`.

### M5: Wi-Fi, Bluetooth & Audio
- [ ] **M5.1 — Wi-Fi & Bluetooth:**
  - **Pre-gate:** Verify NV checksums match M0.1 backup before starting `rmtfs` (in read-only `-r` mode!).
  - **Expect/Pass:** `wlan0` connects to WPA2/WPA3 AP; `bluetoothctl` scans and pairs audio devices.
- [ ] **M5.2 — Audio (QDSP6 / ADSP + Speaker Amps):**
  - **Expect/Pass:** PipeWire routes audio to speakers, earpiece, and headset mic with safe gain limits.

### M6: Battery, Charging & Thermal
- [ ] **M6.1 — Fuel Gauge & Charger (`pm8150b`):**
  - **Expect/Pass:** `/sys/class/power_supply/battery/capacity` and `status` report accurate charge state; thermal zones throttle CPU/GPU safely.

### M7: Dual-Boot / Rootfs Integration (`omamobile-device-r8q`)
- [ ] **M7.1 — Rootfs on `userdata`:**
  - **Details:** Boot Arch Linux ARM + Omarchy Mobile rootfs from `/data/omarchy-root.img` (coexisting with LineageOS) or dedicated `userdata` partition once daily-driver readiness is confirmed.
  - **Package:** Create `packages/omamobile-device-r8q` with UCM audio profiles, firmware extraction script (`tools/extract-vendor-firmware`), and display scaling (`PANEL_SIZE=1080x2400`).

### M8: On-Device Local Text Model (LLM) & Performance Tuning
- [ ] **M8.1 — RAM Compression, MGLRU & VM Tuning (`zram` + `zstd` + `lz4`):**
  - **Details:** Apply the `r8q` 6 GB RAM optimization profile (see §5.1): `zram-size = ram / 2` (2.75 GiB), `zstd` primary compression (with `lz4` or secondary recompression in Linux 7.x), `page-cluster = 0`, `swappiness = 150`, and MGLRU (`min_ttl_ms = 1000` to prevent UI working-set thrashing while an LLM runs).
- [ ] **M8.2 — 120 Hz Fluid Motion & Adreno 650 DVFS Profile:**
  - **Details:** Configure 120 Hz DSI mode (`1080x2400@120`), Adreno 650 `simple_ondemand` devfreq OPP table (`150 MHz`–`587 MHz`), CPU `schedutil` frequency scaling with input boost, and Hyprland 120 Hz spring/bezier curves (see §5.2).
- [ ] **M8.3 — Local AI Inference (`llama.cpp` / Ollama on SM8250):**
  - **Hardware budget:** 5.5 GiB total kernel RAM; Omarchy Mobile + Hyprland uses ~400 MiB at idle, leaving **~5.0 GiB free RAM** (plus ~2.75 GiB compressed zram headroom for cold background pages).
  - **Acceleration:** Compile `llama.cpp` with `-DGGML_VULKAN=ON` (Mesa Turnip on Adreno 650) and ARMv8.2-A `dotprod` + `fp16` intrinsics; run with `--mlock` or `--no-mmap` tuned so weights stay pinned while zram absorbs background app memory.
  - **Recommended Models (Q4_K_M quantization):**
    - `LFM-3-1.7B` / `SmolLM2-1.7B` / `Qwen2.5-1.5B-Instruct` (~1.1 GB RAM, ~12–20 tok/s)
    - `Llama-3.2-3B-Instruct` / `Qwen2.5-3B-Instruct` (~2.0 GB RAM, ~6–10 tok/s)
  - **Expect/Pass:** Local CLI / UI assistant runs completely offline within 2.5 GB RAM with zero swap thrashing and zero dropped frames in Hyprland.

---

## 5. Deep Optimization Guide: RAM Compression & 120 Hz Motions on `r8q`

With **6 GB LPDDR5 RAM (5.5 GiB usable)** and a **120 Hz Super AMOLED (`S6E3FC3`)** driven by **Adreno 650**, the goal is maximum usable memory for apps + local LLMs while keeping 120 fps frame pacing (8.33 ms frame budget) buttery smooth.

### 5.1 Maximal RAM & Compression Architecture

1. **Why `zram-size = ram / 2` (~2.75 GiB) is the sweet spot (Not `ram`):**
   - Verified in Omarchy Mobile on hardware ([`device/tegu/NOTES.md`](../tegu/NOTES.md) 2026-11-08 and [`packages/omamobile-omarchy/zram/91-omamobile.conf`](../../packages/omamobile-omarchy/zram/91-omamobile.conf)): setting `zram-size = ram` causes the kernel to over-commit compressed pages until physical RAM runs out before OOM triggers, stalling reclaim.
   - Setting **`zram-size = ram / 2`** (which `91-omamobile.conf` already sets by default!) gives ~3:1 compression on anonymous userspace memory—turning 1.0 GiB of physical RAM into ~2.75 GiB of fast compressed swap without ever starving the kernel or Wi-Fi/GPU DMA allocators.

2. **Algorithm Choice (`zstd` vs `lz4` + Multi-Stream Recompression in Linux 7.3):**
   - **Default (`zstd`):** Achieves **~2.8:1 to 3.3:1** compression ratio (vs ~2.1:1 for `lz4`). On Snapdragon 865's Cortex-A77 cores (up to 2.84 GHz), single-page (4 KiB) `zstd` decompression takes only a few microseconds.
   - **Advanced Linux 7.x Multi-Tier `zram` (`CONFIG_ZRAM_MULTI_COMP=y`):**
     - Primary compressor: **`lz4`** (ultra-low latency for hot swap-outs during sudden spikes, keeping 120 Hz animations stutter-free).
     - Secondary background recompressor: **`zstd`** (`echo "type=idle" > /sys/block/zram0/recompress` via a periodic systemd timer) compresses cold/idle pages down to maximum `zstd` density in the background.
   - **Optional Writeback (`CONFIG_ZRAM_WRITEBACK=y`):** A 1 GiB backing file on UFS (`/var/lib/zram-writeback.img`) lets completely cold pages flush to UFS 3.0 storage, freeing 100% of their RAM when loading a 3B LLM.

3. **MGLRU (Multi-Gen LRU) + VM Sysctl Tuning:**
   - Enable **`CONFIG_LRU_GEN=y` (MGLRU)** and **`CONFIG_LRU_GEN_ENABLED=y`** in the kernel config.
   - Set `/sys/kernel/mm/lru_gen/min_ttl_ms = 1000`: protects the active working set of the last 1000 ms (Hyprland compositor, Wayland surfaces, touch input handler) from ever being evicted during heavy LLM or compiler runs.
   - Key `vm.*` sysctls (aligned with Omarchy's `99-omarchy-sysctl.conf`):
     - `vm.swappiness = 150` (prefer compressing cold anonymous pages into zram over dropping active executable/shader page cache)
     - `vm.page-cluster = 0` (read/write 1 page at a time to zram; eliminates random I/O latency spikes)
     - `vm.watermark_boost_factor = 0` and `vm.watermark_scale_factor = 125` (smooth background `kswapd` reclaim instead of sudden direct-reclaim pauses that drop frames)
     - `vm.compaction_proactiveness = 0` (prevents background memory compaction from spiking CPU latency during animations)

4. **Kernel Samepage Merging (`KSM`) — Keep Off Except During Heavy Contention:**
   - Avoid continuous KSM scanning on mobile (wastes battery/CPU); if running multiple containers or Waydroid, trigger opportunistic KSM or rely on `zram`.

---

### 5.2 Buttery-Smooth 120 Hz Motions & Graphics Pipeline

To make motions feel instantaneous and fluid on the `S6E3FC3` 120 Hz AMOLED panel:

1. **Display & KMS Pipeline (`msm_drm` + `S6E3FC3`):**
   - Run the panel in native **`1080x2400@120`** command mode with hardware TE (Tearing Effect) synchronization.
   - Enable **VFR (Variable Frame Rate)** in Hyprland (`misc:vfr = true`): drops rendering to 0–1 fps when the screen is static (saving huge battery and freeing GPU bandwidth for local LLMs), and locks instantly to **120 fps** on the very first touch event.

2. **Adreno 650 GPU + Mesa Turnip/Freedreno Optimization:**
   - **Zero-copy direct scanout & explicit sync:** Linux 7.3 + Mesa 25+ on Adreno 650 supports `linux-drm-syncobj-v1` (explicit sync) and hardware overlays in `msm_drm` DPU1, eliminating buffer-copy tearing and cutting compositor GPU load by ~40%.
   - **GPU Devfreq (`simple_ondemand`):** Ensure all Adreno 650 OPPs (`150 MHz` idle up to `587 MHz` boost) and bus bandwidth votes (`icc-bw`) are active in `sm8250-samsung-r8q.dts` so opening the app switcher or scrolling immediately clocks up the memory bus.
   - **Disable heavy multi-pass blur when on battery or running LLMs:** As already configured in [`packages/omamobile-shell/hypr/phone.lua`](../../packages/omamobile-shell/hypr/phone.lua), windows run edge-to-edge opaque (`opacity = "1 1"`, `gaps_in = 0`, `border_size = 0`). Keeping `decoration:blur:enabled = false` (or single-pass `size = 4, passes = 1` for overlays only) saves ~80–120 MB of GPU VRAM and keeps every animation locked at 120 fps.

3. **Tuned 120 Hz Spring-Like Bezier Curves for Hyprland:**
   - High-refresh mobile screens look best with fast initial velocity and a long, gentle deceleration tail (matching iOS / Android 16 spring physics):
   ```ini
   animations {
       enabled = true
       bezier = mobileSpring, 0.22, 1.0, 0.36, 1.0
       bezier = mobileQuick,  0.16, 1.0, 0.30, 1.0
       animation = windows,    1, 3, mobileSpring, slide
       animation = windowsOut, 1, 2, mobileQuick,  popin 92%
       animation = workspaces, 1, 3, mobileSpring, slide
       animation = fade,       1, 2, mobileQuick
       animation = layers,     1, 2, mobileSpring, slide
   }
   ```
   -Combined with Omarchy Mobile's rotation hold patch ([`0002-render-hold-the-last-frame-across-a-rotation.patch`](../../packages/hyprland/0002-render-hold-the-last-frame-across-a-rotation.patch)), orientation changes and workspace swipes remain tear-free.

4. **CPU Scheduler & Touch Input Boost (`schedutil` + `uclamp`):**
   - Set CPU frequency governor to **`schedutil`** across all 3 clusters (4x Cortex-A55 @ 1.8 GHz, 3x Cortex-A77 @ 2.42 GHz, 1x Prime Cortex-A77 @ 2.84 GHz).
   - Pin `Hyprland` and `pipewire` with `Nice=-10` / realtime priority (`rtkit`) and `UCLAMP_MIN` (~15–20%) so even when `llama.cpp` saturates 4–6 threads in the background, touch gestures and 120 Hz frame commits preempt LLM worker threads within `<1 ms`.

---

---

## 7. AI & Agents Settings Architecture: Models & MCPs

To make the S20 FE a true AI-first mobile device, Omarchy Mobile integrates a dedicated **Settings > AI > Agents** configuration subsystem. This subsystem manages on-device and remote LLMs/VLMs downloaded from Hugging Face and connects agents to local and remote tools via Anthropic's **Model Context Protocol (MCP)**.

```
Settings
 └── AI
      └── Agents
           ├── Models (Hugging Face Model Manager)
           │    ├── Text (LLMs)
           │    ├── Vision (VLMs)
           │    ├── Omni / Audio (STT & TTS)
           │    └── Video / Image Generation
           └── MCPs (Model Context Protocol Registry)
                ├── Local MCPs (stdio on-device tools)
                └── Remote MCPs (SSE / HTTP cloud bridges)
```

### 7.1 Configuration Specification (`~/.config/omamobile/ai/agents.json`)

```json
{
  "$schema": "https://omamobile.org/schemas/ai-agents-v1.json",
  "version": 1,
  "default_agent": "agy",
  "inference_backend": "llama.cpp",
  "active_models": {
    "text": "qwen2.5-1.5b-instruct-q4_k_m",
    "vision": "moondream2-q4_k_m",
    "stt": "whisper-tiny.en",
    "tts": "kokoro-82m"
  },
  "models": [
    {
      "id": "qwen2.5-1.5b-instruct-q4_k_m",
      "name": "Qwen 2.5 1.5B Instruct",
      "category": "text",
      "source": "huggingface",
      "repo": "Qwen/Qwen2.5-1.5B-Instruct-GGUF",
      "file": "qwen2.5-1.5b-instruct-q4_k_m.gguf",
      "size_bytes": 1050000000,
      "ram_required_mb": 1100,
      "installed": true,
      "local_path": "~/.cache/omamobile/models/text/qwen2.5-1.5b-instruct-q4_k_m.gguf"
    },
    {
      "id": "llama-3.2-3b-instruct-q4_k_m",
      "name": "Llama 3.2 3B Instruct",
      "category": "text",
      "source": "huggingface",
      "repo": "meta-llama/Llama-3.2-3B-Instruct-GGUF",
      "file": "Llama-3.2-3B-Instruct-Q4_K_M.gguf",
      "size_bytes": 2020000000,
      "ram_required_mb": 2200,
      "installed": false
    },
    {
      "id": "moondream2-q4_k_m",
      "name": "Moondream2 (Tiny VLM)",
      "category": "vision",
      "source": "huggingface",
      "repo": "vikhyatk/moondream2",
      "file": "moondream2-text-model-q4_k_m.gguf",
      "mmproj": "moondream2-mmproj-f16.gguf",
      "size_bytes": 1280000000,
      "ram_required_mb": 1400,
      "installed": false
    },
    {
      "id": "whisper-tiny.en",
      "name": "OpenAI Whisper Tiny (English)",
      "category": "omni",
      "source": "huggingface",
      "repo": "ggerganov/whisper.cpp",
      "file": "ggml-tiny.en.bin",
      "size_bytes": 77800000,
      "ram_required_mb": 120,
      "installed": true
    },
    {
      "id": "kokoro-82m",
      "name": "Kokoro 82M Speech Synth",
      "category": "omni",
      "source": "huggingface",
      "repo": "hexgrad/Kokoro-82M",
      "file": "kokoro-v0_19.onnx",
      "size_bytes": 86000000,
      "ram_required_mb": 110,
      "installed": true
    }
  ],
  "mcps": [
    {
      "id": "omamobile-system",
      "name": "Phone Hardware & Sensors",
      "type": "local",
      "transport": "stdio",
      "command": "/usr/lib/omamobile/mcps/omamobile-mcp-system",
      "args": [],
      "env": {},
      "permissions": ["battery", "wifi", "brightness", "torch", "haptics"],
      "enabled": true
    },
    {
      "id": "local-filesystem",
      "name": "Device File Access",
      "type": "local",
      "transport": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/santiago/Documents", "/home/santiago/Downloads"],
      "enabled": true
    },
    {
      "id": "remote-search",
      "name": "Private Web Search MCP",
      "type": "remote",
      "transport": "sse",
      "url": "https://mcp-search.internal.net/sse",
      "headers": {
        "Authorization": "Bearer ${SEARCH_MCP_KEY}"
      },
      "enabled": false
    }
  ]
}
```

---

### 7.2 Primary Local Engine: Liquid AI LFM3 Family (Preferred for S20 FE)

The **Liquid AI LFM3 (Liquid Foundation Model 3)** family is prioritized as the **primary local agent engine** on the S20 FE (`r8q`). Its non-transformer tri-hybrid architecture (gated convolution + sliding window + global attention) provides sub-quadratic memory complexity, massive 256K context capabilities, and native tool-calling with minimal KV-cache memory pressure.

#### Architecture & Agent Suitability: LFM3-1.7B vs SmolLM2-1.7B

| Metric / Capability | **Liquid AI LFM3-1.7B / 2B (Preferred)** | SmolLM2-1.7B-Instruct | Standard Mobile Transformer |
| --- | --- | --- | --- |
| **Primary Target** | **Local agent harnesses, MCP tool-calling, edge autonomy** | General on-device instruction following & chat | Desktop-scale downsized chat |
| **Architecture** | **Tri-hybrid (gated convolution + sliding window + global attention)** | Standard dense Transformer decoder | Dense Transformer decoder |
| **Context Window** | **256K tokens (sub-quadratic scaling)** | 2,048 tokens (expandable to 8K) | 4K–8K tokens |
| **Native Agent Post-Training** | **Direct optimization for agent harnesses (Hermes, Pi, OpenClaw)** | Basic function-calling and instruction-tuning | Generic synthetic chat |
| **Tool Calling Pattern** | **Native structured calling + deferred tool loading / search (MCP)** | Standard JSON schema prompt formatting | JSON in system prompt |
| **Memory Footprint (Q4)** | **~1.0 GB RAM (minimal KV-cache memory pressure)** | ~1.1 GB RAM (KV-cache grows linearly) | ~1.2–1.8 GB RAM |
| **ARM NEON / GPU Kernels** | **High efficiency on Cortex-A77/A55 with optimized LFM3 kernels** | Standard matrix-multiplication kernels | Standard GEMM / GEMV |

---

### 7.3 Curated Model Catalog by Modality

| Category | Model Name | Hugging Face Repository | Quant / Size | RAM (MB) | Role on S20 FE |
| --- | --- | --- | --- | --- | --- |
| **Text (Agent Core)** | **LFM3-1.7B / 2B** | `LiquidCloud/LFM3-1.7B-GGUF` | Q4_K_M (~1.02 GB) | **~1050 MB** | **Default system agent: MCP tool calls, system settings, 256K context** |
| **Text (Alternative)** | **Qwen 2.5 1.5B** | `Qwen/Qwen2.5-1.5B-Instruct-GGUF` | Q4_K_M (~1.10 GB) | ~1150 MB | Secondary fallback for general multi-lingual translation |
| **Text (Flagship)** | **Llama 3.2 3B** | `meta-llama/Llama-3.2-3B-Instruct-GGUF` | Q4_K_M (~2.02 GB) | ~2200 MB | High-complexity offline synthesis |
| **Vision** | **LFM3-Vision-2B** | `LiquidCloud/LFM3-Vision-2B-GGUF` | Q4_K_M (~1.35 GB) | **~1450 MB** | **Real-time camera scene understanding, visual QA, glance widgets** |
| **Vision (Light)** | **Moondream2 1.8B** | `vikhyatk/moondream2` | Q4_K_M (~1.28 GB) | ~1400 MB | Lightweight OCR and document transcription |
| **Omni** | **LFM3-Omni-2B** | `LiquidCloud/LFM3-Omni-2B-GGUF` | Q4_K_M (~1.48 GB) | **~1600 MB** | **Unified audio/speech + vision + text real-time conversational agent** |
| **Omni (Modular)** | **Whisper Tiny + Kokoro** | `ggerganov/whisper.cpp` + `hexgrad/Kokoro-82M`| FP16 + ONNX | ~230 MB | Modular Voxtype STT + ultra-low-latency 25x TTS |
| **Video** | **LFM3-Video-2B** | `LiquidCloud/LFM3-Video-2B-GGUF` | Q4_K_M (~1.55 GB) | **~1700 MB** | **Temporal multi-frame camera buffer reasoning & video scene QA** |
| **Video (Cloud)** | **Remote Video Bridge** | Remote MCP Tool (`flux-schnell` / `kling`) | Remote | 0 MB | Heavy multi-frame generation offloaded via Remote MCP |

---

### 7.3 MCP (Model Context Protocol) Integration

1. **Local MCPs (On-Device via `stdio`):**
   - Packaged in `/usr/lib/omamobile/mcps/`
   - Gives the local agent control of native phone capabilities without root:
     - `omamobile-mcp-system`: reads battery, Wi-Fi status, thermal zones, sets brightness, triggers haptics.
     - `omamobile-mcp-camera`: captures still frames from `omacam` / libcamera for input into Moondream2.
     - `omamobile-mcp-notes`: interacts with Omarchy's notes and calendar widgets.
2. **Remote MCPs (Cloud / LAN via `SSE` / `Streamable HTTP`):**
   - Configurable per-agent in `agents.json`.
   - Allows the phone agent to query external databases, search engines, or trigger heavy remote GPU tasks while keeping sensitive device credentials local.

---

### 7.4 Dual-Mode AI Architecture: Remote llama.cpp Client & Phone-as-Subagent Server

To maximize flexibility, Omarchy Mobile supports two complementary network operating modes for AI:

#### 1. Client Mode: Offload to Remote llama.cpp / Custom Servers
- **Settings Path:** `Settings > AI > Agents > Inference Backend`
- **Options:** `On-Device (Local)` | `Remote Endpoint (OpenAI/llama.cpp)`
- **Remote Configuration Fields:**
  - `Endpoint URL`: e.g. `http://192.168.1.100:8080/v1` (your desktop llama.cpp server)
  - `Model Identifier`: e.g. `Meta-Llama-3.1-70B-Instruct`
  - `API Key`: Optional auth bearer token
  - `Context Length`: e.g. `32768`
- **Advantage:** Allows the phone to leverage 70B+ flagship models running on your PC/home server, saving phone battery and RAM while keeping Omarchy's voice bar, search, and widget agents fully responsive.

#### 2. Server Mode: Phone as an OpenAI-Compatible Subagent Endpoint
- **Settings Path:** `Settings > AI > Agents > Subagent Server`
- **Features & Controls:**
  - **Toggle:** `Enable Subagent Server` (`omamobile-ai-server.service`)
  - **Bind Interface:**
    - `USB Cable Only (172.16.42.1)`: Private, high-speed, zero Wi-Fi exposure.
    - `Local Wi-Fi (0.0.0.0)`: Broadcast via mDNS (`s20fe-agent.local:8080`).
  - **Port:** Default `8080` (configurable)
  - **Served Model:** Select from installed on-device models (`LFM3-1.7B`, `LFM3-Vision-2B`, `Qwen2.5-1.5B`).
- **OpenAI-Compatible REST Endpoints:**
  - `GET /v1/models`
  - `POST /v1/chat/completions`
  - `POST /v1/completions`
- **Use Case:** Your PC or development environment can treat the S20 FE as a dedicated, low-power edge subagent. Combined with the phone's local MCP tools, your PC can dispatch sub-tasks to the phone:
  ```bash
  # Query the phone subagent over USB from your PC:
  curl http://172.16.42.1:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d '{
      "model": "lfm3-1.7b",
      "messages": [{"role": "user", "content": "Query battery status and report remaining time."}]
    }'
  ```

---

## 8. Pre-Push Checklist (Every Commit)

Before committing or opening any PR, always run:
```sh
tools/pr-check
tests/device-overlays
tests/nv-backup
tests/boot-method
```

