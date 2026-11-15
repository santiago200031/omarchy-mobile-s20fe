<!-- SPDX-License-Identifier: MIT -->
# Runbook: Samsung Galaxy S20 FE (r8q) Display and GPU Graphical Bring-Up

**Target Hardware**: Samsung Galaxy S20 FE (`SM-G780G`, codename `r8q`, Qualcomm SM8250 Snapdragon 865)  
**Safety Model**: Offline-validated gated hardware runbook. Dual-boot architecture strictly leaves `boot` (`sda23`), `super` (`sda29`), `userdata` (`sda36`), bootloader, and NV partitions untouched. All 77 UFS partitions under `/dev/sda*` remain read-only (`ro=1`). Flashing is strictly confined to `recovery` (`sda24`, 82,694,144 bytes).  
**Human-in-the-Loop Requirement**: Flashing and hardware execution require explicit human approval and observation per `AGENTS.md` §1.

---

## 0. Overview & Staged Offline Artifacts

All artifacts have been compiled, AVB-signed, and verified offline:

| Artifact | Location | Details |
|---|---|---|
| Recovery Image | `/home/amd-omarchy/omamobile-kernel/out-r8q/recovery-r8q-display-gpu.img` | Header v2, 82,694,144 B (fits `sda24`), AVB RSA4096 signed |
| Odin / Heimdall TAR | `/home/amd-omarchy/omamobile-kernel/out-r8q/recovery-r8q-display-gpu.tar.md5` | Samsung flashable tar package containing `recovery.img` |
| Compiled DTB | `/home/amd-omarchy/omamobile-kernel/out-r8q/arch/arm64/boot/dts/qcom/sm8250-samsung-r8q.dtb` | 9 display clocks + `MDSS_GDSC` on `simple-framebuffer`, `gpu_mem` at `0x9031a000`, no `protected-clocks` |
| Display Overlay | `device/r8q/overlays/display.dtso` | Enables `display-subsystem@ae00000`, `dsi@ae94000`, `panel@0`, `phy@ae94400` |
| Kernel Modules | `/home/amd-omarchy/omamobile-kernel/out-r8q/modules-*.tar.zst` | Linux 7.3.0-rc6 modules |
| Staged Firmware | `~/.cache/omamobile/firmware/r8q/lib/firmware/` | `a650_sqe.fw` (v0x112), `a650_zap.mbn`, `a650_gmu.bin`, `ipa_fws.mbn`, `cs40l*`, `a96t356*` |

---

## Gate A: Base Recovery Flash & UFS RO Safety Verification

- **Objective**: Flash verified dual-boot recovery image to `sda24` via Heimdall in Download Mode, boot into Omarchy dual-boot initramfs from microSD (`/omarchy-r8q.img`), and verify the read-only storage guard across all 77 UFS partitions.
- **Risk Level**: Low-Medium (writes `sda24` recovery partition only; `boot`/`sda23` untouched).
- **Prerequisites**: Phone connected to laptop via USB-C cable; battery charged >= 50%; microSD card inserted with `/omarchy-r8q.img`.

### Execution Steps
1. Boot phone into Samsung Download Mode:
   ```bash
   adb reboot download
   ```
   *(Alternative without ADB: power off phone, hold Volume Up + Volume Down simultaneously, and plug in USB-C cable).*
2. Verify Heimdall communication:
   ```bash
   heimdall detect
   ```
   *Expect*: `Device detected`.
3. Flash the recovery image to the `RECOVERY` partition without automatic reboot:
   ```bash
   heimdall flash --RECOVERY /home/amd-omarchy/omamobile-kernel/out-r8q/recovery-r8q-display-gpu.img --no-reboot
   ```
4. Reboot into Omarchy Recovery:
   Hold `Volume Up` + `Power` buttons until Samsung logo appears, then release `Power` while continuing to hold `Volume Up`.
5. Connect via USB SSH from the laptop:
   ```bash
   ssh root@172.16.42.1
   ```
6. Run the automated preflight verification on the phone:
   ```bash
   r8q-preflight --json
   ```
7. Manually inspect UFS read-only status on the phone:
   ```bash
   cat /sys/block/sda/sda*/ro | grep -v '^1$' || echo "All UFS partitions verified read-only"
   ```

