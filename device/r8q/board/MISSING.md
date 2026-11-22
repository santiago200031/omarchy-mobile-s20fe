<!-- SPDX-License-Identifier: MIT -->

# Column: Missing / Remaining Atomic Tasks (`r8q`)

Source of truth: [`tasks.json`](tasks.json) — check live percentages and next steps anytime with `tools/r8q-progress` (`--live`), or run `tools/r8q-check-all --quick` for a single read-only repo + worktree + phone health pass. Stock hardware findings live in [HARDWARE-DISCOVERY.md](../HARDWARE-DISCOVERY.md).

As of **`2026-11-22`** (`7.3.0-rc6-omamobile-push1-g358e286ef5a6`, `omamobile-settings 0.1.3-1`, `omamobile-device-r8q 0.1.34-1`):
- **Overall (Hardware + Upstream):** `75.4%` (**49 / 65** atomic tasks)
- **Hardware Bring-Up (`CONTRIBUTING.md` §5.4 Stages 1–16):** `80.4%` (**45 / 56** atomic tasks)
- **Daily Core (Stages 1–8):** `95.5%` (**21 / 22** atomic tasks)
- **Upstream & Docs (Stages U1–U2):** `44.4%` (**4 / 9** atomic tasks)

## Stage-by-Stage Milestones (`CONTRIBUTING.md` §5.4)

| Stage | Subsystem | Done | % | Status | Remaining Atomic Tasks |
| --- | --- | --- | --- | --- | --- |
| `1–2` | Boot, USB gadget, SSH, Native root & Storage guard | `5/5` | `100.0%` | ✅ Achieved | — |
| `3` | Touch & Buttons (`stmfts`, `gpio-keys`) | `3/3` | `100.0%` | ✅ Achieved | — |
| `4` | GPU (`Adreno 650` / Vulkan & recovery) | `5/5` | `100.0%` | ✅ Achieved | — (`802d3d3bc` `-p 512` fixed; `DSI-1` 120 Hz fault survival; `GPU_LAYERS=0` verified faster) |
| `5` | Display (`S6E3FC3 120 Hz`) & Shell UI | `4/4` | `100.0%` | ✅ Achieved | — (`omamobile-settings 0.1.3-1` live) |
| `6` | Battery & Fuel Gauge (`max77705` / `max17042`) | `2/2` | `100.0%` | ✅ Achieved | — |
| `7–8` | Wi-Fi & Bluetooth (`QCA6390`) | `2/3` | `66.7%` | ✅ Achieved | `[net-03]` BT A2DP audio + Wi-Fi concurrency test |
| `9` | Haptics & Torch | `4/4` | `100.0%` | ✅ Achieved | — (`haptics.dtso` `max77705-haptic` + `gcc_gp2_clk_src` `20 kHz` `fbcli` buzz & RT8547 `torch.dtso` both verified physically) |
| `10–11` | Power, Idle, Thermal & DVFS | `2/4` | `50.0%` | ⚠️ Mitigated | `[pwr-03]` scoped CPUfreq `schedutil` trace; `[pwr-04]` deeper `cpuidle` / UFS PM |
| `12` | USB-C Role Switch & Charging Control | `2/4` | `50.0%` | ⚠️ Partial | `[usb-03]` replug recovery & PD sink + `[usb-04]` OTG host under `usbc-deadman` |
| `13` | Suspend (`s2idle`) | `1/3` | `33.3%` | ❌ Disabled | `[slp-02]` attended single `s2idle` w/ watchdog; `[slp-03]` 10-cycle soak |
| `14` | Remoteprocs, Sensors & Audio (`ADSP`/`SLPI`/`CS35L41`/`DMIC3`) | `6/6` | `100.0%` | ✅ Achieved | — (speaker + earpiece + `dmic-va.dtso` `DMIC3` `pw-record` verified) |
| `15a` | Cameras (`CAMSS` / `libcamera`) | `4/5` | `80.0%` | ⚠️ Partial | `[cam-05]` IMX555 (rear 1x) + IMX616 (front) drivers (`omacam` `0.5x <-> 3x` verified) |
| `15b` | Modem (`SDX55M`), GNSS & NFC | `3/6` | `50.0%` | ⚠️ Partial | `[mdm-03..05]` native `sda36` `modem.dtso` boot test, NV re-check, opt-in SIM/QMI/MBIM |
| `16` | Fingerprint | `2/2` | `100.0%` | ⏸️ Deferred (TZ/QSEE) | — ([FINGERPRINT-PLAN.md](../FINGERPRINT-PLAN.md): assessed; XPU SPI + QSEE `securefp` + `/dev/qseecom` + HBM FOD not viable in mainline) |
| `U1` | Kernel Branch Hygiene & Upstream DCO (`wt-r8q`) | `1/5` | `20.0%` | 🔧 Open | `[krn-02..05]` human `Signed-off-by:` on 38 commits, split topic branches, `kernel-pr-check`, upstream PRs |
| `U2` | Repo Registry & Stable Pin | `3/4` | `75.0%` | 🔧 Open | `[doc-04]` re-pin `stable-image.conf` after base-DTS fold |

