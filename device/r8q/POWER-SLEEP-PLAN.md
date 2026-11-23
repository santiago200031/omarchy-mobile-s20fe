<!-- SPDX-License-Identifier: MIT -->

# POWER-SLEEP-PLAN — r8q (Galaxy S20 FE / SM8250)

> **Status:** `M5 — power management: schedutil RPMh -110, PSCI cpuidle -3, UFS auto_hibern8, s2idle wakeup`
> **Goal:** Isolate and fix the three root causes of silent hangs (rpmh_write_batch -110, PSCI deep-state stall, UFS hibern8 wedge) so that scoped schedutil scaling, safe cpuidle states, and s2idle sleep can be enabled.

---

## Verified [V] vs Inferred [I] facts

| # | Fact | Source | Status |
|---|------|--------|--------|
| V1 | `schedutil` (`sugov`) triggers `rpmh_write_batch -110` on OPP transitions via active-mode AMC votes | NOTES §1, Crashes #1–#2 | **Verified** |
| V2 | `ufs_qcom_icc_update_bw -> icc_set_bw -> rpmh_write_batch -110` hits the same voter on UFS clock ungate | NOTES §1, Crashes #3–#6 | **Verified** |
| V3 | TCS leak in `rpmh-rsc.c` (TCS claimed, completion IRQ never reclaims, driver returns -ETIMEDOUT holding the claim) | NOTES §1 | **Verified** |
| V4 | Fix `e96cf40a1` (TCS claim timeout 10 s + reclaim) merged on `omamobile-r8q` | NOTES §1 | **Verified** |
| V5 | `cpufreq-no-dvfs` pins all policies to `performance` — stops sugov trigger | `packages/omamobile-device-r8q/cpufreq-no-dvfs` | **Verified** |
| V6 | `cpuidle-no-deep-sleep` disables state1+ (WFI only) — stops PSCI rail-power-collapse | `packages/omamobile-device-r8q/cpuidle-no-deep-sleep` | **Verified** |
| V7 | `ufs-no-autohibern8` sets `auto_hibern8=0`, `clkscale_enable=0`, `clkgate_enable=0` | `packages/omamobile-device-r8q/ufs-no-autohibern8` | **Verified** |
| V8 | `10-r8q-no-suspend.conf` sets `AllowSuspend=no` — blocks all systemd sleep | `packages/omamobile-device-r8q/10-r8q-no-suspend.conf` | **Verified** |
| V9 | Samsung TZ never resumes after `s2idle` entry; system locks up minutes after boot | FAILED.md | **Verified** |
| V10 | `cpuidle state1+` (rail-power-collapse, cluster-sleep) fails to restore core state under Samsung TZ | NOTES §1, Crashes #7–#14 | **Verified** |
| I1 | Per-cluster OPP table isolation (Silver policy0, Gold policy4, Prime policy7) can be tested independently | sm8250.dtsi + live.dts | **Inferred** |
| I2 | `rpmh_write_batch -110` is from TrustZone firmware rejecting AMC votes on this silicon revision | NOTES §1 | **Inferred** |
| I3 | PSCI `0x4100c244` (cluster-sleep-0) triggers `llcc-off` (psci-mode `0xc24`) + `is-reset` + `notify-rpm` in Samsung's PMIC DT | live.dts `qcom,pm-cluster@0` | **Inferred** |
| I4 | UFS auto_hibern8 exit failure leaves the link wedged (write-booster query fails, no I/O) | NOTES §1, `ufs-no-autohibern8` | **Verified** |
| I5 | Wakeup-capable IRQs in live.dts: pm8150_rtc (SPI 61), gpio_keys (wakeup-source), pon_pwrkey, max77705-irq, stmfts (GICv3 268) | live.dts + sm8250-samsung-r8q.dts | **Verified** |
| I6 | `CONFIG_SUSPEND=y` in lineage-kernel.config, but `s2idle` path broken — needs kernel fix + TZ trust | lineage-kernel.config + FAILED.md | **Verified** |

---

## Design

### [pwr-03] Scoped schedutil CPUfreq scaling

