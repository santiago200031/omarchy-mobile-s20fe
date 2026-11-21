<!-- SPDX-License-Identifier: MIT -->

# Column: Missing / Remaining Atomic Tasks (`r8q`)

Source of truth: [`tasks.json`](tasks.json) — check live percentages and next steps anytime with `tools/r8q-progress` (`--live` queries `~/omamobile-kernel/wt-r8q` + connected phone read-only).

As of **`2026-11-21`** (`7.3.0-rc6-omamobile-push1-gae195e6deaac`, `main` `8c95bd95`):
- **Overall (Hardware + Upstream):** `43.1%` (**28 / 65** atomic tasks)
- **Hardware Bring-Up (`CONTRIBUTING.md` §5.4 Stages 1–16):** `46.4%` (**26 / 56** atomic tasks)
- **Daily Core (Stages 1–8):** `77.3%` (**17 / 22** atomic tasks)
- **Upstream & Docs (Stages U1–U2):** `22.2%` (**2 / 9** atomic tasks)

## Stage-by-Stage Milestones (`CONTRIBUTING.md` §5.4)

| Stage | Subsystem | Done | % | Status | Next Atomic Gap |
| --- | --- | --- | --- | --- | --- |
| `1–2` | Boot, USB gadget, SSH, Native root & Storage guard | `5/5` | `100%` | ✅ Achieved | — |
| `3` | Touch & Buttons (`stmfts`, `gpio-keys`) | `3/3` | `100%` | ✅ Achieved | — |
| `4` | GPU (`Adreno 650` / Vulkan & recovery) | `2/5` | `40%` | ⚠️ Open polish | `[gpu-03]` `-p 512` bisect; `[gpu-04]` DPU post-recovery restore; `[gpu-05]` GPU layers bench |
| `5` | Display (`S6E3FC3 120 Hz`) & Shell UI | `3/4` | `75%` | ✅ Achieved | `[dsp-04]` deploy `omamobile-settings 0.1.3` to phone (`0.1.2-1` live) |
| `6` | Battery & Fuel Gauge (`max77705` / `max17042`) | `2/2` | `100%` | ✅ Achieved | — |
| `7–8` | Wi-Fi & Bluetooth (`QCA6390`) | `2/3` | `67%` | ✅ Achieved | `[net-03]` BT A2DP audio + Wi-Fi concurrency test |
| `9` | Haptics & Torch | `0/4` | `0%` | ❌ Missing | `[hap-01..04]` `live.dts` audit -> `disabled` DT nodes -> safe single-step tests |
| `10–11` | Power, Idle, Thermal & DVFS | `1/4` | `25%` | ⚠️ Mitigated | `[pwr-02..04]` WFI power baseline, scoped CPUfreq `schedutil` trace, deeper `cpuidle` / UFS PM |
| `12` | USB-C Role Switch & Charging Control | `1/4` | `25%` | ⚠️ Partial | `[usb-02..04]` MUIC/TCPC `live.dts`, replug recovery + OTG host under `usbc-deadman` |
| `13` | Suspend (`s2idle`) | `0/3` | `0%` | ❌ Disabled | `[slp-01..03]` wake IRQ audit, attended single `s2idle` w/ watchdog, 10-cycle soak |
| `14` | Remoteprocs, Sensors & Audio (`ADSP`/`SLPI`/`CS35L41`) | `3/6` | `50%` | ⚠️ Partial (playback ✅) | `[aud-04..06]` `qcom,dmic-sample-rate` DT prop, mic record + earpiece UCM2, L/R speaker balance |
| `15a` | Cameras (`CAMSS` / `libcamera`) | `2/5` | `40%` | ❌ Regressed | `[cam-03..05]` fix `port@1` `s5k3l6_out` notifier deadlock, `omacam` tele/uw verify, IMX555/IMX616 drivers |
| `15b` | Modem (`SDX55M`), GNSS & NFC | `2/6` | `33%` | ⚠️ Partial | `[mdm-03..06]` native `sda36` `modem.dtso` boot test, NV re-check, opt-in SIM/QMI/MBIM, GNSS/NFC audit |
| `16` | Fingerprint | `0/2` | `0%` | ❌ Missing | `[fp-01..02]` `live.dts` SPI/TA audit and feasibility note |
| `U1` | Kernel Branch Hygiene & Upstream DCO (`wt-r8q`) | `1/5` | `20%` | 🔧 Open | `[krn-02..05]` human `Signed-off-by:` on 38 commits, split topic branches, `kernel-pr-check`, upstream PRs |
| `U2` | Repo Registry & Stable Pin | `1/4` | `25%` | 🔧 Open | `[doc-02..04]` `CONTRIBUTING.md` §3.1 row, `docs/DEVICES.md` update, re-pin `stable-image.conf` after camera/DMIC fix |

