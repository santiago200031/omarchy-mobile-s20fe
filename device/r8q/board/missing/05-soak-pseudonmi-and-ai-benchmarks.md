<!-- SPDX-License-Identifier: MIT -->

# Missing 05: Freedreno Above 305 MHz & Vulkan LLM (Optional)

- **Priority:** P5 (`M8`), optional performance work.
- **Status (`2026-11-18`):** the soak, pseudo-NMI and CPU AI parts of this card are **done** and recorded in [Success 07](../success/07-stability-soak-nmi-and-ai.md). The Mesa side of Freedreno/Turnip is **done**: `packages/mesa` `1:26.2.4-2.0` builds Gallium freedreno (`msm_dri.so`) and splits `vulkan-freedreno` (`libvulkan_freedreno.so`, `freedreno_icd.json`), validated by a full aarch64 build (signed, indexed). What remains is hardware: installing the packages on r8q, the 305 -> 587 MHz GPU step, and the Vulkan `llama.cpp` run.
- **Back:** [MISSING.md](../MISSING.md) · [Board Overview](../README.md)

## Done (moved to Success 07)
1. **GUI + writeback soak (`M2.7`):** `tools/r8q-stability-soak`, `1800 s`, 6 iterations, `failures=0`.
2. **Pseudo-NMI (`M2.8`):** `CONFIG_ARM64_PSEUDO_NMI=y` with `irqchip.gicv3_pseudo_nmi=1`; `NMI watchdog: Enabled`, no GICv3/TZ regression.
3. **CPU AI baseline (`M8`):** `omamobile-ai` + `LFM3-1.7B`, 15.47 prompt / 10.6-11.0 generation tok/s on the stable kernel ([AI-BENCHMARKS.md](../../AI-BENCHMARKS.md) §2).

## Remaining Work

### 1. Freedreno above 305 MHz
[`80-r8q-gpu.rules`](../../../../packages/omamobile-device-r8q/80-r8q-gpu.rules) caps
`max_freq=305000000` at boot. A transient Vulkan FP32 FMA benchmark measured
**245.3 GFLOP/s at 587 MHz vs 127.6 GFLOP/s at 305 MHz (1.92×)** with no
GMU/RPMh/GPU errors, but that was a short burst, not a sustained desktop or
inference load.

- Raise the cap in one step (`305 → 587 MHz`), keep `power/control=on`, and run
  the 30-minute soak plus a sustained Vulkan workload.
- Watch `dmesg` for GMU faults, RPMh timeouts, GPU hangs and thermal throttling;
  record the hottest zone and the achieved frequency.
- Keep `305 MHz` as the default until the sustained run is clean.

### 2. Vulkan `llama.cpp`
- Install `extra/vulkan-headers` (`shaderc` and `vulkan-freedreno` are already
  present), build `llama.cpp` with `-DGGML_VULKAN=ON`, and confirm
  `llama-server --list-devices` shows the Adreno 650 (the installed build is
  CPU-only).
- Benchmark `LFM3-1.7B` against the CPU baseline (15.47 prompt / 10.6-11.0
  generation tok/s) at 305 MHz and 587 MHz, and record the numbers in
  [AI-BENCHMARKS.md](../../AI-BENCHMARKS.md).
- Set `GPU_LAYERS` in `/etc/omamobile-ai.conf.d/` only for a build that passes;
  keep `GPU_LAYERS=0` as the default.

### 3. Hexagon CDSP (optional)
`/dev/fastrpc-cdsp` reports Hexagon v66, `UNSIGNED_PD_SUPPORT=1`, 4×128B HVX and
256 KiB VTCM, but no `libcdsprpc.so` or Hexagon ELF runtime is staged. Offloading
quantized ops would need that userspace stack first; it is not on the critical
path for the CPU baseline.
