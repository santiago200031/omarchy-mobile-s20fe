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

---

## 12. SDX55M Stage 3A Reset and the Stable Baseline (`2026-11-17`)

- **Stage 3A reset the phone at MHI power-on.** With the read-only Sahara loader waiting, `modprobe mhi_pci_generic` logged `MHI PCI device found: qcom-sdx55m` and `mhi mhi1: Power on setup success`, then USB dropped; the next boot was fresh, the phone never printed `Wait for device to enter SBL or Mission mode`, and no SAHARA port appeared. The MSI-in-IOMMU change (`5be4efd5e`) is line-for-line the reference port's, so the cause is likely this base or this device's PCIe2/IOMMU wiring. Do not repeat it as-is (`MODEM-PLAN.md` §8).
- **The first boot after the reset missed the SD card** (`omamobile: loop: /dev/mmcblk0p1 not present`, initramfs rescue shell). A host-side `sdhci_msm` unbind/bind coincided with a second reset; the following boot enumerated `/dev/mmcblk0p1` and mounted the root image cleanly. A reset can cost more than one boot before USB and the SD card are back.
- **The stable baseline is now a checked record, not a note.** `device/r8q/stable-image.conf` pins `7.3.0-rc6-omamobile-push1-g3667ab368ef9`, sha256 `66fe127f…d8503`, `--dual-boot --overlay camera-uw`, `sda24`. `tools/r8q-stable-check` verifies the image and the phone (release, `sda24` sha256, guard, root, PCIe2/PHY `disabled`, no SDX55M on `mhi_pci_generic`, no Sahara leftovers, modules), and `tools/r8q-flash-recovery --stable [--dry-run]` restores exactly those bytes. Restored on 2026-11-17 and verified: every check passes, `HardwareCorrupted: 0 kB`. `tools/phone-smoke` reports only the two r8q differences a generic test cannot know (root on `/dev/loop0`, boot-time UFS/DSI messages; storage mounted, display working).

---

## 13. AI Supervisor Verified on the Stable Kernel (`2026-11-17`)

- **The supported design is `omamobile-ai.service`, not the old on-demand script.** The package (`omamobile-ai` `0.1.0-1`) runs one Rust supervisor on `172.16.42.1:8080`; it starts `llama-server` on the first request, proxies the OpenAI and llama.cpp router surfaces, and stops the model after `IDLE_TIMEOUT=180`. The BSP `rust/ai-server/` stats stub and the hand-installed `/usr/local/bin/omamobile-ai` script are gone.
- **The r8q tuning lives in `omamobile-device-r8q`.** `etc/omamobile-ai.conf.d/10-sm8250.conf` pins inference to cores 4-6 (`THREADS=3`) and sets the USB listener. The phone still runs `0.1.24`, where `/etc/omamobile-ai.conf.d/20-usb.conf` was created by hand with the same `LISTEN_HOST=0.0.0.0`; `0.1.25` in the repo ships the drop-in.
- **Measured on `7.3.0-rc6-omamobile-push1-g3667ab368ef9`:** 26-token prompt at 15.47 tok/s; 128 generated at 11.01 / 10.74 / 10.56 tok/s over three back-to-back requests (warm-up 11.13), 46.3 °C before and 74.1 °C after. The earlier `gf3a5609d121b` run (15.34 / 11.24) is inside the same spread.
- **`tools/r8q-ai` now drives the API** (`status`, `models`, `ask`, `unload`, `stop`, `start`, `push-model`) with an SSH fallback; `ask` prints llama.cpp's own tok/s on stderr. The supervisor's `/health` also reports RAM, swap, battery and load, which is the quickest read of whether the model is resident.
- **One API detail:** `llama.cpp`'s native `/completion` is not proxied (it answers `{"error":"not found"}`); clients use `/v1/chat/completions` (JSON or SSE) or the router surface. `LFM3-1.7B` emits its answer in `reasoning_content` when `max_tokens` cuts it off, which `tools/r8q-ai` falls back to.

---

## 14. SDX55M Stage 3A: the MSI Path, Explained Offline (`2026-11-17`)

- **Stage 2 stalled because no BHI interrupt arrived.** `mhi_ready_state_transition()` returns only when the device reaches READY, which the device signals with an MSI; `Wait for device to enter SBL or Mission mode` prints after that call (`drivers/bus/mhi/host/boot.c:600`). Stage 2 never printed it, so the host never saw the transition and the reported `PRIMARY BOOTLOADER` is the device waiting.
- **Mainline arms the DWC iMSI-RX at `0x64100000`.** `dw_pcie_setup_rc()` → `dw_pcie_msi_init()` programs `PCIE_MSI_ADDR` with `pp->msi_data`, which `dw_pcie_msi_host_init()` sets to `cfg0_base`; `use_imsi_rx` is on because the DTS has no `msi-parent`/`msi-map`. The endpoint is behind `apps_smmu` (RID `0x100` → SID `0x1d01`), so that write is translated before the controller can match it, and with no mapping it faults or is dropped.
- **Stage 3A changed the wrong half.** `5be4efd5e` mapped the doorbell (`iommu_dma_prepare_msi()`) and let `msi_msg_set_addr()` program the endpoint with an IOVA, but `PCIE_MSI_ADDR` stayed `0x64100000`. The write no longer matched the doorbell and was translated into an ordinary write to the PCIe2 config/DBI block — not a legal DMA target — which fits the immediate reset. The recipe is for memory doorbells (GIC ITS/MBI/v2m), not for an address-matching doorbell in the DWC core.
- **Stock never used iMSI-RX for the modem.** `live.dts` gives each root complex a `qcom,pci-msi` node at the same doorbell `0x17a10040` with its own SPI bank (`pcie0` `0x300-0x31f`, `pcie1` `0x320-0x33f`, `pcie2` `0x340-0x35f`), `msi-parent` on `&pcie2`, and stock `/proc/interrupts` names the modem's ten vectors `gic_msm_pci_msi 0x340+`. Mainline has no driver for that block.
- **Decision:** [MODEM-PLAN](MODEM-PLAN.md) §9 adds a small `qcom,pci-msi` platform MSI controller modeled on `irq-gic-v3-mbi.c` (doorbell + 32 SPIs + `iommu_dma_prepare_msi()`), `msi-parent` on `&pcie2`, and leaves iMSI-RX to PCIe0 (Wi-Fi). The gs201 CP driver in this tree (`pcie-gs201-cp.c`: a `no-map` reserved page as `msi_data`) is the fallback, not the first choice. `mhi_pci_generic` has no polling mode, so dropping MSI does not reach a usable modem.

---

## 15. Stage 4: MSI allocation works, delivery needs the bank offset; SD halt (`2026-11-17`)

- **Driver bug #1 (oops):** compose ran on the leaf irq_data with NULL chip data (`Unable to handle kernel NULL pointer dereference at 0x10` = `msi->doorbell` with NULL `msi`), during the root port's async MSI probe at boot. The oops wedged the async machinery and hung the first `modprobe` in D state. Fixed by reading state/SPI off `data->parent_data`, MBI-style. With the fix, the root port holds a live `QCOM-PCI-MSI` vector and the modem bind allocates ten vectors with no fault and no hang.
- **Driver bug #2 (silent drop):** programmed MSI data was the absolute SPI number (`0x348` in the endpoint's MSI cap). The doorbell selects 1-of-32 by index and classic MSI ORs the vector into shared data, so anything above 31 is dropped silently — no SMMU fault, all counts zero, modem parked in `PRIMARY BOOTLOADER`. Middle-domain hwirq is now the bank offset; fix committed (`ff311d19e`), image `b2d18af5…` packed, **not yet run**.
- **SSH flasher vs rescue shell:** the guard removes `/dev/sda` in the rescue initramfs and sysfs `ro` is read-only there, so the LUN flag can't be cleared and the tool aborts before writing. `r8q-flash-recovery` grew a sysfs fallback (works from Arch, useless in rescue). Flashing from Download Mode/Heimdall works and was verified by readback (`2ca70c35…`); NOTES §2's stale-flash warning held, the readback matched.
- **Storage halt (stop condition):** loop0 write I/O errors across many sectors, no mmc errors, journal aborted, `/` `emergency_ro`, eventually EIO executing binaries. Modem unbound cleanly, phone synced + powered off via SysRq. NV 15/15 `SAME` throughout; no NV write ever happened. Next: SD out, laptop fsck of the partition and the ext4 image (or a new card; rootfs is rebuildable). No modem/flash/reboot work until the root is healthy.

