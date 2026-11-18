<!-- SPDX-License-Identifier: MIT -->

# Column: Failed / Skip (Do Not Repeat)

Every failed approach, root cause, and permanent mitigation learned during `r8q` bring-up.

| Card | Failure Mode | Permanent Rule |
| --- | --- | --- |
| [01 Unreserved XPU/TZ/Knox DRAM](failed/01-unreserved-xpu-tz-knox-dram.md) | `mmc0` ADMA (`0x90800210`) / buddy allocators faulted TZ XPU -> `t+12..14s` freeze | Keep all 10 Samsung `no-map` regions in DT (`1324fe066`) + `offline_unreserved_xpu_pages()` fallback |
| [02 Suspend, `cpuidle`, `schedutil` & UFS PM](failed/02-suspend-cpuidle-dvfs-ufspm.md) | `s2idle` / `state1` / DVFS / UFS gating hang cores or trip RPMh `-110` | Keep `SUSPEND` off, `WFI`-only cpuidle, CPU `performance`, UFS PM disabled |
| [03 `max77705` Built-in & Unbind](failed/03-max77705-builtin-and-unbind.md) | `=y` freezes boot at `0.95s`; `rmmod max77705_charger` unbinds `max17042_battery` | Keep `=m` loaded after boot and pin `9c0000.geniqup` + `980000.i2c` `power/control="on"` |
| [04 Protected DISPCC Clocks & White Flash](failed/04-display-protected-clocks-and-white-flash.md) | `display-mdss.dtso` gave `ESC0_CLK=0` (`-22`); unclocked FB caused white flash | Clear `&dispcc` `protected-clocks` (`display.dtso`) and hold 9 clocks + `MDSS_GDSC` on FB |
| [05 `[omamobile]/mesa`, `misc.vfr` & `@120`](failed/05-omamobile-mesa-and-hyprland-vfr.md) | Mali Mesa lacked `msm_dri.so`; `misc.vfr` threw `looknfeel.lua:1` error | Use `extra/mesa` + `extra/vulkan-freedreno`, `misc.vrr = 0`, `1080x2400@60` |
| [06 Boot & Session Hazards](failed/06-boot-and-session-hazards.md) | Unpaused `&uart6` / `-19` alias, `btmgmt` hang, `1975` RTC cycle, `sddm` on `multi-user.target`, `SIGPIPE` loop, `scp` | Pause `sync_state()`/`caa385175`, `--timeout`, `sysinit.target` RTC, link `sddm`, direct sysfs read, `r8q-push` |
| [07 `stmfts5` 8-Byte vs `fts5cu56a` 16-Byte Events](failed/07-stmfts5-8byte-vs-samsung-16byte-events.md) | `stmfts5` split 16-byte Samsung Type-B touch events -> `unknown FTS5 event 0x80`/`0x1` | Parse `0x60`/`0x61` 16-byte Type-B packets (`5866f774c` / `stmfts-r8q.ko`) and pin `994000.i2c` `power/control="on"` |

| [08 Vulkan `llama.cpp` Adreno GPU Wedge](failed/08-vulkan-llm-adreno-gpu-wedge.md) | `-DGGML_VULKAN=ON` `llama.cpp` model load -> `device lost`, `hangcheck gpu lockup`, `a6xx_recover: cx gdsc didn't collapse`; wedges the GPU and takes `Hyprland` down | Keep `GPU_LAYERS=0` (CPU); treat that combo as a reboot-required wedge; Vulkan *compute* wedges, render is fine |

Back to [Board Overview](README.md).
