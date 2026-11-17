<!-- SPDX-License-Identifier: MIT -->

# Missing 01: Flash Native DT `1324fe066` + Touch `5866f774c` + BT `caa385175` Recovery

- **Priority:** P1 (optional cleanup; live phone runs `gd469bd3bf2a9` safely via `offline_unreserved_xpu_pages()`, `stmfts-r8q.ko`, and `r8q-bt-uart.ko` + `bluetooth.ko` modules in `omamobile-device-r8q` `0.1.18`).
- **Status (`2026-11-17`):** the flash side is **done** — the stable image on `sda24` ([`stable-image.conf`](../../stable-image.conf)) carries the native DT and touch, and `tools/r8q-stable-check --phone` passes. What remains is the human `Signed-off-by:` and the upstream pull requests.
- **Back:** [MISSING.md](../MISSING.md) · [Board Overview](../README.md)

## Remaining Work
1. In `~/omamobile-kernel/wt-r8q` (`omamobile-r8q`), add human maintainer sign-off (`git rebase -i` / `Signed-off-by:`) to `1324fe066` (native XPU `no-map` DT), `5866f774c` (`stmfts` 16-byte Type-B touch support), and `caa385175` (`qcom_geni_serial` alias clamp for Bluetooth UART probing).
2. Rebuild `recovery-omamobile.img` with `device/r8q/kernel.config` (`CONFIG_BT_LE=y`, `RFCOMM`, `BNEP`) and the signed commits (the existing `/home/amd-omarchy/omamobile-kernel/out-r8q/recovery-omamobile.img` SHA256 `2769d7da9701a3ac4b1f3fb51e559414ad979b8dcc642013778a2d7d3e1248e3` is the pre-`caa385175` `5866f774c` build; its `7.3.0-rc6-omamobile-push1-g5866f774c4da` modules are unpacked on the phone). With human approval and Download Mode, flash the rebuilt `RECOVERY` (`sda24`) and push its matching `/usr/lib/modules` tarball.
3. Verify on boot:
   - `uname -r` matches the newly built kernel release (including `caa385175`).
   - `grep -Eq '(90500000-|9e300000-|9fb00000-bcbfffff) .*System RAM' /proc/iomem` returns empty (native DT `no-map` active).
   - `stmfts` (`5-0049`), `998000.serial` / `hci0`, and `bluetooth` (`LE`/`RFCOMM`/`BNEP`) work natively without loading `stmfts-r8q.ko` or `r8q-bt-uart.ko`.
   - Once soak passes, optionally keep or tune `multi-user.target` vs `graphical.target` (currently `sddm.service` is also wanted by `multi-user.target` in `0.1.18`).