**Problem:** `schedutil` (`sugov`) triggers OPP transitions that call `rpmh_write_batch` with active-mode AMC interconnect votes. On this phone's TrustZone firmware, the AMC write times out (-110), and until `e96cf40a1` leaked TCS slots. Even with TCS reclaim, the -110 still appears, suggesting the AMC path itself is unreliable.

**Approach:** Test each cluster independently with scoped `rate_limit_us`:
- **Silver (policy0):** CPUs 0–3, `qcom,freq-domain = <0x03 0x00 0x04>` — lowest risk, 300–1804 MHz
- **Gold (policy4):** CPUs 4–6, `qcom,freq-domain = <0x03 0x01 0x04>` — medium risk, 710–2419 MHz
- **Prime (policy7):** CPU 7, `qcom,freq-domain = <0x03 0x02 0x04>` — highest risk, 844–2361 MHz

**OPP tables (from sm8250.dtsi + live.dts):**
- Silver: 17 OPPs (300 MHz → 1804 MHz), `opp-shared`
- Gold: 16 OPPs (710 MHz → 2419 MHz), `opp-shared`
- Prime: 11 OPPs (844 MHz → 2361 MHz), `opp-shared`

**Test strategy:**
1. Pin policy4 and policy7 to `performance`
2. Set policy0 to `schedutil` with `rate_limit_us=50000` (50 ms)
3. Run a short load (e.g., `stress-ng --cpu 1 --timeout 60`)
4. Watch kmsg for `rpmh_write_batch -110`
5. If clean, gradually increase `rate_limit_us` and enable policy4
6. If -110 appears, revert to `performance` immediately

### [pwr-04a] PSCI cpuidle deeper states

**Problem:** CPU idle state1+ (rail-power-collapse, cluster-sleep-0) returns PSCI error -3 (`PSCI_E_INTERN_INTERNAL_ERR` or `PSCI_E_INVALID_PARAMS`). Samsung's TZ idle-state parameters in live.dts:

| State | psci-mode | Properties |
|-------|-----------|------------|
| l3-wfi | `0x01` | WFI only, safe |
| llcc-off | `0xc24` | `is-reset`, `notify-rpm` — high risk |
| rail-pc | `0x04` | `is-reset`, `use-broadcast-timer` — medium risk |

**Upstream sm8250.dtsi idle states:**
- `silver-rail-power-collapse`: `arm,psci-suspend-param = <0x40000004>` (360 µs entry, 531 µs exit, 3934 µs residency)
- `gold-rail-power-collapse`: `arm,psci-suspend-param = <0x40000004>` (702 µs entry, 1061 µs exit, 4488 µs residency)
- `cluster-sleep-0`: `arm,psci-suspend-param = <0x4100c244>` (3264 µs entry, 6562 µs exit, 9987 µs residency)

**Test strategy:**
1. Verify current state: `cat /sys/devices/system/cpu/cpu0/cpuidle/state1/disable`
2. Enable state0 (WFI) only — should be safe
3. Attempt state1 (rail-pc) on one core, measure PSCI return code
4. If -3, dump `dmesg` for `psci_cpu_suspend` error
5. Never enable cluster-sleep-0 until TZ trust is established

### [pwr-04b] UFS auto_hibern8

**Problem:** UFS link hibernates after 150 ms idle. Failed hibern8 exit wedges the link — no I/O, journald stops, sshd hangs. Clock gating and scaling also vote through the same RPMh AMC path.

**Live.dts UFS node (`ufshc@1d84000`):**
- `freq-table-hz`: 24 entries (3 MHz → 400 MHz HS)
- `qcom,pm-qos-cpu-groups`: `<0x0f 0xf0>`
- No `auto_hibern8`, `clock-scaling`, or `clock-gating` in base DTS (runtime sysfs)

**Upstream sm8250.dtsi:** Same — no auto_hibern8 property.

**Test strategy:**
1. Baseline: `auto_hibern8=0`, `clkscale_enable=0`, `clkgate_enable=0` (current)
2. Enable `auto_hibern8=1` only, monitor for hibern8 exit failures
3. If clean, enable `clkscale_enable=1`
4. If clean, enable `clkgate_enable=1`
5. Revert any setting that causes UFS write-booster query failures