## Complete Step-by-Step Queue (`16` Remaining Atomic Tasks)

### Track A — Local Userspace / Offline Tasks (No Reboot Required)
1. `[net-03]` Pair a BT audio sink and verify A2DP playback alongside `wlp1s0` Wi-Fi traffic.

### Track B — Next Kernel DT Fixes (`AGENTS.md` §1–2)
2. `[doc-04]` *(human-gate)* Fold verified `camera-uw`/`torch`/`dmic-va`/`haptics` DT changes cleanly, pack, verify `tools/phone-smoke` 100% green, and re-pin `device/r8q/stable-image.conf`.

### Track C — High-Risk Hardware Bring-Up (Needs Watchdog / `usbc-deadman` / NV Re-check)
10. `[usb-03]` -> `[usb-04]` *(human-gate via `tools/usbc-deadman`)* Test Type-C replug gadget recovery + PD/OTG role-switch (`max77705` MUIC/TCPM).
11. `[mdm-03]` -> `[mdm-05]` *(human-gate + NV checksum check)* Test `modem.dtso` on native `sda36` root (confirm no SD-card brownout hang), verify `15/15` NV checksums, test SIM/QMI/MBIM data & calls.
12. `[pwr-03..04]` & `[slp-02..03]` *(human-gate w/ watchdog)* Test attended single `s2idle` wake, scope CPUfreq `schedutil` / deeper `cpuidle` / UFS PM, and soak 10 sleep cycles before removing `10-r8q-no-suspend.conf`.
13. `[cam-05]` Reverse/upstream IMX555 (rear wide) and IMX616 (front) V4L2 subdev drivers & DT nodes.

### Track D — Kernel Upstream Submission (`CONTRIBUTING.md` §6.2)
14. `[krn-02]` *(human-only DCO)* Human maintainer adds `Signed-off-by: Santiago Villavicencio` to all **38** unsigned commits in `~/omamobile-kernel/wt-r8q` (`4d5ae30a8..HEAD`).
15. `[krn-03..05]` Split `wt-r8q` into clean subsystem topic branches, pass `tools/kernel-pr-check --build --merge origin/omamobile`, and open PRs against `linux-omarchy-mobile/omamobile`.

---

## Historical Notes Cards (Archive)
The detailed bring-up logs from earlier gates stay preserved in [`missing/`](missing/) for provenance:
- [01 Native DT, Touch & BT Commits](missing/01-flash-native-dt-recovery-1324fe066.md)
- [02 Gate C: Remoteprocs & Audio](missing/02-gate-c-remoteprocs-and-audio.md)
- [03 Wi-Fi, Bluetooth & Modem AMSS](missing/03-wifi-bluetooth-and-modem.md)
- [04 Sensors, Cameras, DPMS & Installer](missing/04-sensors-cameras-dpms-and-installer.md)
- [05 Soak, Pseudo-NMI & AI Benchmarks](missing/05-soak-pseudonmi-and-ai-benchmarks.md)

Back to [Board Overview](README.md).
