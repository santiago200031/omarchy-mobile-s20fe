<!-- SPDX-License-Identifier: MIT -->

# Ultrawide Camera Bring-Up — Samsung Galaxy S20 FE (r8q)

**Sensor:** Samsung S5K3L6 (rear ultrawide, cell 2)
**Status:** Research phase; no mainline sensor driver in kernel tree
**Last Updated:** 2026-11-17

---

## 1. Sensor Details (from `device/r8q/CAMERA-PLAN.md`)

| Property | Value |
|----------|-------|
| Role | Rear ultrawide (cell 2) |
| Model | Samsung S5K3L6 (12 MP) |
| Control bus | QUP I2C2 (vendor `qupv3_se2`, GPIO115/116) |
| I2C Address (7-bit) | `0x2d` (stock DT 8-bit `slave-addr = 0x5A`) |
| CSI-PHY | CSIPHY1 |
| MCLK | MCLK1, 19.2 MHz |
| Mainline driver | None (must be written / ported cleanly as an upstream-style V4L2 subdev driver) |

---

## 2. Current State

- `camcc_sm8250`, `qcom_camss`, and `hi847` are proven working on hardware (`camera-wip` `7ca90963c`, telephoto HI-847 streaming 3264×2448 in `omacam`).
- `device/r8q/overlays/camera.dtso` only enables `camss` (currently wired for CSIPHY2/HI-847), `i2c10`, and `hi847`; it does **not** yet enable QUP I2C2, CSIPHY1, MCLK1, or S5K3L6.
- Because S5K3L6 sits on a QUP I2C bus (like HI-847 on QUP I2C10), the `CAMERA-PLAN.md` findings apply: exact stock `CAM_VIO` / analog / digital rail mapping, GPIO mux/reset/power-enable sequencing, runtime PM before `clk_prepare_enable()` on `MCLK1`, `TITAN_TOP_GDSC` power domain, and 0-based `data-lanes` + `MEDIA_BUS_TYPE_CSI2_DPHY` on `csiphy1_ep` all need verification against stock DT.

---

## 3. Phased Bring-Up Plan

### 3.1 Phase 1: Stock DT Extraction &DT Node (Isolated Overlay)

1. Extract exact S5K3L6 properties from local vendor `live.dts` (never commit vendor dumps): I2C2 SE address, GPIO115/116 pinctrl, reset/LDO-enable GPIOs and polarities, `vreg_*` rail mapping, CSIPHY1 lane polarity/count, and link frequencies.
2. Add the QUP I2C2 / CSIPHY1 / MCLK1 pinctrl and disabled `s5k3l6: camera@2d` node to `sm8250-samsung-r8q.dts` (or keep isolated in `device/r8q/overlays/camera-uw.dtso` until probed).

### 3.2 Phase 2: Driver Implementation (`drivers/media/i2c/s5k3l6.c` + Binding)

1. Implement a clean GPL-2.0 V4L2 subdev sensor driver (runtime PM, DT rails/GPIOs/clocks, `get_selection`, `v4l2_fwnode_device_properties` for orientation/rotation, V4L2 CID controls for HBLANK/VBLANK/exposure/analogue gain matching `libcamera`).
2. Add YAML DT binding under `Documentation/devicetree/bindings/media/i2c/` and validate with `tools/kernel-pr-check --build`.

### 3.3 Phase 3: Hardware Probe & I2C Identification (Attended)

```bash
# Resolve dynamic Linux i2c bus number for QUP2 (do not hardcode i2c-2)
tools/phone-ssh 'grep -H "" /sys/bus/i2c/devices/i2c-*/name'
```

1. Probe with `dmesg -w` streaming over USB first; confirm chip ID read at `0x2d` succeeds with zero I2C timeouts or RPMh stalls.
2. Verify `media-ctl -p` shows `s5k3l6 -> csiphy1 -> csid* -> vfe*` links cleanly.

### 3.4 Phase 4: Raw V4L2 Capture & `libcamera` Integration

1. Capture raw Bayer frames with `v4l2-ctl` / `media-ctl`.
2. Add `CameraSensorHelperS5k3l6` and tuning stub in `packages/libcamera` if not already recognized, and verify in `omacam`.

---

## 4. Comparison with Working HI-847 (Telephoto)

| Property | HI-847 (Tele, Verified) | S5K3L6 (Ultrawide, Pending) |
|----------|-------------------------|-----------------------------|
| Control bus | QUP I2C10 (GPIO129/130) | QUP I2C2 (GPIO115/116) |
| 7-bit I2C addr | `0x21` | `0x2d` |
| CSI-PHY | CSIPHY2 | CSIPHY1 |
| MCLK | MCLK3 (19.2 MHz) | MCLK1 (19.2 MHz) |
| Driver status | ✅ `hi847.c` working | ❌ No mainline driver yet |

---

## 5. Related Files

| File | Purpose |
|------|---------|
| `device/r8q/CAMERA-PLAN.md` | Authoritative SM8250 camera architecture and HI-847 lessons |
| `device/r8q/board/missing/r8q-camera-upstream-reference.dtsi` | Upstream SM8250 camera DT reference |
| `device/r8q/board/missing/r8q-aperture-config.xml` | Camera aperture config reference |