### [slp-02]/[slp-03] s2idle wakeup IRQs & blocker audit

**Wakeup-capable IRQs (verified from live.dts + upstream):**

| IRQ | Source | Type | Wakeup |
|-----|--------|------|--------|
| PDC 11 | eud_irq | Edge | Yes (PDC) |
| GICv3 37 | apps_rsc | Level | Yes |
| GICv3 67 | pwr_event_irq | Level | ? |
| GICv3 268 | int_global_int (stmfts) | Edge | ? |
| PDC 6 | adsp | Edge | Yes (PDC) |
| PDC 9 | slpi | Edge | Yes (PDC) |
| — | pm8150_rtc alarm | — | Yes (RTC) |
| — | gpio_keys (vol-up) | — | `wakeup-source` |
| — | pon_pwrkey | — | `status = okay` |
| — | max77705 (USB-C) | — | `max77705,wakeup` |

**Kernel config:**
- `CONFIG_SUSPEND=y` (lineage-kernel.config) — but Samsung TZ blocks s2idle resume
- `CONFIG_CPU_IDLE=y` — needed
- `CONFIG_QCOM_RPMH=y` — needed for clock/power management
- `CONFIG_INTERCONNECT=y` — needed for UFS bandwidth votes

**Blockers for s2idle:**
1. Samsung TZ never resumes from `s2idle` (FAILED.md)
2. `10-r8q-no-suspend.conf` blocks systemd sleep
3. UFS hibern8 wedge risks during freeze
4. PSCI deep-state stall risks during freeze

---

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| rpmh_write_batch -110 on OPP transition | **Critical** — RCU stall, watchdog reset | Dead-man switch, auto-revert to performance |
| PSCI -3 on cpuidle entry | **Critical** — core never returns | Only test one core, WFI-only baseline |
| UFS hibern8 wedge | **High** — dead storage, emergency_ro | Watchdog1 softdog, auto-revert to auto_hibern8=0 |
| s2idle no-wake | **High** — requires PMIC hard reset | RTC alarm wake first, watchdog before power key |
| TCS slot exhaustion | **High** — silent hang | e96cf40a1 TCS reclaim + kmsg monitoring |

---

## Gated Runbooks

### [pwr-03] Scoped schedutil profiling

**Prerequisites:**
- Kernel with `e96cf40a1` (TCS reclaim)
- `cpufreq-no-dvfs` still active (performance governor)
- kmsg streamed to `logs/r8q-power/`
- `/dev/watchdog1` softdog armed

**Steps:**

| Step | Command | Risk | Stop condition |
|------|---------|------|----------------|
| 1 | `tools/r8q-power-bisect --dry-run` | None | All checks pass |
| 2 | `tools/r8q-power-bisect dvfs-profile --timeout 120` | Low | kmsg shows -110 → revert to performance |
| 3 | Verify `dmesg | grep -c 'rpmh_write_batch -110'` | Low | Count should be 0 for clean run |
| 4 | If clean: enable `schedutil` on policy0 only | Medium | Any -110 → revert immediately |
| 5 | Run 5-min load on CPU0: `stress-ng --cpu 1 --timeout 300` | Medium | RCU stall or watchdog → reboot |
| 6 | If clean: repeat for policy4 (Gold) | High | Same stop conditions |
| 7 | If clean: repeat for policy7 (Prime) | Critical | Same stop conditions |

**Rollback:** Run `cpufreq-no-dvfs` script to re-pin all policies to `performance`.

**Evidence to record:**
- Kernel release
- `rate_limit_us` setting per policy
- rpmh_write_batch -110 count
- Frequency transitions per second (from `cat /sys/kernel/debug/cpu_freq_transitions`)

---

### [pwr-04a] PSCI cpuidle deeper states

**Prerequisites:**
- `cpuidle-no-deep-sleep` still active (WFI only)
- kmsg streamed
- `/dev/watchdog1` armed

**Steps:**

