<!-- SPDX-License-Identifier: MIT -->

# Failed 02: Suspend (`s2idle`), `cpuidle state1`, `schedutil` & UFS PM

- **Symptom:** Repeated silent single-core bus hangs, RCU stalls, and `rpmh_write_batch -110` timeouts between `10 min` and `69 min` of runtime (`Crashes #1-#14`).
- **Back:** [FAILED.md](../FAILED.md) · [Board Overview](../README.md)

## Root Cause
- **`s2idle` / kernel suspend:** Samsung TZ never resumes after `s2idle` entry; system locks up minutes after boot or idle timeout.
- **`cpuidle` deeper sleep states (`state1`+):** Power-collapse fails to restore core state under Samsung TZ, stalling RCU cleanly without console output.
- **`schedutil` (`sugov`) & UFS auto-hibern8/clock-gating:** Frequent active-mode AMC `RPMh` bandwidth/clock votes (`ufs_qcom_icc_update_bw`, CPUfreq DVFS) hit RSC TCS completion timeouts (`-110`), which leaked TCS slots until `e96cf40a1`.

## Permanent Rule
- Keep `# CONFIG_SUSPEND is not set`, `AllowSuspend=no`, and systemd sleep targets masked (`10-r8q-no-suspend.conf`).
- Keep `cpuidle-no-deep-sleep` (`WFI`-only) and `cpufreq-no-dvfs` (CPU `performance`).
- Keep `ufs-no-autohibern8` (`auto_hibern8=0`, `clkscale_enable=0`, `clkgate_enable=0`) until `M6.2`.
