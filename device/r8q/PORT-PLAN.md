<!-- SPDX-License-Identifier: MIT -->

# Omarchy Mobile — Samsung Galaxy S20 FE (`r8q`, SM-G780G) Port Plan

- **Status (2026-11-20):** the phone runs the native UFS root; work happens on the device. Details: [NOTES §40–§42](NOTES.md).

| Subsystem | State |
| --- | --- |
| Boot | `sda24` (recovery) → **native root `sda36`** (LABEL `omamobile-root`); the failing microSD is out of the path — read-only source of last resort |
| Session | sddm autologin → Hyprland + quickshell; confirmed on screen (§41) |
| AI | `omamobile-ai` serves the **pi-tuned LFM3-1.7B** — Q8_0 base + harbor-pi LoRA, 10.5 tok/s out (§42); pi = the session user's mise install |
| RAM | zram 2.7 G + MGLRU (ttl 1 s) + 4 G swapfile = **6.8 G tiered swap** (NOTES §46) |
| Battery | **verified live**: 99 %, charging, 4.35 V, 27.5 °C, `health Good` (NOTES §44) — the old FG-defer claim was a bring-up artifact |
| Display | `1080x2400@60` deliberate (failed-05); **@120 queued behind GPU stabilization** |
| Kernel | `r8q-gpu-smmu-vote` (`ga60f7256c741`) under test; the GPU wedge is root-caused to the adreno SMMU's CX vote (§39), fix in flight on `r8q-gpu-smmu-vote` |
| Modem | Sahara walk clean after the power cycle; the AMSS-jump link fault (`RxErr` on the root port) is the open blocker (§38) |
| Way back | `stable-image.conf` pins the SD-loop-era image (`g3667ab368ef9`): `tools/r8q-flash-recovery --stable`; re-pin a native image once the GPU lands |
- **Milestone:** **The desktop, touchscreen, Wi-Fi (`wlp1s0`), QCA6390 Bluetooth (`hci0`), audio (`cs35l41` `r8q`), SLPI sensors, Volume Up, pseudo-NMI, the HI-847 telephoto and the S5K3L6 ultrawide are live on SM-G780G**, and the `omamobile-ai` supervisor serves `LFM3-1.7B` on `:8080` at 15.47 prompt / 10.6-11.0 generation tok/s ([AI-BENCHMARKS.md](AI-BENCHMARKS.md)). The SDX55M modem **boots to AMSS on mainline** (2026-11-18, [MODEM-PLAN.md](MODEM-PLAN.md) §10): the `qcom,pci-msi` doorbell with GIC-hwirq message data, ported MHI SBL/AMSS support (SBL power-up, context zeroing, mission-mode doorbell re-read), AP2MDM_STATUS driven high, and the full 14-image SAHARA set served over one open MHI channel. The modem enters mission mode itself and exposes `mhi0_QMI/MBIM/IPCR/IP_HW0/IP_SW0` and `/dev/wwan0{qmi,mbim,qcdm,at}0`. Next for daily use: SIM/QMI through userspace on the Arch root, and a kernel release with the three commits signed off. Main/front cameras remain the open bring-up ([CAMERA-PLAN.md](CAMERA-PLAN.md)).
- **Boot model correction (2026-11-20, owner-observed):** `sda24` is this phone's **power-on default** — a plain power-on boots our kernel; there is **no automatic fallback to LineageOS** (`sda23` contents remain untouched but unreachable by an ordinary boot). `Volume Up + Power` is not needed to boot our image; Download Mode (Volume Up + Volume Down + cable from power-off) is the always-available recovery path. Details: [NOTES §24](NOTES.md).

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
| **Missing / Next** | [board/MISSING.md](board/MISSING.md) | [Sensors, Cameras & DPMS](board/missing/04-sensors-cameras-dpms-and-installer.md) (main/front cameras) · [Cellular / Modem](board/missing/03-wifi-bluetooth-and-modem.md) (AMSS reached; userspace SIM/QMI next) · [Freedreno & Vulkan](board/missing/05-soak-pseudonmi-and-ai-benchmarks.md) (optional) · [Native DT & Touch Commits](board/missing/01-flash-native-dt-recovery-1324fe066.md) (upstream sign-offs) |

---

## Reference Docs & Safety Rules

- **Detailed Chronicles:** [NOTES.md](NOTES.md) (crash postmortems & root causes) · [RUNBOOK-DISPLAY-GPU.md](RUNBOOK-DISPLAY-GPU.md) (staged Gate A–D commands & logs) · [CRASH-DOSSIER.md](CRASH-DOSSIER.md)
- **Hardware Baseline:** `SM-G780G` (`r8q`, SM8250, `msm-id=<0x164 0x20001>`, `board-id=<0x8 0x6>`), ~5.5 GiB LPDDR5, `S6E3FC3`/`AMS646YD01` (`1080x2400`), `stmfts` (`i2c5@49`), `max77705` (`i2c0`), dual `cs35l41` (`i2c11`), `sdhc_2` (`mmcblk0p1`).

> **Safety Rules ([AGENTS.md](../../AGENTS.md)):** Never touch `boot`, `super`, `userdata`, bootloader, `vbmeta`, or NV partitions; ask before every flash or reboot; build in container (`tools/build-packages --local r8q`); mask all serials/MACs/IMEIs; run `tools/pr-check` and `tests/packages` before every push.
