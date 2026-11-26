<!-- SPDX-License-Identifier: MIT -->

# Column: Missing / Remaining Atomic Tasks (`r8q`)

Source of truth: [`tasks.json`](tasks.json) — check live percentages and next steps anytime with `tools/r8q-progress` (`--live`), or run `tools/r8q-check-all --quick` for a single read-only repo + worktree + phone health pass. Stock hardware findings live in [HARDWARE-DISCOVERY.md](../HARDWARE-DISCOVERY.md).

As of **`2026-11-26`** (`omamobile-device-r8q 0.1.39-1`, `omamobile-ai 0.1.2-1`, `tqftpserv 1.2-2`, rollback pin `recovery-stable-20261122.img`, `omamobile-settings 0.1.3-1`, `omamobile-omarchy 0.1.49-1`, `omamobile-shell 0.2.141-1`):
- **Overall (Hardware + Upstream):** `91.0%` (**61 / 67** atomic tasks)
- **Hardware Bring-Up (`CONTRIBUTING.md` §5.4 Stages 1–16):** `94.8%` (**55 / 58** atomic tasks)
- **Daily Core (Stages 1–8):** `100.0%` (**22 / 22** atomic tasks)
- **Upstream & Docs (Stages U1–U2):** `66.7%` (**6 / 9** atomic tasks)

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
| `10–11` | Power, Idle, Thermal & DVFS | `6/6` | `100.0%` | ⚠️ Mitigated | — (`[pwr-03]`, `[pwr-04]` root-caused/mitigated; `[pwr-05]` USB AI boost; `[pwr-06]` Wi-Fi PS `3`) |
| `12` | USB-C Role Switch & Charging Control | `3/4` | `75.0%` | ⚠️ Partial | `[usb-04]` OTG host 5V VBUS (`max77705` `0x25` USBC/CCIC driver or powered hub) |
| `13` | Suspend (`s2idle`) | `2/3` | `66.7%` | ⚠️ Partial | `[slp-03]` WFI `s2idle` `10/10` pass (`state1` TZ lockup; flash/unmask decision before removing `10-r8q-no-suspend.conf`) |
| `14` | Remoteprocs, Sensors & Audio (`ADSP`/`SLPI`/`CS35L41`/`DMIC3`) | `6/6` | `100.0%` | ✅ Achieved | — (speaker + earpiece + `DMIC3` `pw-record` verified) |
| `15a` | Cameras (`CAMSS` / `libcamera`) | `4/5` | `80.0%` | ⚠️ Partial | `[cam-05]` IMX555 (rear 1x) + IMX616 (front) drivers (`omacam` `0.5x <-> 3x` verified) |
| `15b` | Modem (`SDX55M`), GNSS & NFC | `6/6` | `100.0%` | ✅ Achieved | — (`[mdm-05]` SIM unlock, `ModemManager`, `mhi_hwip0` LTE data, `omamobile-device-r8q` `0.1.38` + `tqftpserv` `1.2-2` opt-in) |
| `16` | Fingerprint | `2/2` | `100.0%` | ⏸️ Deferred (TZ/QSEE) | — ([FINGERPRINT-PLAN.md](../FINGERPRINT-PLAN.md): deferred, XPU/QSEE/`qseecom` blockers) |
| `U1` | Kernel Branch Hygiene & Upstream DCO (`linux` / `wt-r8q-split`) | `2/5` | `40.0%` | 🔧 In Progress | `[krn-02]` human `Signed-off-by:` (64/74 branch commits across 7 topics), `[krn-04]` post-SoB `kernel-pr-check` + `ts-int-default-state` DT schema fix, `[krn-05]` upstream PRs |
| `U2` | Repo Registry & Stable Pin | `4/4` | `100.0%` | ✅ Achieved | — |

## Complete Step-by-Step Queue (`6` Remaining Atomic Tasks)

### High-Risk / Hardware Bring-Up (`3` tasks — human gate for flashes/reboots)
1. `[usb-04]` *(human-gate via `tools/usbc-deadman-r8q`)* Test USB host (OTG) role switch with external USB peripheral (`max77705` `0x25` USBC/CCIC driver or powered hub) and confirm peripheral role restores cleanly on unplug.
2. `[slp-03]` *(human-gate w/ watchdog)* Decide WFI `s2idle` shipping tradeoff vs `state1` TZ lockup (`POWER-SLEEP-PLAN.md`), re-verify 10-cycle soak on the target kernel (`10/10` SSH/DSI-1/touch/Wi-Fi), and remove `10-r8q-no-suspend.conf` if approved.
3. `[cam-05]` *(human-gate for CCI probe flash & GPL source review)* Complete IMX555 (rear wide 1x) and IMX616 (front) V4L2 drivers from clean GPL/mainline specs ([`CAMERA-PLAN.md`](../CAMERA-PLAN.md) §8.7).

### Kernel Upstream Submission (`3` tasks — `CONTRIBUTING.md` §6.2, [`UPSTREAM-KERNEL-PLAN.md`](../UPSTREAM-KERNEL-PLAN.md))
4. `[krn-02]` *(human-only DCO)* Human maintainer adds `Signed-off-by: Santiago Villavicencio` to all **64** unsigned commits across 7 of the 8 topic branches (`topic/r8q-audio` already passes 100%).
5. `[krn-04]` Drop `input-enable;` on `ts-int-default-state` (`sm8250-samsung-r8q.dts:558`, only `CHECK_DTBS` schema warning) with on-phone touch re-check, then re-run `tools/kernel-pr-check --build --merge origin/omamobile` on all signed topic branches (`dt_binding_check` already `5/5` clean).
6. `[krn-05]` *(human-gate)* Push signed topic branches to user kernel fork and open PRs against `derekross/linux-omarchy-mobile:omamobile`.

---

## Historical Notes Cards (Archive)
The detailed bring-up logs from earlier gates stay preserved in [`missing/`](missing/) for provenance:
- [01 Native DT, Touch & BT Commits](missing/01-flash-native-dt-recovery-1324fe066.md)
- [02 Gate C: Remoteprocs & Audio](missing/02-gate-c-remoteprocs-and-audio.md)
- [03 Wi-Fi, Bluetooth & Modem AMSS](missing/03-wifi-bluetooth-and-modem.md)
- [04 Sensors, Cameras, DPMS & Installer](missing/04-sensors-cameras-dpms-and-installer.md)
- [05 Soak, Pseudo-NMI & AI Benchmarks](missing/05-soak-pseudonmi-and-ai-benchmarks.md)

Back to [Board Overview](README.md).
