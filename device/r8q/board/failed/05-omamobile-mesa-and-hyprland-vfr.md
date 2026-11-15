<!-- SPDX-License-Identifier: MIT -->

# Failed 05: `[omamobile]/mesa` (Mali-only), `misc.vfr = true` & `@120` Bring-Up Mode

- **Symptom:** `Hyprland` crashed (`SIGABRT`, `EGL_NOT_INITIALIZED`), and after Freedreno install, Hyprland showed a `looknfeel.lua:1` red unknown-option banner (`vfr`).
- **Back:** [FAILED.md](../FAILED.md) · [Board Overview](../README.md)

## Root Cause
- **`[omamobile]/mesa 1:26.2.4-1.4`:** Built for Pixel Mali (`panfrost` only) and omits `/usr/lib/dri/msm_dri.so`.
- **`misc.vfr = true`:** In Hyprland's Lua schema (`/usr/share/hypr/stubs/hl.meta.lua`), `misc.vfr` does not exist (`vfr` lives under `debug`, while `misc` takes `vrr`), breaking `looknfeel.lua`.
- **`1080x2400@120` default:** Higher refresh rate stressed early DSI CMD / TE compositor pacing before Adreno tuning.

## Permanent Rule
- Install Arch Linux ARM `extra/mesa`, `extra/vulkan-mesa-implicit-layers`, and `extra/vulkan-freedreno` (sync RTC via `date -u -s @$(date +%s)` before `pacman-key` if clock is behind).
- Remove `vfr` from `misc`; use `misc.vrr = 0` and `DSI-1` `1080x2400@60` (`hyprland-r8q.conf` and `tools/r8q-install` updated on `r8q-touch-and-fixes`).