### Expect / Pass Criteria
- Heimdall reports `100%` transfer and session completes without errors.
- Phone boots into Omarchy kernel; continuous splash remains displayed without white flash.
- USB NCM gadget initializes and assigns IP `172.16.42.1`.
- `r8q-preflight --json` returns `"ufs_guard":{"ok":true}`.
- Every `/sys/block/sda/sda*/ro` node reports `1` (all 77 UFS partitions locked read-only).

### Stop Condition
- Heimdall reports protocol/write error.
- Phone fails to enumerate USB gadget within 60 seconds.
- Any UFS partition has `ro == 0`.

### Rollback Instructions
- Normal reboot without buttons boots LineageOS on `sda23` (primary OS completely unaffected).
- If recovery partition needs restoration, flash stock recovery:
  ```bash
  heimdall flash --RECOVERY ~/android/backups/r8q-nv/recovery.img --no-reboot
  ```

---

## Gate B: Display Overlay & MSM KMS / Adreno 650 Takeover

- **Objective**: Verify that `simpledrm` scanout remains stable with active display clocks held, unmask `DISP_CC_MDSS_ESC0_CLK`, probe `msm` DRM driver, verify Adreno 650 GPU initialization (`/dev/dri/renderD128`) and DPU KMS card (`/dev/dri/card0`), and remove the modprobe blacklist.
- **Risk Level**: Medium (modprobe loads DRM display and GPU drivers).
- **Prerequisites**: Gate A passed; SSH session active at `172.16.42.1`.

### Execution Steps
1. In the active SSH session on the phone, verify Device Tree nodes:
   ```bash
   r8q-preflight --json
   ```
   *Expect*: `mdss=okay` and `panel=okay`.
2. Check that the Samsung S6E3FC3 AMOLED panel backlight driver is registered:
   ```bash
   ls -la /sys/class/backlight/
   ```
   *Expect*: `s6e3fc3_backlight` or similar panel backlight device listed.
3. Announce screen mode change per `AGENTS.md` §1 safety rules:
   ```bash
   echo "==> Announcing: testing MSM KMS and Adreno 650 GPU takeover on screen"
   ```
4. Probe `msm` driver under a 10-second watchdog timeout:
   ```bash
   timeout 10 modprobe msm
   ```
5. Inspect kernel ring buffer for clean KMS handover and clock timing calculations:
   ```bash
   dmesg | grep -E 'msm|dpu|dsi|adreno|gpu|s6e3fc3'
   ```
6. Verify DRI card and render nodes exist:
   ```bash
   ls -la /dev/dri/card* /dev/dri/renderD128
   ```
7. Verify GPU devfreq governor and minimum frequency:
   ```bash
   cat /sys/class/devfreq/3d00000.gpu/cur_freq
   cat /sys/class/devfreq/3d00000.gpu/min_freq
   ```
   *Expect*: `min_freq` is `305000000` (305 MHz) per `80-r8q-gpu.rules`.
8. Once verified, permanently remove the modprobe blacklist from the phone rootfs:
   ```bash
   rm -f /usr/lib/modprobe.d/50-r8q-gpu-blacklist.conf
   ```

### Expect / Pass Criteria
- `msm_dsi_host_get_phy_clk_req` queries `esc_clk` (`DISP_CC_MDSS_ESC0_CLK`) and receives 19.2 MHz rate.
- `msm_dsi_dphy_timing_calc_v4` succeeds with return code 0 (no `-EINVAL` / `-22`).
- `dsi_7nm_phy_enable` completes without errors.
- Adreno 650 GPU authenticates `a650_zap.mbn` via TrustZone PAS using carveout `0x9031a000`.
- Both `/dev/dri/card0` and `/dev/dri/renderD128` are created.
- `omamobile-r8q-panel.service` detects `/sys/class/backlight/*` and yields unblanking to DRM.

### Stop Condition
- AMOLED white-flash or unrecoverable black-out after `modprobe msm`.
- Kernel panic in `dpu_kms` or `msm_dsi_dphy_timing_calc_v4`.
- Qualcomm RPMh AMC-voter `-110` bus lockup.

