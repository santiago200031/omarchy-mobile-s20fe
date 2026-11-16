<!-- SPDX-License-Identifier: MIT -->

# Omarchy Mobile — `r8q` (Galaxy S20 FE SM-G780G) Bring-Up Notes & Postmortems

Chronological diagnostic archive split out of [PORT-PLAN.md](PORT-PLAN.md) so the main port plan stays short and actionable.

**Dashboard:** See [FILE-DASHBOARD.md](FILE-DASHBOARD.md) for a complete file inventory, status overview, and what's missing.
**Board:** See [board/README.md](board/README.md) for the Kanban-style success/failed/missing columns.

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
- **Hyprland `looknfeel.lua` & `monitors.lua`:** Removed invalid `vfr = true` from `misc` in `/home/alarm/.config/hypr/looknfeel.lua` and `hyprland-r8q.conf` (`hl.meta.lua` has no `misc.vfr` — caused the `looknfeel.lua:1` red config error bar) and switched `DSI-1` to `1080x2400@60`. (`tools/r8q-install` is now aligned to `1080x2400@60` without `misc.vfr`.)
- **Bring-up quietness:** Blacklisted `icc_bwmon` and `camcc_sm8250` in `/etc/modprobe.d/omamobile-r8q-bringup.conf`, masked `udisks2.service` (77 UFS partitions), and disabled `voxtype.service` autostart (147 MB model).
- **Verified Desktop (`2026-11-15`):** `sddm`, `Hyprland --watchdog-fd 4`, `quickshell -n -p /usr/share/omarchy/shell`, `squeekboard`, and `Xwayland` running cleanly (`DSI-1 1080x2400@60`, `scale 2.4`, `dpmsStatus: 1`, `gpu_cur=305000000`, `stmfts` touch IRQs active) and confirmed working on screen by the user.

## 6. Touchscreen Fix: Samsung FTS5CU56A 16-Byte Type-B Protocol (`2026-11-15`)

- **Symptom:** Touch interrupts (`stmfts_irq`, IRQ 162 on GPIO 15) fired on screen touches, while kernel logs emitted `unknown FTS5 event` errors and no pointer/touch events reached Hyprland.
- **Root cause:** Disassembly of the LineageOS `r8q` touchscreen driver (`fts_event_handler_type_b`, `fts_set_scanmode`) correlated with live phone `dmesg` confirmed `5-0049` (`fts5cu56a`, `st,stmfts5`) sends **16-byte Samsung Type-B event packets** read via `0x60` (single) / `0x61` (all), rather than upstream `stmfts`'s 8-byte packets (`0x86`): `tchsta = event[0] >> 6` (`1`=press, `2`=move, `3`=release), `slot_id = (event[0] >> 2) & 0x0f`, `x = (event[1] << 4) | ((event[3] & 0xf0) >> 4)`, `y = (event[2] << 4) | (event[3] & 0x0f)`, `ABS_MT_TOUCH_MAJOR = event[4]`, `ABS_MT_TOUCH_MINOR = event[5]`, `pressure = event[6] & 0x3f`.
- **Kernel fix & build (`omamobile-r8q` `5866f774c`):** Updated `drivers/input/touchscreen/stmfts.c` to support Samsung FTS5CU56A 16-byte Type-B events (`checkpatch --strict`: 0 errors, 0 warnings, 0 checks; `W=1 KCFLAGS="-Werror"` clean). Built `7.3.0-rc6-omamobile-push1-g5866f774c4da` (1818 modules), staged modules on the phone, and packed `recovery-omamobile.img` (SHA256 `2769d7da9701a3ac4b1f3fb51e559414ad979b8dcc642013778a2d7d3e1248e3`).
- **Live deployment without rebooting (`gd469bd3bf2a9`):** Because `CONFIG_MODVERSIONS`, `CONFIG_MODULE_SIG`, `CONFIG_LTO`, and `CONFIG_CFI` are disabled, built matching `stmfts-r8q.ko` into `/usr/lib/modules/7.3.0-rc6-omamobile-push1-gd469bd3bf2a9/extra/stmfts-r8q.ko`, pinned `994000.i2c` and `5-0049` (`vreg_tsp_3p0` has `regulator-always-on`) `power/control="on"`, unbound built-in `stmfts` from `5-0049`, and bound `stmfts-r8q`. `Hyprland` opened `/dev/input/event2` (`Touch Device ...: stmfts`) automatically with zero `unknown FTS5 event` errors.
- **Package (`omamobile-device-r8q` `0.1.17`):** Updated `80-r8q-gpu.rules` to pin `994000.i2c` and `5-0049` `power/control="on"` and updated `r8q-liveness-monitor` (`pin_r8q_power_control()`, `ensure_r8q_touch_driver()`).