## Complete Step-by-Step Queue (Exact Execution Order)

### Track A — Immediate Safe Software / Userspace Tasks (No Reboot Required)
1. `[dsp-04]` Build and install `omamobile-settings 0.1.3` onto phone (currently running `0.1.2-1`; exposes upstream Battery + Display pages).
2. `[aud-05]` Verify internal mic capture (`arecord`/`pw-record`) and add earpiece/headset UCM2 profiles alongside `-9.5 dB` `HiFi.conf`.
3. `[aud-06]` Verify left/right (top+bottom) `cs35l41` speaker balance at comfortable low evening volume.
4. `[net-03]` Stream A2DP Bluetooth audio while transferring over `wlp1s0`.
5. `[gpu-03]` Bisect Vulkan `-p 512` compute stall (`llama-bench -ngl 99 -p 64/128/256/512`) and log chunk threshold (`gpu-05` follows if fixed).
6. `[pwr-02]` Record baseline awake/screen-off battery current draw on CPU WFI baseline.
7. `[doc-02]` & `[doc-03]` Add `r8q` single-slot entry (`sda24` recovery, Download Mode buttons) to `CONTRIBUTING.md` §3.1 and sync `docs/DEVICES.md`.

### Track B — Kernel DT & Human-Gated Boot Tasks (`AGENTS.md` §1–2)
8. `[cam-03]` *(human-gate)* Fix `camss` `v4l2-async` deadlock (`port@1` `s5k3l6_out` in `sm8250-samsung-r8q.dts`) so `/dev/video*` binds cleanly and `cam -l` lists sensors -> `[cam-04]` re-verify `libcamera`/`omacam`.
9. `[aud-04]` *(human-gate)* Add `qcom,dmic-sample-rate` (and VA macro DT properties) in `sm8250-samsung-r8q.dts` to clear `va_macro` clock warning.
10. `[gpu-04]` *(human-gate)* Verify/restore DPU scanout after GPU fault recovery if display freezes.
11. `[doc-04]` *(human-gate)* Pack, verify with `tools/phone-smoke`, and re-pin `device/r8q/stable-image.conf`.

### Track C — Remaining Hardware Subsystems (`CONTRIBUTING.md` §5.4)
12. `[hap-01]` -> `[hap-04]` *(DT audits first, human-gated overlay boots second)* Bring up haptics actuator and rear flash/torch LED.
13. `[usb-02]` -> `[usb-04]` *(human-gate via `tools/usbc-deadman`)* Type-C PD / replug gadget recovery / OTG host role-switch.
14. `[mdm-03]` -> `[mdm-05]` *(human-gate + NV checksum check)* Boot `modem.dtso` on native `sda36` root (verifying no microSD-era brownout), re-check 15/15 NV checksums, test SIM/QMI/MBIM data & calls in opt-in service; `[mdm-06]` audit GNSS & NFC.
15. `[pwr-03..04]` & `[slp-01..03]` *(human-gate w/ watchdog)* Audit wake IRQs, test single attended `s2idle` wake, and scope CPUfreq `schedutil` / deeper `cpuidle` / UFS PM.
16. `[cam-05]` & `[fp-01..02]` Reverse/upstream IMX555 + IMX616 sensor drivers and document fingerprint SPI/TZ feasibility.

### Track D — Kernel Upstream Submission (`CONTRIBUTING.md` §6.2)
17. `[krn-02]` *(human-only DCO)* Human maintainer adds `Signed-off-by: Santiago Villavicencio` to the 38 unsigned commits in `~/omamobile-kernel/wt-r8q` (`omamobile..HEAD`).
18. `[krn-03..05]` Split `wt-r8q` into clean subsystem topic branches, pass `tools/kernel-pr-check --build --merge origin/omamobile`, and open PRs against `linux-omarchy-mobile/omamobile`.

---

## Historical Notes Cards (Archive)
The detailed bring-up logs from earlier gates stay preserved in [`missing/`](missing/) for provenance:
- [01 Native DT, Touch & BT Commits](missing/01-flash-native-dt-recovery-1324fe066.md)
- [02 Gate C: Remoteprocs & Audio](missing/02-gate-c-remoteprocs-and-audio.md)
- [03 Wi-Fi, Bluetooth & Modem AMSS](missing/03-wifi-bluetooth-and-modem.md)
- [04 Sensors, Cameras, DPMS & Installer](missing/04-sensors-cameras-dpms-and-installer.md)
- [05 Soak, Pseudo-NMI & AI Benchmarks](missing/05-soak-pseudonmi-and-ai-benchmarks.md)

Back to [Board Overview](README.md).
