<!-- SPDX-License-Identifier: MIT -->

# SM-G780G (`r8q`) Porting Board & Progress Dashboard

This board tracks the hardware bring-up and upstreaming state of the **Samsung Galaxy S20 FE (`r8q`, SM8250)**.

- **Live Machine Baseline (`2026-11-21`):** kernel `7.3.0-rc6-omamobile-push1-gae195e6deaac` (`r8q-gpu-fault-quiesce`), native UFS root on `/dev/sda36` (`LABEL=omamobile-root`, `77/77` UFS partitions `ro=1`), `omamobile-device-r8q 0.1.29-1`, `pd-mapper 1.1-2`, `omamobile-settings 0.1.3-1`.
- **Stable Pin ([`../stable-image.conf`](../stable-image.conf)):** `out-r8q/recovery-fault-quiesce.img` (`adeddb31…660a6`, flashed to `sda24`).
- **Helper Scripts:** [`tools/r8q-progress`](../../../tools/r8q-progress) (`--live`, `--next N`, `--json`) + [`tools/r8q-check-all`](../../../tools/r8q-check-all) (`--quick` for one-shot repo + worktree + phone health pass), backed by [`tasks.json`](tasks.json).

| Track / Metric | Progress | Notes |
| --- | --- | --- |
| **Overall (Hardware + Upstream)** | **60.0% (39/65)** | Full SM8250 hardware bring-up + kernel DCO/PR readiness |
| **Hardware Bring-Up (`CONTRIBUTING.md` §5.4 Stages 1–16)** | **62.5% (35/56)** | Native root, 120 Hz display, Vulkan GPU, Wi-Fi/BT, audio + earpiece, stock HW audit complete |
| **Daily Core (Stages 1–8 + UI)** | **81.8% (18/22)** | Usable daily palmtop on desk with `omamobile-settings 0.1.3` + `LFM3-1.7B` AI (`10.5 tok/s`) |
| **Upstream & Docs (U1–U2)** | **44.4% (4/9)** | Repo CI gates, `CONTRIBUTING.md` §3.1 & `DEVICES.md` updated; 38 kernel commits awaiting human `Signed-off-by:` |

## Board Columns

| Column | Summary File | Scope |
| --- | --- | --- |
| **Success / Hardware-Verified** | [SUCCESS.md](SUCCESS.md) | 7 verified cards (`01`–`07`), native `sda36`, `120 Hz`, `SamsungGalaxyS2` audio |
| **Failed / Permanent Rules** | [FAILED.md](FAILED.md) | 8 permanent avoidance rules (`01`–`08`, incl. `s2idle`/DVFS, `max77705 =y`, Vulkan LLM wedge) |
| **Missing / Atomic Queue** | [MISSING.md](MISSING.md) | Full stage-by-stage completion %, remaining step-by-step atomic tasks, and `tasks.json` map |

See also: [Master File Dashboard](../FILE-DASHBOARD.md) · [Port Plan](../PORT-PLAN.md) · [Chronological Notes](../NOTES.md).