---

## 7. QCA6390 Bluetooth (`hci0`), RTC Ordering Cycle (`1975` -> `2026`), and Boot GUI (`2026-11-15`)

- **Wi-Fi (`wlp1s0`):** `ath11k_pci 0000:01:00.0` (`QCA6390 hw2.0`, `fw 0x101f0625`) verified online alongside `omamobile-r8q-wifi.service`.
- **Why Bluetooth (`998000.serial` / `hci0`) reported `No default controller available`:**
  1. **GENI serial `-19` / `-ENOSPC` without `/aliases/serial*`:** `sm8250-samsung-r8q.dts` omits `serial*` aliases because adding static `serial0 = &uart6` caused early-boot probe-order hangs. In `drivers/tty/serial/qcom_geni_serial.c` (`get_port_from_line()`), `int max_alias_num = of_alias_get_highest_id("serial");` returned `-19`, causing `ida_alloc_range(&port_ida, -18, nr_ports - 1, GFP_KERNEL)` to fail (`-ENOSPC`). Fixed permanently in `wt-r8q` (`caa385175`) with `max(..., -1)` (`checkpatch --strict` and `W=1` clean), and fixed live on `gd469bd3bf2a9` via `r8q-bt-uart.ko` (calls `device_links_supplier_sync_state_pause()` and adds runtime `serial0` alias).
  2. **`CONFIG_BT_LE=y` & profiles:** `arm64 defconfig` leaves `# CONFIG_BT_LE is not set`; enabled `CONFIG_BT_LE=y`, `CONFIG_BT_RFCOMM=m`, `CONFIG_BT_RFCOMM_TTY=y`, `CONFIG_BT_BNEP=m`, `CONFIG_BT_BNEP_MC_FILTER=y`, `CONFIG_BT_BNEP_PROTO_FILTER=y` in `device/r8q/kernel.config` and deployed matching `.ko` modules to `extra/`.
  3. **`btmgmt --timeout` & unconfigured `hci0`:** After `hci_qca` downloads `qca/htbtfw20.tlv` and `qca/htnv20.bin`, QCA6390 sets `HCI_QUIRK_USE_BDADDR_PROPERTY` and waits in `Unconfigured controller` (`missing options: public-address`). `ensure_r8q_bt_uart()` in `r8q-liveness-monitor` generates a `02:XX:XX:XX:XX:XX` address (`0600`) in `/var/lib/omamobile/bt-public-addr`, configures `btmgmt --timeout 3 --index 0 public-addr`, and powers `hci0` on (`BR/EDR` + `LE`). Always pass `--timeout` to `btmgmt` (`v5.88`) and `--index none` for `config`, because `-i 0 config` prints `hci62528` and non-timeout `btmgmt` blocks SSH shells.
