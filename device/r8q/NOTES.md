<!-- SPDX-License-Identifier: MIT -->

# Omarchy Mobile — `r8q` (Galaxy S20 FE SM-G780G) Bring-Up Notes & Postmortems

Chronological diagnostic archive split out of [PORT-PLAN.md](PORT-PLAN.md) so the main port plan stays short and actionable.

---

## 1. Kernel Fix for the RPMh / RCU Wedge (Crashes #1–#14, 2026-11-12 – 2026-11-14)

### Symptom & Bisection Timeline
- **Crashes #1–#2 (`schedutil`):** System hung ~11 min after boot (`rpmh_write_batch -110` in `sugov`) due to OPP transitions timing out over active-mode AMC `RPMh` votes. Pinning all CPU policies to `performance` eliminated the `sugov` trigger.
- **Crashes #3–#6 (Wi-Fi-off bisect):** Proved UFS clock ungating (`ufshcd_ungate_work -> ufs_qcom_icc_update_bw -> icc_set_bw -> rpmh_write_batch -110`) hit the same voter even during idle background journal/SSH I/O. With `performance` + `cpuidle` `state1` off + UFS `auto_hibern8=0` + `clkscale_enable=0`/`clkgate_enable=0` (`0.1.7`), idle survived past 17 min.
- **Crashes #7–#11:** Crashes returned even with `0.1.7` mitigations live (`700 s` stalls in pstore; NMI-dead cores across little CPU 1, big CPUs 4/6, and prime CPU 7 with zero kernel error lines).
- **Crash #13 (2026-11-13, `cpu7=0` soak):** Offlining CPU 7 delayed idle failure to `1757 s` (`29.3 min`), then crashed at `1768–1780 s` (`watchdog0: pretimeout event`, RCU stalls across CPUs 1, 4, 6) seconds after a 156 MiB UFS writeback began over USB.
- **Crash #14 (2026-11-13, last on old kernel):** Dual-boot RO (`sda*` untouched) set a **69 min** idle record with zero stalls, then died **~57 min in** during chunk 5/7 of a 12 GB rootfs writeback to the microSD over USB NCM (chunks 0–4 verified; chunk 5 left at `492,466,176 B`, prompting chunk resume support in `tools/r8q-push`).

### Root Cause: TCS Leak in `rpmh-rsc.c`
A resource state change claims a TCS slot, and the completion IRQ is the only path that reclaims it. On a `10 s` completion timeout (`RPMH_TIMEOUT_MS`), the driver returned `-ETIMEDOUT` while **keeping** the TCS claimed. Subsequent `rpmh_rsc_send_data()` callers then spun forever with IRQs disabled waiting for a free TCS—invisible to softlockup detectors and silent in pstore.

Four commits on `omamobile-r8q` (`~/omamobile-kernel/wt-r8q`), adapted from Qualcomm's upstream series *"[PATCH v4 0/3] Output debug information from RSC"*:
- `a807c8708`: `soc: qcom: cmd-db: export RPMh accelerator type string helper` (Maulik Shah)
- `df727e3ec`: `soc: qcom: cmd-db: add reverse address-to-name lookup` (Maulik Shah)
- `ebeabc490`: `soc: qcom: rpmh-rsc: Output debug information from RSC` (Raju P.L.S.S.S.N)
- `e96cf40a1` (fixing `d1f598f2c`): `soc: qcom: rpmh-rsc: bound the TCS claim wait and reclaim stuck TCSes` (`RPMH_TCS_CLAIM_TIMEOUT_MS=10000`, `rpmh_rsc_reclaim_timeout()` on all three timeout paths, lock-safe `rpmh_rsc_dump_state()`)

Verified booted on `7.3.0-rc6-omamobile-push1-ge96cf40a1ee9` (2026-11-14) alongside `omamobile-r8q-liveness.service` (FIFO-99 per-CPU heartbeat + monitor).

---

## 2. The 0.630 s Boot Freeze & Heimdall Flash Verification Rule (2026-11-14)

