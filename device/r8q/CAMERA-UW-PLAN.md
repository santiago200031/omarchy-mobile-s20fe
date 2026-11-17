<!-- SPDX-License-Identifier: MIT -->

# Ultrawide Camera Bring-Up — Samsung Galaxy S20 FE (r8q)

**Sensor:** Samsung S5K3L6 (rear ultrawide, cell 2)
**Status:** Verified end-to-end on hardware (`7.3.0-rc6-omamobile-push1-g3667ab368ef9` + `libcamera 0.7.2-4.25`: raw `2104x1560`/`4208x3120` CSI-2 capture + `libcamerasrc` JPEG)
**Last Updated:** 2026-11-17

---

## 1. Verified Hardware Details (`live.dts` `qcom,cam-sensor2`)

| Property | Value |
|----------|-------|
| Role | Rear ultrawide (`cell-index = <2>`) |
| Model | Samsung S5K3L6 (12 MP) |
| Control bus | Mainline `&i2c2` (`i2c@988000`, stock `qupv3_se2_i2c`, `GPIO 115`/`116` `qup2`) |
| I2C Address (7-bit) | `0x2d` (stock DT `slave-addr = <0x5a>`) |
| CSI-PHY | CSIPHY1 (`csiphy-sd-index = <1>`) |
| MCLK | `CAM_CC_MCLK1_CLK` (`TLMM GPIO 95`, `19.2 MHz` / `0x124f800`) |
| Reset GPIO | `TLMM GPIO 125` (`RCAM2_RST_N`) |
| LDO Enable GPIO | `TLMM GPIO 33` (`RCAM2_LDO_EN`) |
| Power & Domain | `vddio` = `vreg_l7c_1p8` (`1.8 V`); `power-domains = <&camcc TITAN_TOP_GDSC>` |
| Orientation / Roll | `yaw = 180` (`orientation = <1>` Back), `roll = 90` (check `270` matching HI-847) |
| Mainline driver | None yet (must be implemented cleanly as an upstream-style V4L2 subdev driver) |

---

## 2. Current State

- `camcc_sm8250`, `qcom_camss`, and `hi847` are verified working on hardware (`fold-overlay-dt` `f3a5609d1`, telephoto HI-847 streaming `3256x2448` in `omacam`).
- `device/r8q/overlays/camera.dtso` only enables `camss` (currently wired for CSIPHY2/HI-847), `i2c10`, and `hi847`; QUP `&i2c2`, CSIPHY1 (`&camss` `port@1`), MCLK1, and S5K3L6 are not wired yet.
- Applying lessons from HI-847 (`device/r8q/CAMERA-PLAN.md`):
  - `TITAN_TOP_GDSC` power-domain + runtime PM active before `clk_prepare_enable()` on `MCLK1`.
  - Exact `vreg_l7c_1p8` (`CAM_VIO`) + `GPIO 33` (`ldo-enable`) + `GPIO 125` (`reset`) sequencing.
  - 0-based `data-lanes` + `MEDIA_BUS_TYPE_CSI2_DPHY` on `csiphy1` endpoint; accurate `.llp` / `HBLANK` / `pixel_rate` / `get_selection()` / `v4l2_fwnode_device_properties`.

---

## 3. Phased Bring-Up Plan

### 3.1 Phase 1: Isolated DT Overlay (`&i2c2` Probe Ready)

1. Verified in `sm8250.dtsi` and `sm8250-samsung-r8q.dts` (`2026-11-17`):
   - `&gpi_dma0` (`900000.dma-controller`) is already `status = "okay"`.
   - `qup_i2c2_default` (`gpio115`, `gpio116`, `function = "qup2"`) is defined in `sm8250.dtsi`.
   - Zero TLMM GPIO conflicts found in `sm8250-samsung-r8q.dts` for `33`, `95`, `115`, `116`, `125`.
