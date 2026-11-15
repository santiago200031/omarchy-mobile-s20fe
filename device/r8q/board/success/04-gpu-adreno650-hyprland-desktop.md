<!-- SPDX-License-Identifier: MIT -->

# Success 04: Adreno 650, Freedreno & Hyprland Graphical Desktop

- **Status:** Gate D passed (`2026-11-15`); touchscreen 16-byte Type-B support fixed live (`stmfts-r8q.ko`) and committed in kernel `5866f774c`.
- **Back:** [SUCCESS.md](../SUCCESS.md) · [Board Overview](../README.md)

## What Works
- **Adreno 650 GPU & firmware:** `qcom/a650_sqe.fw` (`0x112`), `qcom/a650_gmu.bin` (`v2.1.8`), and `a650_zap.mbn` (`0x9031a000`) probe cleanly.
- **Devfreq & PM pinning (`80-r8q-gpu.rules` `0.1.18`):** `3d00000.gpu` pinned to `305000000` Hz (`min_freq` + `max_freq`, `simple_ondemand`), with `power/control="on"` for `3d00000.gpu`, `3d6a000.gmu`, `9c0000.geniqup`, `980000.i2c`, `994000.i2c`, `998000.serial`, and `5-0049`.
- **Userspace 3D stack:** Arch Linux ARM `extra/mesa 1:26.2.7-1`, `extra/vulkan-mesa-implicit-layers 1:26.2.7-1`, and `extra/vulkan-freedreno 1:26.2.7-1` provide `msm_dri.so` / Turnip / Freedreno.
- **Omarchy Wayland desktop (`alarm`, UID 1000):** `sddm` (enabled for `multi-user.target` so it starts automatically even with `systemd.unit=multi-user.target`) + `uwsm` + `Hyprland --watchdog-fd 4` + `quickshell -n -p /usr/share/omarchy/shell` + `squeekboard` + `Xwayland`, with `hyprland-r8q.conf` (`1080x2400@60`, `misc.vrr = 0`, blur/shadows/animations off) and numeric lock-screen PIN `1234` (`tools/r8q-install`).
- **Touch & Input:** `5-0049` (`fts5cu56a` on `i2c5` / `994000.i2c`) rebound live from built-in `stmfts` to `stmfts-r8q.ko` (16-byte Type-B `0x60`/`0x61` parser; permanently integrated in `wt-r8q` commit `5866f774c`), `Hyprland` bound to `Touch Device: stmfts` (`/dev/input/event2`), plus `gpio_keys` (`VolUp`/`VolDown`) and `qpnp_pon` Power key active.
