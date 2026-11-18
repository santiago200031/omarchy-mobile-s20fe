<!-- SPDX-License-Identifier: MIT -->

# Failed 08: Vulkan `llama.cpp` Wedges the Adreno 650

- **Date:** `2026-11-18`. Kernel `7.3.0-rc6-omamobile-push1-g3667ab368ef9` (stable), `vulkan-freedreno 1:26.2.4-2.0` (Turnip), `llama.cpp` `1.0.0` (main) built on the phone with `-DGGML_VULKAN=ON`.
- **Back:** [FAILED.md](../FAILED.md) · [Board Overview](../README.md)

## What was tried

The Vulkan half of board card 05: build `llama.cpp` with `-DGGML_VULKAN=ON`
(confirmed `llama-server --list-devices` reports `Vulkan0: Turnip Adreno (TM)
650 (4160 MiB)`), then benchmark `LFM3-1.7B-Q4_K_M` with all layers on the GPU
(`llama-bench -ngl 99 -p 512 -n 128`) at a pinned `305 MHz`.

## What happened

Model load **locked up the GPU**:

- `ggml_vulkan: device lost on Vulkan0` → `vk::Device::waitSemaphores: ErrorDeviceLost` during `llama_model_load`.
- Kernel: `msm_dpu … hangcheck detected gpu lockup rb 0!` with the submitted fence stuck (e.g. `completed fence: 5917` / `submitted fence: 5923`).
- `adreno 3d00000.gpu: [drm:a6xx_recover [msm]] *ERROR* cx gdsc didn't collapse` — the GMU **CX power-domain** recovery fails to cycle, so the kernel cannot reset the GPU in place. The `recover_worker` loops.
- The wedge persisted: `vkcube` (which ran fine at 587 MHz before) then **segfaulted** (`TU: warning: Failed to set BO name with DRM_MSM_GEM_INFO: -2` → core dump), and the hangcheck started flagging **`Hyprland`** as the offending task (`offending task: Hyprland (Hyprland --watchdog-fd 4)`), taking the graphical session down (`hyprland` + `omamobile-shell` inactive).
- Recovery required a **reboot**; the GPU could not be un-wedged live.

## Root-cause read

Not a frequency problem: it faulted at the lowest OPP (`305 MHz`), during the
first real Vulkan compute/memory pass (loading a 1.7 B Q4_K_M model and the
first matmul), not under a trivial render (vkcube). The `cx gdsc didn't
collapse` points at the GMU CX power domain refusing to collapse on recovery —
the same hangcheck/GMU class the card 04 "Adreno `debayer_egl` hangcheck
follow-up" tracks. On this Mesa/Turnip build the Vulkan compute path that
`llama.cpp` uses drives the Adreno 650 into a lockup that the a6xx recovery
path cannot clear.

## Permanent rule

- **Keep `GPU_LAYERS=0` (CPU) as the default** in `/etc/omamobile-ai.conf.d/`; do not set GPU layers for `llama.cpp` until the Turnip/GMU recovery path is fixed and a Vulkan LLM run is clean.
- Treat any `hangcheck detected gpu lockup` + `cx gdsc didn't collapse` as a **wedge that needs a reboot**, not something to recover live. If a Vulkan workload wedges the GPU, stop, record it, and reboot before further GPU work.
- The Vulkan *render* path (vkcube / Hyprland compositing) is stable at 587 MHz; it is specifically the **Vulkan compute/LLM** path that wedges. Don't conflate the two in a benchmark.

## Evidence

`/root/llama/bench305.txt` (the `ErrorDeviceLost` model-load failure), the
`dmesg` hangcheck/`a6xx_recover` lines, and the subsequent `vkcube` segfault on
the same boot. Build source kept at `/root/llama/llama-src` (build tree
`build/`, `bin/llama-server` reports the Adreno 650).
