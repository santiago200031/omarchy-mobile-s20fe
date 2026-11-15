<!-- SPDX-License-Identifier: MIT -->

# Missing 04: Sensors, Cameras & DPMS Power Key

- **Priority:** P4 (`M5.1` UX & hardware subsystem follow-ups; `tools/r8q-install` alignment completed on branch `r8q-touch-and-fixes`).
- **Back:** [MISSING.md](../MISSING.md) · [Board Overview](../README.md)

## Completed in `r8q-touch-and-fixes`
- **Installer Alignment ([tools/r8q-install](../../../../tools/r8q-install)):** Updated `monitors.lua` to `mode = "1080x2400@60"` (`scale = 2.25`) and removed invalid `vfr = true` under `misc` in `looknfeel.lua` (`misc.vrr = 0`, matching `hyprland-r8q.conf`).

## Remaining Work
1. **DPMS & Lock Screen (`M5.1`):** Verify `LockGlance.qml` backlight wake and Power-key DPMS off/on (`ae94000.dsi.0`) safely without triggering `s2idle`, before re-enabling `idle.lock`.
2. **Sensors & Cameras:** Validate SLPI-backed sensors (`M5.1`) after `remoteprocs.dtso`, then evaluate camera clocks (`camcc_sm8250`, currently blacklisted in `/etc/modprobe.d/omamobile-r8q-bringup.conf`) and UFS unmount/storage (`udisks2`, currently masked).

## Camera (2026-11-15): moved to [CAMERA-PLAN.md](../../CAMERA-PLAN.md)

The camera reading in this card was wrong: the camera modules were already
built and installed (`camcc-sm8250.ko`, `i2c-qcom-cci.ko`, `qcom-camss.ko`,
`hi847.ko`, `s5kgn1.ko`, `s5kjn1.ko`), and `camcc_sm8250` is blacklisted on
purpose (boot-time clock/RPMh churn, [failed/06](../failed/06-boot-and-session-hazards.md)).
The real gaps are the device tree (CAMSS, the CCI buses and every sensor node
were disabled or absent) and the sensor drivers: this board's cameras are a
Sony IMX555 main, a Samsung S5K3L6 ultrawide, an SK Hynix HI-847 telephoto and
a Sony IMX616 front, of which only the HI-847 has a driver in the tree.

Kernel side is implemented and built: the HI-847 driver now works from device
tree (regulators, reset and LDO-enable GPIOs, runtime PM), `sm8250-samsung-r8q.dts`
describes CAMCC's CAM_VIO rail, CAMSS/CSIPHY2 and the telephoto sensor, and
[overlays/camera.dtso](../../overlays/camera.dtso) enables them for one boot.
The staged tests are the gated runbook in [CAMERA-PLAN.md](../../CAMERA-PLAN.md)
§5; the other three sensors are follow-ups (§6).
