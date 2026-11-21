<!-- SPDX-License-Identifier: MIT -->

# On-Device AI & Accelerator Probe — Samsung Galaxy S20 FE (r8q)

**Status:** `omamobile-ai` supervisor (CPU, `LFM3-1.7B`) verified end-to-end; Stage 4 `[gpu-03..05]` closed (`802d3d3bc` fixes `-p 512` hangcheck timeout; `power-profiles-daemon` `performance` unlocks `2.42/2.84 GHz` -> **`70.18 t/s` `pp512` / `12.48` `tg128`** on `Q8_0` `4-6` vs **`23.39` `pp512` / `1.36` `tg128`** on Turnip Vulkan); `GPU_LAYERS=0` stays default
**Last Updated:** 2026-11-21

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
| **`omamobile-llama` (backend)** | ✅ Installed from the package | `llama.cpp` pinned `98d67c8d536dc584897c4adb2510509796517222` (2026-11-18), `omamobile-llama 1.0.0.20261118.98d67c8-1`; static CPU build, `-march=armv8.2-a+dotprod+fp16`, no OpenMP, `GGML_VULKAN=OFF`; installs `/usr/bin/{llama-server,llama-cli,llama-bench}`. |

---

## 2. CPU Baseline on the Stable Kernel (`7.3.0-rc6-omamobile-push1-g3667ab368ef9`)

### 2.1 2026-11-18 refresh — packaged `omamobile-llama` `98d67c8`

The backend moved from the hand-built `1.0.0-dev` (commit `1719dbb`,
`/usr/local/bin`) to `omamobile-llama 1.0.0.20261118.98d67c8-1` (`/usr/bin`,
static, CPU-only, `-march=armv8.2-a+dotprod+fp16`, no OpenMP). The old binary is
backed up at `/root/llama-old/llama-server-1.0.0-dev-1719dbb`; `/usr/local/bin`
no longer shadows the package.

`llama-bench` on the three Gold cores (`taskset -c 4-6`, `-t 3`, two runs):

| test | t/s |
|---|---|
| pp512 | **35.18 ± 0.01** |
| tg128 | **15.70 ± 0.03** |

The supervisor, same prompt and 128 generated tokens as the old baseline
(three runs after the model loaded):

| Run | Prompt tokens | Prompt tok/s | Generated | Generated tok/s |
|---|---|---|---|---|
| 1 | 27 | 32.19 | 128 | **15.29** |
| 2 | 4 | 28.22 | 128 | 15.26 |
| 3 | 4 | 27.83 | 128 | 15.28 |

Thermals: 47.5 °C (hottest of 28 zones) before the `llama-bench` pair, 68.7 °C
after. Generation is **~15.3 tok/s vs 10.6-11.0** on the old build (~1.4x);
`llama-bench` reports `build: unknown (0)` because the release tarball has no
git metadata, but the package version pins the commit.

### 2.2 2026-11-17 baseline (kept for comparison)

On the old hand-built binary, `omamobile-ai.service` active and the model
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

# standardised benchmark on the phone (three Gold cores):
OMAMOBILE_SSH_KEY=~/.ssh/omamobile-r8q tools/phone-ssh \
  'taskset -c 4-6 llama-bench \
     -m /usr/share/omamobile-ai/models/LFM3-1.7B-Q4_K_M.gguf \
     -t 3 -p 512 -n 128 -r 2'
