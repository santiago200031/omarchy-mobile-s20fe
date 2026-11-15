<!-- SPDX-License-Identifier: MIT -->

# Missing 04: Sensors, Cameras, DPMS Power Key & Installer Alignment

- **Priority:** P4 (`M5.1` UX & repo tooling cleanup).
- **Back:** [MISSING.md](../MISSING.md) · [Board Overview](../README.md)

## Remaining Work
1. **DPMS & Lock Screen (`M5.1`):** Verify `LockGlance.qml` backlight wake and Power-key DPMS off/on (`ae94000.dsi.0`) safely without triggering `s2idle`, before re-enabling `idle.lock`.
2. **Installer Alignment ([tools/r8q-install](../../../../tools/r8q-install)):** Update lines `98` and `111-116` so fresh installs write `mode = "1080x2400@60"` and remove invalid `vfr = true` under `misc` in `looknfeel.lua` (also install `extra/mesa` + `extra/vulkan-freedreno`).
3. **Sensors & Cameras:** Validate SLPI-backed sensors (`M5.1`) after `remoteprocs.dtso`, then evaluate camera clocks (`camcc_sm8250`, currently blacklisted in `/etc/modprobe.d/omamobile-r8q-bringup.conf`) and UFS unmount/storage (`udisks2`, currently masked).
