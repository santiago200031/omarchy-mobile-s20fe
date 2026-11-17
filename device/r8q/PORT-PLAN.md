<!-- SPDX-License-Identifier: MIT -->

# Omarchy Mobile — Samsung Galaxy S20 FE (`r8q`, SM-G780G) Port Plan

- **Status (2026-11-17):** Stable kernel `7.3.0-rc6-omamobile-push1-g3667ab368ef9` (`camera-uw-wip`), image `~/omamobile-kernel/out-r8q-uw/recovery-cam-uw.img` sha256 `66fe127f…d8503`, packed with `tools/r8q-boot pack --dual-boot --overlay camera-uw` and pinned in [`stable-image.conf`](stable-image.conf). It boots `recovery` (`sda24`) with `/omarchy-r8q.img` on microSD (`mmcblk0p1`); **all `77/77` `/dev/sda*` UFS partitions remain `ro=1`** with LineageOS untouched on `sda23`/`sda29`/`sda36`; `PCIe2` and its PHY are `disabled`. `tools/r8q-stable-check` verifies both the image hash and the phone, and `tools/r8q-flash-recovery --stable` is the way back. Kernel worktree `~/omamobile-kernel/wt-r8q` (build with `DEVICE=r8q`).
- **Milestone:** **The desktop, touchscreen, Wi-Fi (`wlp1s0`), QCA6390 Bluetooth (`hci0`), audio (`cs35l41` `r8q`), SLPI sensors, Volume Up, pseudo-NMI, the HI-847 telephoto and the S5K3L6 ultrawide are live on SM-G780G**, and the `omamobile-ai` supervisor serves `LFM3-1.7B` on `:8080` at 15.47 prompt / 10.6-11.0 generation tok/s ([AI-BENCHMARKS.md](AI-BENCHMARKS.md)). The SDX55M modem reached Stage 2 (`mhi0` BHI ready) and Stage 3A (`PCIe2` MSI) and is **parked**: Stage 3A resets the phone when `mhi_pci_generic` binds ([MODEM-PLAN.md](MODEM-PLAN.md) §8), and the phone is back on the stable image. The offline design is recorded in [MODEM-PLAN.md](MODEM-PLAN.md) §9: the stock `qcom,pci-msi` memory doorbell (`0x17a10040`, 32 SPIs from `0x340`) with `msi-parent` on `&pcie2`, replacing the DWC iMSI-RX path for the modem; the driver, binding and gated runbook are the next laptop work. Main/front cameras remain the open bring-up ([CAMERA-PLAN.md](CAMERA-PLAN.md)).

---

## File Dashboard ([FILE-DASHBOARD.md](FILE-DASHBOARD.md))

A comprehensive file-based dashboard tracking all subsystems, their status, and what files are involved:

| File | Purpose |
|------|--------|
| [FILE-DASHBOARD.md](FILE-DASHBOARD.md) | Master dashboard — all files, their status, and what's missing |
| [board/README.md](board/README.md) | Kanban board overview |
| [board/SUCCESS.md](board/SUCCESS.md) | 7 verified-success cards |
| [board/FAILED.md](board/FAILED.md) | 7 known failures with permanent rules |
| [board/MISSING.md](board/MISSING.md) | 4 open items in priority order (cameras, parked modem, optional GPU/Vulkan, upstream sign-offs) |

## File-Based Porting Board ([board/](board/README.md))

The porting state is also maintained as a file-based Kanban board under [`device/r8q/board/`](board/README.md):

| Column | Summary | Cards |
| --- | --- | --- |
| **Success / Working** | [board/SUCCESS.md](board/SUCCESS.md) | [Dual-Boot RO](board/success/01-dual-boot-ufs-ro-guard.md) · [RPMh & Liveness](board/success/02-rpmh-rsc-and-liveness-net.md) · [Display / S6E3FC3](board/success/03-display-mdss-s6e3fc3.md) · [Adreno & Desktop](board/success/04-gpu-adreno650-hyprland-desktop.md) · [USB, Battery & ZRAM](board/success/05-usb-power-battery-zram.md) · [Wi-Fi, BT & Clock](board/success/06-wifi-bluetooth-and-clock.md) · [Soak, NMI & AI](board/success/07-stability-soak-nmi-and-ai.md) |
| **Failed / Skip** | [board/FAILED.md](board/FAILED.md) | [XPU/TZ/Knox DRAM](board/failed/01-unreserved-xpu-tz-knox-dram.md) · [`s2idle`/cpuidle/PM](board/failed/02-suspend-cpuidle-dvfs-ufspm.md) · [`max77705` / unbind](board/failed/03-max77705-builtin-and-unbind.md) · [DISPCC / white flash](board/failed/04-display-protected-clocks-and-white-flash.md) · [Mesa / `vfr` / `@120`](board/failed/05-omamobile-mesa-and-hyprland-vfr.md) · [Boot/session hazards](board/failed/06-boot-and-session-hazards.md) · [`fts5cu56a` 16-byte events](board/failed/07-stmfts5-8byte-vs-samsung-16byte-events.md) |
| **Missing / Next** | [board/MISSING.md](board/MISSING.md) | [Sensors, Cameras & DPMS](board/missing/04-sensors-cameras-dpms-and-installer.md) (main/front cameras) · [Cellular / Modem](board/missing/03-wifi-bluetooth-and-modem.md) (design done, parked) · [Freedreno & Vulkan](board/missing/05-soak-pseudonmi-and-ai-benchmarks.md) (optional) · [Native DT & Touch Commits](board/missing/01-flash-native-dt-recovery-1324fe066.md) (upstream sign-offs) |

---

## Reference Docs & Safety Rules

- **Detailed Chronicles:** [NOTES.md](NOTES.md) (crash postmortems & root causes) · [RUNBOOK-DISPLAY-GPU.md](RUNBOOK-DISPLAY-GPU.md) (staged Gate A–D commands & logs) · [CRASH-DOSSIER.md](CRASH-DOSSIER.md)
- **Hardware Baseline:** `SM-G780G` (`r8q`, SM8250, `msm-id=<0x164 0x20001>`, `board-id=<0x8 0x6>`), ~5.5 GiB LPDDR5, `S6E3FC3`/`AMS646YD01` (`1080x2400`), `stmfts` (`i2c5@49`), `max77705` (`i2c0`), dual `cs35l41` (`i2c11`), `sdhc_2` (`mmcblk0p1`).

> **Safety Rules ([AGENTS.md](../../AGENTS.md)):** Never touch `boot`, `super`, `userdata`, bootloader, `vbmeta`, or NV partitions; ask before every flash or reboot; build in container (`tools/build-packages --local r8q`); mask all serials/MACs/IMEIs; run `tools/pr-check` and `tests/packages` before every push.
