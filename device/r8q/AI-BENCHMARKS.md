<!-- SPDX-License-Identifier: MIT -->

# On-Device AI & Accelerator Probe — Samsung Galaxy S20 FE (r8q)

**Status:** `omamobile-ai` supervisor (CPU, `LFM3-1.7B`) verified end-to-end on the stable kernel `2026-11-17`; GPU Vulkan 1.3 and 587 MHz scaling measured; Vulkan LLM build optional
**Last Updated:** 2026-11-17

The supported design is the `omamobile-ai` supervisor package: one small Rust
process that owns `llama-server`, starts it lazily on the first request, proxies
the OpenAI and llama.cpp router surfaces, and unloads the model after
`IDLE_TIMEOUT`. `tools/r8q-ai` on the laptop drives that API. The old on-demand
`/usr/local/bin/omamobile-ai` script and the BSP `rust/ai-server/` stats stub are
gone.

---

## 1. Verified Accelerator & Runtime State

| Accelerator / Runtime | Status | Verified Details |
|---|---|---|
| **Adreno 650 GPU (`3d00000.gpu`)** | ✅ Usable | `Turnip Adreno (TM) 650` (`0x5143:0x6050002`), **Vulkan 1.3.354**, 4160.0 MiB `DEVICE_LOCAL` heap, compute subgroup `128`, `shaderFloat16=1`, `shaderInt8=1` (measured `2026-11-17` on `gf3a5609d121b`). |
| **GPU Devfreq (`simple_ondemand`)** | ✅ Tested up to 587 MHz | [80-r8q-gpu.rules](../../packages/omamobile-device-r8q/80-r8q-gpu.rules) caps `max_freq=305000000` at boot. Transient Vulkan FP32 FMA benchmark measured **`127.6 GFLOP/s` (`0.168 s`) at `305 MHz`** vs **`245.3 GFLOP/s` (`0.088 s`, 1.92×) at `587 MHz`** with zero GMU/RPMh/GPU errors. |
| **Dedicated NPU (`npu@9910000`)** | ❌ Not usable | Mainline Linux has no Qualcomm SM8250 NPU kernel driver (`npu_mem` is deleted in `sm8250-samsung-common.dtsi`). |
| **Hexagon CDSP (`8300000.remoteproc`)** | ⏳ HW live, userspace missing | `remoteproc1` (`cdsp.mbn`) running; `/dev/fastrpc-cdsp` `GET_DSP_INFO` reports **Hexagon v66 (`0x8f66`)**, **`UNSIGNED_PD_SUPPORT=1`**, **`4× 128B HVX`**, **`256 KiB VTCM`** (`HMX=0`). No `libcdsprpc.so` or Hexagon v66 ELF runtime staged yet. |
| **`omamobile-ai` (`:8080`, CPU)** | ✅ Verified on the stable kernel | `LFM3-1.7B-Q4_K_M.gguf` (`1.1G`, `THREADS=3`, `CPU_AFFINITY=4-6`, `GPU_LAYERS=0`), `llama-server` CPU-only (`--list-devices`: `(none)`). See §2 for today's numbers. |

---

## 2. CPU Baseline on the Stable Kernel (`2026-11-17`)

Kernel `7.3.0-rc6-omamobile-push1-g3667ab368ef9` (the image pinned in
[`stable-image.conf`](stable-image.conf)), `omamobile-ai.service` active, model
already resident. One warm-up plus three runs of the same request
(`max_tokens=128`, `temperature=0`), timings from llama.cpp's `timings` block:

| Run | Prompt tokens | Prompt tok/s | Generated | Generated tok/s |
|---|---|---|---|---|
| warm-up | 26 | 15.45 | 128 | 11.13 |
| 1 | 26 | 15.47 | 128 | **11.01** |
| 2 | 26 | 15.47 | 128 | 10.74 |
| 3 | 26 | 15.47 | 128 | 10.56 |

Thermals: 46.3 °C before the run, 74.1 °C after three back-to-back 128-token
answers. The spread from 11.01 to 10.56 tok/s is thermal drift, not a code path
change; quote **10.6-11.0 tok/s generation** for this kernel and heatsink state.

An earlier measurement on `gf3a5609d121b` read 15.34 prompt / 11.24 generation
tok/s, inside the same spread.

### Reproducing

