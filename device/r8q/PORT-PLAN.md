<!-- SPDX-License-Identifier: MIT -->

# Omarchy Mobile — Samsung Galaxy S20 FE (`r8q`) Port & Integration Plan

- **Status (2026-11-14):** LineageOS 23.2 (`4.19.325`) still untouched on `boot` (`sda23`) + `super` (`sda29`) + F2FS `userdata` (`sda36`). Dual-boot RO on the **old** kernel held a **69 min idle record** with zero stalls (idle is survivable; sustained UFS+USB writeback is the reliable trigger — crash #14 landed ~57 min into chunk 5/7 of a 12 GB writeback). Root cause of the wedge found in the tree (RPMh TCS leak, §0.1); kernel rebuilt with `CONFIG_SOFTLOCKUP_DETECTOR=y` and flashed to `sda24` (79M, SHA256 `e0be1708...f951e`, AVB signed). **That build froze at 0.630 s of boot — our own lock leak in the new timeout path, not the hardware (§0.2)**; the phone was wedged until it was restarted, and came back on the *old* image, which is how we learned the heimdall write to `sda24` had not landed (§0.2, third lesson). Rebuilt and repacked with the fix: `197ef0a3...253671`, kernel `ge96cf40a1ee9`, being flashed now. Device package `0.1.12` (liveness net + battery telemetry) built, signed and tested (all suites pass), not yet installed. The rootfs re-transfer is parked: its 12 GB is only needed for the rebuilt rootfs, and the fixed kernel needs just a 17 MB module injection into the live one.
- **Kernel branch:** `omamobile-r8q` (`~/omamobile-kernel/wt-r8q`, out `~/omamobile-kernel/out-r8q`, build with `DEVICE=r8q`)
- **Upstream:** `derekross/linux-omarchy-mobile` (`v7.3-rc6`)

---

## 0. Current Status & Open Work

Mainline `7.3.0-rc6` boots from `recovery` (`sda24`); LineageOS untouched on `boot` (`sda23`) + `super`. Dual-boot (`tools/r8q-boot pack --dual-boot`) boots `/omarchy-r8q.img` from `mmcblk0p1` exFAT (`sdhci@8804000`) while keeping all of UFS (`sda*` including `userdata`) read-only. Hardware verified or mapped: USB NCM (`172.16.42.1`), `ttyGS0`, UFS, `stmfts` touch (`5-0049`), Wi-Fi (`wlp1s0`), Hyprland (`simpledrm` + `llvmpipe`), 120 Hz AMOLED panel, `Adreno650v3` (`587 MHz`), dual `cs35l41`, `max77705`+`pca9468`.

**2026-11-14 state (offline fixes staged, awaiting Gate A flash):** the RPMh/RCU fix (§0.1) is live on `ge96cf40a1ee9` (`0.1.13`). Offline updates now staged in `omamobile-r8q` + `omamobile-device-r8q 0.1.14` + `device/r8q/overlays/*.dtso`:
- **Display & GPU (`M3.3`):** upstream `a650_sqe.fw` `0x112` (`>= 0x095`, `31964` B) verified & staged via `tools/r8q-extract-firmware` into `~/.cache/omamobile/firmware/r8q/lib/firmware/qcom/` and `qcom/sm8250/`; `samsung,s6e3fc3-r8q` panel + `&mdss`/`&mdss_dsi0` DT nodes added (disabled in base DTB, enabled by `device/r8q/overlays/display.dtso` which also clears `&dispcc`'s `protected-clocks = [];`); fixed `80-r8q-gpu.rules` (`KERNEL=="3d00000.gpu"`, min OPP `305000000`); updated `omamobile-r8q-panel.service` to skip `fb0` unblank when `/sys/class/backlight/*` exists.
- **Audio & Remoteprocs (`M5.3`):** `gpi.c` EE offsets (`0x1000`/`0x6000`) for `"samsung,r8q"`, `sm8250.c` TDM clock/slot/channel-map ops + `S32_LE` fixup, `TERTIARY_TDM_RX_0` (`dai@38`) + dual `cs35l41@40`/`@41` DT nodes (disabled in base DTB, staged via `audio-amps.dtso` and `audio-sound.dtso`), `&adsp`/`&cdsp`/`&slpi` (`remoteprocs.dtso`), and UCM2 `conf.d/sm8250/Samsung-Galaxy-S20-FE.conf`.
- **Network, Mirrorlist & Preflight (`M7`/`M4`):** `omamobile-mirrorlist` points at `https://pkgs.omamobile.org/stable/$arch`; `20-usb-gadget.network` sets `Metric=2000` + `DNS=9.9.9.9` with `omamobile-device-r8q.install` removing stale `/etc/systemd/network/20-usb-gadget.network`; new Rust CLI `r8q-preflight` validates firmware/SQE, UFS RO guard, `msm` blacklist, and DT overlays. Note: `0023-lock-a-glance-view-for-touch-only-devices.patch` + `hypr/phone.lua` already wire `wakeTouch()` and `XF86PowerOff`, so restoring `/sys/class/backlight/*` via `display.dtso` resolves the `displayDark` blocker without touching shell lock logic (`idle.lock` stays off until Gate B verifies `msm` + panel).

### Fixed in Tree / Active on Hardware

| Symptom | Root Cause | Fix & State |
| --- | --- | --- |
| Dead power button / no wake source | `CONFIG_POWER_RESET_QCOM_PON=m`, no early modules | Built-in `=y` — **live** |
| Freeze minutes after boot (`s2idle`) | `s2idle` never resumes on Samsung TZ | `# CONFIG_SUSPEND is not set`, logind gate, masked targets — **live** |
| Hangs needed a button hold | Watchdog drivers unset/modular | `CONFIG_QCOM_WDT=y`, `CONFIG_SOFT_WATCHDOG=y`, `RuntimeWatchdogSec=30s` — **live** |
| Disk I/O freeze after 150 ms idle | UFS `auto_hibern8` exit fails + unsupported WB query | `omamobile-r8q-ufs.service` writes `0` (`0.1.4`); kernel skips WB attr — **live** |
| RCU stalls on cores in deep `cpuidle` | `state1` power-collapse hangs cores (Samsung TZ) | Service disables `state1` (WFI only) (`0.1.4`) — **live** |
| ~11 min crash (`rpmh_write_batch -110` in `sugov`) | `schedutil` OPP transitions time out on AMC `RPMh` votes | All policies pinned `performance` (`0.1.6`) — **live** |
| ~11 min crash, Wi-Fi off (`ufshcd_ungate_work` -> same `-110`) | UFS clock ungating votes `icc_set_bw` over AMC `RPMh` | Runtime `clkscale_enable=0` + `clkgate_enable=0` — **live; queued in `0.1.7`** |
| Boot hung on `multi-user.target` | Hand-installed `omamobile-bluetooth.service` hung in `btmgmt` | Unit disabled; exclude until BT verified — **live** |
| `bt-agent` user unit restart-loop | BlueZ unit without `/usr/bin/bt-agent` | `systemctl --user mask` — **live** |
| `zram-setup` hung at boot | `CONFIG_ZRAM=m`, no modules | Built-in `CONFIG_ZRAM=y` + 4 GiB `/swapfile` — **live** |
| Blind hangs on network/UI wedge | No USB serial getty | `console=ttyGS0` + `serial-getty@ttyGS0` (`0.1.4`) — **live** |
| Panel blanks at 10 min, never wakes | `consoleblank=600` ignores DRM pageflips | `omamobile-r8q-panel.timer` unblanks `fb0` every 4 min (`0.1.4`); `consoleblank=0` in next flash — **live** |
| Sluggish UI on `llvmpipe` | Animations/blur with no GPU | Disabled in `hyprland-r8q.conf` until `renderD128` is live (`0.1.4`) — **live** |
| Missing modules for `-dirty` kernel | Tarball `utsrelease` mismatch | Rebuilt/pushed 1,818 modules (`depmod` clean) — **live** |
| Silent wedge on one core, no detector fires | a hung core stops ticking; RCU stall only *reports* | `CONFIG_SOFTLOCKUP_DETECTOR=y` + `softlockup_panic=1` in `tools/r8q-boot` cmdline (panics on a 10 s atomic spin; `panic=10` then resets) — **live on `ge96cf40a1ee9`** (`sysctl` and cmdline both read `1`) |
| Wedge class with no ticks at all (`0.1.11` and earlier) | power-side hang invisible to every kernel detector | Userspace per-CPU liveness net (`omamobile-r8q-liveness.service`, FIFO-99 heartbeat + monitor: 2 confirmations of a >10 s stale online core → `sysrq c` → `panic=10`) in `0.1.12` — **live in `0.1.13`**: 8 heartbeats, each `1` line / `21` B, advancing `609.25 s → 613.25 s`; `--check-only` found nothing stale |
| `omamobile-r8q-battery` reports nothing | MAX77705/MAX17042 were never auto-loaded | `0.1.13`'s `r8q-battery-modprobe` loads `max17042_battery` + `max77705-charger` — **live**: `capacity=99`, `voltage_now=4.33 V`, `temp=30.5 °C`, `health=Good`, charger `USB/Charging` |
| Device packages would not install at all | phone RTC ~4 days behind + missing keyring (§0.3) | `pacman-key --init`, clock set from the laptop, then `--lsign-key` — **live**, `0.1.13-1` installed |

### 0.1 Kernel fix for the RPMh/RCU wedge (2026-11-14)

The wheel that took crashes #1-#14 is a **TCS leak in `rpmh-rsc`**. A resource state change claims a TCS; the only place a TCS is reclaimed is the completion IRQ. On a 10 s completion timeout (`RPMH_TIMEOUT_MS`) the driver returned its error and *kept* the TCS, so `rpmh_rsc_send_data()` on the next vote (any `sugov` OPP transition, any UFS clock ungate) waited **forever** for a free TCS: a spin with interrupts off that no watchdog on that core can see, which is exactly the pstore signature (a CPU NMI-dead while its neighbours answer, zero error lines).

Four commits on `omamobile-r8q` (`~/omamobile-kernel/wt-r8q`), adapted from Qualcomm's upstream series *"[PATCH v4 0/3] Output debug information from RSC"* (`20260913-rpmh-timeout-debug-v1-v4-0-e94d3e416ea1@oss.qualcomm.com`, plus the v5 review fixes: return values of `cmd_db_read_name()`/`irq_get_irqchip_state()` are checked, and the per-TCS AOSS state is accumulated rather than overwritten):

| Commit | Author | What |
| --- | --- | --- |
| `a807c8708` | Maulik Shah | `soc: qcom: cmd-db: export RPMh accelerator type string helper` |
| `df727e3ec` | Maulik Shah | `soc: qcom: cmd-db: add reverse address-to-name lookup` |
| `ebeabc490` | Raju P.L.S.S.S.N | `soc: qcom: rpmh-rsc: Output debug information from RSC` |
| `d1f598f2c` | ours | `soc: qcom: rpmh-rsc: bound the TCS claim wait and reclaim stuck TCSes` — `RPMH_TCS_CLAIM_TIMEOUT_MS=10000` returns `-ETIMEDOUT` instead of waiting forever, `rpmh_rsc_reclaim_timeout()` runs on all three timeout paths, and `rpmh_rsc_dump_state()` is lock-safe |

Adapted patches keep their original authors and trailers (nothing is copied from another project; the series is upstream Qualcomm work under GPL-2.0-only). Checkpatch `--strict` is clean on the three adapted commits; ours reports only the expected missing `Signed-off-by`, which a human adds.

**State: booted and verified 2026-11-14.** Release `7.3.0-rc6-omamobile-push1-ge96cf40a1ee9`, image SHA256 `197ef0a3…253671`, flashed raw with `heimdall flash --RECOVERY … .img --no-reboot` after reading `sda24` back and finding the *previous* release still there (§0.2, third lesson). The boot reports `release-match: 1`, `softlockup_panic` on the cmdline and in `sysctl` as `1`, no `rpmh` timeout lines, root `/dev/loop0 ext4` (UFS still read-only). The 1,818 modules for this release are injected into the live rootfs (`.ko` count and `modules.dep` lines both `1818`, `max77705`/`max77705_charger`/`max17042_battery`/`snd-soc-sm8250` all resolve) and `depmod` ran on the phone. The 12 GB `omamobile-rootfs-r8q-*.img.zst` transfer stays parked until the phone is stable, as it is only needed for the *rebuilt* rootfs. The first `-110` must still be captured with `rpmh_rsc_debug`.

### 0.2 The boot freeze of 2026-11-14 (our own bug, found from the screen)

The first boot of that image froze at **0.630 s**, right after `input: gpio-keys as ... input3`, with `Run /init as init process` **never printed** — so the initramfs never ran and this was not the crash wall. Read from the panel (the only channel there is: no USB, no console, no pstore without a reboot):

```
[ 0.536656] scsi host0: ufshcd
[ 0.536403] mmc0: SDHCI controller on 8804000.mmc [8804000.mmc] using ADMA
[ 0.624711] scsi 0:0:0:49488: Well-known LUN   SAMSUNG KLUDG4UHDB-B2D1  0090 PQ: 0 ANSI: 6
[ 0.630164] input: gpio-keys as /devices/platform/gpio-keys/input/input3
```

UFS, `loop`, `zram` and `softdog` had all come up; the next vote after UFS's async probe completes is its interconnect sync-state, i.e. an `rpmh_write_batch()`. **The cause was our own `d1f598f2c`:** `rpmh_rsc_send_data()` returned `-ETIMEDOUT` on a claim timeout while still holding `drv->lock`. `wait_event_lock_irq_timeout()` re-acquires the lock before returning, so that early `return` left a spinlock held and IRQs disabled behind it; every later RPMh user spun on it forever, and the `dev_warn` that was meant to announce it blocked on the console semaphore of a CPU wedged on the same lock. That is why the screen froze with no new line at all.

Two lessons, both now in the code:

- The timeout path of a lock-taking wait macro must release the lock explicitly; the "it returns with the lock held" asymmetry is the whole bug (fixed in `e96cf40a1`, which reuses upstream's own call shape and tests `tcs_id` instead of the discarded return value).
- **This kernel has no hard-lockup detector** (`CONFIG_ARM64_PSEUDO_NMI` and `CONFIG_HARDLOCKUP_DETECTOR` are both unset), so an IRQs-off spin prints nothing, ever. `CONFIG_SOFTLOCKUP_DETECTOR` needs the tick and cannot see it. Enabling the pseudo-NMI watchdog is the follow-up (§M2.8), one boot at a time.

A third lesson cost a whole flash cycle, and it is a rule now: **a flash is only real when the release string changes.** He was told the heimdall write to `sda24` succeeded; the phone, still alive on the old kernel, says otherwise:

```
$ uname -r
7.3.0-rc6-omamobile-push1-g69c7b002164a
$ dd if=/dev/sda24 bs=1M count=96 | strings | grep -o '7\.3\.0-rc6-omamobile-push1-g[0-9a-f]*' | sort -u
7.3.0-rc6-omamobile-push1-g69c7b002164a
```

The partition held the *previous* image, which is also why the 0.63 s freeze had no `softlockup_panic=1` behind it. So the freeze was the old code's unbounded TCS claim wait — the original crash class, now reproduced at boot — and not a second regression. Before trusting any flash: `dd` the partition and grep its release, and after the boot compare `uname -r`. sda24 read back `ro=1` from the storage guard as it should.

### 0.3 Why a device package would not install (2026-11-14)

`pacman -U` failed on a phone that had been running our packages for days:

```
error: omamobile: key "54A3D15B…DAE7638" is unknown
error: keyring is not writable
error: failed to prepare transaction (invalid or corrupted database)
```

Three separate causes, and only the last one is about trust:

1. **`/etc/pacman.d/gnupg` did not exist** on this install — `pacman-key --init` had never run, so nothing could be verified at all.
2. **After `--init`, `pacman-key --add` silently imports nothing and `--lsign-key` fails** with `could not be locally signed`, because **the phone's RTC was ~345,000 s (4 days) behind**: the port key is dated *in the future*, gpg discards its user ID (`new key but contains no user ID - skipped`), and a keyless entry cannot be signed. `pacman-key --list-keys` then shows `pub:i:` (invalid), not `pub:-:`. Fix: set the clock from the laptop first (`date -u -s @$(date +%s)`), import, and `--lsign-key` succeeds (`→ Locally signed 1 key`, then `[ full ]`).
3. The signed `[omamobile]` database needs that same key trusted, so 2 blocks 1.

Lesson: **check the clock before touching any signing on this phone.** A skew large enough to matter shows up as a gpg warning (`key … was created N days in the future (time warp or clock problem)`), and every later step fails with an error that names the wrong thing (key, keyring, database). Also note the installed version was `0.1.10`, not the `0.1.11` the plan had assumed — always read `pacman -Q` rather than trusting the plan.

### Open Blockers (Priority Order)

1. **UFS RPMh AMC-vote soak & `0.1.7` package:** Wi-Fi-off bisection proved crashes #3-#6 came from `ufshcd_ungate_work -> ufs_qcom_icc_update_bw -> icc_set_bw -> rpmh_write_batch (-110)` (same voter as the `sugov` crash, different trigger: any UFS clock ungate, even background journal/SSH I/O). Mitigations VERIFIED 2026-11-12: with `performance` + no deep idle + `auto_hibern8=0` + `clkscale/clkgate=0` (+ Wi-Fi off for the bisect), the phone idled past 17 min with zero `ramoops` events — first survival beyond the ~11-min wall. `0.1.7` installed on-device (UFS helper verified live via `restart`, not `start`, on an already-active oneshot). Evening 2026-11-12: crashes #7 (~22:34), #8 (~22:47) and #9 (~23:46) all landed with the full `0.1.7` mitigations live — silencing the votes does NOT hold. Crash #8's pstore (our own read): first stall at 700 s, repeated stalls past 919 s, CPU 7 NMI-dead three times while CPUs 0/2 answered theirs, zero error lines throughout — one wedged prime core. Crash #10 (`0.1.9` boot `29d4ee8d`): first stall 552 s, silent (no NMI dump captured) — 0.1.9 changes nothing about the wall, as expected. Install persists across resets (`0.1.9-1` intact, both units auto-active on the next boot). Boot `ae226d76` (`0.1.9`) ran clean to 652 s, then died at the wall — confirmed crash #11: first stall 700 s, CPU 4 NMI-dead while CPU 1 answered, zero error lines. Victims now span little (1), big (4, 6) and prime (7): the wedge is not prime-specific. CPU-7-offline soak STARTED 2026-11-13 on boot `6e38eaed` at 635 s (announced, reversible; dead-man timer armed then removed — verified no `cpu7-deadman` unit/timer exists, `cpu7=0` holds past the T+120 s fire point). RESULT 2026-11-13: `cpu7=0` survived idle to **1757 s (29.3 min) with zero stalls**, then **crashed (#13) at `1768-1780 s`** seconds after an `scp` push of `LFM3-1.7B-Q4_K_M.gguf` began writing to `sda36` (156 MiB written). `console-ramoops-0` on boot `bd4fac14` captured `[ 1768.090086] watchdog0: pretimeout event` and `[ 1780.137259] rcu: INFO: rcu_preempt detected stalls on CPUs/tasks: 1, 4, 6 (detected by 2, t=5252 jiffies, ncpus=7)`. Offlining CPU 7 delays idle failure, but does NOT cure the wedge under sustained UFS+USB writeback. Soak phase ENDS here; userspace mitigations exhausted. Idle record since: **69+ min with zero stalls** on boot `bd4fac14` (dual-boot RO, nothing writing), so idle alone no longer reproduces the wall. Crash **#14** (2026-11-13, the last one on the old kernel): the 69 min boot died **~57 min in**, during chunk 5/7 of the 12 GB rootfs writeback over USB NCM to the microSD — chunks 0-4 were sha-verified on the card and chunk 5 was left partial at 492,466,176 B, which is why `tools/r8q-push` now resumes instead of restarting. Trigger summary: sustained UFS writeback + USB NCM traffic is reliable, idle is not. Root cause and fix: §0.1. **The wedge fix is in the kernel, booted nowhere yet.**
2. **Adreno 650 & `S6E3FC3` Display (`M3.3`) — offline fixes ready for Gate B:** `tools/r8q-extract-firmware` now checks `sqe_version` (`buf[4..8] & 0xfff`) and replaces the LineageOS `0x093` `a650_sqe.fw` with verified upstream `linux-firmware` (`0x112`, `31964` B) under both `qcom/a650_sqe.fw` and `qcom/sm8250/a650_sqe.fw`. Kernel + DT changes (`panel-samsung-s6e3fc3.c` for `samsung,s6e3fc3-r8q`, `&mdss`/`&mdss_dsi0`/`panel@0` disabled in base DTB, `display.dtso` overlay clearing `&dispcc` `protected-clocks`, `80-r8q-gpu.rules` matching `3d00000.gpu` / `305000000`, `omamobile-r8q-panel.service` yielding to `/sys/class/backlight/*`) are built offline; pending Gate B flash/boot on hardware (`50-r8q-gpu-blacklist.conf` remains active until `display.dtso` alone boots cleanly). Note: this unit's active panel via `/proc/cmdline` is `ss_dsi_panel_S6E3FC3_AMS646YD01_FHD`.
3. **Staged peripherals (one per boot via overlays, `ttyGS0` armed):** Wi-Fi UX/persistence (`usb0` unmanaged); `bluetoothctl` scan with timeout guard (`bt-agent` stays masked); ADSP/CDSP/SLPI (`remoteprocs.dtso`) -> dual `CS35L41` (`audio-amps.dtso`) -> `TERTIARY_TDM_RX_0` `&sound` + `sm8250.c` TDM ops (`audio-sound.dtso`) + UCM2 `Samsung-Galaxy-S20-FE.conf`; `MAX77705` `-EINVAL` + `/sys/class/power_supply/*`.
4. **`omamobile-update` fetch failure (`0.1.14` offline fix ready):** `omamobile-device-r8q 0.1.14` updates `20-usb-gadget.network` (`Metric=2000`, `DNS=9.9.9.9` matching `cheetah`), cleans stale `/etc/systemd/network/20-usb-gadget.network` (with `Metric=900` or missing metric) in `.install`, and updates `omamobile-mirrorlist` to `https://pkgs.omamobile.org/stable/$arch`.
5. **Lock-screen touch (`LockGlance.qml`):** `0023-lock-a-glance-view-for-touch-only-devices.patch` already wires `wakeTouch()` -> `wakeRequested()` -> `runWake()` (`displayDark = false`) and `hypr/phone.lua` binds `XF86PowerOff`; `r8q` only lacked `/sys/class/backlight/*` under `simpledrm`. Keep `idle.lock { enabled = false }` until Gate B verifies `panel-samsung-s6e3fc3` + backlight wake on hardware.
6. **Next `recovery` flash:** bake `consoleblank=0` into cmdline on next human-approved `recovery` write. Never touch `boot`/`super`/bootloader.

> **Operational Rules ([AGENTS.md](../../AGENTS.md), [CONTRIBUTING.md](../../CONTRIBUTING.md)):** read-only probes first; **one hardware change per boot**; human approval before any reboot, flash, or partition write. Build in container (`tools/build-packages --local r8q`), never compile on the phone. Enable units via `/etc` (`systemctl enable`). Never commit identifiers (serials, MACs, IMEIs, NV dumps).

---

## 1. Hardware & Partition Reference

| Subsystem | Spec | Source |
| --- | --- | --- |
| **Device** | Galaxy S20 FE Snapdragon (`SM-G780G`/`SM-G781B`), `r8q` | `ro.product.device` |
| **SoC/CPU** | Snapdragon 865 (`SM8250`/`kona`), Kryo 585 (1x2.84 + 3x2.42 A77, 4x1.80 A55) | `ro.soc.model=SM8250` |
| **GPU/RAM** | Adreno 650 (Turnip/Freedreno) / 6 GB LPDDR5 (~5.5 GiB) | `MemTotal: 5747672 kB` |
| **Storage** | UFS (`1d84000.ufshc`, `sda`-`sdd`), **single-slot Non-A/B** | `ro.build.ab_update=false` |
| **Bootloader** | Samsung S-Boot/ABL (`G780GXXSHEYJ1`, `SVB1.0`, `orange`) | `dtbo_idx=7`, `dtb_idx=0` |
| **Display** | `ss_dsi_panel_S6E3FC3_AMS646YD01_FHD` (verified active via `/proc/cmdline`, `lcd_id=0x805081`; stock DTB also defines alternate `EA8079B_AMS646YB01`) 1080x2400 AMOLED 120/60 Hz DSI-cmd (`te-gpio=66`, `h=92/92/92`, `v=8/12/4`, `clk=0x42294180`) | Live `/proc/cmdline` & LineageOS DTB |
| **GPU (Live)** | `kgsl-3d0@3d00000` (`Adreno650v3`, `chipid=0x6050002`, `587 MHz`), `a650_gmu.bin`, `a650_sqe.fw`, `a650_zap.{mdt,b00..b02}` | Live DTB / `/sys/class/kgsl` |
| **USB/Radio** | DWC3 (`a600000.dwc3`), QCA6390 (PCIe+UART, `hsuart8` / `998000` wakeup), `nq@28` (`i2c@984000`, `gpio6/7/110/111`) NFC | Live DTB / `dmesg` |
| **Touch** | `stm,fts_touch` (`fts5cu56a_r8`) on `994000.i2c` (`0x49`, `irq=gpio15`, `tsp_ldo_en`); `CONFIG_TOUCHSCREEN_STMFTS=y` in mainline (`CONFIG_TOUCHSCREEN_FTS5CU56A` did not exist) | Live DTB / `/proc/bus/input/devices` |
| **PMIC/Power** | `pm8150/b/l`, `980000.i2c`: `max77705@66` (MUIC/charger/FG) + `pca9468@57` (direct charger) + `0-003b` (`mfc-charger` wireless) | Live DTB / `/sys/class/power_supply` |
| **Audio** | `i2c@a8c000`: dual `cs35l41@40` (`cs35l40-bot`, `reset=gpio69`) + `@41` (`cs35l40-rcv`, `reset=gpio75`), `irq=gpio84`, `48kHz/24-bit` | Live DTB / `dmesg` |
| **MicroSD** | `sdhci@8804000` (`&sdhc_2`, `4-bit`, `vmmc=L9C`, `vqmmc=L6C`, `cd-gpios=tlmm 77 GPIO_ACTIVE_HIGH` `bias-disable`) | Live DTB / `/vendor/etc/fstab.qcom` |

| Partition | Block Device | Rule |
| --- | --- | --- |
| `boot` | `sda23` | LineageOS 23.2 (`4.19.325`) — **untouched primary OS** |
| `recovery` | `sda24` | Dual-boot selector slot (`recovery-omamobile.img` or LineageOS `recovery.img` via `heimdall --no-reboot`) |
| `dtbo` / `super` | `sda22` / `sda29` | Android DTBO + dynamic partitions |
| `userdata` | `sda36` | Android F2FS + hardware `inlinecrypt` FBE (`107 GiB` — kept untouched for LineageOS in `--dual-boot` mode) |
| `mmcblk0p1` | `8804000.sdhci` | Dual-boot host (`119 GiB` exFAT microSD): holds `/omarchy-r8q.img` ext4 rootfs via `omamobile.loop=/omarchy-r8q.img` while `sda*` stays 100% RO |
| `efs`, `sec_efs`, `persist` | `sda9`, `sda12`, `sda8` | **NV/calibration — READ-ONLY (M0 backup)** |
| `modemst1/2`, `fsg`, `fsc` | `sda1/2`, `sdd3`, `sda3` | **Modem EFS — READ-ONLY (M0 backup)** |
| `modem`, `dsp` | `sda20`, `sda21` | Radio/DSP firmware (read-only extraction) |
| `xbl*`, `abl`, `tz`, `aop`, `hyp`, `vbmeta*` | `sdb`, `sdd` range, `sda27` | **Bootloader/TZ — NEVER TOUCH** |

`device/r8q/device.conf`: `BOOT_METHOD=ram-only`, `UFS_RW=""` — Pixel tools cannot touch `boot`/`super`. Normal reboot lands LineageOS (`sda23`); `adb reboot recovery`/VolUp+Power boots Omarchy (`sda24`). **Scope:** recovery-slot boot only; no other boot flows or third-party kernel params apply.

---

## 2. Architecture (`gs201` vs `sm8250`)

| Layer | `cheetah` (Pixel 7 Pro) | `r8q` (S20 FE) |
| --- | --- | --- |
| **Slots/Image** | A/B, Header v4 (`fastboot`) | Single-slot, Header v2 + `recovery_dtbo` + DTB + AVB RSA4096 |
| **Display/GPU** | Exynos DECON + Mali-G710 | `simpledrm` -> MDSS/DPU1 + `S6E3FC3 / AMS646YD01` (active) / `EA8079B`; Adreno 650 + signed `a650_zap.mbn` |
| **Power** | Exynos ACPM | Qualcomm `RPMh` (`performance` + UFS `clkscale/clkgate=0` until AMC fix) |
| **Audio/Sensors** | Google AoC | QDSP6/ADSP + `CS35L41` & SLPI (`fastrpc`) |

---

## 3. Gated Milestones (M0-M8)

- **M0 — Capture & NV Backup:** [x] **M0.1:** 11 critical partitions, double-read `SHA256SUMS` (`~/android/backups/r8q-nv/`). [x] **M0.2:** Header v2 offsets + live FDT (`msm-id=<0x0164 0x020001>`, `board-id=<0x08 0x06>`) to live DTB reference.
- **M1 — Kernel & DTB:** [x] **M1.1:** `7.3.0-rc6-omamobile-push1` + `sm8250-samsung-r8q.dtb` + 1,818 modules in `omamobile-build:24.04` (`PON=y`, `QCOM_WDT=y`, `SOFT_WATCHDOG=y`, `ZRAM=y`, no `SUSPEND`, UFS WB fix).
- **M2 — Guard, Recovery Boot & Stability:** [x] **M2.1:** `tools/r8q-boot` packs `Image` + guard + `recovery_dtbo` + DTB + AVB footer (`sda24`). [x] **M2.2:** first bare-metal boot (USB NCM `172.16.42.1`, `ttyGS0`, SSH; early Alpine prototype superseded by Arch rootfs). [x] **M2.3:** `0.1.4` installed (UFS `auto_hibern8=0`, 30 s watchdog, sleep masked, `state1` off, panel timer, serial getty, flat Hyprland). [x] **M2.4:** `0.1.6` installed (auto-pins `performance`; Rust-only LFM3-1.7B server). [x] **M2.5:** `0.1.11` installed + verified live; soak phase complete — CPU-7-offline soak survived 1757 s (29.3 min) with zero stalls, then crashed (#13) at 1768-1780 s during sustained UFS+USB writeback. Userspace mitigations exhausted; kernel fix needed for RPMh/RCU wedge. Device package 0.1.11 tested (all 32 suites pass). Recovery image staged and verified (79M, SHA256 7f72b8f7...93487). [x] **M2.6:** wedge root cause (RPMh TCS leak, §0.1) fixed in the kernel: four commits `a807c8708`/`df727e3ec`/`ebeabc490`/`e96cf40a1`, `CONFIG_SOFTLOCKUP_DETECTOR=y`; device package `0.1.12` adds the per-CPU liveness net and battery telemetry (built, signed, tested). The first image of it froze at 0.630 s on our own lock leak (§0.2, `d1f598f2c` -> `e96cf40a1`) and was rebuilt. [ ] **M2.7:** *validation* — the boot half is **done 2026-11-14**: `uname -r` reads `...ge96cf40a1ee9` after reading `sda24` back *before* the boot, `softlockup_panic=1` is live on cmdline and sysctl, the 1,818 modules are injected into the live rootfs with `depmod` clean, and device package `0.1.13` is installed with both nets running (`omamobile-r8q-liveness`: 8 heartbeats at `1` line / `21` B, advancing; `omamobile-r8q-battery`: `max170xx_battery` + `max77705-charger` visible, `capacity=99`). Remaining: rerun the exact workload that killed #14 (12 GB writeback to the microSD) and require survival past 57 min with zero `ramoops` events; capture `rpmh_rsc_debug` output on the first `-110`. [ ] **M2.8:** enable the pseudo-NMI watchdog (`CONFIG_ARM64_PSEUDO_NMI=y`, which is what turns on `CONFIG_HARDLOCKUP_DETECTOR` on arm64) so an IRQs-off spin panics and resets instead of freezing silently, as §0.2's bug did — its own boot, alone, because it is a new detector on a platform that may not tolerate it.
- **M3 — Display & GPU:** [x] **M3.1:** `simpledrm` 1080x2400 FB + stable `llvmpipe` Hyprland. [x] **M3.2:** re-extracted the vendor blobs from the owner's own LineageOS image on 2026-11-14 (`vendor.new.dat.br` → EROFS → `dump.erofs`/`fsck.erofs`), confirming what is actually in `/firmware`: `a650_gmu.bin`, `a650_sqe.fw`, `htbtfw20.tlv` + `qca6390/` (BT, so M5.2 is unblocked), and `cs35l40-{bot,rcv}-dsp1-spk-prot.{bin,wmfw}` + their `-calib.bin` (M5.3). **Correction: the SQE shipped there is version `0x093`, not the `0x112` this row used to claim** — see blocker 2. Live LineageOS 23.2 (`4.19.325`) smoothness analysis (2026-11-13): `SurfaceFlinger` runs Hardware Composer `DEVICE` composition on `ae00000.qcom,mdss_mdp` with `ss_dsi_panel_EA8079B_AMS646YB01_FHD` (`1080x2400` @ `120.00 Hz` / `60.00 Hz` DSI command mode, `TE` pin sync on `gpio66`, `cont_splash_region@9c000000` size `0x2300000`), `Adreno650v3` (`587 MHz`), and `schedutil` (`1.80/2.42/2.84 GHz`) with 500 ms touch boost. Mainline `simpledrm` lacks DPU composition, TE sync, and Adreno acceleration (`llvmpipe` fallback). [ ] **M3.3:** *blocked on firmware* — needs `a650_sqe.fw` ≥ `0x95` (blocker 2); everything else is in place and the panel work is independent: write the mainline `S6E3FC3 / AMS646YD01` 120 Hz DSI panel driver from the captured live DTB timings/init commands (`~/omamobile-kernel/reference-r8q/live.dts`) and enable the `disabled` `display-subsystem@ae00000` + `dsi@ae96000` nodes, then unblacklist `msm` and verify `renderD128` + `vulkaninfo` (pin GPU devfreq first — same `-110` trap).
- **M4 — Touch, Keys & Sensors:** [x] **M4.1:** `stmfts` on `/dev/input/event3`, `gpio_keys` `event2`, `qpnp_pon` `event0`; the 2026-11-15 live-console test also responded to volume-up and volume-down presses. [ ] **M4.2:** fix `LockGlance.qml` touch/wake + power-key binding, re-enable `idle.lock`. [ ] **M4.3:** bring up SLPI (`slpi.mbn` + `fastrpc`), verify via `iio-sensor-proxy` (2 `/sys/bus/iio` devices exist, no `remoteproc` yet).
- **M5 — Wi-Fi, BT & Audio:** [x] **M5.1:** `wlp1s0` associates (off for UFS-bisect); keep `usb0` unmanaged. [ ] **M5.2:** `bluetoothctl` scan with timeout guard (`bt-agent` stays masked). [ ] **M5.3:** `kona-mtp-snd-card` in `aplay -l` + UCM2 (`HiFi.conf`) playback.
- **M6 — Battery, Thermal, Power:** [ ] **M6.1:** *done 2026-11-14 in `0.1.13`* — `r8q-battery-modprobe` loads `max17042_battery` and `max77705-charger` as modules (keeping them `=m` is what avoids the `0.95 s` freeze), and the shell reads `/sys/class/power_supply/max170xx_battery` (`capacity=99`, `voltage_now=4.33 V`, `temp=30.5 °C`, `health=Good`) plus `max77705-charger` (`USB`, `Charging`). The earlier probe work stands: chip on I2C bus 0 addr `0x66`, rev `0x02` = `PASS2`; live FDT shows only `pmic@66` owns the PMIC GPIO interrupt. `max77705-charger` reported `online=1`, `present=1`, `health=Good`, `USB`, `4400000` uV, and `/sys/class/power_supply/max170xx_battery` reported `capacity=99`, `voltage_now=4302343`, `voltage_ocv=4320859`, `temp=260` (26.0 °C), `current_now=-126718`, `health=Good` (`i2cget -y 0 0x36` reg `0x00=0x0080`, `0x06=0x63ed`, `0x09=0xd6ba`). Next: enable `fuel-gauge@36` with `=m` auto-load and read `max170xx_battery` in `ai-server`/shell. [ ] **M6.2:** RPMh AMC BCM-voter `-110` quirk + TZ `state1` fix in kernel/DTB (reference: `sm8250.dtsi` `apps_rsc` ACTIVE 2 / SLEEP 3 / WAKE 3 / CONTROL 1; live FDT confirmed `qcom,tcs-config = <2 2 0 3 1 3 3 1>` matches `sm8250.dtsi`, and `psci` runs OSI mode after `failed to set PC mode: -3`).
- **M7 — Rootfs & Shell:** [x] **M7.1:** coexistent Arch rootfs on `userdata`; device package container-built (`--local r8q`, `keys/r8q`). [ ] **M7.2:** `omamobile-shell` scale-2 + `omamobile-settings` panel (replaces `cage`/`cog`/`darkhttpd` prototype).
- **M8 — On-Device AI (LFM3-1.7B) & Tuning:** [x] **M8.1:** `zram ram/2` (`zstd`), 4 GiB `/swapfile`, MGLRU `min_ttl_ms=1000`, `swappiness=150`, `page-cluster=0` (`0.1.10` adds `dirty_background_bytes=16M` / `dirty_bytes=64M` writeback cap). [ ] **M8.2 [BLOCKED ON M3.3]:** 120 Hz animations + Adreno devfreq (staged, inactive on `simpledrm`). [x] **M8.3:** LFM3-1.7B `Q4_K_M` ~15.7 tok/s on Kryo 585 (`tools/r8q-ai`; `tools/omamobile-ai` pins inference to online A77 cores `4-7` via `taskset` and binds `127.0.0.1:8080`). [x] **M8.4:** Verified 2026-11-13 on `sda36`: `/home/alarm/models/LFM3-1.7B-Q4_K_M.gguf` (`1.0G`, `chown 1000:1000`, sha256 `3793e1e6e2266919d6e015287221b41100175a98eddaba92e111aed3b05c93c4` matching HF LFS) streamed over `omamobile.initramfs` SSH with zero errors (`0.1.10-1` installed; `omamobile-ai-server` moved to `:18081` in `0.1.9`, clearing `:8080` for `llama-server`). [ ] **M8.5:** `GGML_VULKAN` bench after M3.3 (deprioritized behind phone stability/usability). [ ] **M8.6 (pi with our LFM3 — end of priority after phone stability & hardware bring-up):** 1) `mise` on phone (`pacman -S mise` if absent), `mise use -g pi` (verify the backend serves an aarch64 asset at install time, else the registry's npm backend); 2) provider entry in `~/.pi/agent/models-store.json` (`api openai-completions`, `baseUrl http://127.0.0.1:8080/v1`, model id = live `/v1/models`) + `settings.json` default; 3) `pi -p` roundtrip. Real endpoint is our own `tools/omamobile-ai` `llama-server` (OpenAI `/v1`, on-demand + idle watchdog). Warmup: the server auto-starts on any `omamobile-ai chat` and idles off — start it with a long timeout before a pi session.

Model policy: **LFM3-1.7B-`Q4_K_M` is the sole default** (`/v1/models` lists just it; `/home/alarm/models/` empty until `tools/r8q-ai push-model`). Catalog (verified HF repos 2026-11-12): `LiquidAI/LFM3-1.7B-GGUF` (`Q4_K_M`/`QAD-Q4_0`, ~1.39 GB) - `Qwen/Qwen2.5-1.5B-Instruct-GGUF` (fallback) - `meta-llama/Llama-3.2-3B-Instruct-GGUF` (heavy) - `LiquidAI/LFM3-VL-2B-GGUF` + `mmproj` (vision/video) - `vikhyatk/moondream2` (light OCR) - `LiquidAI/LFM3-Omni-2B-GGUF` (`--reasoning off`) - `LiquidAI/LFM3-Audio-2B-GGUF` (ASR) - `ggerganov/whisper.cpp` + `hexgrad/Kokoro-82M` (modular STT/TTS) - `ggml-org/embeddinggemma-2-GGUF` (+`unsloth`, base `google/embeddinggemma-2`, ~350 MB RAG).

---

## 4. Rules, Checklist & Next Steps

- **Memory:** `zram-size = ram / 2`, 4 GiB `/swapfile` overflow, MGLRU `min_ttl_ms=1000`, `swappiness=150`, `page-cluster=0` (later: `MULTI_COMP` `lz4`+`zstd`).
- **Scheduling:** `performance` governor + UFS `clkscale/clkgate=0` until M6.2; pin GPU devfreq on M3.3 bring-up; restore `schedutil` + `UCLAMP_MIN` + `misc:vfr=true` + 120 Hz only after soak-verified fixes.
- **Packaging:** everything in `packages/omamobile-device-r8q/`, container-built (`--local r8q`); vendor firmware to `~/.cache/omamobile/firmware/r8q/` or `~/omamobile-kernel/reference-r8q/vendor-firmware/` (never commit blobs). Getting a blob out of the owner's own LineageOS image, no phone and no root: `unzip vendor.new.dat.br vendor.transfer.list` → `brotli -d` → `sdat2img.py vendor.transfer.list vendor.new.dat vendor.img` (v4 "new" runs only, `101370` blocks) → `dump.erofs --ls --path=/firmware vendor.img` to list and `fsck.erofs --path=/firmware/<name> --extract=<file> vendor.img` to pull one out (it writes a *file*, not a directory, for a file path). `a650_zap` needs no file at all: `sm8250.dtsi` reads ZAP from the `gpu_mem` reserved region that the bootloader fills.
- **Transfers:** never `scp` the 12 GB rootfs in one go (crash #14 died mid-transfer); `tools/r8q-push` sends zstd chunks of 512 MB with a gap between them, sha256 per chunk, and resumes from whatever the phone already verified (`--decompress --sha <raw sha256>`). A kernel rebuild changes the release, so `bin/omamobile modules` output must be injected into the loop image before it is swapped in — the container has no `depmod`, so run it on the phone.
- **Checklist (every commit):** `tools/pr-check`, `tests/packages`.
- **Next 4 (dual-boot stability & hardware usability first; AI at the end):** 1) **Dual-boot setup (`2026-11-13`):** LineageOS 23.2 (`4.19.325`) restored cleanly on `boot` (`sda23`) + `super` (`sda29`) + F2FS `inlinecrypt` `userdata` (`sda36`). Because `userdata` uses Android inline-crypto FBE, `initramfs/init` (`boot_loop()`) + `tools/r8q-boot pack --dual-boot` now support booting `/omarchy-r8q.img` on exFAT microSD (`mmcblk0p1`, `sdhci@8804000`, `CONFIG_EXFAT_FS=y`, `CONFIG_NLS_UTF8=y`) while leaving **every UFS partition (`sda*`) 100% read-only**. Also fixed dead `CONFIG_TOUCHSCREEN_FTS5CU56A=y` -> `CONFIG_TOUCHSCREEN_STMFTS=y`. Note: adding `aliases { serial0 = &uart6; }` to `sm8250-samsung-r8q.dts` froze early boot at `ttyHS0` / `a600000.usb` (`998000` is a HSUART, `a90000` is `hsuart8`); keep `aliases { serial0 = &uart6; }` out. Next: rebuild kernel/DTB (`&sdhc_2` + `L6C`/`L9C` + `gpio77`), stage Arch ARM + modules into `/omarchy-r8q.img` on `mmcblk0p1`, and verify `RECOVERY` dual-boot SSH (`172.16.42.1`). 2) **Enable persistent M6.1 battery (`=m`):** turn on `fuel-gauge@36` after early boot with `MAX77705`/`MAX17042` as modules (`980000.i2c` confirmed `max77705@66` + `pca9468@57` + `0-003b` `mfc-charger` on LineageOS) and wire `max170xx_battery` into `ai-server`/shell. 3) **Hardware usability one per boot:** verify `CONFIG_TOUCHSCREEN_STMFTS=y` (`5-0049` `fts5cu56a_r8`, `tsp_ldo_en`, `gpio15`), Adreno 650 (`M3.3`) + `S6E3FC3 / AMS646YD01` (or `EA8079B`) 120 Hz AMOLED panel from captured `live.dts`, BT (`qca6390`), `CS35L41` audio (`i2c@a8c000`), and SLPI sensors. 4) **Last:** the ~11-57 min `RPMh`/RCU wedge is fixed in the kernel (§0.1, M2.6) and needs hardware validation (M2.7); its `rpmh_rsc_debug` output is the confirmation artefact. Then verify `LFM3-1.7B-Q4_K_M.gguf` (`M8.4`/`M8.6`) under Hyprland.
- **Clean-room principle:** all userspace glue stays packaged in `omamobile-device-r8q` (MIT); kernel/DTB quirks are structured for the kernel tree (`GPL-2.0-only`). Everything here is implemented by us from our own hardware measurements; nothing is copied from anywhere.

---

## 5. Gated Flash & Hardware Bring-Up Runbook (Pending Human Approval)

> **Pre-flight rule:** Keep `50-r8q-gpu-blacklist.conf` intact and test **one hardware overlay per boot** after Gate A confirms safe USB SSH + read-only UFS + updated modules/firmware/package. Never flash `boot` (`sda23`), `super` (`sda29`), `userdata` (`sda36`), bootloader, or NV partitions; write only `recovery` (`sda24`, `82694144` bytes).

### Gate A — Flash Safe Base `recovery-omamobile.img` & Sync Rootfs

- **Risk:** Medium (writes `sda24` only via `heimdall --no-reboot`; `boot`/`sda23` LineageOS untouched as fallback).
- **Commands (human-approved only):**
  1. Verify packed image matches `sda24` size (`82694144` B):
     `KERNEL_OUT=~/omamobile-kernel/out-r8q tools/r8q-boot verify ~/omamobile-kernel/out-r8q/recovery-omamobile.img`
  2. Reboot ADB-connected phone into Download Mode and flash `RECOVERY`:
     `adb reboot download`
     `tools/r8q-flash-recovery --dual-boot ~/omamobile-kernel/out-r8q/recovery-omamobile.img`
  3. Boot recovery (`Volume Up + Power` on USB/after reboot), log in over USB SSH (`172.16.42.1`).
  4. Push updated `modules-<rel>.tar.zst`, `~/.cache/omamobile/firmware/r8q/lib/firmware`, and `omamobile-device-r8q 0.1.14` package (`depmod -a` on phone; `systemctl restart systemd-networkd` to drop stale USB gadget route metric), then run read-only verification:
     `r8q-preflight --json`
- **Expect / Pass:**
  - `recovery-omamobile.img` is `<= 82694144` B and AVB-verified; `heimdall` reports 100% upload on `RECOVERY`.
  - Phone boots `simpledrm` + USB NCM `172.16.42.1`, UFS `sda*` remains `ro=1`, `depmod` clean, and `r8q-preflight --json` exits 0 (`sqe_version >= 0x095`, `ufs_ro=true`, `msm_blacklisted=true`).
- **Stop condition:** Any `heimdall` error, USB gadget failure after 60 s, or `ufs_ro=false` -> stop immediately and check `dmesg` / `ttyGS0`.
- **Rollback:** Normal reboot lands LineageOS on `sda23`, or re-flash stock/Lineage `recovery.img` to `sda24` in Download Mode.

### Gate B — M3 Display & Adreno 650 (`display.dtso`)

- **Risk:** Medium-High (enables `mdss` + `dsi0` + `s6e3fc3-r8q`, then unblacklists `msm`; keep USB SSH + `ttyGS0` log stream open).
- **2026-11-15 offline diagnosis:** the failed `msm separate_gpu_kms=1` probe used
  `display-mdss.dtso`, which intentionally leaves `dispcc` protected clocks
  unchanged. In the 7nm PHY implementation, `-EINVAL` from the timing
  calculator is returned when either the DSI bit-clock or escape-clock request
  is zero; the staged `display.dtso` overlay is therefore the correct next
  isolation step because it releases `dispcc` protected clocks while keeping
  the GPU blacklist active. The staged kernel is
  `7.3.0-rc6-omamobile-push1-gd469bd3bf2a9` from `omamobile-r8q` at
  `d469bd3bf`, and the offline image
  `~/omamobile-kernel/out-r8q/recovery-r8q-display-gpu.img` is header-verified,
  AVB-signed, dual-boot packed, and has SHA256
  `0cbb2048c3678fbd997720193ca52b5751317f0ff794a0eeb167b6c17aec1d88`.
  No phone write has been performed for this image.
- **2026-11-14 test result:** `display-clocks.dtso` (only `protected-clocks = []`) was packed and flashed to `RECOVERY` (`sda24`) after LineageOS was restored. That run produced no USB-SSH connection; the host observed Download Mode, then a short LineageOS enumeration, followed by Download Mode again. No kernel log was captured, so it remains inconclusive rather than proof of a display-clock fault. The follow-up `display-mdss.dtso` overlay enables only MDSS/DSI0/panel and leaves `dispcc` protected clocks unchanged. It booted successfully from `RECOVERY`: `r8q-preflight --json` reported `gpu_ready=true`, `ufs_guard.ok=true` with 77 UFS partitions read-only, and `mdss=okay`/`panel=okay`; `/dev/dri/card0` is present while `msm` remains blacklisted. This records the MDSS/panel milestone; GPU unblacklisting and `renderD128` validation remain a separate branch-gated step.
- **2026-11-15 branch test:** explicitly loading `msm separate_gpu_kms=1` created `/dev/dri/renderD128`, but KMS takeover failed at `dsi_7nm_phy_enable` with `-22` (`PHY timing calculation failed`). The panel then reported DCS failures and repeated DPU vblank timeouts, including a panel-dead event; USB SSH later recovered. Stop here: do not unblacklist `msm` at boot or flash this probe as a default. The next branch change must reconcile the mainline DSI mode/PHY timing with the captured vendor panel timings before another hardware load.

- **Commands (human-approved only):**
  1. Pack & flash `display` overlay only (`50-r8q-gpu-blacklist.conf` still active on first boot):
     `KERNEL_OUT=~/omamobile-kernel/out-r8q tools/r8q-boot pack --dual-boot --overlay display ~/omamobile-kernel/out-r8q/recovery-display.img`
     `tools/r8q-flash-recovery --dual-boot ~/omamobile-kernel/out-r8q/recovery-display.img`
  2. Boot `RECOVERY`, run `r8q-preflight --require-gpu`, and check `/sys/class/backlight/*`, `dmesg | grep -E 'dpu|dsi|s6e3fc3'`.
  3. Once `display.dtso` alone boots safely, run an announced test load (`modprobe msm`, `timeout 10` watchdog/SSH guard) and verify `/dev/dri/renderD128`, `adreno` GMU/SQE firmware load, `3d00000.gpu` devfreq min `305000000`, and Hyprland wake/touch + `backlight` unblanking before removing `50-r8q-gpu-blacklist.conf`.
- **Expect / Pass:** `s6e3fc3` registers `/sys/class/backlight/*`, `renderD128` initializes cleanly with upstream SQE `0x112`, `omamobile-r8q-panel.service` skips `fb0` blanking, and `LockGlance.qml` wakes cleanly on touch/power button.
- **Stop condition:** Panel black/freeze, `-110` on `rpmh`/`adreno_smmu`, or kernel oops in `msm`/`dpu` -> do not remove `50-r8q-gpu-blacklist.conf`; inspect `/sys/fs/pstore/console-ramoops-0`.
- **Rollback:** Re-flash base `recovery-omamobile.img` (without `--overlay display`) and keep `blacklist msm`.

### Gate C — M7 Audio & Remoteprocs (`remoteprocs.dtso`, `audio-amps.dtso`, `audio-sound.dtso`)

- **Risk:** Medium (first boot `remoteprocs` alone, second adds `audio-amps`, third adds `audio-sound`; one overlay per boot).
- **Commands (human-approved only):**
  1. Boot 1: pack `--overlay remoteprocs`, flash, verify `adsp`/`cdsp`/`slpi` probe and `qrtr-ns` + `pd-mapper` + `tqftpserv` in `journalctl -b`.
  2. Boot 2: pack `--overlay remoteprocs --overlay audio-amps`, flash, verify `cs35l41` at `0x40` and `0x41` on `i2c11` (`a8c000.i2c`) with `cirrus/cs35l40-{bot,rcv}-*` firmware loaded.
  3. Boot 3: pack `--overlay remoteprocs --overlay audio-amps --overlay audio-sound`, flash, run `r8q-preflight --require-audio`, check `aplay -l` (`Samsung-Galaxy-S20-FE` / UCM2), and run a short announced `speaker-test` at low volume.
- **Expect / Pass:** Remoteprocs online with zero `-110`/GPI crashes, both CS35L41 amps bound cleanly, `aplay -l` enumerates `TERTIARY_TDM_RX_0`, and audio plays cleanly.
- **Stop condition:** `gpi` DMA timeout, remoteproc crash, or I2C probe NACK on `0x40`/`0x41` -> stop and remove the newest overlay.
- **Rollback:** Re-pack without the failing overlay and re-flash `sda24`.
