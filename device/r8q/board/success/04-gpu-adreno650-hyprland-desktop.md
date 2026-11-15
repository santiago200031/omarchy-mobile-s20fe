<!-- SPDX-License-Identifier: MIT -->

# Success 04: Adreno 650, Freedreno & Hyprland Graphical Desktop

- **Status:** Gate D passed (`2026-11-15`), confirmed on screen by the user and verified across `495 s+` active GUI uptime.
- **Back:** [SUCCESS.md](../SUCCESS.md) · [Board Overview](../README.md)

## What Works
- **Adreno 650 GPU & firmware:** `qcom/a650_sqe.fw` (`0x112`), `qcom/a650_gmu.bin` (`v2.1.8`), and `a650_zap.mbn` (`0x9031a000`) probe cleanly.
- **Devfreq & PM pinning (`80-r8q-gpu.rules` `0.1.16`):** `3d00000.gpu` pinned to `305000000` Hz (`min_freq` + `max_freq`, `simple_ondemand`), with `power/control="on"` for `3d00000.gpu` and `3d6a000.gmu`.
- **Userspace 3D stack:** Arch Linux ARM `extra/mesa 1:26.2.7-1`, `extra/vulkan-mesa-implicit-layers 1:26.2.7-1`, and `extra/vulkan-freedreno 1:26.2.7-1` provide `msm_dri.so` / Turnip / Freedreno.
- **Omarchy Wayland desktop (`alarm`, UID 1000):** `sddm` + `uwsm` + `Hyprland --watchdog-fd 4` + `quickshell -n -p /usr/share/omarchy/shell` + `squeekboard` + `Xwayland`, with `hyprland-r8q.conf` (`1080x2400@60`, `misc.vrr = 0`, blur/shadows/animations off).
- **Input:** `stmfts` touchscreen (`i2c5@49`, `stmfts_irq`), `gpio_keys` (`VolUp`/`VolDown`), and `qpnp_pon` Power key live and responsive.