2. Added `device/r8q/overlays/camera-uw-probe.dtso` enabling `&{/soc@0/geniqup@9c0000/i2c@988000}` (`i2c2`) isolated from `camss` / `CSIPHY1` for a safe one-change-per-boot Stage 1 bus check (`tools/r8q-boot --dual-boot --overlay camera-uw-probe`).
3. Next: add `cam_mclk1_default` (`gpio95`), `cam_uw_active` (`gpio33`, `gpio125`), and disabled `s5k3l6: camera@2d` + `&camss` `port@1` in `sm8250-samsung-r8q.dts` alongside the `s5k3l6` kernel driver.

### 3.2 Phase 2: Driver Implementation (`drivers/media/i2c/s5k3l6xx.c` + Binding — DONE 2026-11-17)

1. Added `drivers/media/i2c/s5k3l6xx.c` (`v4l2-cci`, `19.2/24/25 MHz` MCLK PLLs, `TITAN_TOP_GDSC` runtime PM before MCLK enable, `ldo-enable-gpios`, `get_selection`, `v4l2_fwnode_device_properties`, `4208x3120` / `2104x1560` / `1052x780` `SGRBG10_1X10` modes) and `Documentation/devicetree/bindings/media/i2c/samsung,s5k3l6xx.yaml` on kernel branch `camera-uw-wip`.
2. Enabled `CONFIG_VIDEO_S5K3L6XX=m` in `device/r8q/kernel.config` and added `device/r8q/overlays/camera-uw.dtso` (`&camss` `port@1`, `&i2c2`, `camera@2d`).

### 3.3 Phase 3 & 4: Hardware Probe, Raw Capture & `libcamera` Integration (PASSED — 2026-11-17)

1. Booted `recovery-cam-uw.img` (`7.3.0-rc6-omamobile-push1-g3667ab368ef9`, `--dual-boot --overlay camera-uw`) on `sda24`:
   - `s5k3l6xx 2-002d: identified Samsung S5K3L6 (id 0x30c6 rev 0xb0)` with zero QUP I2C or RPMh errors.
   - `media-ctl -d /dev/media0 -p` confirmed `s5k3l6xx 2-002d` -> `msm_csiphy1` alongside `hi847 10-0021` -> `msm_csiphy2`.
2. Raw V4L2 MIPI CSI-2 capture (`msm_csiphy1 -> msm_csid1 -> msm_vfe1_rdi0`, `pgAA`): streamed live frames at `2104x1560` (`4,118,400` bytes, `min=0 max=252 mean=20.25 stdev=27.24`) and full-resolution `4208x3120`.
3. Added `packages/libcamera/0059-libcamera-Add-the-Samsung-S5K3L6-sensor.patch` (`CameraSensorHelperS5k3l6xx` `code / 32` gain, `s5k3l6xx.yaml`, properties/delays), installed `libcamera 0.7.2-4.25`, and verified `cam -l` (2 back cameras, zero warnings) + `gst-launch-1.0 libcamerasrc` JPEG capture (`72K`).

---

## 4. Comparison with Working HI-847 (Telephoto)

| Property | HI-847 (Tele, Verified) | S5K3L6 (Ultrawide, Verified) |
|----------|-------------------------|------------------------------|
| Control bus | `&i2c10` (`a88000.i2c`, GPIO 129/130) | `&i2c2` (`988000.i2c`, GPIO 115/116) |
| 7-bit I2C addr | `0x21` | `0x2d` |
| CSI-PHY | CSIPHY2 | CSIPHY1 |
| MCLK | MCLK3 (GPIO 97, 19.2 MHz) | MCLK1 (GPIO 95, 19.2 MHz) |
| Reset / LDO GPIO | GPIO 78 / GPIO 135 | GPIO 125 / GPIO 33 |
| Driver status | ✅ `hi847.c` working | ✅ `s5k3l6xx.c` + `libcamera 0.7.2-4.25` working |

---

## 5. Related Files

| File | Purpose |
|------|---------|
| `device/r8q/CAMERA-PLAN.md` | Authoritative SM8250 camera architecture and HI-847 lessons |
| `device/r8q/board/missing/r8q-camera-upstream-reference.dtsi` | Upstream SM8250 camera DT reference |
| `device/r8q/board/missing/r8q-aperture-config.xml` | Camera aperture config reference |