```

`pacman -Q omamobile-llama` shows the pinned package; the binary is
`/usr/bin/llama-server` (the old hand-built `/usr/local/bin` copy is backed up in
`/root/llama-old/`).

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
| `MODEL_PATH` | `/usr/share/omamobile-ai/models/LFM3-1.7B-Q8_0.gguf` (`20-pi-tuned.conf`) | GGUF to serve (`Q4_K_M` base default) |
| `MODEL_ID` | `LFM3-1.7B` | Name clients ask for |
| `THREADS` | `4` | llama.cpp threads (`10-sm8250.conf`, one per Cortex-A77 core) |
| `CPU_AFFINITY` | `4-7` | 3x Gold (`2.42 GHz`) + 1x Prime (`2.84 GHz`); 0-3 stay with desktop/system |
| `CTX_SIZE` | `32768` | Context window |
| `GPU_LAYERS` | `0` | CPU inference (CPU beats Adreno 650 on `LFM3-1.7B` by `2.4x-3.2x` `pp512` and `9.2x-11.6x` `tg128`) |
| `IDLE_TIMEOUT` | `180` | Seconds of inactivity (TTL) before `llama-server` is stopped and RAM freed |
| `LISTEN_HOST` / `LISTEN_PORT` | `0.0.0.0` (drop-in) / `8080` | Supervisor on the USB link; the base config alone listens on loopback |
| `LLAMA_PORT` | `18080` | Private loopback port for `llama-server` |

### 2.3 `2026-11-21` CPU (`balanced` vs `performance`) vs Turnip Vulkan (`7.3.0-rc6-omamobile-push1-g802d3d3bc512`)

Measured on live hardware (`LFM3-1.7B`, `-p 512 -n 128 -r 2` unless noted). Unlocking `power-profiles-daemon` `performance` raises CPU clusters from `1.34 / 1.77 / 2.17 GHz` (`balanced`) to `1.80 / 2.42 / 2.84 GHz` (`peak thermal 66.4 °C`, zero errors):

| Backend / Model | Config | `pp512` (`t/s`) | `tg128` (`t/s`) |
|---|---|---|---|
| **CPU `Q8_0`** (`performance`, `4-7` `-t 4`) | `2.42 / 2.84 GHz` | **`75.84 ± 0.18`** | `12.46 ± 0.04` |
| **CPU `Q8_0`** (`performance`, `4-6` `-t 3`) | `2.42 GHz` | **`70.18 ± 1.73`** | `12.48 ± 0.03` |
| **CPU `Q8_0`** (`balanced`, `4-6` `-t 3`) | `1.77 GHz` | `56.62 ± 0.00` | `12.60 ± 0.08` |
| **CPU `Q4_K_M`** (`performance`, `4-6` `-t 3`) | `2.42 GHz` | **`43.69 ± 0.21`** | **`16.73 ± 0.18`** |
| **CPU `Q4_K_M`** (`balanced`, `4-6` `-t 3`) | `1.77 GHz` | `35.16 ± 0.00` | `15.78 ± 0.01` |
| **Vulkan `Q4_K_M`** (`-ngl 99`, `HC=250`) | `802d3d3bc`, Turnip a650 | `23.39 ± 0.01` | `1.36 ± 0.00` |
| **Vulkan `Q4_K_M -fa 1`** (`-ngl 99 -ub 64`) | Turnip a650 | `22.09 ± 0.00` | `1.37` (`-n 16`) |
| **Vulkan `Q8_0`** (`-ngl 99 -ub 64`) | Turnip a650 | `23.71 ± 0.00` | `1.04` (`-n 16`) |

With `DRM_MSM_HANGCHECK_PROGRESS_RETRIES=31` (`802d3d3bc`), bare `llama-bench -ngl 99 -p 64,128,256,512 -n 16` passes cleanly at default `hangcheck_period_ms=250` (`30.26 / 30.17 / 27.24 / 23.39 t/s`, `tg16 1.37 ± 0.00`, `0` GPU errors), but CPU remains `GPU_LAYERS=0` default.

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

1. **Vulkan LLM (`GGML_VULKAN=ON`)** — **working (`802d3d3bc`)**, passes `-p 64..512` cleanly without hangcheck (`[gpu-03..05]` closed), but slower than Kryo 585 Gold (`1.36` vs `12.5-16.7 tok/s` generation). CPU stays `GPU_LAYERS=0` default.
2. **Adreno `libcamera` SoftISP (`debayer_egl`):** Re-test removing `LIBCAMERA_SOFTISP_MODE=cpu` (`packages/omamobile-device-r8q/70-r8q-libcamera.conf`) against `587 MHz` / `SWIspWorker` hangcheck debugging.
3. **Hexagon CDSP Compute:** now planned in [`NPU-PLAN.md`](NPU-PLAN.md): a private `ggml-hexagon` fork for Hexagon v66 (HVX-only, no HMX), needing Hexagon SDK 5.4 and `libcdsprpc`.
