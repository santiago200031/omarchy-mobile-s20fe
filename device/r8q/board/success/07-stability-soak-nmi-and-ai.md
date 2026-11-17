<!-- SPDX-License-Identifier: MIT -->

# Success 07: Stability Soak, Pseudo-NMI Watchdog & On-Device AI

- **Status:** Verified on live SM-G780G (`2026-11-17`) under `7.3.0-rc6-omamobile-push1-g3667ab368ef9` (the image pinned in [`stable-image.conf`](../../stable-image.conf)), `omamobile-ai` `0.1.0-1`.
- **Back:** [SUCCESS.md](../SUCCESS.md) · [Board Overview](../README.md)

## What Works
- **30-minute GUI + writeback soak (`M2.7`):** `tools/r8q-stability-soak` completed `1800 s` over 6 iterations with `failures=0` on `gf3a5609d121b`, with the desktop, storage writeback and radios active; no RCU stall, RPMh hang or watchdog.
- **Hardlockup / pseudo-NMI (`M2.8`):** `CONFIG_ARM64_PSEUDO_NMI=y` booted with `irqchip.gicv3_pseudo_nmi=1`; `NMI watchdog: Enabled` and IRQs-off backtraces work, with no GICv3 / Samsung-TZ regression.
- **On-device AI (`M8`, CPU):**
  - `omamobile-ai.service` owns `llama-server`: lazy start on the first request, idle unload after `IDLE_TIMEOUT=180`, `Restart=on-failure`, `ConditionPathExists=` on the GGUF so a model-less phone stays inactive instead of crash-looping.
  - Routes: `/health`, `/v1/models`, `/v1/chat/completions`, `/v1/unload`, plus the llama.cpp router surface (`/models`, `/props`, `/models/load`, …) so clients that validate a llama.cpp server accept the phone.
  - `LFM3-1.7B-Q4_K_M.gguf` (`1.1G`) pinned to the three Gold cores (`CPU_AFFINITY=4-6`, `THREADS=3`, `CTX_SIZE=32768`), CPU only (`GPU_LAYERS=0`).
  - Measured on the stable kernel: **15.45-15.47 prompt tok/s**, **11.13 warm-up / 10.56-11.01 generation tok/s** for 128 tokens, 46.3 °C → 74.1 °C over three back-to-back answers. Reproduce with `tools/r8q-ai ask`.
  - `tools/r8q-ai` drives the API at `http://172.16.42.1:8080` and falls back to the same API over `tools/phone-ssh`; `push-model` verifies the sha256 before selecting a model.
- **RAM behaviour:** the supervisor keeps ~1.5 GiB resident only while the model is loaded; `tools/r8q-ai unload` (or the idle timeout) returns it, leaving ~2.5 GiB available on the 5.5 GiB phone.

## Boundaries
- GPU inference is not enabled: `80-r8q-gpu.rules` caps the Adreno 650 at 305 MHz, and the installed `llama-server` is CPU-only. Vulkan `llama.cpp` and the 587 MHz rule remain optional work ([Missing 05](../missing/05-soak-pseudonmi-and-ai-benchmarks.md)).
- The dedicated NPU has no mainline driver; the Hexagon CDSP has no userspace runtime staged (see [AI-BENCHMARKS.md](../../AI-BENCHMARKS.md) §1).

## Verify
```sh
tools/r8q-stable-check --phone
OMAMOBILE_SSH_KEY=~/.ssh/omamobile-r8q tools/r8q-ai status
OMAMOBILE_SSH_KEY=~/.ssh/omamobile-r8q tools/r8q-ai ask "Reply with exactly: ok"
```
