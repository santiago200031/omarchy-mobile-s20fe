<!-- SPDX-License-Identifier: MIT -->

# Missing 01: Flash Native DT `1324fe066` Recovery & Verify Boot Target

- **Priority:** P1 (optional cleanup; live phone runs `gd469bd3bf2a9` safely via `offline_unreserved_xpu_pages()` fallback).
- **Back:** [MISSING.md](../MISSING.md) · [Board Overview](../README.md)

## Remaining Work
1. In `~/omamobile-kernel/wt-r8q` (`omamobile-r8q`), add human maintainer sign-off (`git commit --amend -s`) to `1324fe066`.
2. With human approval and Download Mode, flash `/home/amd-omarchy/omamobile-kernel/out-r8q/recovery-omamobile.img` (SHA256 `8dec9656073dd663769eff7a587fc74e2321068abda825feb45a7636d47c2cef`) to `RECOVERY` (`sda24`); modules for `7.3.0-rc6-omamobile-push1-g1324fe066cbb` are already unpacked and `depmod`-indexed on the phone.
3. Verify on boot:
   - `uname -r` equals `7.3.0-rc6-omamobile-push1-g1324fe066cbb`.
   - `grep -Eq '(90500000-|9e300000-|9fb00000-bcbfffff) .*System RAM' /proc/iomem` returns empty (native DT `no-map` active).
   - Once soak passes, optionally switch cmdline off `systemd.unit=multi-user.target` so `graphical.target` starts automatically.
