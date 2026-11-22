<!-- SPDX-License-Identifier: MIT -->

# Column: Missing / Remaining Atomic Tasks (`r8q`)

Source of truth: [`tasks.json`](tasks.json) — check live percentages and next steps anytime with `tools/r8q-progress` (`--live`), or run `tools/r8q-check-all --quick` for a single read-only repo + worktree + phone health pass. Stock hardware findings live in [HARDWARE-DISCOVERY.md](../HARDWARE-DISCOVERY.md).

As of **`2026-11-22`** (`7.3.0-rc6-omamobile-push1-g4d235a595f28`, `omamobile-settings 0.1.3-1`, `omamobile-device-r8q 0.1.36-1`, `omamobile-omarchy 0.1.49-1`, `omamobile-shell 0.2.141-1`):
- **Overall (Hardware + Upstream):** `80.6%` (**54 / 67** atomic tasks)
- **Hardware Bring-Up (`CONTRIBUTING.md` §5.4 Stages 1–16):** `84.5%` (**49 / 58** atomic tasks)
- **Daily Core (Stages 1–8):** `100.0%` (**22 / 22** atomic tasks)
- **Upstream & Docs (Stages U1–U2):** `55.6%` (**5 / 9** atomic tasks)

## Stage-by-Stage Milestones (`CONTRIBUTING.md` §5.4)

| Stage | Subsystem | Done | % | Status | Remaining Atomic Tasks |
| --- | --- | --- | --- | --- | --- |
| `1–2` | Boot, USB gadget, SSH, Native root & Storage guard | `5/5` | `100.0%` | ✅ Achieved | — |
| `3` | Touch & Buttons (`stmfts`, `gpio-keys`) | `3/3` | `100.0%` | ✅ Achieved | — |
| `4` | GPU (`Adreno 650` / Vulkan & recovery) | `5/5` | `100.0%` | ✅ Achieved | — (`802d3d3bc` `-p 512` fixed; `DSI-1` 120 Hz fault survival; `GPU_LAYERS=0` verified faster) |
| `5` | Display (`S6E3FC3 120 Hz`) & Shell UI | `4/4` | `100.0%` | ✅ Achieved | — (`omamobile-settings 0.1.3-1`, `omamobile-shell 0.2.141-1`, `omamobile-omarchy 0.1.49-1`) |
| `6` | Battery & Fuel Gauge (`max77705` / `max17042`) | `2/2` | `100.0%` | ✅ Achieved | — |
| `7–8` | Wi-Fi & Bluetooth (`QCA6390`) | `3/3` | `100.0%` | ✅ Achieved | — (`net-03` BT A2DP + Wi-Fi concurrency verified) |
| `9` | Haptics & Torch | `4/4` | `100.0%` | ✅ Achieved | — (`haptics` + `RT8547` `torch` verified physically) |
| `10–11` | Power, Idle, Thermal & DVFS | `4/6` | `66.7%` | ⚠️ Mitigated | `[pwr-03]` scoped CPUfreq `schedutil` trace; `[pwr-04]` deeper `cpuidle` / UFS PM |
| `12` | USB-C Role Switch & Charging Control | `3/4` | `75.0%` | ⚠️ Partial | `[usb-04]` OTG host 5V VBUS (`max77705` `0x25` USBC/CCIC driver or powered hub) |
| `13` | Suspend (`s2idle`) | `1/3` | `33.3%` | ❌ Disabled | `[slp-02]` attended single `s2idle` w/ watchdog; `[slp-03]` 10-cycle soak |
| `14` | Remoteprocs, Sensors & Audio (`ADSP`/`SLPI`/`CS35L41`/`DMIC3`) | `6/6` | `100.0%` | ✅ Achieved | — (speaker + earpiece + `DMIC3` `pw-record` verified) |
| `15a` | Cameras (`CAMSS` / `libcamera`) | `4/5` | `80.0%` | ⚠️ Partial | `[cam-05]` IMX555 (rear 1x) + IMX616 (front) drivers (`omacam` `0.5x <-> 3x` verified) |
| `15b` | Modem (`SDX55M`), GNSS & NFC | `3/6` | `50.0%` | ⚠️ Partial | `[mdm-03..05]` native `sda36` `modem.dtso` boot test, NV re-check, opt-in SIM/QMI/MBIM |
| `16` | Fingerprint | `2/2` | `100.0%` | ⏸️ Deferred (TZ/QSEE) | — ([FINGERPRINT-PLAN.md](../FINGERPRINT-PLAN.md): deferred, XPU/QSEE/`qseecom` blockers) |
| `U1` | Kernel Branch Hygiene & Upstream DCO (`wt-r8q`) | `1/5` | `20.0%` | 🔧 Open | `[krn-02..05]` human `Signed-off-by:` on 42 commits, split topic branches, `kernel-pr-check`, upstream PRs |
| `U2` | Repo Registry & Stable Pin | `4/4` | `100.0%` | ✅ Achieved | — |

## Complete Step-by-Step Queue (`13` Remaining Atomic Tasks)

### High-Risk / Hardware Bring-Up (`9` tasks — human gate for flashes/reboots)
1. `[mdm-03]` -> `[mdm-05]` *(human-gate + NV checksum check)* Test `modem.dtso` on native `sda36` root (confirm no SD-card brownout hang), verify `15/15` NV checksums, test SIM/QMI/MBIM data & calls.
2. `[usb-04]` *(human-gate via `tools/usbc-deadman-r8q`)* Port GPL `max77705` `0x25` USBC/CCIC driver (or test with externally powered hub) for full USB host OTG VBUS + Type-C role reporting.
3. `[pwr-03]` & `[pwr-04]` *(human-gate w/ watchdog)* Scope CPUfreq `schedutil` trace and deeper `cpuidle` / UFS PM.
4. `[slp-02]` & `[slp-03]` *(human-gate w/ watchdog)* Attended single `s2idle` wake, then soak 10 sleep cycles before removing `10-r8q-no-suspend.conf`.
5. `[cam-05]` *(human-gate)* Implement mainline V4L2 subdev drivers & DT nodes for IMX555 (rear wide) and IMX616 (front) from clean GPL/mainline specs.

### Kernel Upstream Submission (`4` tasks — `CONTRIBUTING.md` §6.2)
6. `[krn-02]` *(human-only DCO)* Human maintainer adds `Signed-off-by: Santiago Villavicencio` to all **42** unsigned commits in `~/omamobile-kernel/wt-r8q` (`origin/omamobile..HEAD`).
7. `[krn-03]` -> `[krn-05]` Split `wt-r8q` into clean subsystem topic branches (drop revert noise, gate shared drivers cleanly behind `samsung,r8q` match data), pass `tools/kernel-pr-check --build --merge origin/omamobile`, and open PRs against `linux-omarchy-mobile/omamobile`.

---

## Historical Notes Cards (Archive)
The detailed bring-up logs from earlier gates stay preserved in [`missing/`](missing/) for provenance:
- [01 Native DT, Touch & BT Commits](missing/01-flash-native-dt-recovery-1324fe066.md)
- [02 Gate C: Remoteprocs & Audio](missing/02-gate-c-remoteprocs-and-audio.md)
- [03 Wi-Fi, Bluetooth & Modem AMSS](missing/03-wifi-bluetooth-and-modem.md)
- [04 Sensors, Cameras, DPMS & Installer](missing/04-sensors-cameras-dpms-and-installer.md)
- [05 Soak, Pseudo-NMI & AI Benchmarks](missing/05-soak-pseudonmi-and-ai-benchmarks.md)

Back to [Board Overview](README.md).
