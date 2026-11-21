<!-- SPDX-License-Identifier: MIT -->

# Galaxy S20 FE (`r8q`, SM8250) Fingerprint Feasibility Assessment & Plan

**Status (`2026-11-22`):** Assessed (`[fp-01]` + `[fp-02]`, `r8q/fp-02-feasibility`)
**Hardware:** Goodix `GW9558` (`goodix,gw9558x`) optical under-display SPI sensor (`qupv3_se7_spi` / `spi@99c000`)
**TEE:** Qualcomm QSEE (`qseecom@82400000`, version `0x1402000`), Trusted App `securefp` (`goodixopt`)
**Verdict:** **Deferred / Not viable in mainline.** No open native `libfprint` driver is possible, and reproducing Pixel 7 Pro's (`cheetah`) minimal Android chroot on `r8q` carries major out-of-tree kernel, TrustZone, panel-HBM, touchscreen-FOD, and Samsung HAL blockers that are out of scope for daily-driver bring-up.

---

## 1. Hardware & Stock Architecture Inventory

Evidence from `reference-r8q/live.dts`, `reference-r8q/dmesg.txt`, `reference-r8q/lineage-kernel.config`, and the extracted stock vendor rootfs/firmware:

| Component | Stock `r8q` Evidence | Notes |
| --- | --- | --- |
| **Platform sensor node** | `gw9558@0` (`compatible = "goodix,gw9558x"`) | `goodix,chip_id = "GW9558"`, `goodix,modelinfo = "G781"`, `spi-max-frequency = 25 MHz` (`0x17d7840`), `goodix,gpio_pwr` = `gpio67` (`fps_ldo`), `goodix,gpio_reset` = `gpio131` (`fps_rst`); no AP `interrupts` property wired on this node |
| **Optical geometry & white mask** | `goodix,position`, `goodix,rb = "525,215,-1,FFFFFF"` | Under-display optical circle location/size and full-white illumination value (`#FFFFFF`) |
| **SPI controller & factory node** | `qupv3_se7_spi: spi@99c000` -> `gw9558-spi@0` (`goodix,gw9558x_factory`) | Uses `GPIO 20–23` (`qup7`). In upstream `arch/arm64/boot/dts/qcom/sm8250-samsung-common.dtsi`, `&tlmm` marks `gpio-reserved-ranges = <20 4>, /* SPI (fingerprint scanner) */` because Qualcomm XPU/TZ owns those pins during normal operation |
| **Stock kernel config & dmesg** | `CONFIG_SENSORS_FINGERPRINT=y`, `CONFIG_QSEECOM=y`, `CONFIG_QTEE_SHM_BRIDGE=y` | `dmesg.txt`: `fps_gw9558_work_func_debug: ldo: 0, sleep: 0, tz: 1 type: goodixopt` |
| **QSEE reserved DRAM** | `qseecom@82400000` (`qseecom_region` 28 MiB, `qseecom_ta_region` 48 MiB) | Dedicated XPU/TZ memory regions reserved in DT (`no-map` in mainline `sm8250-samsung-r8q`) |
| **TrustZone Applications (TA)** | `securefp.mdt` + `securefp.b00..b07` (~18 MiB across split segments; `securefp.b04` = 12.8 MiB) | Signed, encrypted QSEE TA loaded by `libbauthtzcommon.so` (`QSEECom_start_app(..., "securefp")`), alongside `skeymast` / `skm` (`QKM`/`SKM`) and auth-token handles |
| **Stock userspace HAL chain** | `android.hardware.biometrics.fingerprint-service.samsung` -> `libsfp_sensor.so` / `libbauthserver.so` -> `libbauthtzcommon.so` -> `libQSEEComAPI.so` (`/dev/qseecom`) + `qseecomd` | Uses Samsung AIDL implementation (`hardware/samsung/aidl/fingerprint/Fingerprint.cpp`), `libgf_in_system_lib.so`, `libdrmfs.so`, `ssgtzd`, `android.hardware.gatekeeper@1.0-service-mdfpp`, `android.hardware.keymaster@4.0-service.samsung`, and `vendor.qti.hardware.qseecom@1.0` / `qteeconnector@1.0` |
| **Display & Touch FOD hooks** | Panel `samsung,gamma_mode2_hbm_tx_cmds_revA` (`53 20`, `51 03 ff`), `samsung,fod-dim-lut`; `init.fingerprint.rc` writes `"fod_enable,1,1,0"` to `/sys/class/sec/tsp/cmd`; `stm,support_fod` | Optical capture requires HBM illumination over the sensor circle plus touchscreen FOD press-zone coordination (`/dev/uinput` / `ro.vendor.fingerprint.request_touch_event`) |

---

## 2. Why a Native Mainline / `libfprint` Driver Is Not Possible

1. **XPU-locked SPI bus (`GPIO 20–23`):** Qualcomm TrustZone reserves `qupv3_se7` SPI (`gpio20..23`) via XPU access control (`sm8250-samsung-common.dtsi` `gpio-reserved-ranges = <20 4>`). Unprivileged AP Linux cannot claim or bit-bang `spi@99c000` in normal operation without tripping an XPU/TZ fault; `goodix,gw9558x` (`gw9558@0`) on stock only manages power (`gpio67`) and reset (`gpio131`) while QSEE runs the SPI transfers.
2. **Closed TA firmware and template crypto (`securefp`):** Image capture, lens/pixel calibration, anti-spoof processing, template enrollment/matching, and hardware key wrapping all execute inside the signed, encrypted `securefp` QSEE TrustZone image. Neither Goodix nor Samsung publishes the `GW9558` SPI protocol or firmware keys.

---

## 3. Why a `cheetah`-Style Minimal Android Chroot Is Blocked Today

