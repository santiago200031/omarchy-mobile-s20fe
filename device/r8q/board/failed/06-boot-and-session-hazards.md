<!-- SPDX-License-Identifier: MIT -->

# Failed 06: Boot & Session Bring-Up Hazards

- **Symptom:** Early boot hangs (`serial0 = &uart6`, unguarded `bt-agent`), massive monitor CPU burn (`1h49m` over `8.5h`), session instability (`udisks2` / `voxtype` / `icc_bwmon` / `camcc_sm8250`), and failed transfers (`scp`).
- **Back:** [FAILED.md](../FAILED.md) · [Board Overview](../README.md)

## Root Causes & Permanent Rules
1. **`aliases { serial0 = &uart6; }`**: `uart6` is the QCA6390 Bluetooth HSUART; aliasing it hung early boot at `ttyHS0` / `a600000.usb`. **Rule:** Never alias `serial0` to `&uart6`.
2. **`r8q-liveness-monitor` `online_cpus | grep`**: Running `online_cpus | grep -qx "$c"` per PID inside the loop triggered a `SIGPIPE` / subshell storm. **Rule:** Read `$SYS/devices/system/cpu/cpu$c/online` directly in shell.
3. **Unready session daemons & clocks**: `udisks2` (probing 77 RO UFS partitions), `voxtype` (147 MB Whisper model autoload), unpowered `icc_bwmon`/`camcc_sm8250` clocks, and `bt-agent` stressed bring-up. **Rule:** Keep `icc_bwmon` + `camcc_sm8250` blacklisted (`/etc/modprobe.d/omamobile-r8q-bringup.conf`), `udisks2`/`bt-agent` masked, and `voxtype` disabled until their subsystems are brought up.
4. **12 GB `scp` image copy / trusting Heimdall exit alone**: `scp` died mid-writeback (crash #14), and an older flash left `sda24` stale. **Rule:** Use chunked resumable `tools/r8q-push` and verify `uname -r` after every flash.