- **Storage relapsed (same evening):** after one healthy boot (256 MB clean),
  the next boot went `emergency_ro` again with fresh loop0 I/O errors (96
  and counting), EIO executing `/usr/bin/blockdev`, and a confused `sda24`
  readback — so neither `modules-push` nor the flash could run. The card
  reads fine on the laptop, which points at the phone-side path (socket,
  controller, power) or a dying card that only fails under the phone's
  timing — not at capacity (no ENOSPC anywhere) and not at our kernel
  (errors span releases and idle I/O). Reliability verdict: the SD path
  cannot be trusted for gated modem runs until a card reads AND writes
  cleanly across several phone boots. Phone synced + powered off via SysRq;
  NV 15/15 `SAME`, `sda24` untouched.

- **Stage 4 rescue: MSI delivery proven (`2026-11-17`).** The SD-less
  rescue image works: PCIe2 enumerates after `insmod phy-qcom-qmp-pcie`
  (that driver is a module; rescue has no udev), MHI binds, and the
  device reaches SBL on its own. A debugfs doorbell trigger + GICD
  SETSPI control split the path cleanly: SETSPI proves the GIC side,
  and doorbell writes with data = GIC SPI hwirq (872/873) fire the
  right vectors while every other encoding stays silent. The fired
  interrupts drove a genuine SBL transition (host EE SECONDARY
  BOOTLOADER, M0) and `/dev/wwan0sahara0` appeared. Root cause of all
  earlier silence: the message data never selected an allocated SPI.
  The fix programs the GIC hwirq, matching the downstream
  `pci-msm-msi` source. Phone powered off cleanly via SysRq; `sda24`
  still holds the trigger-test image, NV untouched.

- **Stage 4 rescue: end-to-end MSI victory (`2026-11-17`).** With the
  driver programming the GIC SPI hwirq as message data (downstream's
  rule), the modem's own BHI interrupt arrived with zero manual
  triggers: counts on BHI + event vectors, host EE SECONDARY
  BOOTLOADER at M0, `mhi0_SAHARA` created, and `/dev/wwan0sahara0`
  attached after `mhi_wwan_ctrl` (+ `wwan`, `gnss` deps) was insmodded.
  No Python in rescue, so the Sahara download itself waits for healthy
  storage. The storage halt stands independently: SD still suspect,
  phone powered off via SysRq, `sda24` untouched since the rescue
  flash, NV 15/15 `SAME` throughout.

- **Sahara serving works end-to-end (`2026-11-17`).** With MSI delivery
  fixed, the modem walked through 14 Sahara sessions (mdmddr, apdp,
  multi_image_qti, devcfg, tz, hyp, acdb, sbl-adjacent configs and the
  three mdm1m9kefs NV partitions read straight off UFS, all status 0,
  every DONE acked). The Rust loader (`tools/r8q-sahara-rs`, static musl,
  verified against a fake modem under qemu first) served them all from
  `/tmp` over the rescue shell with no Python anywhere. The modem then
  closed the Sahara port itself; the host sits at SBL/M0 awaiting the
  AMSS jump. NV partitions were only ever read, never written.