- **Symptom:** The first RPMh-fix kernel build appeared to freeze on screen at `0.630 s` right after `input: gpio-keys as ... input3`, before `Run /init as init process`.
- **Two causes uncovered:**
  1. **Lock leak in `d1f598f2c`:** `wait_event_lock_irq_timeout()` re-acquires `drv->lock` before returning on timeout; an early `return -ETIMEDOUT` left `drv->lock` held with IRQs disabled, hanging the UFS async interconnect sync-state vote immediately after `0.630 s`. Fixed in `e96cf40a1` by releasing the lock cleanly on timeout and testing `tcs_id`.
  2. **Stale flash on `sda24`:** `dd if=/dev/sda24` + `uname -r` revealed the *previous* image (`...g69c7b002164a`) was still in `sda24` after an earlier Heimdall attempt.
- **Permanent rule:** Never trust a flash from exit code alone; verify `uname -r` after boot matches the freshly packed kernel release. Also keep in mind `CONFIG_HARDLOCKUP_DETECTOR` / `CONFIG_ARM64_PSEUDO_NMI` are unset, so IRQs-off spins print nothing until pseudo-NMI watchdog is tested separately (M2.8).

---

## 3. `pacman-key` Failure from RTC Clock Skew (2026-11-14)

- `pacman -U` failed with `key "54A3D15B…DAE7638" is unknown` / `keyring is not writable` because `/etc/pacman.d/gnupg` was uninitialized AND the phone's RTC was ~4 days behind (making freshly signed keys appear created in the future, causing `gpg` to discard the user ID as `new key but contains no user ID - skipped`).
- **Fix:** Sync the phone clock from the laptop (`date -u -s @$(date +%s)`) before `pacman-key --init` / `--add` / `--lsign-key` or `pacman -U` operations.

---

## 4. Display & GPU Bring-Up: White Flash, DSI `-22`, and Mesa Driver Mismatch (2026-11-14 – 2026-11-15)

### A. Early `simpledrm` White-Flash -> Fixed
- **Failure:** Without explicit clock and power-domain references, common clock / genpd init gated unused display clocks while ABL had left the bootloader framebuffer live (`0x9c000000`), flashing the panel bright white or causing downstream failures.
- **Fix:** `framebuffer@9c000000` in `sm8250-samsung-common.dtsi` holds all 9 MDSS/DISPCC clocks plus `power-domains = <&dispcc MDSS_GDSC>`. Combined with `vreg_disp_avdd` (`gpio61`) and `vreg_disp_vdd3` (`gpio93`) marked `regulator-always-on` / `regulator-boot-on`, early `simpledrm` boots with zero white flash (note: before `msm` loads, `simpledrm` displays a multi-column split artifact because ABL leaves the panel in 2-LM DSC command mode while `simpledrm` writes uncompressed `1080*4` stride into `0x9c000000`).

