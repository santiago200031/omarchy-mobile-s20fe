<!-- SPDX-License-Identifier: MIT -->

# Missing 01: Flash Native DT `1324fe066` + Touch `5866f774c` Recovery

- **Priority:** P1 (optional cleanup; live phone runs `gd469bd3bf2a9` safely via `offline_unreserved_xpu_pages()` and `stmfts-r8q.ko` runtime rebind in `omamobile-device-r8q` `0.1.17`).
- **Back:** [MISSING.md](../MISSING.md) · [Board Overview](../README.md)

## Remaining Work
1. In `~/omamobile-kernel/wt-r8q` (`omamobile-r8q`), add human maintainer sign-off (`git rebase -i` / `Signed-off-by:`) to `1324fe066` (native XPU `no-map` DT) and `5866f774c` (`stmfts` 16-byte Type-B touch support).
2. With human approval and Download Mode, flash `/home/amd-omarchy/omamobile-kernel/out-r8q/recovery-omamobile.img` (SHA256 `2769d7da9701a3ac4b1f3fb51e559414ad979b8dcc642013778a2d7d3e1248e3`) to `RECOVERY` (`sda24`); modules for `7.3.0-rc6-omamobile-push1-g5866f774c4da` are already unpacked and `depmod`-indexed on the phone.
3. Verify on boot:
   - `uname -r` equals `7.3.0-rc6-omamobile-push1-g5866f774c4da` (after rebuild if re-signed).
   - `grep -Eq '(90500000-|9e300000-|9fb00000-bcbfffff) .*System RAM' /proc/iomem` returns empty (native DT `no-map` active).
   - `stmfts` (`5-0049`) works natively without loading `stmfts-r8q.ko`.
   - Once soak passes, optionally switch cmdline off `systemd.unit=multi-user.target` so `graphical.target` starts automatically.