- **SBL stall after the full image set (`2026-11-17`).** Both cold and warm
  boots converge: the modem requests its image set over Sahara (tz, hyp,
  sec.elf, aop, acdb, multi_image_qti, devcfg, apdp, apps, qdsp6sw, and
  the kefs/mdmddr partitions read off UFS), acks every DONE — then goes
  silent with no EE-change interrupt. BHI_EXECENV stays 0x1 (SBL),
  errcode 0. It asserts the wake line, mhi_wwan_ctrl's Sahara channel
  dies with EIO, and a later rebind fails ("Device failed to clear MHI
  Reset"). The event-ring MSIs work (3040/2999 counts), so this is not
  delivery: the SBL is waiting for a post-Sahara host step that stock's
  downstream host stack performs and mainline's mhi_pci_generic does not
  (candidates: the SBL→AMSS MHI handshake, downstream PM/LPM hooks, or
  a flashless AMSS load step). Unresolved; needs vendor source study or
  a stock-capture comparison, not more blind rebinding.

- **Root cause of the AMSS stall found in the Black Shark 3 docs (`2026-11-17`).**
  Caullen Sasnett's postmarketos-blackshark-klein documents the identical
  SM8250+SDX55M combination and reaches AMSS with two MHI core patches that
  mainline lacks: (1) SBL is a real EE — mainline rejects it at power-up
  ("SECONDARY BOOTLOADER is not a valid EE for power on", our exact rebind
  error), loses the EE event when SBL is reached during BHI, and leaves stale
  ring base registers for the SBL to read; (2) at the SBL->AMSS jump the
  modem moves its doorbell offsets (CHDBOFF 0x0 -> 0x300), and mainline
  keeps ringing the cached SBL doorbells so every AMSS channel START is
  invisible. The r8q runs fit both: the channel EIO after apps.mbn was the
  transition beginning, not failing. Both patches ported to 7.3.0-rc6
  (kernel commit `0f77e4da5`), build clean. The vendor loader itself
  (libmdmimgload.so, inspected from the extracted vendor EROFS) is a pure
  Sahara server — no post-Sahara magic step exists on stock; the stock
  mdm_helper just waits for the MDM2AP_STATUS GPIO.

- **Full image set served on ONE channel; AP2MDM_STATUS was never asserted (`2026-11-18`).**
  Holding one open MHI channel across all image mini-sessions (instead of
  reopening per image) let the modem walk the complete 14-image set with every
  DONE acked: IDs 34, 40, 36, 41, 25, 33, 42, 23, 16, 17, 20, 29, 8 (qdsp6sw,
  83 MB), 6 (apps). The reference port's notes confirm the rule: stock's `ks`
  serves with `-n` (no reset) and the SBL allows one SAHARA session per boot.
  After the set the modem stayed in SBL (BHI_EXECENV 0x1, no error) and the
  host never saw an EE event, while MDM2AP_STATUS (gpio3) reads HIGH. Stock
  drives AP2MDM_STATUS (gpio56) high from its vendor modem driver before
  power-on; in mainline nothing claims gpio56/57, so they sit as inputs, low
  (`gpio56: in low`). The modem-gateway reference for this flashless setup
  asserts gpio56 high + gpio57 low from the PCIe2 pinctrl state. Ours now does
  the same (modem.dtso).

- **THE MODEM REACHES AMSS ON MAINLINE (`2026-11-18`).** With AP2MDM_STATUS
  (gpio56) driven high from the PCIe2 pinctrl state, the SDX55M walked the
  complete 14-image SAHARA set on one open MHI channel and jumped to mission
  mode by itself after `apps.mbn`: the kernel logged
  `Mission mode doorbell offsets: CHDBOFF=0x300 ERDBOFF=0x700` (exactly the
  reference values), `/sys/kernel/debug/mhi/mhi0/states` reads
  `EE: MISSION MODE`, the AMSS channels enumerated (`mhi0_QMI`,
  `mhi0_MBIM`, `mhi0_IPCR`, `mhi0_IP_HW0`, `mhi0_IP_SW0`) and the modem's
  real ports appeared: `/dev/wwan0qmi0`, `/dev/wwan0mbim0`,
  `/dev/wwan0qcdm0`, `/dev/wwan0at0`. The three fixes that mattered, in
  order of discovery: the `qcom,pci-msi` doorbell (GIC SPI hwirq as
  message data), the MHI SBL/AMSS handling (SBL power-up, context-register
  zeroing, M0 at SBL, mission-mode doorbell re-read), and AP2MDM_STATUS
  high + one SAHARA channel per boot. QRTR is empty because no userspace
  daemon runs in the rescue shell; the modem is up regardless.

- **Stable Omarchy restored after the AMSS milestone (`2026-11-18`).**
  `recovery-cam-uw.img` (`66fe127f…`, the pinned stable image) flashed
  to `sda24` via Download Mode/Heimdall and verified on the phone:
  `uname -r` reads `7.3.0-rc6-omamobile-push1-g3667ab368ef9`, root is
  `/dev/loop0` ext4 `rw`, the guard holds 77/77 UFS partitions
  read-only, PCIe2 is `disabled` with no `mhi_pci_generic` and no wwan
  ports, Hyprland and quickshell run, and the 15-partition NV check is
  all `SAME` (`param` differs as expected). The modem image that
  reached AMSS stays recorded in MODEM-PLAN (`0e3c74b5…`), and the
  kernel branch `r8q-modem-msi` holds the commits.

- **A restored stable image needs the stable release's modules (`2026-11-18`).**
  Flashing `recovery-cam-uw.img` alone boots `g3667ab368ef9` with an
  empty `/usr/lib/modules/7.3.0-rc6-omamobile-push1-g3667ab368ef9`, so
  `msm` cannot load: simpledrm stays up, Hyprland runs with no render
  node, and the screen is black with an all-black `/dev/fb0` even though
  `hyprctl` reports the monitor and DPMS fine. The last working UI had
  run a test kernel whose modules were installed, which is why the
  missing folder only showed after switching back. Fix:
  `KERNEL_OUT=$HOME/omamobile-kernel/out-r8q-uw bin/omamobile
  modules-push`, then a reboot (loading `msm` by hand fails:
  `DSI PLL(0) lock failed`, since simpledrm already owns the panel).
  After that: `msm` bound, `card1` + `renderD128`, backlight
  `ae94000.dsi.0` at 1023/1023, Hyprland up, guard 77/77. The stable
  restore procedure is therefore **flash the image AND push its
  modules**; add the modules step to `stable-image.conf`'s notes when
  the restore is next formalised.

- **Session close: repo and phone state (`2026-11-18`).** The SDX55M
  bring-up and its documentation are on `main` (merged incrementally,
  branch deleted, `origin/main` in step). The kernel side lives on the
  local branch `r8q-modem-msi` in `~/omamobile-kernel/wt-r8q` (6
  commits: qcom,pci-msi binding and driver with the GIC-hwirq message
  data and debugfs bring-up knobs, the MHI SBL/AMSS support, a
  MAINTAINERS entry); `tools/kernel-pr-check` passes except the human
  `Signed-off-by`, which goes through the maintainer's publish flow.
  The phone runs the pinned stable release with the UI working (msm,
  card1 + renderD128, backlight up, guard 77/77, modem off), and the
  AMSS-capable image (`0e3c74b5…`) stays reproducible from
  `out-r8q-msi`. Next for cellular: userspace SIM/QMI on the Arch root
  (qrtr-ns, ModemManager) with a kernel carrying the three commits, and
  the camera bring-up stays the separate open work.

- **Freedreno/Turnip validated on the phone (`2026-11-18`).** The Mesa
  branch's packages were installed from the local signing key and
  verified on hardware: `vulkaninfo` reports
  `deviceName = Turnip Adreno (TM) 650`, `driverID =
  DRIVER_ID_MESA_TURNIP`, `driverInfo = Mesa 26.2.4-arch1.2.0.omamobile5`,
  Vulkan 1.3.354; `eglinfo` reports `OpenGL core profile version 4.6
  (Core Profile) Mesa 26.2.4-arch1.2.0.omamobile5` with vendor
  `freedreno`. The Turnip ICD is standalone: after restoring Arch's
  newer `mesa 1:26.2.7-1` from the rollback cache, `vulkan-freedreno
  1:26.2.4-2.0` still enumerates the Adreno 650, so the phone's end
  state is newest Mesa + our Turnip package. Rollback packages stay in
  `/var/cache/pacman/pkg`; the test packages in `/tmp/mesa-test` (RAM).

## 16. Adreno 650 Sustained at 587 MHz: Clean; Cap Adopted (`2026-11-19`)

The last open item on the freedreno side of board card 05 was whether the
Adreno 650 could hold its top OPP (587 MHz) under a real, sustained Vulkan
load. The earlier evidence was only a short FP32 FMA burst (245.3 GFLOP/s at
587 vs 127.6 at 305, 1.92×), so the cap was raised and held under load:

- `max_freq` lifted live to `587000000`; governor `userspace` pinned at
  587 MHz (the devfreq offers only `userspace`/`simple_ondemand` — there is no
  `performance` governor); a real Vulkan workload (`vkcube` on Turnip, render
  fds confirmed) rendering on the panel; `power/control=on` on GPU and GMU.
- **~33 minutes sustained at 587 MHz** (20 min + 13 min `vkcube`,
  `12:29`–`13:03`), sampled every 15 s.
- **Clean:** frequency held 587 MHz throughout; no GMU faults, no RPMh
  timeouts, no GPU hangs, no thermal trips. `gpu-top` 40-43 °C /
  `gpu-bottom` 41-43 °C; hottest of all 28 zones peaked at 49 °C (one sample)
  and sat 44-45 °C — large margin.
- After the run the governor was restored to `simple_ondemand`; the device now
  scales 305-587 MHz and idles at 305.
- `80-r8q-gpu.rules` now sets `min_freq=305000000`, `max_freq=587000000` (min
  stays 305 so it scales down; max is the cap, not a pin).
  `omamobile-device-r8q` 0.1.25 → **0.1.26**; the rule was pushed to the
  phone's `/usr/lib/udev/rules.d/` and re-triggered, so it survives a reboot.

## 17. Vulkan `llama.cpp` Wedges the Adreno 650 (`2026-11-18`)

The other open item on board card 05 was a Vulkan (`GGML_VULKAN`) `llama.cpp`
run. It was built on the phone (needs `vulkan-headers` + `spirv-headers`; the
ICD was already there) and the Adreno 650 shows up — but the first real
compute pass **wedges the GPU**:

- `llama.cpp` `1.0.0` (main) built on the phone with `-DGGML_VULKAN=ON`
  (`cmake -B build -DGGML_VULKAN=ON -DGGML_NATIVE=ON`, `make -j8`, ~1 min).
  `llama-server --list-devices` reports
  `Vulkan0: Turnip Adreno (TM) 650 (4160 MiB, 3381 MiB free)`.
- `llama-bench -m LFM3-1.7B-Q4_K_M.gguf -ngl 99 -p 512 -n 128`, pinned at
  **305 MHz**, fails at model load: `device lost on Vulkan0`,
  `vk::Device::waitSemaphores: ErrorDeviceLost`.
- Kernel: `hangcheck detected gpu lockup rb 0!` (fence stuck), and
  `adreno 3d00000.gpu: [drm:a6xx_recover [msm]] *ERROR* cx gdsc didn't
  collapse` — the GMU CX power domain won't cycle, so the kernel cannot reset
  the GPU in place. The recovery worker loops.
- The wedge persisted: `vkcube` (fine at 587 MHz before) then **segfaulted**,
  and the hangcheck started flagging **`Hyprland`** as the offending task,
  taking `hyprland`/`omamobile-shell` down. Only a **reboot** cleared it.

It is not a frequency problem (it faulted at the lowest OPP) and not the
render path (vkcube/Hyprland compositing were stable at 587 MHz): it is the
Vulkan **compute** path `llama.cpp` uses, plus a GMU recovery that can't
collapse CX — the same hangcheck class card 04 tracks for `debayer_egl`.
`GPU_LAYERS=0` (CPU) stays the default; a Vulkan LLM run waits on a
Turnip/GMU fix. Full record: [board/failed/08](board/failed/08-vulkan-llm-adreno-gpu-wedge.md).

## 18. llama.cpp Refresh and the `omamobile-llama` Package (`2026-11-18`)

The hand-built `/usr/local/bin/llama-server` (`1.0.0-dev`, commit `1719dbb`)
was replaced by the packaged backend:

- `omamobile-llama 1.0.0.20261118.98d67c8-1`, pinned to llama.cpp
  `98d67c8d536dc584897c4adb2510509796517222` (2026-11-18). The package is
  static, CPU-only (`GGML_VULKAN=OFF`, `GGML_OPENMP=OFF`), built with
  `-march=armv8.2-a+dotprod+fp16` and `GGML_NATIVE=OFF` so the same flags work
  in `makepkg` on the phone and in the aarch64 container. It installs
  `/usr/bin/{llama-server,llama-cli,llama-bench}`.
- Built natively on the phone with `tools/phone-install-package
  packages/omamobile-llama` (~9 min, all cores) and installed with `pacman -U`.
  The old binary is at `/root/llama-old/llama-server-1.0.0-dev-1719dbb`;
  `/usr/local/bin` no longer shadows the package, and `omamobile-ai` was
  restarted against the new binary.
- **Benchmark** (`taskset -c 4-6`, `-t 3`, two runs):
  `pp512 35.18 ± 0.01 tok/s`, `tg128 15.70 ± 0.03 tok/s`.
  Through the supervisor, same prompt and 128 generated tokens, three runs:
  **15.29 / 15.26 / 15.28 generation tok/s** (27-32 prompt tok/s).
  Thermals 47.5 °C before the pair, 68.7 °C after (hottest of 28 zones).
  The old baseline was 10.6-11.0 generation tok/s, so this is ~1.4x.
- `llama-bench` prints `build: unknown (0)` because the release tarball has no
  git metadata; the package version pins the commit. The source, flags and
  benchmark are recorded in [AI-BENCHMARKS.md](AI-BENCHMARKS.md) §2.1 and
  `packages/omamobile-llama/README.md`.

## 19. Modem Normal Boot: SBL Reached, but the SD Card Browns Out (`2026-11-18`)

The first **normal** (non-rescue) boot of the modem kernel reached SBL:
`recovery-modem.img` (sha256 `344eafe5…`, kernel
`7.3.0-rc6-omamobile-push1-g70a925aeee09`, `--dual-boot --overlay modem`)
brought PCIe2 up, enumerated the SDX55 (`0002:01:00.0 [17cb:0306]`), and
`mhi1` reported `Power on setup success` / `Device already in SBL`.

At the `mhi_pci_generic` bind, `mmc0: card 0001 removed` fired 0.3 s later.
The root went `rw,...,emergency_ro`, `wwan.ko`/`blockdev`/`sha256sum` returned
EIO, `mhi_wwan_ctrl` could not load, and no `/dev/wwan*` appeared.

- The stock DT shows `sdhc_2: cd-gpios = <&tlmm 77>` (not AP2MDM 56/57),
  `sdhc_2` has no `vmmc-supply`, and `pcie2` has `vreg-1p8`/`vreg-0p9`/
  `vreg-cx`/`gdsc-vdd` but no 3.3 V rail. The mainline `pcie-qcom` logs
  `supply vddpe-3v3 not found, using dummy regulator`. The modem's power-on
  browns out the always-on 3.3 V rail shared with the SD card.
- The rescue image (no SD root) remains the only working modem configuration.
- Recovery: heimdall wrote the stable image (`66fe127f…`) to `sda24`; the SD
  card re-detected, the root is read-write again, and the data backup is
  `~/omamobile-kernel/reference-r8q/root-data-20261118.tar.zst`
  (sha256 `3762e761…`, 1.7 GiB).
- Next: move the root to `userdata` (UFS), or model a modem 3.3 V load
  switch from the schematic if the hardware has one; then package the
  userspace (`ModemManager` is installed; `qrtr`, read-only `rmtfs`,
  `pd-mapper`, `tqftpserv`) and test SIM/QMI. Full record:
  [MODEM-PLAN.md](MODEM-PLAN.md) §12.

## 20. Root on `userdata`: the SD card leaves the boot path (2026-11-18)

The root ran from `/omarchy-r8q.img` on the microSD (loop0). Powering the SDX55M
dropped the card 0.3 s after `mhi_pci_generic` bound and the root went
`emergency_ro` (§19). The root now lives on `userdata` (`sda36`, 107 GiB, ext4
label `omamobile-root`), so the SD card is not in the boot path at all.

**Install** ([RUNBOOK-USERDATA-ROOT](RUNBOOK-USERDATA-ROOT.md)): a rescue-shell
image with `UFS_RW=userdata`, `rsync` and `mkfs.ext4`; `mkfs.ext4 -L
omamobile-root`; `rsync -aHAX --numeric-ids --bwlimit=80000` from the SD image
(13.9 GiB, 265 s, post-copy dry run 0 differences); then a native image with no
`omamobile.loop=` whose init refuses anything but the `omamobile-root` label.
The first boot ran `e2fsck -f` + `resize2fs` to 105 GiB.

**Verified:** `/dev/sda36 ext4 rw`; `systemd` running; Hyprland and quickshell
up; exactly one writable UFS partition (`sda36`), every other `sda*` `ro=1`; no
ext4 or I/O errors. One benign `ufshcd_query_attr_retry` (`idn 31`, `-22`) at
25 s.

**Gotchas:**
- The loop-root rule (`is_non_ufs_root`) sets **every** UFS partition
  read-only, so a loop root cannot be the one that formats `userdata`: use the
  initramfs rescue shell (native allowlist, no loop token) for the install.
- `reboot` in the rescue initramfs returns 0 and does nothing. Use
  `sync; echo b > /proc/sysrq-trigger`.
- `/sys/block/sda/ro` is `0444` on this kernel; a tool that clears the LUN flag
  through sysfs gets EACCES. The rescue mode removes `/dev/sda` but leaves the
  disk writable, so only the partition flag needs clearing
  (`tools/r8q-flash-recovery`, `44ca4352`).
- LineageOS's userdata was overwritten (the accepted cost); the Android system
  (`boot` sda23, `super` sda29) and every NV/calibration partition are
  untouched.

**Modem retest on this root (same day).** `recovery-modem-native.img`
(`g70a925aeee09`, `--overlay modem`) booted from `sda36`. The SD card **dropped
again** at modem power-on (`mmc0: card 0001 removed`) and the root stayed `rw`,
which is exactly what the move was for. The driver stayed unbound until the
stale rescue-test blacklist from MODEM-PLAN §5 was moved aside
(`/etc/modprobe.d/omamobile-r8q-sahara-bringup.conf` → `/root/bringup-backup/`);
then `mhi_pci_generic` bound, `mhi1` powered the SDX55 on, the MHI found it in
SBL, and `/dev/wwan0sahara0` appeared. The SDX55's `QCOM-PCI-MSI` vectors are
live and USB/Wi-Fi survived. Next: the Sahara firmware load on that channel.

## 21. SDDM autologin loops every second; one autologin only (2026-11-18)

The desktop was up, but the phone was hot: CPU zones **77-84 C**, load **12**,
about **12,700 journal lines/min**, with `systemd-logind` and `journald`
spinning while `sddm-helper` authenticated the same autologin every second:

```
Session started true
[PAM] Closing session
Auth: sddm-helper exited with 5
Removing display ... Adding new display ...
```

**Cause:** the `omarchy.desktop` session is `uwsm start -g -1 -e -D Hyprland
hyprland.desktop`; `uwsm` returns once the session is up, so SDDM reads the
session as ended, and the firstboot's `Relogin=true` starts a fresh autologin
immediately. The session itself was fine (Hyprland and quickshell running);
every retry only added a PAM/logind/udev storm.

**Fix:** `Relogin=false` from the r8q device package's
`/etc/sddm.conf.d/zz-r8q-relogin.conf` (it sorts after `autologin.conf`, so it
wins). One autologin, no retry: load **0.4**, **98.9 % idle**, one Hyprland,
about **41 C**, and SDDM stays active. `systemctl enable sddm` keeps it on
boot; the liveness monitor starts `display-manager.service` when it is enabled.
The cost is the `TOUCH-PLAN` 0.4 caveat: a manual Logout shows the greeter (no
on-screen keyboard). The lock screen is unaffected.

The same session confirmed the NPU park: the CDSP secure boot rejects the
unsigned v66 skel ([NPU-PLAN](NPU-PLAN.md) P1).

## 22. The daily root is the native stable image (2026-11-18)

The NPU test image was replaced with `out-r8q-uw/recovery-stable-native.img`
(sha256 `7799b1e2…`): stable kernel `g3667ab368ef9`, the `camera-uw` overlay,
and the userdata root. Verified: `/dev/sda36`, sddm active with one Hyprland
and quickshell, ~44 C, battery Full, and the `s5k3l6xx` (ultrawide) and `hi847`
(tele) sensors ENABLED in the media graph.

**First boot after the flash stopped before the USB gadget** (boot logo/text
console, no enumeration). A forced restart (Power + Volume Down) booted
normally. The image and root checks are identical on the second boot, so it
reads as a one-off initramfs/USB-path stop, not a bad image; watch for a
repeat and capture the initramfs console first if it happens.

The old SD-root dual-boot image (`66fe127f…`) stays the way back when the
userdata root itself needs replacing; `tools/r8q-flash-recovery --stable`
still restores it.

## 23. Modem reaches AMSS on the userdata root, then a completion timeout (2026-11-18)

With the one-channel Sahara loader started before the modem powered on, the
SBL's full set was served over one open channel (`acdb`, `qdsp6sw` 83 MB,
`apps`, `apdp`, ...). The MHI entered **`EE: MISSION MODE`** and the mission
channels (`wwan0qmi0`, `wwan0mbim0`, `wwan0at0`) appeared: the first
mission-mode window on the userdata root.