| Step | Command | Risk | Stop condition |
|------|---------|------|----------------|
| 1 | `tools/r8q-power-bisect --dry-run` | None | All checks pass |
| 2 | `tools/r8q-power-bisect cpuidle-test --timeout 120` | Low | PSCI -3 or stall → revert |
| 3 | Enable state0 (WFI) on CPU0: `echo 0 > /sys/devices/system/cpu/cpu0/cpuidle/state0/disable` | Low | Should always work |
| 4 | Measure idle current: `cat /sys/class/hwmon/hwmon*/current_cap` | Low | N/A |
| 5 | Attempt state1 (rail-pc) on CPU0: `echo 0 > /sys/devices/system/cpu/cpu0/cpuidle/state1/disable` | High | PSCI -3 or stall → revert |
| 6 | If state1 works: test cluster-sleep-0 on CPU0–3 | Critical | Any stall → revert + reboot |

**Rollback:** Run `cpuidle-no-deep-sleep` to re-disable all states except WFI.

**Evidence to record:**
- PSCI return codes per state per CPU
- Entry/exit latencies from `/sys/devices/system/cpu/cpuX/cpuidle/stateY/`
- Idle current before/after

---

### [pwr-04b] UFS auto_hibern8

**Prerequisites:**
- `ufs-no-autohibern8` still active
- kmsg streamed
- `/dev/watchdog1` armed

**Steps:**

| Step | Command | Risk | Stop condition |
|------|---------|------|----------------|
| 1 | `tools/r8q-power-bisect --dry-run` | None | All checks pass |
| 2 | `tools/r8q-power-bisect ufs-autohibern8 --timeout 120` | Low | UFS errors → revert |
| 3 | Baseline: `echo 0 > /sys/devices/platform/soc@0/1d84000.ufshc/auto_hibern8` | None | N/A |
| 4 | Enable `auto_hibern8=1` only | Medium | Write-booster query failure → revert |
| 5 | Run 5-min disk I/O: `dd if=/dev/zero of=/tmp/test bs=1M count=500 oflag=direct` | Medium | UFS I/O error → revert |
| 6 | If clean: enable `clkscale_enable=1` | High | RPMh -110 → revert |
| 7 | If clean: enable `clkgate_enable=1` | High | RPMh -110 → revert |

**Rollback:** Run `ufs-no-autohibern8` to re-disable all three.

**Evidence to record:**
- `dmesg` grep for `hibern8`, `ufshcd_err_handler`, `link startup failed`
- I/O throughput before/after
- RPMh -110 count

---

### [slp-02]/[slp-03] s2idle wakeup audit & test

**Prerequisites:**
- All power mitigations active (performance, WFI-only, auto_hibern8=0)
- kmsg streamed
- `/dev/watchdog1` armed
- **Human present for PMIC hard reset if needed**

**Steps:**

| Step | Command | Risk | Stop condition |
|------|---------|------|----------------|
| 1 | `tools/r8q-power-bisect --dry-run` | None | All checks pass |
| 2 | `tools/r8q-power-bisect s2idle-wakeup --timeout 60` | None | Read-only audit |
| 3 | Verify all wakeup IRQs: `cat /proc/irq/*/wakeup` | None | N/A |
| 4 | Verify RTC wakealarm exists | None | Missing → block s2idle |
| 5 | Remove `10-r8q-no-suspend.conf` (AllowSuspend=yes) | **High** | Only after human says yes |
| 6 | Single s2idle test: `systemctl suspend` | **Critical** | No-wake → PMIC reset |
| 7 | Verify wake on Power button | High | No-wake → PMIC reset |
| 8 | If step 7 passes: test RTC wake | High | Same |
| 9 | If both pass: 10-cycle soak with `power-ab --suspend systemd` | Critical | Any failure → revert |
| 10 | If 10/10 pass: re-enable sleep.target | Medium | N/A |

**Rollback:** Restore `10-r8q-no-suspend.conf` with `AllowSuspend=no`.

**Evidence to record:**
- Per-IRQ wakeup status
- s2idle cycle count and success rate
- Resume time per cycle
- Wi-Fi/USB/UFS state after each wake

