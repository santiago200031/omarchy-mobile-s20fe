<!-- SPDX-License-Identifier: MIT -->

# Column: Missing / Next Work Queue

Remaining bring-up gates and polish items ordered by priority.

| Priority | Card | Scope |
| --- | --- | --- |
| P1 | [01 Flash Native DT `1324fe066` + Touch `5866f774c` Recovery](missing/01-flash-native-dt-recovery-1324fe066.md) | Human `Signed-off-by:` in `wt-r8q` + optional flash of `recovery-omamobile.img` (`2769d7da...`) |
| P2 | [02 Gate C: Remoteprocs & Audio](missing/02-gate-c-remoteprocs-and-audio.md) | `remoteprocs.dtso` (`adsp`/`cdsp`/`slpi`) -> `audio-amps.dtso` (`cs35l41`) -> `audio-sound.dtso` (UCM2) |
| P3 | [03 Wi-Fi, Bluetooth & Modem](missing/03-wifi-bluetooth-and-modem.md) | `wlp1s0` (`ath11k` + ASPM), `qca6390` BT on `uart6`, cellular/modem after Wi-Fi+audio soak |
| P4 | [04 Sensors, Cameras & DPMS Wake](missing/04-sensors-cameras-dpms-and-installer.md) | SLPI sensors, cameras, `LockGlance.qml` wake/DPMS (`tools/r8q-install` aligned to `@60`, `vrr = 0`) |
| P5 | [05 Stability Soak, Pseudo-NMI & AI Bench](missing/05-soak-pseudonmi-and-ai-benchmarks.md) | 30-min GUI soak (`M2.7`), `CONFIG_ARM64_PSEUDO_NMI=y` (`M2.8`), `GGML_VULKAN` & `LFM3-1.7B` (`M8`) |

Back to [Board Overview](README.md).
