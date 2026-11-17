<!-- SPDX-License-Identifier: MIT -->

# Failed 04: Protected DISPCC Clocks (`display-mdss.dtso`) & Unclocked FB White Flash

- **Symptom:** Either an immediate bright white AMOLED flash during early boot, or `/dev/dri/renderD128` probing while `dsi_7nm_phy_enable()` failed with `-22` (`-EINVAL`) and DPU vblank timeouts (`panel-dead`).
- **Back:** [FAILED.md](../FAILED.md) · [Board Overview](../README.md)

## Root Cause
- **White flash:** Without explicit clock + genpd references on `framebuffer@9c000000`, `clk_disable_unused` / genpd power-off cut ABL's live scanout clocks. (Note: before `msm` loads, `simpledrm` still shows a multi-column split image because ABL scans out 2-LM DSC while `simpledrm` writes linear `1080*4` stride.)
- **`-22` (`dsi_7nm_phy_enable`):** `display-mdss.dtso` left `&dispcc` `protected-clocks` intact, so `clk_get_rate(msm_host->esc_clk)` returned `0` for `DISP_CC_MDSS_ESC0_CLK` inside `msm_dsi_dphy_timing_calc_v4()`.

## Permanent Rule
- Never use `display-mdss.dtso` or `display-clocks.dtso`; use `device/r8q/overlays/display.dtso` with `protected-clocks` removed completely in `sm8250-samsung-r8q.dts`.
- **Why `display-clocks.dtso` (`protected-clocks = [];`) breaks DSI:** In `drivers/of/property.c`, `of_prop_next_u32()` does not check `prop->length >= sizeof(__be32)` when `cur == NULL`, so a 0-byte `protected-clocks;` property causes `qcom_cc_drop_protected()` to read 4 out-of-bounds bytes (`3` = `DISP_CC_MDSS_BYTE0_CLK_SRC`), dropping `disp_cc_mdss_byte0_clk_src` from `dispcc` and leaving `disp_cc_mdss_byte0_clk` orphaned (`disp_cc_mdss_byte0_clk status stuck at 'off'`, black screen after console handoff).
- Keep all 9 MDSS/DISPCC clocks + `MDSS_GDSC` referenced on `framebuffer@9c000000` in `sm8250-samsung-common.dtsi`.