- **Why `mise` / `pacman-key` failed with TLS / key-in-the-future errors on reboot:** `omamobile-r8q-rtc.service` previously had `DefaultDependencies=yes` + `WantedBy=multi-user.target` while declaring `Before=systemd-timesyncd.service`, creating a `sysinit.target` ordering cycle that made systemd delete `systemd-timesyncd.service` and skip timely clock sync (leaving the PM8150 RTC at `May 1975`). Fixed with `DefaultDependencies=no`, `WantedBy=sysinit.target`, `Before=sysinit.target time-sync.target systemd-timesyncd.service`, `ensure_r8q_clock()` (60 s epoch save + NTP kick), and `r8q-rtc-epoch` `timesync/clock` / `2026-11-15` floor. Verified `curl -fsSI https://mise.jdx.dev/releases.tsv` (`HTTP/2 200`), `mise doctor`, and `pacman-key --list-sigs` clean.
- **Boot GUI with `systemd.unit=multi-user.target`:** Symlinked `sddm.service` under `multi-user.target.wants` and added `ensure_r8q_desktop()` to `r8q-liveness-monitor` (`0.1.18`) and `tools/r8q-install`.

---

## 8. P1 Native Kernel & Gate C Remoteprocs + Audio (`2026-11-16`)

- **P1 verified (`7.3.0-rc6-omamobile-push1-gcaa38517520a`, `omamobile-r8q-p1-signoff`):**
  - Built into `~/omamobile-kernel/out-r8q-p1`, modules pushed, packed (`recovery-omamobile.img`), and flashed to `sda24` over USB SSH (`tools/r8q-flash-recovery`).
  - `/proc/iomem` XPU regions properly reserved (`HardwareCorrupted: 0 kB`); `stmfts` and `qcom_geni_serial` (`998000.serial` → `hci0`) bound natively without OOT `.ko` workarounds.
  - Updated `r8q-liveness-monitor` (`0.1.19`) to retry `hci0` `public-addr` configuration until `hci_qca` creates `hci0` (~9.5 s post-boot).
- **Gate C Step 1 (`display,remoteprocs`) & Step 2 (`+audio-amps`):**
  - `remoteproc0` (`slpi`), `remoteproc1` (`cdsp`), `remoteproc2` (`adsp`) all reached `running`; `a00000.dma-controller` (`gpi_dma1`) and `a8c000.i2c` (`i2c11`) probed dual `cs35l41` (`11-0040`, `11-0041`, Rev A0).
- **Gate C Step 3 (`+audio-sound`) — Why `TERTIARY_TDM_RX_0` failed `-110` (`PLL_UNLOCK`) & Fix:**
  - Base DT `d469bd3bf` wired `&sound` to `TERTIARY_TDM_RX_0` (`0x38`, `mi2s2_*` `gpio133..137`, 2 slots, `tdm-sync-mode=1`, `tdm-invert-sync=1`), leaving `cs35l41` without BCLK (`Enable(1) failed: -110`, `PLL_UNLOCK=1`).
  - Stock `live.dts` and `dmesg` proved `r8q` routes speaker amps via `PRIMARY_TDM_RX_0` (`0x18`) on `MI2S0` (`gpio138..141`), `4x32-bit` slots (`6.144 MHz` BCLK), `tdm-sync-mode=0`, `tdm-sync-src=1`, `tdm-invert-sync=0`, `tdm-data-delay=1`.
  - Updated `device/r8q/overlays/audio-sound.dtso` (`pri_tdm_active`/`pri_tdm_sleep`, `dai@18`, `&q6afedai 0x18`, `4x32-bit`) and `packages/omamobile-device-r8q/ucm2/Samsung/r8q/HiFi.conf` (`PRIMARY_TDM_RX_0 Audio Mixer MultiMedia1`, `ASP`, `Left/Right Analog/Digital PCM Volume`).
  - **Verified end-to-end (`pw-cat --playback` → `Built-in Audio Stereo Speakers` → `MultiMedia1` → `PRIMARY_TDM_RX_0` → `11-0040`/`11-0041`):** both `cs35l41` amps reported `GLOBAL_EN=1`, `AMP_EN=1` (`0002018=0x21`), `PUP_DONE=1` (`0010090=0x41404000`), and `PLL_LOCK=1` / `PLL_UNLOCK=0` (`0010098=0x2000000f`) with zero kernel errors.

