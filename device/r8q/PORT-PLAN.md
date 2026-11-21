<!-- SPDX-License-Identifier: MIT -->

# Omarchy Mobile — Samsung Galaxy S20 FE (`r8q`, SM-G780G) Port Plan

- **Status (`2026-11-21`):** the phone runs the native UFS root on `/dev/sda36` cleanly (`omamobile-device-r8q 0.1.30-1`, `recovery-cam-torch-dmic.img` verified live over `stable-image.conf`); run `tools/r8q-progress` for live % & atomic tasks ([`board/tasks.json`](board/tasks.json)). Details: [NOTES §40–§52](NOTES.md).

| Subsystem | State |
| --- | --- |
| Boot | `sda24` (recovery) → **native root `sda36`** (LABEL `omamobile-root`, `77/77` UFS `ro=1`); the failing microSD is out of the path |
| Session | `sddm` autologin → Hyprland + `quickshell` + `squeekboard` (`omamobile-settings 0.1.3-1` built & installed natively on phone) |
| AI | `omamobile-ai` serves the **pi-tuned LFM3-1.7B** — Q8_0 base + harbor-pi LoRA, `10.5 tok/s` out (§42); `pi` = the session user's `mise` install |
| RAM | `zram` `2.7 G` + MGLRU (`ttl 1 s`) + `4 G` swapfile = **`6.8 G` tiered swap** (NOTES §46) |
| Battery | **verified live**: `99–100 %`, `4.37 V`, `27.5 °C`, `health Good`, thermals `38.0–42.3 °C` (NOTES §44 + `phone-smoke`) |
| Display | **`1080x2400@120` achieved** (NOTES §49): zero DSI/TE errors, compute under load clean at the doubled refresh |
| GPU | **Vulkan compute demonstrated** (NOTES §48): `pp64 30.23 t/s`, `tg16 1.37 t/s`, clean exit; a fault-induced recovery ran once and **the desktop survived it**. Serving stays on CPU; the `-p 512` stall bisect & DPU post-recovery restore remain |
| Audio & Sensors | **Stage 14 achieved (speakers + earpiece + DMIC3 mic)** (`2026-11-21`, NOTES §51–§52): `pd-mapper 1.1-2` + `omamobile-device-r8q 0.1.30-1` (`-9.5 dB` L/R CS35L41 + `Earpiece`), `dmic-va.dtso` (`vreg_l4c_1p8` + `dmic23_active` + `2.4 MHz`) + `50-r8q-audio.conf` (`S16LE`/`48k`), full-duplex + `pw-record` clean (`rms=10.29`), SLPI accel/light live |
| Cameras | **ultrawide & telephoto restored via `camera-uw.dtso`** (`NOTES §52`): `cam -l` lists `s5k3l6xx` + `hi847`, WirePlumber exposes both sources, `phone-smoke cameras ok (2 listed by libcamera)`; interactive `omacam` check & IMX555/IMX616 drivers open (base DTS fold tracked in `krn-03`) |
| Torch & Haptics | **Torch node ready (`white:torch` `664 root:video`)** via `torch.dtso` + `20-r8q-torch.rules` (`NOTES §52`); human strobe check & shell tile toggle pending; haptics (`LINEAR_0832RC`) offline-researched in [HARDWARE-DISCOVERY.md](HARDWARE-DISCOVERY.md) |
| Kernel | `r8q-gpu-fault-quiesce` (`7.3.0-rc6-omamobile-push1-gae195e6deaac`) **running, pinned** in `stable-image.conf` (`adeddb31…660a6`); **38 of 83** commits ahead of `omamobile` still need human `Signed-off-by:` |
| Modem | SDX55M reached `AMSS` (`EE: MISSION MODE`, `MODEM-PLAN` §10) in rescue boot; parked in daily boot (`pcie2` disabled); next: test `modem.dtso` on native `sda36` + opt-in SIM/QMI/MBIM |
| Way back | [`stable-image.conf`](stable-image.conf) pins `out-r8q/recovery-fault-quiesce.img` (`gae195e6deaac`, `adeddb31…660a6`): `tools/r8q-flash-recovery --stable` |
- **Progress Snapshot (`2026-11-21`, `tools/r8q-progress`):** **64.6% overall (42/65 atomic tasks)** · **67.9% Hardware Stages 1–16 (38/56)** · **81.8% Daily Core Stages 1–8 (18/22)** · **44.4% Upstream & Docs (4/9)**. Run `tools/r8q-check-all --quick` for a one-shot health pass; see [board/MISSING.md](board/MISSING.md) for the exact step-by-step atomic backlog.
- **Boot model correction (2026-11-20, owner-observed):** `sda24` is this phone's **power-on default** — a plain power-on boots our kernel; there is **no automatic fallback to LineageOS** (`sda23` contents remain untouched but unreachable by an ordinary boot). `Volume Up + Power` is not needed to boot our image; Download Mode (Volume Up + Volume Down + cable from power-off) is the always-available recovery path. Details: [NOTES §24](NOTES.md).

