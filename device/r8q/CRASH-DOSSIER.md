<!-- SPDX-License-Identifier: MIT -->

# r8q watchdog-reset dossier (our hardware measurements, 2026-11-12)

Phone: SM-G780G (r8q), Snapdragon 865. Kernel `7.3.0-rc6-omamobile`, S-Boot
recovery-slot boot. All data below comes from this phone's own `pstore` and
live reads — no other source.

## Failure signature (9 crashes, one pattern)

- Silent RCU stall (`rcu_preempt detected stalls`), no prior error, no NMI
  backtrace from the wedged core, then a hardware watchdog reset.
- MTBF ~11 min under idle-to-light load, with every mitigation active.
- Crash #8 (our clearest capture): first stall at 700 s uptime, repeated
  stalls past 919 s; **CPU 7 NMI-dead three times while CPUs 0 and 2
  answered their NMIs**; zero `rpmh`/oops/watchdog-bark lines throughout.
- Victim cores vary across crashes (1, 0, 7, 6, 4) but CPU 7 recurs; CPU 7
  is the prime core.

## Two crashes WITH a first cause (both `rpmh_write_batch -110`)

- Crash #1: `sugov` worker -> `qcom_cpufreq_hw` DVFS OPP transition ->
  interconnect bandwidth vote -> `rpmh_write_batch` timeout on the AMC TCS.
- Crashes #3-#6 (Wi-Fi-off bisect): `ufshcd_ungate_work` ->
  `ufs_qcom_icc_update_bw` -> `icc_set_bw` -> same `-110` (any UFS clock
  ungate, even background journal/SSH I/O).

## Mitigations live (all verified read-back on the phone)

- All policies pinned `performance`; deep `cpuidle` state1 off (WFI only);
  UFS `auto_hibern8=0`, `clkscale_enable=0`, `clkgate_enable=0`; 30 s
  `qcom_wdt`; suspend targets masked; panel unblank timer.

## Exonerated by bisect (our tests, our data)

- Wi-Fi (QCA6390) OFF did not prevent the crash — radio is not the trigger.
- UFS vote silencing (`clkscale`/`clkgate=0`) did not move the wall —
  crashes #7/#8/#9 all landed with every knob at 0.

## Lead hypothesis for the kernel handoff

A core wedges in/after a power state where even FIQ/NMI cannot reach it;
the `-110`s read as symptoms (TCS commit timing out against a wedged power
controller), not the root cause. Proposed next experiment (ours, pending
phone window): offline CPU 7 for one soak; if the ~11-min wall moves, the
prime core's collapse/wake path is the quarry.

## Asks for the kernel side

1. Why a CPU can go NMI-deaf on this SoC/firmware (GIC/FIQ reachability
   across PSCI states), and how to instrument the CPU7 collapse/wake path.
2. Whether the AMC TCS bank can report which bank stalls (`tcs-in-use` /
   state at `-110`) on this tree.
3. Later (M3.3, separate): Adreno probe ordering (`gpucc` + `rpmhpd`
   before `msm`) and a DT-described way to skip `SECVID` register writes
   the TrustZone traps.