About a second later the root port logged a **Completion Timeout**
(`[14] CmpltTO | Requester`, Uncorrectable Non-Fatal), the link went down, the
slot reset and the modem returned to SBL. No SMMU context fault. The host's
first AMSS transaction never completed, so either the AMSS was not answering
yet or the shared 3.3 V rail sagged under the AMSS load (the same rail that
browns the SD). The loader's `unexpected cmd 9` after the `mode=2` HELLO is
the last SBL command and is harmless. Full record: [MODEM-PLAN](MODEM-PLAN.md)
§12 run 3.

## 24. There is no dual-boot fallback: a plain power-on boots our recovery image (2026-11-20)

The owner corrected the boot model after repeated observation (latest
2026-11-20, several power cycles in one session): **powering the phone
off and back on, with no key combo, boots `sda24` — our recovery
image, our kernel.** The S20 FE in this configuration has no working
"fall back to LineageOS" behavior, and never had one in this install:
every documented claim that "a normal reboot without buttons boots
LineageOS on `sda23`" was wrong. LineageOS still sits untouched on
`sda23`/`sda29` (contents never written by this work), but it is not
the power-on default and cannot be reached by an ordinary boot; a
forced restart (Power + Volume Down) lands in our kernel as well.
The mechanism by which the bootloader prefers `recovery` on this
phone (boot-recovery flag state, bootloader condition, or AVB state)
is **not diagnosed**; the observation is the owner's, repeated
across cold boots on 2026-11-20.