---

## 9. P4 SLPI Sensors (`hexagonrpc` + `vreg_l8c_1p8`) & Volume Up Root Cause (`2026-11-16`)

- **SLPI Sensors (`slpi-sensors.dtso`, `hexagonrpc 0.5.0-2`, `omamobile-device-r8q 0.1.20`):**
  - Initial `ssccli --sensor accelerometer` found QRTR node `9` (`ssc` service `400`) but failed on `'registry' sensor unavailable, is hexagonrpcd running?`.
  - After packaging `linux-msm/hexagonrpc` `0.5.0`, verbose tracing revealed three blockers causing SLPI `sns_registry_sensor.c:99` (`SNS_REG_INIT`) and `sns_thread_manager.c:437` (`SEE Sensor exceeded processing time limit`) crashes:
    1. `rpcd_builder.c` did not map `/persist/sensors/registry/sns_reg_version` or `/sys/class/sensors/ssc_core/ssc_hw_rev`; patched in `packages/hexagonrpc/0001-rpcd-builder-map-sns_reg_version-and-ssc_hw_rev.patch`.
    2. Leaving LineageOS/vendor `.json` files in `/usr/share/qcom/.../sensors/config/` caused `sns_registry` to attempt writing `temp.json` on the read-only HexagonFS; using the pre-parsed `persist` registry (`191` files from `/dev/sda8` read-only) with an empty `sensors/config/` dir avoids `temp.json` writes.
    3. `sm8250-samsung-r8q.dts` omitted `vreg_l8c_1p8` (`PM8150L L8C`, `1.8 V`, bound as `sensor_vdd-supply` on `qcom,ssc@5c00000` in stock `live.dts`), leaving the I3C/I2C sensor bus unpowered. Added `device/r8q/overlays/slpi-sensors.dtso` (`vdd-l1-l8-supply = <&vreg_s4a_1p8>`, `vreg_l8c_1p8: ldo8`).
  - **Verified live on hardware (`7.3.0-rc6-omamobile-push1-gcaa38517520a`):**
    - `ssccli`: `accelerometer` (`LSM6DSO`), `light` (`TCS3407`), `magnetometer` & `compass` (`AK0991X`) all stream real readings; `proximity` returns `UNKNOWN` (Samsung optical/virtual proximity on `stk31610`).
    - Added `79-r8q-iio-sensor-proxy.rules` (`ssc-accel ssc-light ssc-compass` on `fastrpc-sdsp*`) and `10-r8q-hexagonrpcd.conf` (`ConditionPathExists` + `-R /usr/share/qcom/sm8250/samsung/r8q`); `monitor-sensor` reports `Has accelerometer`, `Has ambient light sensor`, and `Has compass` with live updates.
- **Volume Up key & speaker gain fixes (`vol-up.dtso`, `omamobile-device-r8q 0.1.21`):**
  - Stock `live.dts` (`gpio_keys/vol_up` + `pinctrl@c000`) wires `KEY_VOLUMEUP` to `&pm8150l_gpios 3 GPIO_ACTIVE_LOW` (`pm8150l_gpio3`), whereas `sm8250-samsung-common.dtsi` had `&pm8150_gpios 3`. Added `device/r8q/overlays/vol-up.dtso` and verified on hardware (`gpiochip1` `gpio3` `in high pull-up`, `/proc/interrupts` IRQ `179` `spmi-gpio 2 Edge Volume Up`).
  - Raised `HiFi.conf` speaker gains (`Analog PCM Volume` `10` → `18`, `Digital PCM Volume` `790` → `817` = `0.00 dB`) to match stock `GAIN_CTRL = 0x253` / `DIG_VOL_CTRL = 0x8001` (`+11.4 dB`), verified live on both `11-0040` and `11-0041`.
  - **Reboot USB quirk:** After warm `reboot-to recovery`, the host USB-C link may not re-enumerate `a600000.usb` until the USB-C cable is unplugged and replugged once.