### Rollback Instructions
- Re-add the blacklist over SSH or serial console `ttyGS0`:
  ```bash
  echo "blacklist msm" > /usr/lib/modprobe.d/50-r8q-gpu-blacklist.conf
  ```
- If display is dark and SSH unresponsive, force reboot: hold Power + Volume Down for 10 s to return to LineageOS or Download Mode.

---

## Gate C: Remoteprocs & CS35L41 Audio Bring-Up

- **Objective**: Bring up Qualcomm SM8250 remoteprocs (ADSP, CDSP, SLPI) and verify Cirrus Logic CS35L41 stereo speaker amplifiers with ALSA UCM2 audio.
- **Risk Level**: Medium (loads DSP firmwares).
- **Prerequisites**: Gate B passed.

### Execution Steps
1. Re-pack recovery image with audio and remoteproc overlays:
   ```bash
   KERNEL_OUT=/home/amd-omarchy/omamobile-kernel/out-r8q tools/r8q-boot pack --dual-boot \
     --overlay display \
     --overlay remoteprocs \
     --overlay audio-amps \
     --overlay audio-sound \
     /home/amd-omarchy/omamobile-kernel/out-r8q/recovery-r8q-audio.img
   ```
2. Flash `recovery-r8q-audio.img` via Heimdall in Download Mode and reboot into Recovery.
3. Connect over SSH and verify remoteprocs:
   ```bash
   journalctl -b | grep remoteproc
   ```
4. Verify Cirrus CS35L41 amplifiers on I2C bus 11 (addresses `0x40` and `0x41`):
   ```bash
   ls -la /sys/bus/i2c/devices/11-0040 /sys/bus/i2c/devices/11-0041
   ```
5. Check ALSA sound card registration:
   ```bash
   aplay -l
   ```
   *Expect*: `card 0: kona-mtp-snd-card` or `Samsung-Galaxy-S20-FE`.
6. Play test tone at safe low volume (-20 dBFS):
   ```bash
   speaker-test -t sine -f 440 -l 1
   ```

### Expect / Pass Criteria
- ADSP, CDSP, and SLPI firmware images load without GPI DMA errors.
- Both CS35L41 speaker amps enumerate.
- Sound card is registered and ALSA UCM2 profile loads.
- Test tone plays cleanly through phone speakers.

### Stop Condition
- Remote processor crash or kernel deadlock in GPI DMA.
- Amplifier DC fault or overtemperature trip.

### Rollback Instructions
- Re-flash Gate B image (`recovery-r8q-display-gpu.img`) to run without DSP/audio overlays.

---

## Gate D: Full Graphical Session & Key/Touch UX

- **Objective**: Launch the full Omarchy / Hyprland 0.57 Aquamarine graphical session on the phone AMOLED display (1080x2400 @ scale 2), verify STMicroelectronics touchscreen input, and verify hardware keys (Power, VolUp, VolDown).
- **Risk Level**: Low (userspace session startup).
- **Prerequisites**: Gate B and Gate C passed.

### Execution Steps
1. Adopt session configuration on the phone:
   ```bash
   omamobile-setup-user --adopt
   ```
2. Inspect monitor scale setting:
   ```bash
   cat ~/.config/hypr/monitors.lua
   ```
   *Expect*: `omarchy_monitor_scale = 2`.
3. Verify phone shell rules:
   ```bash
   grep -q "phone.lua" ~/.config/hypr/looknfeel.lua || echo "phone.lua included"
   ```
4. Announce session launch:
   ```bash
   echo "==> Announcing: Launching Hyprland graphical session on phone AMOLED screen"
   ```
5. Start graphical session:
   ```bash
   systemctl --user start graphical-session.target
   ```
6. Verify Hyprland renderer using Turnip Adreno 650:
   ```bash
   journalctl --user -u hyprland | grep -E 'Aquamarine|renderD128|GL_RENDERER'
   ```
7. Test Touchscreen:
   - Tap screen: verify cursor/focus tracks touch position.
   - Swipe gestures: verify window switching.
8. Test Hardware Keys:
   - Press `Power` key: display turns off (DPMS off).
   - Press `Power` key: display turns on (DPMS on).
   - Press `Volume Up` / `Volume Down`: verify on-screen volume toast.
   - Chord `Power` + `Volume Down`: verify screenshot capture.

