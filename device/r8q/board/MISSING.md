<!-- SPDX-License-Identifier: MIT -->

# Column: Missing / Next Work Queue

Last reviewed `2026-11-17` against the stable image
(`7.3.0-rc6-omamobile-push1-g3667ab368ef9`, [`stable-image.conf`](../stable-image.conf)).

| Priority | Card | Scope |
| --- | --- | --- |
| P1 | [04 Sensors, Cameras & DPMS](missing/04-sensors-cameras-dpms-and-installer.md) | **Main/front cameras only:** `IMX555` rear wide and `IMX616` front (tele `HI-847` and ultrawide `S5K3L6` are verified); Adreno `debayer_egl` hangcheck follow-up. |
| P2 | [03 Cellular / Modem](missing/03-wifi-bluetooth-and-modem.md) | **Parked.** Stage 2 (`mhi0` BHI ready) and Stage 3A (`PCIe2` MSI) are done offline; Stage 3A resets the phone when `mhi_pci_generic` binds (`5be4efd5e`). A resume needs a different MSI/IOMMU or MHI design, then an attended gated test. [MODEM-PLAN](../MODEM-PLAN.md) §8. |
| P3 | [05 Stability Soak, Pseudo-NMI & AI](missing/05-soak-pseudonmi-and-ai-benchmarks.md) | **Optional:** GPU devfreq above `305 MHz` and a Vulkan (`GGML_VULKAN`) `llama.cpp` benchmark. The soak, pseudo-NMI and CPU AI baseline are done ([Success 07](../success/07-stability-soak-nmi-and-ai.md)). |
| Upstream | [01 Native DT & Touch Commits](missing/01-flash-native-dt-recovery-1324fe066.md) | The flash side is done (the stable image carries the native DT and touch). What remains is the human `Signed-off-by:` on `1324fe066`, `5866f774c` and `caa385175`, then the pull requests. |

## Closed since 2026-11-15

| Was | Card | Evidence |
| --- | --- | --- |
| P1 flash | [01](missing/01-flash-native-dt-recovery-1324fe066.md) | Stable image on `sda24` boots the native DT + touch; `tools/r8q-stable-check --phone` passes. Sign-off is the only remaining part (row above). |
| P2 audio/DSP | [02](missing/02-gate-c-remoteprocs-and-audio.md) | ADSP/CDSP/SLPI up, both `cs35l41` amplifiers live with `r8q` speaker protection, UCM2 stereo verified. |
| P3 wireless | [03](missing/03-wifi-bluetooth-and-modem.md) | `wlp1s0` and `hci0` verified; 30-minute GUI/writeback/radio soak passed with `failures=0`. |
| P4 sensors/DPMS | [04](missing/04-sensors-cameras-dpms-and-installer.md) | SLPI sensors via `iio-sensor-proxy`, DPMS + physical Power-key wake, HI-847 telephoto, S5K3L6 ultrawide. |
| P5 soak/NMI/AI | [05](missing/05-soak-pseudonmi-and-ai-benchmarks.md) | Soak `1800 s` / 6 iterations / `failures=0`; `NMI watchdog: Enabled`; `omamobile-ai` serving `LFM3-1.7B` at 15.47 prompt / 10.6-11.0 generation tok/s. |

Back to [Board Overview](README.md).