What this changes in practice:

- After any flash to `sda24`, "reboot the phone" is enough: power off,
  power on, and the new image runs. `Volume Up + Power` is not a
  required step (it selects recovery on a stock Samsung and probably
  still works, but the default power-on already boots `sda24`).
- The safety story of the install is **not** "Android is the fallback
  OS"; it is: `sda24` is the phone's single boot slot, it is always
  re-flashable through Download Mode (Volume Up + Volume Down + USB
  cable from power-off, hardware-level, survives anything we can
  write), and the NV/calibration partitions stay read-only
  (`tools/nv-backup --check` continues to verify 14/15 `SAME` +
  `param` as expected).
- Documents and runbooks that said otherwise were corrected on
  2026-11-20: `RUNBOOK-DISPLAY-GPU.md` (boot and force-restart
  steps), `RUNBOOK-USERDATA-ROOT.md` (the Android contingency),
  `MODEM-PLAN.md` (the flash runbook boot steps),
  `board/success/01-dual-boot-ufs-ro-guard.md`, `FILE-DASHBOARD.md`,
  `board/README.md` and this file. The "LineageOS untouched"
  **partition** claims stay true and stay; only the boot-default
  claims changed.

## 25. USB gadget goes missing on the new image — investigation (2026-11-20)

The phone's gadget (dwc3, NCM+ACM) goes missing on the new image
(g634…, branch `r8q-modem-nom3` @ 0fd2c260d); the stable image
(`omamobile-r8q-signed` @ 3667ab368, `out-r8q-uw/recovery-cam-uw.img`,
66fe127f…) brings it up. Details + diagnostic plan + tooling:
[USB-GADGET-PLAN.md](USB-GADGET-PLAN.md). Phone state (user reads the
logs): "no usb host, crashing in 20s so abl falls back to slot a" —
the phone is down (gadget missing, SSH times out; laptop: nothing on the
USB bus).

What the tree delta shows (stable → reverted, same mainline base):

- DTS +60: the modem PCIe MSI bank (`pcie2_msi`, `&pcie2 { msi-parent }`)
  — PCIe2 stays disabled; enabled by the modem overlay.
- MHI host: `boot.c` +36, `init.c` +88, `main.c` +57, `pci_generic.c` +197,
  `pm.c` +139, `common.h` +7 — the modem driver (event-ring polling,
  no_m2, wake_db, doorbell, keep-devices, SFR, M3 cycle [removed]).
- **No drivers/usb or drivers/phy changes** — the gadget drivers are
  identical; the out-tree DTB diff is phandle-only (benign); the modules
  trees are identical; the Image hashes differ.

Ranked gadget-delta candidates (investigation: general subagent running —
see [USB-GADGET-PLAN.md](USB-GADGET-PLAN.md) §B): (1) the native-vs-SD-root
image difference; (2) the modem DTS/MHI changes interacting with the
gadget (unlikely — different drivers); (3) the phone crashing early on
the new image (the modem?), hiding the gadget registration — testable by
bisecting the modem commits (pack/flash per half); (4) the phone-side
diagnostics (USB-GADGET-PLAN.md §E).

Experiment record: the flow's "reverted image" experiment — "flash ok at
attempt 1" + "gadget not up within 180s" — **inconclusive** (the phone
stayed in download mode — the heimdall flash could not complete:
"Protocol initialisation failed!" ×8 — the USB path was flaky; the phone
is now on the reverted image — "great, that worked" after a manual
heimdall flash succeeded).

Stable escape available: flash `out-r8q-uw/recovery-cam-uw.img`
(66fe127f…) to sda24 via heimdall in Download Mode (USB-GADGET-PLAN.md §E4).
The user said the phone is "left there" — the revert work proceeds
unattended (docs, branches).

## 26. PCIe2 endpoint MSI fault — the gadget/cellular blocker, with mainline fix (2026-11-20)

