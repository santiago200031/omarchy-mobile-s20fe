<!-- SPDX-License-Identifier: MIT -->

# On-Device AI Benchmarks — Samsung Galaxy S20 FE (r8q)

**Status:** Pending GPU devfreq tuning & benchmark execution
**Last Updated:** 2026-11-17

---

## 1. Hardware Context

| Component | Details |
|-----------|---------|
| SoC | Qualcomm Snapdragon 865 (SM8250) |
| GPU | Adreno 650 (`3d00000.gpu`, Freedreno) |
| Live GPU devfreq (`2026-11-17`) | `cur_freq=305000000`, `governor=simple_ondemand`, `available_frequencies=305000000 400000000 441600000 490000000 525000000 587000000` |
| Remoteprocs (`2026-11-17`) | `slpi` (sensors), `cdsp` (compute DSP), `adsp` (audio DSP) — all `running` |

---

## 2. Existing On-Device AI Tooling in This Repo

- `tools/r8q-ai`: On-demand local AI controller over USB SSH (`172.16.42.1`).
  Supported subcommands: `status`, `models`, `select <model.gguf>`, `query "<prompt>"`, `ask "<prompt>"`, `stop`, `install`, `push-model <local.gguf>`.
- `packages/omamobile-device-r8q/rust/ai-server/`: On-device AI server package source.
- `packages/hexagonrpc`: SLPI FastRPC reverse-tunnel daemon (`hexagonrpc` `0.5.0-2` installed for SLPI sensors).

---

## 3. Benchmark & Validation Plan

### 3.1 Pre-Flight Check (Read-Only)

```bash
# Check remoteproc states
tools/phone-ssh 'for r in /sys/class/remoteproc/remoteproc*; do echo "$(basename $r): $(cat $r/name) = $(cat $r/state)"; done'

# Check GPU devfreq and render node
tools/phone-ssh 'ls -la /dev/dri/renderD128 && cat /sys/class/devfreq/3d00000.gpu/{cur_freq,available_frequencies,governor}'

# Check battery status (r8q fuel gauge is max170xx_battery, not BAT0)
tools/phone-ssh 'cat /sys/class/power_supply/max170xx_battery/{capacity,status,voltage_now}'

# Check on-device AI setup
tools/r8q-ai status
```

### 3.2 Benchmark Items Pending

1. **Freedreno / Adreno 650 devfreq scaling:** Confirm `3d00000.gpu` scales cleanly across `305000000..587000000` Hz under load without GPU hangchecks or RPMh timeouts.
2. **GGML Vulkan vs CPU inference (`LFM3-1.7B` / local GGUF via `tools/r8q-ai`):** Push a test model with `tools/r8q-ai push-model <model.gguf>`, measure tok/s on CPU and Vulkan (`vulkaninfo`), and watch `dmesg` for GPU/RPMh errors.
3. **Adreno SoftISP GPU path (`libcamera`):** Re-test removing `LIBCAMERA_SOFTISP_MODE=cpu` (`packages/omamobile-device-r8q/70-r8q-libcamera.conf`) only if the `SWIspWorker` GPU hangcheck lockup is resolved cleanly.

---

## 4. Status

| Item | Status | Notes |
|------|--------|-------|
| `tools/r8q-ai` + `ai-server` | ✅ In repo | Not yet benchmarked with a model on r8q |
| Adreno 650 devfreq perf tuning | ⏳ Pending | Currently resting at 305 MHz (`simple_ondemand`) |
| `GGML_VULKAN` + CPU `LFM3-1.7B` run | ⏳ Pending | Needs M2.7 soak clean first |
| `libcamera` EGL debayering | 🔴 Blocked | Hits `SWIspWorker` GPU hangcheck; CPU SoftISP active |