### B. `dsi_7nm_phy_enable -22` -> Traced & Fixed (`display.dtso`)
- **Failure:** Initial probe using `display-mdss.dtso` (which left `&dispcc`'s `protected-clocks` intact) created `/dev/dri/renderD128` but failed KMS at `dsi_7nm_phy_enable()` -> `msm_dsi_dphy_timing_calc_v4()` returning `-22` (`-EINVAL`) followed by DPU vblank timeouts and `panel-dead`.
- **Root cause:** `dsi_calc_clk_rate_6g()` queried `clk_get_rate(msm_host->esc_clk)`, which returned `0` while `DISP_CC_MDSS_ESC0_CLK` remained protected in `&dispcc`.
- **Fix (`7.3.0-rc6-omamobile-push1-gd469bd3bf2a9`):** Removed `protected-clocks` on `&dispcc` in `sm8250-samsung-r8q.dts` and used `device/r8q/overlays/display.dtso`. Panel timings were verified identical to downstream `ss_dsi_panel_S6E3FC3_AMS646YD01_FHD` (`hfp=80, hsw=84, hbp=88; vfp=15/16, vsw=2, vbp=2`).
- **Live Verification (2026-11-15, Gate A + Gate B):**
  - Gate A booted `gd469bd3bf2a9` with zero white flash, 77 UFS partitions `ro=1`, `r8q-preflight --json` reporting `gpu_ready=true`, `audio_ready=true`, all 8 firmware checks `ok=true` (`a650_sqe.fw` `0x112`, `a650_gmu.bin` v2.1.8).
  - Gate B (`modprobe msm`): returned `0`, `dpu hardware revision:0x60000000`, `Initialized msm 1.13.0 for ae01000.display-controller on minor 1`, `fb0: msmdrmfb frame buffer device` (`135x150` console), `panel-samsung-s6e3fc3 ae94000.dsi.0: taken over from the bootloader`, `/dev/dri/card1` + `/dev/dri/renderD128`, `card1-DSI-1` `1080x2400@120` + `60`, `/sys/class/backlight/ae94000.dsi.0` (`511/1023`), GPU devfreq `305000000` Hz, and `stmfts` touch + GPIO volume keys responsive.

### C. Gate D Session Launch: `[omamobile]/mesa` (Mali-only) -> `extra/mesa` Freedreno + Adreno 3D Hang Isolation (2026-11-15)
- **Rootfs state on `/dev/loop0` (`/omarchy-r8q.img`):** All Omarchy packages were pre-installed (`hyprland 0.57.0-3.3`, `quickshell 0.3.2`, `omamobile-shell 0.2.127-1`, `sddm 0.21.0-7`, `uwsm`), `/home` was empty, and `/proc/cmdline` held `systemd.unit=multi-user.target` (explaining why only the Linux console was visible on screen).
- **Mesa discovery:** Provisioned user `alarm` (`uid=1000`), `monitors.lua` (`DSI-1` `1080x2400@120` @ `2.25`), and `sddm` autologin. On launch, `Hyprland` aborted (`SIGABRT`) with `EGL_NOT_INITIALIZED (0x12289): DRI2: failed to load driver` on `/dev/dri/card1`. Inspection showed `[omamobile]/mesa 1:26.2.4-1.4` only builds `panfrost` (Pixel Mali) and omits `/usr/lib/dri/msm_dri.so`.
- **Freedreno unblock:** Installed Arch Linux ARM `extra/mesa 1:26.2.7-1`, `extra/vulkan-mesa-implicit-layers 1:26.2.7-1`, and `extra/vulkan-freedreno 1:26.2.7-1` (PGP-verified offline against key `68B3537F39A313B3E574D06777193F152BDBE6A6` after syncing UTC clock). Restarting `seatd` + `sddm` launched `Hyprland --watchdog-fd 4`, `quickshell -n -p /usr/share/omarchy/shell`, `squeekboard`, `Xwayland`, `omamobile-doubletap`, `omamobile-osk-guard`, and `omamobile-sleepd` cleanly at `t+8s`.
- **Initial `t+12..14s` hang (`gd469bd3bf2a9`):** At `t+14s` as 3D rendering started, SSH banner exchange stalled before a watchdog reset (`multi-user.target` on cmdline kept recovery safe). See §5 below for the full pstore root cause and resolution.

---

## 5. Gate D Unblocked: Unreserved XPU/TZ/Knox Memory, RPMh I2C Churn, and Hyprland Lua Fixes (2026-11-15)

### A. Root Cause of the `t+12..14s` Freeze (`console-ramoops-0`)
- `console-ramoops-0` on `7.3.0-rc6-omamobile-push1-gd469bd3bf2a9` showed `mmc0: Timeout waiting for hardware interrupt` with `ADMA Ptr: 0x90800210` (`0x90400000..0x90a00000` is TrustZone-protected `cdsp_secure_heap` XPU memory) followed by `rcu_preempt` stalls on CPUs `0,2,3,6,7` and `rpmh_write_batch` `-110` on `HW IRQ 37 pending at GIC` because CPU 0 wedged on the XPU violation.
- Diffing live `/proc/iomem` against Samsung `r8q` downstream DTB/DTBO exposed **nine unreserved `no-map` XPU/TZ/debug/Knox TIMA regions** totaling `5,880` 4 KiB pages (`23,520 KiB`):
  - `xbl_aop_mem` upper part (`0x80603000..0x80700000`, `253` pages)
  - `reserved_xbl_uefi_log` (`0x80880000..0x80894000`, `20` pages)
  - `removed_mem` gaps (`0x85e00000..0x86200000`, `1,024` pages; `0x8681a000..0x86900000`, `230` pages)
  - `cdsp_secure_heap` upper part (`0x90500000..0x90a00000`, `1,280` pages)
  - `dfps_data_mem` (`0x9e300000..0x9e400000`, `256` pages)
  - `sec_debug_mem` (`0x9f000000..0x9f800000`, `2,048` pages)
  - `ss_plog_mem` (`0x9f800000..0x9fa00000`, `512` pages)
  - `kaslr_mem` (`0xa0001000..0xa0002000`, `1` page)
  - `tima_mem` Samsung Knox TIMA hypervisor (`0xb0100000..0xb0200000`, `256` pages)
- **Fix:**
  - Live-validated immediately by offlining all `5,880` pages via `/sys/devices/system/memory/hard_offline_page` (`HardwareCorrupted: 23520 kB` in `/proc/meminfo`).
  - Reserved all 10 Samsung `r8q` `no-map` regions natively in `sm8250-samsung-common.dtsi` (kernel commit `1324fe066`, built into `7.3.0-rc6-omamobile-push1-g1324fe066cbb` with modules staged on phone).
  - Added `offline_unreserved_xpu_pages()` in `r8q-liveness-monitor` (`0.1.16`) as a boot-time fallback when running on older DTBs whose `/proc/iomem` still exposes those ranges as `System RAM`.

### B. Secondary Fixes & Live Session Verification
- **I2C1 RPMh churn vs `fw_devlink`:** Unloading `max77705_charger` also unbound `max17042_battery` via `fw_devlink` (`supplier:i2c:0-0069--i2c:0-0036`), removing `/sys/class/power_supply/max170xx_battery`. Instead, keeping `max77705_charger` loaded and pinning `9c0000.geniqup` and `980000.i2c` (plus `3d00000.gpu` and `3d6a000.gmu`) to `power/control="on"` in `80-r8q-gpu.rules` eliminated `BCM`/`QUP0` RPMh churn while preserving battery telemetry (`capacity=99`).
- **Monitor `SIGPIPE` storm:** Replaced `online_cpus | grep -qx "$c"` inside `r8q-liveness-monitor`'s heartbeat check loop (`1h49m` wasted CPU over `8.5h`) with a direct read of `$SYS/devices/system/cpu/cpu$c/online`.
- **Hyprland `looknfeel.lua` & `monitors.lua`:** Removed invalid `vfr = true` from `misc` in `/home/alarm/.config/hypr/looknfeel.lua` and `hyprland-r8q.conf` (`hl.meta.lua` has no `misc.vfr` — caused the `looknfeel.lua:1` red config error bar) and switched `DSI-1` to `1080x2400@60`. (`tools/r8q-install` still emits the old `misc.vfr` / `@120` lines until updated.)
- **Bring-up quietness:** Blacklisted `icc_bwmon` and `camcc_sm8250` in `/etc/modprobe.d/omamobile-r8q-bringup.conf`, masked `udisks2.service` (77 UFS partitions), and disabled `voxtype.service` autostart (147 MB model).
- **Verified Desktop (`2026-11-15`):** `sddm`, `Hyprland --watchdog-fd 4`, `quickshell -n -p /usr/share/omarchy/shell`, `squeekboard`, and `Xwayland` running cleanly (`DSI-1 1080x2400@60`, `scale 2.4`, `dpmsStatus: 1`, `gpu_cur=305000000`, `stmfts` touch IRQs active) and confirmed working on screen by the user.
