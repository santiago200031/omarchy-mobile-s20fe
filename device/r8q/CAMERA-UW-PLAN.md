<!-- SPDX-License-Identifier: MIT -->

# Ultrawide Camera Bring-Up — Samsung Galaxy S20 FE (r8q)

**Sensor:** Samsung S5K3L6 (rear ultrawide, cell 2)
**Status:** Stock DT wiring extracted (`live.dts`); no mainline sensor driver in kernel tree
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

### 3.2 Phase 2: Driver Implementation (`drivers/media/i2c/s5k3l6.c` + Binding)

1. Implement a clean GPL-2.0 V4L2 subdev sensor driver (runtime PM, `vddio` / LDO enable / reset / MCLK1, `get_selection`, `v4l2_fwnode_device_properties`, `HBLANK`/`VBLANK`/exposure/analogue gain controls for `libcamera`).
2. Add YAML DT binding under `Documentation/devicetree/bindings/media/i2c/` and validate with `tools/kernel-pr-check --build`.

### 3.3 Phase 3: Hardware Probe & I2C Identification (Attended)

```bash
# Resolve dynamic Linux i2c bus number for 988000.i2c (do not hardcode i2c-2)
tools/phone-ssh 'grep -H "" /sys/bus/i2c/devices/i2c-*/name'
```

1. Stream `dmesg -w` over USB first; verify chip ID read at `0x2d` succeeds with zero QUP I2C timeouts or RPMh stalls.
2. Verify `media-ctl -p` shows `s5k3l6 -> csiphy1 -> csid* -> vfe*` links cleanly alongside `hi847`.

### 3.4 Phase 4: Raw V4L2 Capture & `libcamera` Integration

1. Capture raw Bayer frames with `v4l2-ctl` / `media-ctl`.
2. Add `CameraSensorHelperS5k3l6` / tuning stub in `packages/libcamera` if needed, and verify in `omacam`.

---

## 4. Comparison with Working HI-847 (Telephoto)

| Property | HI-847 (Tele, Verified) | S5K3L6 (Ultrawide, Pending) |
|----------|-------------------------|-----------------------------|
| Control bus | `&i2c10` (`a88000.i2c`, GPIO 129/130) | `&i2c2` (`988000.i2c`, GPIO 115/116) |
| 7-bit I2C addr | `0x21` | `0x2d` |
| CSI-PHY | CSIPHY2 | CSIPHY1 |
| MCLK | MCLK3 (GPIO 97, 19.2 MHz) | MCLK1 (GPIO 95, 19.2 MHz) |
| Reset / LDO GPIO | GPIO 78 / GPIO 135 | GPIO 125 / GPIO 33 |
| Driver status | ✅ `hi847.c` working | ❌ No mainline driver yet |

---

## 5. Related Files

| File | Purpose |
|------|---------|
| `device/r8q/CAMERA-PLAN.md` | Authoritative SM8250 camera architecture and HI-847 lessons |
| `device/r8q/board/missing/r8q-camera-upstream-reference.dtsi` | Upstream SM8250 camera DT reference |
| `device/r8q/board/missing/r8q-aperture-config.xml` | Camera aperture config reference |
