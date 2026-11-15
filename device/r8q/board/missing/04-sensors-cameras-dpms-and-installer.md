<!-- SPDX-License-Identifier: MIT -->

# Missing 04: Sensors, Cameras & DPMS Power Key

- **Priority:** P4 (`M5.1` UX & hardware subsystem follow-ups; `tools/r8q-install` alignment completed on branch `r8q-touch-and-fixes`).
- **Back:** [MISSING.md](../MISSING.md) · [Board Overview](../README.md)

## Completed in `r8q-touch-and-fixes`
- **Installer Alignment ([tools/r8q-install](../../../../tools/r8q-install)):** Updated `monitors.lua` to `mode = "1080x2400@60"` (`scale = 2.25`) and removed invalid `vfr = true` under `misc` in `looknfeel.lua` (`misc.vrr = 0`, matching `hyprland-r8q.conf`).

## Remaining Work
1. **DPMS & Lock Screen (`M5.1`):** Verify `LockGlance.qml` backlight wake and Power-key DPMS off/on (`ae94000.dsi.0`) safely without triggering `s2idle`, before re-enabling `idle.lock`.
2. **Sensors & Cameras:** Validate SLPI-backed sensors (`M5.1`) after `remoteprocs.dtso`, then evaluate camera clocks (`camcc_sm8250`, currently blacklisted in `/etc/modprobe.d/omamobile-r8q-bringup.conf`) and UFS unmount/storage (`udisks2`, currently masked).
