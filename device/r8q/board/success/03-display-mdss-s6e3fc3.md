<!-- SPDX-License-Identifier: MIT -->

# Success 03: Early Framebuffer -> MSM KMS & S6E3FC3 Panel

- **Status:** Gate A & Gate B passed on `SM-G780G` (`7.3.0-rc6-omamobile-push1-gd469bd3bf2a9`).
- **Back:** [SUCCESS.md](../SUCCESS.md) · [Board Overview](../README.md)

## What Works
- **Zero white flash (`simpledrm` handoff):** `framebuffer@9c000000` in `sm8250-samsung-common.dtsi` holds all 9 MDSS/DISPCC clocks plus `power-domains = <&dispcc MDSS_GDSC>`, paired with `vreg_disp_avdd` (`gpio61`) and `vreg_disp_vdd3` (`gpio93`) `regulator-always-on` / `regulator-boot-on`.
- **MSM DPU/DSI handover (`display.dtso`):** Dropping `&dispcc` `protected-clocks` lets `msm_dsi_host_get_phy_clk_req` read `DISP_CC_MDSS_ESC0_CLK` (`19.2 MHz`) so `dsi_7nm_phy_enable` succeeds cleanly.
- **Panel & Backlight:** `panel-samsung-s6e3fc3` (`ae94000.dsi.0`, matching `ss_dsi_panel_S6E3FC3_AMS646YD01_FHD` timings) takes over smoothly from ABL, registering `fb0: msmdrmfb` and `/sys/class/backlight/ae94000.dsi.0`.

## Evidence
- `modprobe msm` = `0`, `dpu hardware revision:0x60000000`, `/dev/dri/card1` + `/dev/dri/renderD128`.
- `hyprctl monitors`: `DSI-1` active at `1080x2400@60.00000 at 0x0`, `dpmsStatus: 1`, `dsi_isr` and `msm` IRQs advancing smoothly.