---

## File Dashboard & Progress Tracker ([FILE-DASHBOARD.md](FILE-DASHBOARD.md))

| File | Purpose |
|------|--------|
| [`tools/r8q-progress`](../../tools/r8q-progress) | CLI progress % reporter + `--live` read-only phone/worktree checker |
| [board/tasks.json](board/tasks.json) | Single source of truth for Stages 1–16 & upstream atomic tasks |
| [FILE-DASHBOARD.md](FILE-DASHBOARD.md) | Master dashboard — live state, file inventory, and what's missing |
| [board/README.md](board/README.md) | Kanban board overview & summary percentages |
| [board/SUCCESS.md](board/SUCCESS.md) | 7 verified-success cards |
| [board/FAILED.md](board/FAILED.md) | 8 known failures with permanent rules |
| [board/MISSING.md](board/MISSING.md) | Stage-by-stage % table + ordered atomic queue |

## File-Based Porting Board ([board/](board/README.md))

The porting state is also maintained as a file-based Kanban board under [`device/r8q/board/`](board/README.md):

| Column | Summary | Cards |
| --- | --- | --- |
| **Success / Working** | [board/SUCCESS.md](board/SUCCESS.md) | [Dual-Boot RO](board/success/01-dual-boot-ufs-ro-guard.md) · [RPMh & Liveness](board/success/02-rpmh-rsc-and-liveness-net.md) · [Display / S6E3FC3](board/success/03-display-mdss-s6e3fc3.md) · [Adreno & Desktop](board/success/04-gpu-adreno650-hyprland-desktop.md) · [USB, Battery & ZRAM](board/success/05-usb-power-battery-zram.md) · [Wi-Fi, BT & Clock](board/success/06-wifi-bluetooth-and-clock.md) · [Soak, NMI & AI](board/success/07-stability-soak-nmi-and-ai.md) |
| **Failed / Skip** | [board/FAILED.md](board/FAILED.md) | [XPU/TZ/Knox DRAM](board/failed/01-unreserved-xpu-tz-knox-dram.md) · [`s2idle`/cpuidle/PM](board/failed/02-suspend-cpuidle-dvfs-ufspm.md) · [`max77705` / unbind](board/failed/03-max77705-builtin-and-unbind.md) · [DISPCC / white flash](board/failed/04-display-protected-clocks-and-white-flash.md) · [Mesa / `vfr` / `@120`](board/failed/05-omamobile-mesa-and-hyprland-vfr.md) · [Boot/session hazards](board/failed/06-boot-and-session-hazards.md) · [`fts5cu56a` 16-byte events](board/failed/07-stmfts5-8byte-vs-samsung-16byte-events.md) · [Vulkan LLM wedge](board/failed/08-vulkan-llm-adreno-gpu-wedge.md) |
| **Missing / Next** | [board/MISSING.md](board/MISSING.md) | Stage-by-stage completion % (`CONTRIBUTING.md` §5.4 Stages 1–16 + Upstream U1–U2) and ordered step-by-step atomic tasks (`board/tasks.json`) |

---

## Reference Docs & Safety Rules

- **Detailed Chronicles:** [NOTES.md](NOTES.md) (crash postmortems & root causes) · [RUNBOOK-DISPLAY-GPU.md](RUNBOOK-DISPLAY-GPU.md) (staged Gate A–D commands & logs) · [CRASH-DOSSIER.md](CRASH-DOSSIER.md)
- **Hardware Baseline:** `SM-G780G` (`r8q`, SM8250, `msm-id=<0x164 0x20001>`, `board-id=<0x8 0x6>`), ~5.5 GiB LPDDR5, `S6E3FC3`/`AMS646YD01` (`1080x2400`), `stmfts` (`i2c5@49`), `max77705` (`i2c0`), dual `cs35l41` (`i2c11`), `sdhc_2` (`mmcblk0p1`).

> **Safety Rules ([AGENTS.md](../../AGENTS.md)):** Never touch `boot`, `super`, `userdata`, bootloader, `vbmeta`, or NV partitions; ask before every flash or reboot; build in container (`tools/build-packages --local r8q`); mask all serials/MACs/IMEIs; run `tools/pr-check` and `tests/packages` before every push.
