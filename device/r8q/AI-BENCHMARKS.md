<!-- SPDX-License-Identifier: MIT -->

# On-Device AI & Accelerator Probe — Samsung Galaxy S20 FE (r8q)

**Status:** GPU Vulkan 1.3 + 587 MHz scaling + CPU `LFM3-1.7B` baseline verified (`2026-11-17`); Vulkan LLM build optional
**Last Updated:** 2026-11-17

---

## 1. Verified Accelerator & Runtime State (`2026-11-17`, `gf3a5609d121b`)

| Accelerator / Runtime | Status | Verified Details |
|---|---|---|
| **Adreno 650 GPU (`3d00000.gpu`)** | ✅ Usable | `Turnip Adreno (TM) 650` (`0x5143:0x6050002`), **Vulkan 1.3.354**, 4160.0 MiB `DEVICE_LOCAL` heap, compute subgroup `128`, `shaderFloat16=1`, `shaderInt8=1`. |
| **GPU Devfreq (`simple_ondemand`)** | ✅ Tested up to 587 MHz | [80-r8q-gpu.rules](../../packages/omamobile-device-r8q/80-r8q-gpu.rules) caps `max_freq=305000000` at boot. Transient Vulkan FP32 FMA benchmark measured **`127.6 GFLOP/s` (`0.168 s`) at `305 MHz`** vs **`245.3 GFLOP/s` (`0.088 s`, 1.92×) at `587 MHz`** with zero GMU/RPMh/GPU errors. |
| **Dedicated NPU (`npu@9910000`)** | ❌ Not usable | Mainline Linux has no Qualcomm SM8250 NPU kernel driver (`npu_mem` is deleted in `sm8250-samsung-common.dtsi`). |
| **Hexagon CDSP (`8300000.remoteproc`)** | ⏳ HW live, userspace missing | `remoteproc1` (`cdsp.mbn`) running; `/dev/fastrpc-cdsp` `GET_DSP_INFO` reports **Hexagon v66 (`0x8f66`)**, **`UNSIGNED_PD_SUPPORT=1`**, **`4× 128B HVX`**, **`256 KiB VTCM`** (`HMX=0`). No `libcdsprpc.so` or Hexagon v66 ELF runtime staged yet. |
| **`omamobile-ai` (`:8080`, CPU)** | ✅ Baseline measured | `LFM3-1.7B-Q4_K_M.gguf` (`1.1G`, `THREADS=3`, `CPU_AFFINITY=4-6`, `GPU_LAYERS=0`): **`15.34 tok/s` prompt / `11.24 tok/s` generation**. Installed `/usr/local/bin/llama-server` is CPU-only (`--list-devices`: `(none)`). |

---

## 2. Existing On-Device AI Tooling in This Repo

- `tools/r8q-ai`: On-demand local AI controller over USB SSH (`172.16.42.1`).
  Subcommands: `status`, `models`, `select <model.gguf>`, `query "<prompt>"`, `ask "<prompt>"`, `stop`, `install`, `push-model <local.gguf>`.
- `omamobile-ai` (`0.1.0-1` on `omamobile-ai` branch): Lazy-spawning supervisor on `:8080` configured via `/etc/omamobile-ai.conf`.
- `packages/omamobile-device-r8q/rust/ai-server/`: Stats stub (`18081`).

---

## 3. Optional Future Work

1. **Vulkan LLM (`GGML_VULKAN=ON`) & `587 MHz` GPU rule:** Install `extra/vulkan-headers` (`shaderc` and `vulkan-freedreno` are already installed), build `llama.cpp` with `-DGGML_VULKAN=ON`, uncap `80-r8q-gpu.rules` `max_freq` from `305000000` to `587000000`, and re-benchmark `LFM3-1.7B` vs `11.24 tok/s` CPU.
2. **Adreno `libcamera` SoftISP (`debayer_egl`):** Re-test removing `LIBCAMERA_SOFTISP_MODE=cpu` (`packages/omamobile-device-r8q/70-r8q-libcamera.conf`) against `587 MHz` / `SWIspWorker` hangcheck debugging.
3. **Hexagon CDSP Compute:** Stage `libcdsprpc` + Hexagon v66 HVX runtime/kernel ELF (`UNSIGNED_PD_SUPPORT=1`) if offloading quantized ops to CDSP.