**Root cause (verified-ranked, subagent analysis + mainline commit):** the
external SDX55M modem on PCIe2 sits behind `apps_smmu`, and its endpoint MSI
writes target the RC `cfg0` address, which is **not mapped in the endpoint's
IOMMU domain** — the writes fault, so the modem's MHI event rings never raise
an interrupt. (Mainline commit message of the fix, verbatim: "the writes
fault, so the MHI event rings never raise an interrupt and the modem never
leaves the primary bootloader after BHI loads sbl1.") On the user's tree the
modem does reach SBL (the loader serves it — the PCIe2 writes only break in
AMSS), and the new image's gadget goes missing — the PCIe2 fault takes the
SoC dwc3 up with it.

**The fix is mainline and is applied to the tree:** `5be4efd5e` ("pci: dwc:
prepare PCIe2 MSI in the endpoint IOMMU domain", mainline tip of the PCIe2
work; mainline = `3667ab368` + `7896cfb4b` (`net: wwan: WWAN_PORT_SAHARA`) +
`4d5ae30a8` (`bus: mhi: pci_generic: load sbl1 on r8q`) + `5be4efd5e`). The
fix: `dw_pcie_irq_domain_alloc()` calls `iommu_dma_prepare_msi(desc,
pp->msi_data)` and `dw_pci_setup_msi_msg()` uses the allocated IOVA
(`msi_msg_set_addr`) for the PCIe2 host (`cfg0_base == 0x64100000`); PCIe0
(the QCA6390 WLAN) is unchanged. Applied (staged) to `wt-r8q`
(`drivers/pci/controller/dwc/pcie-designware-host.c`, +37/-2, 5 fix markers),
kernel branch `r8q-modem-nom3` at `0fd2c260d` (the M3-cycle patch still
reverted there), image packed: `out-r8q/recovery-omamobile.img`, sha256
`c51bd505…`.

**The second mainline-ahead commit is secondary** (`7896cfb4b` +
`4d5ae30a8`: WWAN_PORT_SAHARA + r8q SBL loading via BHI — the port serves
SBL with the loader tool, so these are less load-bearing here; test them
after the PCIe2 fix if needed).

**Test plan (the user's next step):**
1. Power the phone off; from off, hold **Volume Up + Power** (release Power,
   keep Vol Up) → boots `sda24` (the PCIe2-fix image).
2. Tell me when it's up; I check:
   - `lsusb` → expect the gadget (`1d6b:0104` "Omarchy Mobile"),
   - `ls /sys/class/udc` via SSH → expect `a600000.usb`,
   - if the gadget is up → **Run 30**: loader + DIAG client + MMIO probe —
     the PCIe2 fix should arm UL consumption and the +32.4 s death disappears.
3. If not: the PCIe2-bisect plan (the subagent): builds from the stable
   base adding the PCIe2 commits one at a time (commit list on the subagent
   report, §B) — "two-crash rule": if some build crashes the phone, bisect
   there.

**State:** the phone is in its current state (reverted image, gadget missing,
USB/SSH unreachable). The PCIe2-fix image is packed and staged in the tree.
The other blocker of the objective — the GPU/Vulkan + NPU (GMU cx gdsc
wedge, secure-boot wall) — is unchanged (separate investigation,
NPU-PLAN.md P1 §4; the ADR/GPN is offline).

## 27. Adreno 650 CX-gdsc wedge — the Vulkan compute blocker, mainline fix

**The wedge** (`board/failed/08`, well recorded — llama-bench -ngl 99 at
305 MHz → `device lost on Vulkan0` + `msm_dpu: hangcheck detected gpu
lockup rb 0!` + `a6xx_recover: cx gdsc didn't collapse`; `vkcube`
segfaults; render path — vkcube/Hyprland — is fine): a compute submit
trips the Adreno 6xx lockup; hangcheck → `a6xx_recover()`; the CX gdsc
can't collapse (a6xx_gmu.c's `cxpd_notifier_cb` on `GENPD_NOTIFY_OFF`
completes `pd_gate` — mainline polls nothing else), `recover_worker`
loops, session dies, reboot required.

**Root cause (subagent analysis, saved at
`BOARD-REPORT-GPU-WEDGE.md`): mainline gap, QUIC kernelthread v4.0
0/7 "Improve GPU Recovery": "Currently, cx doesn't collapse due to a
devlink between gpu and its smmu. So the struct gpu device needs to
be runtime suspended to ensure that the iommu driver removes its vote
on cx gdsc."** Adreno 650 IS mainline on SM8250 (a6xx_gpu.c,
a6xx_gmu.c, `gpu@3d00000`/`gmu@3d6a000`; mainline sm8250.dtsi disables
adreno+gmu, the port enables them). The mainline `a6xx_recover()`
already does `pm_runtime_put_sync(&gpu->pdev->dev)` then
`wait_for_completion_timeout(&gmu->pd_gate, ...)` — the wedge: the
put_sync doesn't complete the suspend, so the iommu keeps its cx vote.

**The fix (branch `r8q-gmu-recover-fix`):** in `a6xx_recover()`
(wt-r8q/drivers/gpu/drm/msm/adreno/a6xx_gpu.c:1626, mainline
a6xx_gpu.c:1626) add, before the wait:
```c
	pm_runtime_suspend(&gpu->pdev->dev);
```
(commit `29cfed48c`). Also on the branch: `f7f3096b1`, the PCIe2
endpoint-MSI mainline fix (see NOTES §26) — both fixes tested on the
phone when it's up. Packed image: `out-r8q/recovery-omamobile.img`,
sha256 `463ff67a…`.

**Test plan (the phone):**
1. Boot the phone: after the PCIe2 test's boot — from the OMARCHY
   desktop — the GPU is already up (the render path works at 587 MHz).
   - PCIe2 fix verification: `lsusb` → gadget; `ls /sys/class/udc` →
     `a600000.usb` (Run 30).
   - GPU wedge fix verification (needs the user to run the wedge
     repro, per the `08` rules): repro: `llama-bench -m
     LFM3-1.7B-Q4_K_M.gguf -ngl 99 -p 512 -n 128` → capture wedges.
     Expect: before fix — wedges (reboot required); after fix —
     `a6xx_recover: cx gdsc didn't collapse` gone, recovery completes.
     Full VLANG path: `vkcube` + Hyprland compositing (should be
     unchanged). Then the LLM on GPU: `omamobile-ai serve` +
     `GPU_LAYERS=99` LLM queries.
2. Note the render-path rule from board/failed/08: `GPU_LAYERS=0` is
   the default; treat compute-wedge as reboot-required; don't confuse
   the wedges (the wedge takes Hyprland down with it).

## 28. UFS: mainline SM8250 UFS broken — "0 disks" on the PCIe2-fault image, 2026-11-20 (subagent)

**What "0 disks" means** — `initramfs/init:436`: `ls -d /sys/block/sd* | wc -l` → 0 = the UFS produced no SCSI disks; `boot_arch` looks for `PARTNAME=userdata` on `/sys/block/sda` and finds nothing → the init stays in the rescue shell. The phone is on the PCIe2-fault image (`463ff67a…`, mainline tip); the user reads the boot console: `omamobile: === ufs-probe: 0 disks, 0 partitions ===`, "Interconnect & Clock sync_state blocked ... due to 1d84000.ufs" (the UFS HC + the aggre1/gem_noc + gcc-sm8250 clocks), and the DT dependency cycle between `/soc@0/phy@1d87000` (UFS PHY) and `/soc@0/ufshc@1d84000` (UFS HC). The phone "crashes in 20 s, ABL falls back to slot a" — the PCIe2 fault's reset loop takes the phone down during early boot.

**Mainline status (the port's mainline tree — branch `omamobile` @ 2db2bc35b, Linux 7.3-rc6):** mainline sm8250 = mainline @2db2bc35b, **different from the stable branch** `omamobile-r8q-signed` @ 3667ab368 — the stable boots fine with UFS, so the breakage is on the mainline tip side, not the stable side.

**The mainline UFS stack (all mainline, nothing port-changed):**
- HC: `qcom,sm8250-ufshc` → `drivers/ufs/host/ufs-qcom.c` (`ufs_qcom_power_up_sequence` 488, `ufs_qcom_icc_init` 1554 / `ufs_qcom_icc_set_bw` 916 — the flow votes BW via the ICC set, so if that sync never completes, the UFS HC is left unsynced → 0 disks).
- PHY: `qcom,sm8250-ufshc-phy` → `drivers/phy/qualcomm/phy-qcom-qmp-ufs.c` (`qmp_ufs_probe` 2333, `qmp_ufs_power_on` 2061, `qmp_ufs_phy_init` 2155, `qmp_ufs_phy_calibrate` 2086 — `reset_control_assert(qmp->ufs_reset)` (the shared GCC_UFS_PHY_BCR via the PHY's reset = the HC's `phys[0]`), `qmp_ufs_phy_calibrate` → `reset_control_deassert` → poll `PCS_READY`).
- Mainline DTS (`sm8250.dtsi`): `ufs_mem_hc` 2516 (`power-domains = <&gcc UFS_PHY_GDSC>`, `resets = <&gcc GCC_UFS_PHY_BCR>`, `phys = <&ufs_mem_phy>`, icc `aggre1_noc→mc_virt` "ufs-ddr" + `gem_noc→config_noc` "cpu-ufs", clock-controllers `gcc-sm8250` @0x1500000 etc.), `ufs_mem_phy` 2588 (`resets = <&ufs_mem_hc 0>` — the cycle).
- The cycle (B): main.) — this is expected on mainline sm8250 and is not by itself a breakage — it means the mainline flow has no probe-order constraint between the HC and the PHY, so the HC (`1d84000.ufs`) can be the device left waiting in sync_state — the user reads this as "the gcc sm8250 sync state pending due to <id>.phy" — actually the phone's line paraphrases the UFS HC (the message is on the blocked device; the PCIe2's own line is the PCIe2 GDSC one).
- The mainline ICC flow (C): the phone's "sync_state blocked: Multiple interconnect providers and clock controllers are held in sync_state() pending due to 1d84000.ufs..." — the mainline `drivers/interconnect/core.c:1218 icc_sync_state()` is what pends; if that set never completes during the boot, the UFS HC is left without its icc/clk state synced — consistent with a blocked probe/link state.