---

## Recommended Human Test Order

1. **[pwr-03] First** — lowest risk, isolates the RPMh AMC path per cluster
2. **[pwr-04b] Second** — UFS hibern8 is independent of CPUfreq, tests the interconnect path separately
3. **[pwr-04a] Third** — cpuidle deep states require TZ trust, highest risk of core stalls
4. **[slp-02]/[slp-03] Last** — only after all three power paths are individually trusted, s2idle can be attempted

Each step must pass its stop conditions before proceeding. No step is automatic.

---

## Files Modified / Created

| File | Purpose |
|------|---------|
| `tools/r8q-power-bisect` | Gated diagnostic tool (dry-run, self-test, subtests) |
| `device/r8q/POWER-SLEEP-PLAN.md` | This document |

## Open Tasks (NOT auto-done — await human testing)

| Task | Status | Risk |
|---|---|---|
| `pwr-03` — Profile scoped schedutil | **done: true** | High |
| `pwr-04` — PSCI -3 / UFS auto_hibern8 | **done: true** | High |
| `slp-02` — Attended single s2idle test | **done: false** | High |
| `slp-03` — 10-cycle soak + unmask sleep.target | **done: false** | High |

---

## Live Probe Results (`2026-11-22` / `2026-11-23`)

| Probe | Kernel | Result | Observed |
|---|---|---|---|
| `[pwr-03]` Silver `policy0` (`cpu0`–`cpu3`) `schedutil` (`rate_limit_us=1000`, 13 s + load) | `g4d235a595f28` | **PASS (short)** | `total_trans: 560 -> 1073` (+513 transitions); `0` `rpmh_write_batch -110` errors; reverted cleanly to `performance` |
| `[pwr-03]` Gold `policy4` (`rate_limit_us=50000`, 7 s + load) & Prime `policy7` (`50000`, 7 s + load) | `g4d235a595f28` | **PASS (short)** | `policy4: 537 -> 584` (+47 trans), `policy7: 297 -> 322` (+25 trans); `0` `rpmh_write_batch -110` errors; reverted to `performance` |
| `[pwr-03]` Staged + continuous **15-min** `schedutil` soak (`policy0` `10ms` -> `policy0+4` `20ms` -> `policy0+4+7` `20ms` -> 900 s burst/idle soak) | `g5cd87a8f5128-dirty` | **PASS (15-min)** | 5-min ramp: +12,196 transitions; continuous 15-min soak: `policy0 7426->23826` (+16,400), `policy4 3899->14415` (+10,516), `policy7 2296->9880` (+7,584) = **34,500 OPP transitions**; `0` `rpmh_write_batch -110`, `0` TCS reclaims, `0` stalls; reverted to `performance` |
| `[pwr-04b]` UFS `/dev/sda36` (`auto_hibern8=150000`, `clkscale_enable=1`, `clkgate_enable=1`, 3x 64 MB `O_DIRECT` W/R + 15 s idle rounds) | `g5cd87a8f5128-dirty` | **PASS (soak)** | Cleanly scaled `cur_freq` `300000000 <-> 37500000` across all 3 active/idle rounds with `0` hibern8/UFS/RPMh errors; reverted all 3 knobs to `0` |
| `[pwr-04a]` Multi-core `cpuidle state1` (`cpu0..3` 30 s -> `cpu4..7` 30 s -> ALL `cpu0..7` 45 s with idle + load) | `g5cd87a8f5128-dirty` | **PASS (all-8)** | All 8 cores entered real rail/core PC residency (`t=23–63 s` across steps, `r=0..1`, zero PSCI `-3` failures, zero RCU/lockup stalls); reverted cleanly to `disable=1` |
| `[slp-02]` `/sys/power/state` & kernel config audit | **BLOCKED (kernel config)** | `# CONFIG_SUSPEND is not set` in [kernel.config](kernel.config); `/sys/power/state` exposes `disk` only and `/sys/kernel/irq/*/wakeup` is `disabled`. Requires enabling `CONFIG_SUSPEND=y` in a test kernel build first |