```sh
OMAMOBILE_SSH_KEY=~/.ssh/omamobile-r8q tools/r8q-ai status
OMAMOBILE_SSH_KEY=~/.ssh/omamobile-r8q tools/r8q-ai ask \
  "Write one paragraph explaining why the sky is blue during the day and red at sunset."
```

`ask` prints the answer and, on stderr, `[<n> prompt tok @ <x> tok/s; <m>
generated @ <y> tok/s]`. For the raw JSON (the `timings` block above), POST the
same body to `/v1/chat/completions`.

---

## 3. The Supervisor and the Tool

### `omamobile-ai` (`packages/omamobile-ai`)

One binary (`/usr/bin/omamobile-ai`), one unit (`omamobile-ai.service`), config
in `/etc/omamobile-ai.conf` plus drop-ins in `/etc/omamobile-ai.conf.d/`.
The r8q drop-in (`packages/omamobile-device-r8q/etc/omamobile-ai.conf.d/10-sm8250.conf`)
pins inference to the three Gold cores.

| Key | r8q value | Meaning |
|---|---|---|
| `MODEL_PATH` | `/usr/share/omamobile-ai/models/LFM3-1.7B-Q4_K_M.gguf` | GGUF to serve |
| `MODEL_ID` | `LFM3-1.7B` | Name clients ask for |
| `THREADS` | `3` | llama.cpp threads (one per pinned core) |
| `CPU_AFFINITY` | `4-6` | Gold cores; 0-3 stay with the desktop, 7 is excluded |
| `CTX_SIZE` | `32768` | Context window |
| `GPU_LAYERS` | `0` | CPU inference until a Vulkan llama.cpp is built |
| `IDLE_TIMEOUT` | `180` | Seconds before the model is unloaded |
| `LISTEN_HOST` / `LISTEN_PORT` | `0.0.0.0` (drop-in) / `8080` | Supervisor on the USB link; the base config alone listens on loopback |
| `LLAMA_PORT` | `18080` | Private loopback port for `llama-server` |

Routes: `/health`, `/v1/models`, `/v1/chat/completions`, `/v1/unload`, and the
llama.cpp router surface (`/models`, `/props`, `/models/load`, ...) so a client
that validates a llama.cpp server accepts the phone. The unit has
`ConditionPathExists=` on the GGUF, so a phone without a model stays inactive
instead of crash-looping.

### `tools/r8q-ai` (laptop)

Drives the API at `http://172.16.42.1:8080`, and falls back to the same API over
`tools/phone-ssh` when the laptop cannot reach the USB address.

```sh
tools/r8q-ai status                  # model, RAM, swap, battery, load
tools/r8q-ai models                  # served model + GGUFs on the phone
tools/r8q-ai ask "<prompt>"          # one chat completion (alias: query)
tools/r8q-ai unload                  # free the model's RAM; supervisor stays up
tools/r8q-ai stop | start            # the unit itself
tools/r8q-ai push-model <file.gguf>  # upload, verify sha256, select, restart
```

`push-model` writes to `/var/lib/omamobile-ai/models/`, verifies the sha256
before moving the file into place, and records it in
`/etc/omamobile-ai.conf.d/20-model.conf`. `OMAMOBILE_AI_URL` and `MAX_TOKENS`
override the base URL and answer length.

---

## 4. Optional Future Work

1. **Vulkan LLM (`GGML_VULKAN=ON`) & `587 MHz` GPU rule:** Install `extra/vulkan-headers` (`shaderc` and `vulkan-freedreno` are already installed), build `llama.cpp` with `-DGGML_VULKAN=ON`, uncap `80-r8q-gpu.rules` `max_freq` from `305000000` to `587000000`, and re-benchmark `LFM3-1.7B` against the 10.6-11.0 tok/s CPU baseline.
2. **Adreno `libcamera` SoftISP (`debayer_egl`):** Re-test removing `LIBCAMERA_SOFTISP_MODE=cpu` (`packages/omamobile-device-r8q/70-r8q-libcamera.conf`) against `587 MHz` / `SWIspWorker` hangcheck debugging.
3. **Hexagon CDSP Compute:** Stage `libcdsprpc` + Hexagon v66 HVX runtime/kernel ELF (`UNSIGNED_PD_SUPPORT=1`) if offloading quantized ops to CDSP.