[`device/cheetah/FINGERPRINT-RESEARCH.md`](../cheetah/FINGERPRINT-RESEARCH.md) works on Pixel 7 Pro because Google ships open-source **Trusty** kernel support, a clean standalone Goodix HAL (`libQSEECom`-free), mainline/vendor-derived panel **LHBM** sysfs commands (`53 30`/`53 20` already in `s6e3fc3` for P10), and `/dev/sg*` RPMB access. On `r8q`, four structural gaps stand in the way:

1. **QSEECOM vs. Trusty / Upstream `qcomtee` Mismatch:**
   - Mainline 7.3 on `r8q` enables `CONFIG_QCOM_QSEECOM=y` (kernel-internal client bus; `qcom_scm` logs `qseecom: found qseecom with version 0x1402000` -> `qseecom: untested machine, skipping` on SM8250 with **no `/dev/qseecom` userspace chardev**) and `CONFIG_QCOMTEE=m` (`/dev/tee0` reports `qcomtee: QTEE version 0.0.0`, upstream MINK/GP-TEE ABI).
   - Samsung's `libQSEEComAPI.so`, `qseecomd`, and `libbauthtzcommon.so` explicitly open **`/dev/qseecom`** and require Qualcomm's out-of-tree `QSEECOM_IOCTL_*` chardev ABI (`QSEECom_start_app_V2`, `QSEECom_send_cmd`, ION/DMA-BUF shared buffers, `qseecomd` RPMB/FS listeners). Carrying Qualcomm's downstream `qseecom.c` forward onto Linux 7.3 is a large non-upstreamable kernel patchset.
2. **RPMB & Storage-Guard Interaction (`AGENTS.md` §1):**
   - `out-r8q/.config` currently leaves `# CONFIG_RPMB is not set`, and the storage guard locks all 77 UFS partitions read-only (`ro=1`). While RPMB is technically separate from GPT partitions, `qseecomd` (`libdrmfs.so`) also services vendor secure-filesystem requests targeting `/efs/biometrics` and `/data/vendor/biometrics` and links alongside Samsung Gatekeeper/Keymaster (`skeymast`, `mdfpp`). Running `qseecomd` safely would require carefully wrapping or stubbing filesystem listeners to `userdata` only so NV/EFS partitions can never be touched.
3. **Missing Panel HBM / FOD & Touchscreen FOD Infrastructure:**
   - `drivers/gpu/drm/panel/panel-samsung-s6e3fc3.c` on `r8q` only drives normal `MIPI_DCS_SET_DISPLAY_BRIGHTNESS` (`max_brightness = 1023`) and has no HBM (`53 20`, `samsung,fod-dim-lut`) toggle for optical sensor illumination.
   - `drivers/input/touchscreen/stmfts.c` supports 16-byte Type-B touch events on `r8q`, but exposes neither Samsung's `/sys/class/sec/tsp/cmd` (`fod_enable,1,1,0`) nor FOD press-area state notifications. Hyprland/Wayland lockscreen integration would also need to render the `#FFFFFF` FOD spot (`525,215`) synchronized with panel HBM frames.
4. **Heavy Samsung Vendor HAL Dependency Chain:**
   - Unlike Pixel's single-purpose Goodix HAL, `android.hardware.biometrics.fingerprint-service.samsung` pulls in Samsung's Binder/Auth stack (`libsfp_sensor.so`, `libbauthserver.so`, `libbauthtzcommon.so`, `libfloatingfeature.so`, `libsecnativefeature.so`), Qualcomm QSEE/QTEE services, and Samsung Gatekeeper/Keymaster (`mdfpp`) for Hardware Auth Token (HAT) verification.

---

## 4. Recommendation & Future Prerequisite Roadmap

**Recommendation:** Keep **Stage 16 (Fingerprint) deferred / unsupported** for `r8q`. Prioritizing power/suspend (`s2idle`), USB-C role switching, modem bring-up, haptics, and upstreaming the existing `wt-r8q` patch stack gives far higher daily-driver value with zero out-of-tree TEE maintenance burden.

If a future contributor chooses to attempt an out-of-tree Android-chroot bring-up anyway, the required milestones are:

1. **Out-of-tree QSEECOM transport:** Either port CAF `drivers/misc/qseecom.c` + DMA-BUF listener support to Linux 7.3 (`/dev/qseecom`), or verify whether SM8250 TZ (`0x1402000`) accepts MINK `smcinvoke` calls via upstream `/dev/tee0` (`qcomtee`) behind a userspace `libQSEEComAPI.so` shim.
2. **Safe AP power & storage wiring:** Add a guarded AP helper driver for `gw9558@0` (`gpio67` LDO, `gpio131` reset, `/dev/goodix_fp`) modeled after `drivers/misc/goodix-fp.c`, enable `CONFIG_RPMB` (`CONFIG_UFS_BSG` / `/dev/sg*`), and confine `qseecomd` listeners strictly to `userdata` overlays (`AGENTS.md` §1).
3. **Panel HBM + Touch FOD:** Port `s6e3fc3` HBM/FOD DSI commands from `samsung,gamma_mode2_hbm_tx_cmds_revA` behind a sysfs attribute (matching `cheetah` `LHBM`), and add optional `fod_enable` press reporting to `stmfts` + Wayland lock-screen spot rendering.
4. **Minimal chroot + AIDL->`libfprint` bridge:** Package `qseecomd`, `android.hardware.keymaster@4.0-service.samsung`, `android.hardware.gatekeeper@1.0-service-mdfpp`, `android.hardware.biometrics.fingerprint-service.samsung`, and a local AIDL-to-`fprintd` bridge mirroring `omamobile-fp`.