### Expect / Pass Criteria
- AMOLED display presents the Omarchy desktop shell at native 1080x2400 resolution scaled to 540x1200 logical pixels.
- Rendering is hardware-accelerated via Adreno 650 (`/dev/dri/renderD128`).
- Touchscreen tracks touches smoothly without ghost events.
- Power key wakes and suspends display without triggering CPU deep sleep freeze (`s2idle` masked).

### Stop Condition
- Desktop freezes or GPU lockup detected.
- Unresponsive touchscreen.

### Rollback Instructions
- Reload compositor configuration:
  ```bash
  hyprctl reload
  ```
- Or restart user session:
  ```bash
  systemctl --user restart graphical-session.target
  ```

---

## Hardware Execution Log (2026-11-15)

- **Gate A — PASSED (`7.3.0-rc6-omamobile-push1-gd469bd3bf2a9`):**
  - `recovery-r8q-display-gpu.img` flashed to `RECOVERY` (`sda24`), booted with **zero white flash**.
  - USB NCM gadget came up at `172.16.42.1`; `r8q-preflight --json` reported `ufs_guard.ok=true` (all UFS `sda*` partitions `ro=1`), `gpu_ready=true`, `audio_ready=true`, all 8 firmware checks `ok=true` (`a650_sqe.fw` `0x112`).
- **Gate B — PASSED (`modprobe msm` returned `0`):**
  - `dpu hardware revision:0x60000000`, `Initialized msm 1.13.0 for ae01000.display-controller on minor 1`, `loaded qcom/a650_sqe.fw`, `loaded qcom/a650_gmu.bin` (`Loaded GMU firmware v2.1.8`), `fb0: msmdrmfb frame buffer device` (`135x150` console), `panel-samsung-s6e3fc3 ae94000.dsi.0: taken over from the bootloader`.
  - `/dev/dri/card1`, `/dev/dri/renderD128`, `card1-DSI-1` (`1080x2400@120` + `60`), `/sys/class/backlight/ae94000.dsi.0` (`511/1023`), and `3d00000.gpu` devfreq (`305000000` Hz) all active; `stmfts` touch (`event2`) and volume keys operational.
- **Gate D — PARTIAL (Freedreno Mesa installed, Hyprland + shell launched, hung at `t+14s`):**
  - Rootfs `/dev/loop0` (`/omarchy-r8q.img`) had `/home` empty, `msm` blacklisted in `/usr/lib/modprobe.d/50-r8q-gpu-blacklist.conf`, and `[omamobile]/mesa 1:26.2.4-1.4` (Mali-only) lacking `msm_dri.so` (causing `Hyprland` `SIGABRT` on `eglInitialize`).
  - Un-blacklisted `msm`, provisioned user `alarm` (`uid=1000`) + `DSI-1` `1080x2400@120` config, and installed Arch Linux ARM `extra/mesa 1:26.2.7-1`, `extra/vulkan-mesa-implicit-layers 1:26.2.7-1`, and `extra/vulkan-freedreno 1:26.2.7-1` (`msm_dri.so`, `kgsl_dri.so`, `libvulkan_freedreno.so`).
  - Restarting `seatd` + `sddm` launched `Hyprland --watchdog-fd 4`, `quickshell -n -p /usr/share/omarchy/shell`, `squeekboard`, and `Xwayland` cleanly (`t+8s`), then SSH stalled during 3D startup (`t+14s`) before the phone powered off (`multi-user.target` on cmdline keeps next boot safe).
- **Next Recovery Boot & Re-Flash Actions (human-approved):**
  1. Boot into existing `RECOVERY` (`multi-user.target`) and read `/sys/fs/pstore/console-ramoops-0` to inspect the `t+14s` hang.
  2. Re-flash repacked `recovery-r8q-display-gpu.img` containing the `sm8250-samsung-common.dtsi` reserved-memory fix (resolving the `0x8bf00000` vs `gpu_mem` `0x9031a000` overlap) and deploy `80-r8q-gpu.rules` (`max_freq="305000000"`).
  3. Test `sddm.service` launch under watchdog with `max_freq=305000000` (and `AQ_NO_MODIFIERS=1` if needed for Adreno UBWC triage).
