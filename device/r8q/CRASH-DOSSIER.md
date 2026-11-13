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
- Victim cores span both clusters — little (1), big (4, 6), prime (7) —
  and the NMI-dead core varies crash to crash (crash #11: CPU 4 dead
  while CPU 1 answered; crash #8: CPU 7 dead three times). The quarry
  is the shared power-state path, not one core.

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

## The decisive experiment (2026-11-13, boot `6e38eaed`)

CPU 7 (prime) offlined at **635 s** uptime on an already-running boot; dead-man
timer armed then removed (verified no unit/timer survived, `cpu7=0` held).

- Survived **idle** to **1726 s (28.8 min)** and **1757 s** with zero RCU
  stalls — 656+ s past the previous baseline record's first stall (1070 s).
- **Then crashed under heavy UFS + USB NCM write I/O (`1758-1780 s`):** as soon
  as an `scp` push of `LFM3-1.7B-Q4_K_M.gguf` began writing to `sda36` (156 MiB
  written before reset), `console-ramoops-0` (read on boot `bd4fac14`)
  recorded:
  - `[  635.226882] psci: CPU7 killed (polled 0 ms)`
  - `[ 1768.090086] watchdog0: pretimeout event`
  - `[ 1780.137259] rcu: INFO: rcu_preempt detected stalls on CPUs/tasks: 1-...0, 4-...0, 6-...0 (detected by 2, t=5252 jiffies, g=65833, q=31 ncpus=7)`
- What this proves: **offlining CPU 7 does NOT make the kernel immune.**
  Under idle load the wall moved past 1757 s, but sustained UFS + USB NCM write
  I/O still wedged **three cores simultaneously (`1, 4, 6`)** in non-idle
  kernel context (`idle=.../1/...`) with `cpu7=0` (`ncpus=7`), and `watchdog0`
  pretimed out 12 s before the RCU stall report. Userspace mitigations are
  exhausted.

## Lead hypothesis for the kernel handoff

A core (or multiple cores simultaneously, as in crash #13 on `6e38eaed`)
wedges inside the kernel without prior error lines and stops answering
interrupts/NMIs; the earlier `-110`s read as symptoms (TCS commit timing out
against a wedged power controller), not the root cause. The CPU-7-offline
soak moved the idle wall (552-1070 s baseline -> 1757 s idle), yet sustained
UFS+USB writeback (`scp`, 156 MiB) immediately triggered a multi-core (`1, 4, 6`)
stall with `ncpus=7`. Both the shared PSCI/RPMh power path and sustained UFS
writeback/DMA/IOMMU bursts are now implicated.

## Firmware facts (own dmesg)

- S-Boot PSCI: SMC Calling Convention v1.1, OSI mode supported, but
  `failed to set PC mode: -3` — the kernel runs OSI (per-CPU suspend),
  not platform-coordinated. Any collapse/wake quirk lives on
  this path.

## Asks for the kernel side

1. Why any CPU can go NMI-deaf on this SoC/firmware (GIC/FIQ reachability
   across PSCI states; victims span little/big/prime), and how to
   instrument the collapse/wake path.
2. Whether the AMC TCS bank can report which bank stalls (`tcs-in-use` /
   state at `-110`) on this tree.
3. Later (M3.3, separate): Adreno probe ordering (`gpucc` + `rpmhpd`
   before `msm`) and a DT-described way to skip `SECVID` register writes
   the TrustZone traps.

## Second defect: `MAX77705` probe `-EINVAL` (own dmesg)

A DT pinctrl conflict, not a silicon or regulator problem. Live log:

    qcom-spmi-gpio c440000.spmi:pmic@4:gpio@c000: pin gpio11 already
      requested by 0-0069; cannot claim for 0-0066
    qcom-spmi-gpio ...: error -EINVAL: pin-10 (0-0066)
    max77705 0-0066: error -EINVAL: Error applying setting, reverse things back

- `charger@69` (`maxim,max77705-charger`) is bound and reports telemetry
  (Charging/Fast/Good, 4.4 V, 900 mA) and holds SPMI PMIC `gpio11`.
- `pmic@66` (the MFD parent) cannot claim the same pin, so its probe dies
  and the fuel gauge never registers.
- Chip revision is irrelevant: PMICREV (`0x01`) reads `0x02` = PASS2.
- Ask: one owner for `gpio11` in the DTB (kernel tree); the MFD and charger
  nodes must not both request it.