**The PCIe2 connection**: the PCIe2 breakage (endpoint missing from the bus — "gcc sm8250 sync state pending due to <id>.phy", the modem never leaves PBL) is a separate mainline gap on mainline sm8250 — mainline PCIe2 is `status = "disabled"` and the PCIe2-MSI fix (5be4efd5e) is mainline (applied to the port, f7f3096b1) but insufficient on this device. The PCIe2 investigation subagent (session `ses_e07ac9606ffe6fyzD8HVOq81ke`) is finding the PCIe2 root cause + bisect plan.

**Ranked hypotheses (the UFS mainline-tip breakage)**:
1. The mainline sm8250 tip's UFS flow is stuck: the UFS HC is the blocked device — its icc/clk set (`1d84000.ufs`) never completes sync_state → 0 disks, no UFS error in dmesg (a blocked probe). Plausible mechanism: the PCIe2 fault's reset loop wedging the power-domain/clock sync machinery the UFS flow uses.
2. The PHY (1d87000) never gets to `PCS_READY` — SerDes stuck — then `ufshcd_hba_enable` should print an error; the absence of any UFS error argues against a hard failure, so lower rank.
3. A mainline regression between mainline 3667ab368 (the stable — UFS fine) and the mainline tip 2db2bc35b (the UFS breakage) in the 7.3-rc window — `ufs-qcom.c` and sm8250.dtsi's last mainline commits are both the 7.3-rc6 merge; a mainline sm8250 UFS change there is the likely root.
4. The cycle/defer-loop (a `devm_probe_defer` loop that never resolves) — lower rank (mainline expects the cycle).
5. r8q-specific — low.

**UFS bisect plan ("two-crash rule")** — builds, each: flash/boot, wait ~35 s, read `=== ufs-probe: N disks ===` (or `ls /dev/sda*` over SSH):
1. Stable image `out-r8q-uw/recovery-cam-uw.img` (66fe127f…) — baseline: UFS fine, PCIe2 fine.
2. Current mainline image (463ff67f…) — PCIe2 fault + `0 disks`.
3. Port tree `wt-r8q` (@ f7f3096b1) — PCIe2 should be fixed (doorbell); the UFS is the question.
4. Bisect mainline `3667ab368..2db2bc35b` on mainline — find the UFS regression commit; build at each step (`bin/omamobile kernel` into an explicit `KERNEL_OUT`).
5. Two-crash rule: if a built image crashes the phone or the bus stops answering, stop and restart from a known-good build; the last-good/first-bad pair is the regression window.
6. Expected: the mainline commit that broke mainline sm8250 UFS is identified; then decide a port commit ("add mainline sm8250 UFS fix") for PORT-PLAN.

**Phone-side diagnostics** (once the phone is reachable — the initramfs rescue shell reads dmesg): `dmesg | grep -iE 'ufs|scsi|sd[a-z]|1d84000|1d87000'`; `ls /sys/class/scsi_host/`; `ls /sys/bus/platform/devices/ | grep -i ufs; cat /sys/bus/platform/devices/1d84000.ufs/power/runtime_status`; `ls /proc/device-tree/soc@0/ufshc@1d84000/ /proc/device-tree/soc@0/phy@1d87000/`; `cat /sys/kernel/debug/clk/clk_summary | grep -i ufs`.

**UFS phone state (the user reads it now)**: stuck in the initramfs console (`=== ufs-probe: 0 disks ===`, `omamobile: no /sys/block/sd*`); SSH times out (the SoC USB gadget is down — `a600000.usb: deferred probe pending: dwc3: failed to initialize core` — also a broken path); the PCIe2 endpoint missing from the laptop bus ("gcc sm8250 sync state pending due to <id>.phy").

## 29. Bisect: the no-modem image ALSO fails — UFS 0 disks with PCIe2 disabled (2026-11-20)

Flashed the bisect image `out-r8q/recovery-nomodem.img` (sha256 23e7f68c…, the
current tree f7f3096b1 packed with NO overlays — PCIe2/pcie2_phy stay
`disabled` in the base DT). Result: **it fails the same way** — the user reads
the console: "no userdata partition, ufs did not enumerate" + the fw_devlink
"Fixed dependency cycle(s) with /soc@0/phy@1d87000 and /soc@0/ufshc@1d84000"
line. No gadget on the laptop bus, SSH times out (checks over several
minutes). **So the fault is NOT the modem overlay's PCIe2 enable** — the
modem-overlay images and the no-overlay image fail identically; the working
stable (66fe127f…) is the only image that boots.

Established facts:
- The stable image was packed `--dual-boot --overlay camera-uw`
  (NOTES.md:188; a prior modem-era image NOTES.md:466 was packed
  `--dual-boot --overlay modem`). My packs (463ff67a…, c51bd505…, b55a3440…,
  d87d5999…, 23e7f68c…) were all packed WITHOUT `--dual-boot`.
- BUT the `--dual-boot` delta is post-enumeration only (tools/r8q-boot):
  it empties the storage-guard RW allowlist (all UFS read-only) and adds
  `omamobile.loop=/omarchy-r8q.img` to the cmdline; without it the guard
  allows RW on userdata + mounts the root by label "omamobile-root". None
  of that can stop the UFS from *enumerating* (0 disks) — so the pack flag
  is likely NOT the root cause, but it IS a real difference to record.
- The doorbell node `pcie2_msi` (msi-controller@17a10040, 32 SPIs 832-863,
  no status property = okay) is in the BASE DT of every failing image and
  `CONFIG_QCOM_PCI_MSI=y` — it probes unconditionally, even with PCIe2
  disabled. Its reg (0x17a10040, 4 bytes) sits inside the GIC-600
  distributor/intc region at 0x17a00000 (the SETSPI alias page) — a
  `devm_ioremap_resource` there may conflict with the GIC driver's own
  mem_region request → possible forever-deferred probe → fw_devlink holds
  supplier sync_state ("gcc sm8250 sync state pending due to <id>.phy") →
  downstream probes stall (UFS 0 disks, dwc3 "failed to initialize core").
  Prime suspect for the whole cascade.

Two subagents running (2026-11-20):
1. Image forensics (session ses_e07167bf2ffexB76NkFpcPAFVP): extract both
   images (stable 66fe127f… vs failing 23e7f68c…) and diff cmdline, DTB,
   ramdisk /init, kernel version — the full delta table.
2. Kernel-delta boot-time analysis (session ses_e0716209effeZC1oIaYVKHiMYm):
   the 3667ab368→f7f3096b1 code delta vs UFS enumeration with PCIe2
   disabled — the doorbell irqchip probe vs the GIC mem_region conflict,
   and the fw_devlink sync_state stall mechanism.

## 30. ROOT CAUSE of every failed boot since 2026-11-18: the builds ran without DEVICE=r8q (2026-11-20, image forensics)

The image-forensics subagent (session ses_e07167bf2ffexB76NkFpcPAFVP)
extracted and diffed the working stable image (66fe127f…) vs the failing
no-modem image (23e7f68c…) component by component. **The UFS/clock/power
DTB nodes are byte-identical; the cmdline differs only by
`omamobile.loop=`; the ramdisk guard files are the expected pack-flag
deltas; the kernel UFS source is identical. The difference is the kernel
CONFIG:**

- Failing kernel (`g0fd2c260d21d-dirty`, out-r8q, all my builds since
  2026-11-18: 463ff67a…, c51bd505…, b55a3440…, d87d5999…, 23e7f68c…):
  `CONFIG_SCSI_UFS_QCOM=m`, `CONFIG_PHY_QCOM_QMP_UFS=m`,
  `# CONFIG_QCOM_PCI_MSI is not set`, USB PHYs `=m` — **defconfig +
  omamobile.config WITHOUT the device/r8q/kernel.config fragment**.