---

## 10. P3 Wireless/Modem Backup, P4 DPMS Cycle & CS35L41 Diagnostics (`2026-11-16`)

- **Concurrent Wi-Fi + Bluetooth + USB NCM (`7.3.0-rc6-omamobile-push1-gcaa38517520a`):**
  - `wlp1s0` (`ath11k_pci`, `omamobile-r8q-wifi.service` active) sustained `15/15` ICMP pings (`0%` loss, `rtt min/avg/max = 11.304/13.392/15.320 ms`) and 3 concurrent HTTPS fetches (`HTTP 200`) at `5180.0 MHz` (`576.4–720.6 MBit/s`, `-60 dBm`) alongside `btmgmt find -l` LE discovery on `hci0` (`br/edr` + `le` powered, zero missing options) and active USB NCM (`172.16.42.1`), with zero wireless errors in `dmesg`.
  - Observed `dwc3 a600000.usb: remote wakeup not configured` and one transient 2-second USB NCM re-enumeration during a sustained 195 MiB read over SSH (`uptime` stayed continuous).
- **DPMS & Lock Screen cycle (remote `hyprctl`):**
  - Verified `hyprctl dispatch 'hl.dsp.dpms({ action = "enable" })'` and `'hl.dsp.dpms({ action = "disable" })'` cleanly transition `DSI-1` between `dpmsStatus: False` (`card1-DSI-1/dpms=Off`, `ae94000.dsi.0/bl_power=4`) and `dpmsStatus: True` (`card1-DSI-1/dpms=On`, `bl_power=0`, `brightness=1023`), with `omarchy-shell lock status` reporting `locked: true, secure: true` and zero DRM/DSI errors in `dmesg`. Attended Power-key wake check remains for the human.
- **NV/Modem Off-Repo RO Backup (`AGENTS.md` §1):**
  - Verified all 10 NV/modem UFS partitions (`modemst1`, `modemst2`, `fsc`, `mdm1m9kefs1`, `mdm1m9kefs2`, `mdm1m9kefsc`, `persist`, `efs`, `sec_efs`, `modem`) `ro=1` and backed them up read-only off-repo to `~/omamobile-kernel/reference-r8q/nv-backup/` (`sha256sum -c SHA256SUMS`: `10` ok). Confirmed `/lib/firmware/qcom/sm8250/samsung/r8q/modem.mbn` + `modemr.jsn` are staged, while `rmtfs`, `tqftpserv`, and `pd-mapper` daemons are not yet installed.
- **CS35L41 Speaker State Inspection:**
  - Live `regmap/11-004[01]/registers` + `amixer -c 0`: both amps run `PCM Source = ASP`, `PWR_CTRL3 = 0x01000000` (`wkfet_amp=Y`), `BSTCVRT_VCTRL1 = 0x00` (`bst_ctl=0`), `AMP_GAIN_CTRL = 0x240` (`Analog PCM Volume = 18`), `AMP_DIG_VOL_CTRL = 0x8000` (`Digital PCM Volume = 817`, `0.00 dB`). Stock `/lib/firmware/cirrus/cs35l40-{bot,rcv}-dsp1-spk-prot.{wmfw,bin}` exist on the phone, unused while `cs35l41.c` hardcodes `dsp->part = "cs35l41"` and `PCM Source = ASP`.

---

## 11. Rear Telephoto Camera (`hi847` + `qcom-camss`) Bring-Up (`2026-11-16` – `2026-11-17`, `camera-wip`)

