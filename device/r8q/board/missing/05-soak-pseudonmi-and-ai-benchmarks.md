<!-- SPDX-License-Identifier: MIT -->

# Missing 05: Stability Soak, Pseudo-NMI Watchdog & On-Device AI

- **Priority:** P5 (`M2.7`, `M2.8`, `M8`).
- **Back:** [MISSING.md](../MISSING.md) · [Board Overview](../README.md)

## Remaining Work
1. **Active GUI + Writeback Soak (`M2.7`):** Run a 30-min interactive desktop + storage writeback soak on `main` (`0.1.16` + `1324fe066`) with zero RCU stalls, RPMh hangs, or watchdogs.
2. **Hardlockup / Pseudo-NMI Evaluation (`M2.8`):** Test `CONFIG_ARM64_PSEUDO_NMI=y` alone on an experimental boot to capture IRQs-off hardlockup backtraces without breaking GICv3 on Samsung TZ.
3. **Freedreno Performance & On-Device AI (`M8`):** Re-evaluate GPU devfreq above `305 MHz` and compositor blur/animations once thermally/stability-proven, then benchmark Vulkan (`GGML_VULKAN`) and `LFM3-1.7B-Q4_K_M` CPU (`omamobile-ai-server`) on desktop.