- Working kernel (`g3667ab368ef9`, out-r8q-uw): `CONFIG_SCSI_UFS_QCOM=y`,
  `CONFIG_PHY_QCOM_QMP_UFS=y`, `CONFIG_QCOM_PCI_MSI=y`, USB PHYs `=y`.
- The r8q initramfs carries no /lib/modules and loads no modules → **no
  driver ever bound ufshc@1d84000/phy@1d87000 → 0 disks** ("no userdata
  partition, ufs did not enumerate"); the USB PHYs as modules → dwc3
  "failed to initialize core" → no gadget. One config bug explains every
  symptom on every image I packed.
- WHY: `bin/omamobile kernel` was invoked without `DEVICE=r8q`;
  `omamobile.conf` defaults `DEVICE=cheetah`, and no
  `device/cheetah/kernel.config` exists, so the device fragment was
  silently skipped. The failure is invisible at pack time (hence the new
  pack assertion below).
- **Consequences for the record:** the PCIe2/GDSC/doorbell hypotheses in
  §26/§28/§29 were chasing this config bug — on my images the doorbell
  driver was never even compiled. The mainline PCIe2 endpoint-MSI fix
  (f7f3096b1) has NOT had a fair test yet; the modem work must be
  retested on a correctly-built image. The earlier out-r8q-modem3
  (b60fde52, Nov 17) images predate my builds and may have been
  correctly configured — their PCIe2-fault evidence stands separately.
- **Fix:** rebuild with `DEVICE=r8q KERNEL_DIR=… KERNEL_OUT=… bin/omamobile
  kernel` (running now), verify `out-r8q/.config` has
  `CONFIG_SCSI_UFS_QCOM=y` + `CONFIG_PHY_QCOM_QMP_UFS=y` +
  `CONFIG_QCOM_PCI_MSI=y`, then pack `--dual-boot` and re-test.

## 31. Corrections from the kernel-delta analysis (2026-11-20, subagent ses_e0716209effeZC1oIaYVKHiMYm)

The tree-A(3667ab368)→tree-B(f7f3096b1) kernel-delta analysis exonerates
the entire source delta for the failing boots and corrects two earlier
hypotheses:

- **§29's doorbell-vs-GIC mem_region "prime suspect" is disproven.** The
  sm8250.dtsi `intc` node claims GICD 0x17a00000–0x17a0FFFF and GICR
  0x17a60000+; the doorbell's `reg = 0x17a10040..0x17a10043` lies OUTSIDE
  both windows (the GIC-600 SETSPI alias page, deliberately unclaimed by
  the mainline DT) — no `devm_request_mem_region` collision is possible,
  and even an overlap would be a one-shot -EBUSY, not a defer loop. The
  probe is warm-proven-good: runs 16–18 (kernel g238696e3884c, same DT with
  the doorbell node) logged `doorbell 0x0000000017a10040 reads 0x0, 32
  SPIs from 832` plus 62 ufshcd/sd lines and MHI mission mode. The probe
  writes nothing, defers never, and no SPI is allocated until a PCIe
  endpoint requests an MSI.
- **§28's "mainline regression 3667ab368..2db2bc35b" hypothesis does not
  apply to this lineage**: merge-base(3667ab368, f7f3096b1) is exactly
  3667ab368 — the wt-r8q branch has no mainline window; 2db2bc35b is a
  different branch.
- The whole delta is boot-inert with PCIe2 disabled: MHI/wwan/qrtr run
  only from an MHI PCI device; the pcie-designware-host.c changes are
  gated on cfg0 0x64100000 (pcie2 only; pcie0 stays byte-identical); the
  a6xx recover change runs only on GPU recovery. And decisively:
  `git diff 69e130a27 0fd2c260d` is EMPTY — the first failing image
  (c51bd505) ran content-identical kernel code to the last proven-good
  kernel (Run 25, g69e130a27a94, full system up). The kernel code never
  broke; the build config did (§30).
- Remaining cold-vs-warm nuance for the retest: every good tree-B boot in
  the logs was WARM (push/reboot-to/stage loops); every failing boot was a
  COLD power cycle after a heimdall flash — but all the cold failures were
  the DEVICE-less builds (§30), so the config explains them. The retest
  order: first a cold boot of the correct-config tree-B with NO modem
  overlay (tests UFS + gadget + doorbell-cold + gives a working daily
  image), THEN the `--dual-boot --overlay modem` image for the endpoint-MSI
  fix's first fair test. If the modem image cold-fails with the doorbell
  probe visible, the fallback is the subagent's discriminator: kexec the
  same kernel warm from the running system (warm-good + cold-bad isolates
  the doorbell), and the fix is `status = "disabled"` on the doorbell node
  in the base DT + enabling it in the modem overlay.

## 32. The doorbell root cause: stock NEVER reads the MSI page — and the GPU stack is back (2026-11-20)

**The LineageOS recovery.img reverse-engineering (session
ses_e0642a82affel8u3T2Q4x80IVY, kernel 4.19.325-cip136-st20-perf)
found the design difference that killed our boots.** Stock's doorbell
DT node declares `reg = <0x17a10040 0x0>` — ZERO SIZE, an
address-as-data — so no driver can ever map or read it. The vendor
kernel's of_msi/pld_get_msi_address path reads `reg.start` as a VALUE,
hands it to the PCIe endpoint as the MSI target, and maps it into the
ENDPOINT's IOMMU domain; the endpoint's DMA write of the SPI number
raises the interrupt. The AP never executes a load from 0x17a10040 in
stock — the only stock-legal AP-side touch is a WRITE to GICD_SETSPI_NSR
at 0x17a00040. Our driver (drivers/irqchip/irq-qcom-pci-msi.c) is the
only configuration in which a CPU load from the GIC-600 SETSPI alias
page executes during kernel start (devm_ioremap_resource + probe-time
readl_relaxed + ioremap(0x17a00000, SZ_64K)) — and that is the
configuration that power-offs the phone on cold/ssh-reboot boots.

**The ablation confirms it end to end.** With the doorbell
`status = "disabled"` in the base DT (kernel commit c8be07674, the
modem overlay enables it with pcie2) the correct-config tree boots:
`tools/r8q-test-image` PASSED (kernel gc8be07674331, UFS 4 disks, gadget
a600000.usb; log logs/r8q/test-20261120-171712-dmesg.log), after the
forensics-proven config fix (NOTES 30) and with both fix commits in.

**The GPU stack is back on the new tree.** The msm module for
gc8be07674331 was built (`bin/omamobile modules` with DEVICE=r8q),
pushed with tools/r8q-push, extracted into /usr/lib/modules on the
phone + depmod, and — after a REBOOT (NOTES 346: loading msm by hand
fails; the module must load at boot) — **renderD128 + card1 came up:
"adreno 3d00000.gpu" probed, qcom/a650_sqe.fw loaded, and vulkaninfo
reports "Turnip Adreno (TM) 650", API 1.3.354.** The installed
omamobile-llama is deliberately CPU-only (its PKGBUILD: "the
Adreno/Vulkan compute path wedges the GPU on this board —
board/failed/08"), so a Vulkan build was configured from the same
on-phone source tree (build-vk/, GGML_VULKAN=ON; the vulkan-headers +
shaderc packages are installed) to run the wedge repro against the
29cfed48c fix.

**The doorbell driver fix (the modem path, next):** per the RE —
(1) drop the probe-time readl + devm_ioremap_resource: keep
`res->start` as the address value for `iommu_dma_prepare_msi()` (stock's
"map msi address" mechanism); (2) change our DT to stock's zero-size
`reg = <0x0 0x17a10040 0x0 0x0>`; (3) drop or make write-only the
probe-time ioremap/debugfs path (the stock-legal AP-side debug is a
WRITE to GICD_SETSPI_NSR 0x17a00040). The cold-boot analysis subagent
is sanity-checking the fix shape against the GIC-600 alias-page
behavior (session ses_e06315ea9ffeAPOrA3ULb56tbF).
