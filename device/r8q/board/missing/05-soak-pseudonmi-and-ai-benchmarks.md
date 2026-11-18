<!-- SPDX-License-Identifier: MIT -->

# Missing 05: Freedreno Above 305 MHz & Vulkan LLM (Optional)

- **Priority:** P5 (`M8`), optional performance work.
- **Status (`2026-11-19`):** the soak, pseudo-NMI and CPU AI parts of this card are **done** and recorded in [Success 07](../success/07-stability-soak-nmi-and-ai.md). The CPU backend is now packaged (`omamobile-llama` `1.0.0.20261118.98d67c8-1`) and measures **15.3 generation tok/s / 35.2 pp512 tok/s** on 2026-11-18 ([AI-BENCHMARKS.md](../../AI-BENCHMARKS.md) §2.1). The Mesa side of Freedreno/Turnip is **done**: `packages/mesa` `1:26.2.4-2.0` builds Gallium freedreno (`msm_dri.so`) and splits `vulkan-freedreno` (`libvulkan_freedreno.so`, `freedreno_icd.json`), validated by a full aarch64 build (signed, indexed) and **on the phone**: `vulkaninfo` shows `Turnip Adreno (TM) 650` (Vulkan 1.3.354) and `eglinfo` shows OpenGL 4.6 on `freedreno`, with the Turnip ICD working even beside Arch's newer `mesa 26.2.7`. The 305 -> 587 MHz GPU step is now **done** (sustained 587 MHz, `omamobile-device-r8q` 0.1.26). The Vulkan `llama.cpp` build is **done but blocked**: it builds and sees the Adreno 650, but the Vulkan compute/LLM path wedges the GPU (see [Failed 08](../failed/08-vulkan-llm-adreno-gpu-wedge.md)), so CPU stays the default. What remains: the optional Hexagon CDSP offload ([NPU-PLAN.md](../../NPU-PLAN.md), private fork), and a Turnip/GMU fix before Vulkan LLM can be retried.
- **Back:** [MISSING.md](../MISSING.md) · [Board Overview](../README.md)

## Done (moved to Success 07)
1. **GUI + writeback soak (`M2.7`):** `tools/r8q-stability-soak`, `1800 s`, 6 iterations, `failures=0`.
2. **Pseudo-NMI (`M2.8`):** `CONFIG_ARM64_PSEUDO_NMI=y` with `irqchip.gicv3_pseudo_nmi=1`; `NMI watchdog: Enabled`, no GICv3/TZ regression.
3. **CPU AI baseline (`M8`):** `omamobile-ai` + `LFM3-1.7B`; the backend is the packaged `omamobile-llama` `1.0.0.20261118.98d67c8-1`, measured on 2026-11-18 at **pp512 35.18 tok/s / tg128 15.70 tok/s** and **15.26-15.29 generation tok/s** through the supervisor ([AI-BENCHMARKS.md](../../AI-BENCHMARKS.md) §2.1).

## Done (2026-11-19)

### Freedreno above 305 MHz — adopted 587 MHz as the default cap
The transient Vulkan FP32 FMA benchmark (245.3 GFLOP/s at 587 MHz vs 127.6 at 305, 1.92×) was a short burst, so the cap was raised with a sustained run:

- `max_freq` lifted live to `587000000`, governor `userspace` pinned at 587 MHz (the devfreq only offers `userspace`/`simple_ondemand`; there is no `performance`), a real Vulkan workload (`vkcube` on Turnip, render fds confirmed) running on the panel, `power/control=on` on the GPU and GMU.
- **~33 minutes sustained at 587 MHz** (20 min + 13 min `vkcube`, `12:29`–`13:03`), sampled every 15 s.
- **Result: clean.** Frequency held `587 MHz` for the whole run; no GMU faults, no RPMh timeouts, no GPU hangs, no thermal trips. GPU zones `gpu-top` 40-43 °C / `gpu-bottom` 41-43 °C; hottest of all 28 zones peaked at **49 °C** (single sample) and sat at **44-45 °C** — large thermal margin.
- On completion the governor was restored to `simple_ondemand`; the device now scales `305-587 MHz` and idles at 305 (no load).
- `80-r8q-gpu.rules` now sets `min_freq=305000000`, `max_freq=587000000` (min kept at 305 so it scales down; max is the cap, not a pin). `omamobile-device-r8q` bumped to **0.1.26**; the rule was pushed to the phone's `/usr/lib/udev/rules.d/` and re-triggered, so it survives a reboot.
- Logs: `logs/r8q/gpu587-sample.log`, `logs/r8q/gpu587-sample2.log`, `logs/r8q/gpu587-vkcube.log`, `logs/r8q/gpu587-vkcube2.log`, `logs/r8q/gpu587-dmesg.log` (gitignored).

## Remaining Work

### 2. Vulkan `llama.cpp` — **built, but wedges the GPU** (see [Failed 08](../failed/08-vulkan-llm-adreno-gpu-wedge.md))
- Done: `vulkan-headers` + `spirv-headers` installed, `llama.cpp` `1.0.0` built on the phone with `-DGGML_VULKAN=ON` (make, `-j8`, ~1 min), `llama-server --list-devices` reports `Vulkan0: Turnip Adreno (TM) 650 (4160 MiB)`.
- **Blocker:** `llama-bench -ngl 99` on `LFM3-1.7B-Q4_K_M` faults at model load, even pinned at `305 MHz`: `device lost on Vulkan0` (`waitSemaphores: ErrorDeviceLost`), then `hangcheck detected gpu lockup rb 0` + `a6xx_recover: cx gdsc didn't collapse`. The GPU wedges, and `Hyprland` was taken down with it; it needed a reboot. The Vulkan *compute/LLM* path wedges; the *render* path (vkcube, Hyprland compositing) is fine, even at 587 MHz.
- **So:** keep `GPU_LAYERS=0` (CPU) as the default; no GPU benchmark numbers until the Turnip/GMU recovery path is fixed and a Vulkan LLM run is clean.

### 3. Hexagon CDSP (optional)
`/dev/fastrpc-cdsp` reports Hexagon v66, `UNSIGNED_PD_SUPPORT=1`, 4×128B HVX and
256 KiB VTCM, but no `libcdsprpc.so` or Hexagon ELF runtime is staged. Offloading
quantized ops would need that userspace stack first; it is not on the critical
path for the CPU baseline.