- **Fixed three blockers (`wt-r8q` `camera-wip` @ `f3be9fb6d`, `7.3.0-rc6-omamobile-push1-gf3be9fb6de08`, `recovery-omamobile-camera.img` sha256 `4ce46855…`):**
  1. `vreg_l7c_1p8` (`L7C` `1.8 V`, not `L7A`) eliminated `i2c@a88000` timeouts.
  2. `pinctrl-0 = <&cam_mclk3_default &cam_tele_active>` (`GPIO 97` `MCLK3`, `GPIO 78`, `GPIO 135`) allowed `hi847 10-0021` to probe (`0x0847`).
  3. `&camss` `port@2` (`csiphy2_ep`) needed `clock-lanes = <7>;`, 0-based `data-lanes = <0 1 2 3>;`, and `bus-type = <MEDIA_BUS_TYPE_CSI2_DPHY>;` (matching `camss-csiphy-3ph-1-0.c` `BIT(pos * 2)` and `camss-csid.c`), plus explicit `DEVICE=r8q` for `bin/omamobile kernel|modules`.
- **Verified raw capture on `/dev/video0`:** after enabling `media-ctl` links (`msm_csiphy2:1 -> msm_csid0:0` and `msm_csid0:1 -> msm_vfe0_rdi0:0`), `v4l2-ctl --stream-mmap=4 --stream-count=2` captured `19,975,680` bytes at `3264x2448` (`pgAA`) and `5,013,504` bytes at `1632x1224` (`pgAA`) cleanly (`rc=0`, `vfe0` and `csid0` IRQs active).
- **`display-clocks.dtso` black-screen root cause (`2026-11-16`):** Packing `--overlay display-clocks` added an empty 0-byte `protected-clocks;` property to `&dispcc` (`/soc@0/clock-controller@af00000`). In `drivers/of/property.c`, `of_prop_next_u32()` lacks a `length >= sizeof(__be32)` guard on `cur == NULL`, so `qcom_cc_drop_protected()` read index `3` (`DISP_CC_MDSS_BYTE0_CLK_SRC`) out of bounds and dropped `disp_cc_mdss_byte0_clk_src` (`55` clocks instead of `56`), orphaning `disp_cc_mdss_byte0_clk` (`status stuck at 'off'`, black screen after console). Repacked with `--overlay display --overlay remoteprocs --overlay audio-amps --overlay audio-sound --overlay slpi-sensors --overlay vol-up --overlay camera` (`0f195080…`, `protected-clocks` `FDT_ERR_NOTFOUND`).
- **`omacam` viewfinder, SoftISP, timings & orientation (`2026-11-17`, `7.3.0-rc6-omamobile-push1-g7ca90963c426`, `omamobile-device-r8q 0.1.24`, `libcamera 0.7.2-4.24`):**
  - `70-omamobile-dma-heap.rules` fixed `/dev/dma_heap` `0600` permissions; `70-r8q-libcamera.conf` defaults `LIBCAMERA_SOFTISP_MODE=cpu` (`FD650` `debayer_egl` hit GPU `hangcheck` lockups); `79-r8q-iio-sensor-proxy.rules` sets `ACCEL_MOUNT_MATRIX="-1, 0, 0; 0, -1, 0; 0, 0, 1"`.
  - `libcamera` patch `0058` adds `CameraSensorHelperHi847` (`blackLevel_ = 4096`, `AnalogueGainLinear{ 1, 16, 0, 16 }`), `hi847.yaml`, and `CameraSensorProperties`.
  - `wt-r8q` `63cbf400a` + `7ca90963c` scale `.llp` to `3680`/`1840` (`HBLANK=+416/+208`, `30.02`/`60.01` fps), implement `hi847_get_selection()` and `v4l2_fwnode_device_parse()`, and set `orientation = <1>; rotation = <270>;`. Verified live on `omacam` (`3256x2448` @ 30.02 fps, UI and viewfinder orientation confirmed normal).
