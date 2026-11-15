<!-- SPDX-License-Identifier: MIT -->

# Omarchy Mobile — Samsung Galaxy S20 FE (`r8q`, SM-G780G) Port Plan

- **Status (2026-11-15):** Kernel `7.3.0-rc6-omamobile-push1` (`omamobile-r8q` worktree `~/omamobile-kernel/wt-r8q`, out `~/omamobile-kernel/out-r8q`, build with `DEVICE=r8q`). Dual-boot boots `recovery` (`sda24`) with `/omarchy-r8q.img` on microSD (`mmcblk0p1`); **all `77/77` `/dev/sda*` UFS partitions remain `ro=1`** with LineageOS untouched on `sda23`/`sda29`/`sda36`.
- **Milestone:** **Gate D graphical desktop is live on SM-G780G** (`sddm` + `Hyprland` + `quickshell` + `squeekboard` via `msm`/Adreno 650 + `extra/mesa`, `omamobile-device-r8q` `0.1.16`). Clean DT rebuild `1324fe066` (`7.3.0-rc6-omamobile-push1-g1324fe066cbb`, SHA256 `8dec9656073dd663769eff7a587fc74e2321068abda825feb45a7636d47c2cef`) and modules are staged and ready when approved.

---

## File-Based Porting Board ([board/](board/README.md))

The porting state is maintained as a file-based Kanban board under [`device/r8q/board/`](board/README.md):

| Column | Summary | Cards |
| --- | --- | --- |
| **Success / Working** | [board/SUCCESS.md](board/SUCCESS.md) | [Dual-Boot RO](board/success/01-dual-boot-ufs-ro-guard.md) · [RPMh & Liveness](board/success/02-rpmh-rsc-and-liveness-net.md) · [Display / S6E3FC3](board/success/03-display-mdss-s6e3fc3.md) · [Adreno & Desktop](board/success/04-gpu-adreno650-hyprland-desktop.md) · [USB, Battery & ZRAM](board/success/05-usb-power-battery-zram.md) |
| **Failed / Skip** | [board/FAILED.md](board/FAILED.md) | [XPU/TZ/Knox DRAM](board/failed/01-unreserved-xpu-tz-knox-dram.md) · [`s2idle`/cpuidle/PM](board/failed/02-suspend-cpuidle-dvfs-ufspm.md) · [`max77705` / unbind](board/failed/03-max77705-builtin-and-unbind.md) · [DISPCC / white flash](board/failed/04-display-protected-clocks-and-white-flash.md) · [Mesa / `vfr` / `@120`](board/failed/05-omamobile-mesa-and-hyprland-vfr.md) · [Boot/session hazards](board/failed/06-boot-and-session-hazards.md) |
| **Missing / Next** | [board/MISSING.md](board/MISSING.md) | [Flash DT `1324fe066`](board/missing/01-flash-native-dt-recovery-1324fe066.md) · [Gate C Audio/DSP](board/missing/02-gate-c-remoteprocs-and-audio.md) · [Wi-Fi/BT/Modem](board/missing/03-wifi-bluetooth-and-modem.md) · [Sensors/Cam/DPMS/Install](board/missing/04-sensors-cameras-dpms-and-installer.md) · [Soak/NMI/AI](board/missing/05-soak-pseudonmi-and-ai-benchmarks.md) |

---

## Reference Docs & Safety Rules

- **Detailed Chronicles:** [NOTES.md](NOTES.md) (crash postmortems & root causes) · [RUNBOOK-DISPLAY-GPU.md](RUNBOOK-DISPLAY-GPU.md) (staged Gate A–D commands & logs) · [CRASH-DOSSIER.md](CRASH-DOSSIER.md)
- **Hardware Baseline:** `SM-G780G` (`r8q`, SM8250, `msm-id=<0x164 0x20001>`, `board-id=<0x8 0x6>`), ~5.5 GiB LPDDR5, `S6E3FC3`/`AMS646YD01` (`1080x2400`), `stmfts` (`i2c5@49`), `max77705` (`i2c0`), dual `cs35l41` (`i2c11`), `sdhc_2` (`mmcblk0p1`).

> **Safety Rules ([AGENTS.md](../../AGENTS.md)):** Never touch `boot`, `super`, `userdata`, bootloader, `vbmeta`, or NV partitions; ask before every flash or reboot; build in container (`tools/build-packages --local r8q`); mask all serials/MACs/IMEIs; run `tools/pr-check` and `tests/packages` before every push.
