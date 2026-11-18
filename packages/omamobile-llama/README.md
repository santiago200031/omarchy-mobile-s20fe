# omamobile-llama

`llama.cpp` built for the phone, CPU only, as the inference backend for
[`omamobile-ai`](../omamobile-ai). It installs:

- `/usr/bin/llama-server` — the supervisor's backend, on a private loopback port;
- `/usr/bin/llama-cli` — a one-shot prompt/diagnostic tool;
- `/usr/bin/llama-bench` — the repeatable benchmark (`pp512` / `tg128`).

## Source and build

| | |
|---|---|
| Upstream | [`ggml-org/llama.cpp`](https://github.com/ggml-org/llama.cpp), MIT |
| Pinned commit | `98d67c8d536dc584897c4adb2510509796517222` (2026-11-18) |
| Compiler flags | `-march=armv8.2-a+dotprod+fp16` (Kryo 585), `GGML_NATIVE=OFF` |
| Links | static ggml, no OpenMP |
| GPU | **off** (`GGML_VULKAN=OFF`) |

`GGML_NATIVE=OFF` with explicit `GGML_CPU_ARM_ARCH` makes the build identical
in `makepkg` on the phone and in the project's aarch64 container.

## Why CPU only

The Adreno 650 renders fine (Turnip, 587 MHz), but the Vulkan **compute** path
wedges the GPU: `device lost`, `hangcheck detected gpu lockup`, and
`a6xx_recover: cx gdsc didn't collapse`; only a reboot clears it. That is
[Failed 08](../../device/r8q/board/failed/08-vulkan-llm-adreno-gpu-wedge.md),
so `GPU_LAYERS=0` is the default and this package does not ship a Vulkan
build. A separate Vulkan tree can be built by hand for experiments, never as
the service binary.

## Use

`omamobile-ai.service` spawns `llama-server` from `PATH`; the package puts it
in `/usr/bin`, so a distro package or a hand-built `/usr/local/bin` copy must
not shadow it. Benchmark with the supervisor's affinity:

```sh
taskset -c 4-6 llama-bench \
  -m /usr/share/omamobile-ai/models/LFM3-1.7B-Q4_K_M.gguf \
  -t 3 -p 512 -n 128 -r 3
```

The three Gold cores (`cpu4-6`, 2.42 GHz) are what `omamobile-ai` uses;
`cpu7` is the prime core and `cpu0-3` are the A55 cluster.

## License

MIT, like upstream. The upstream text is installed as
`/usr/share/licenses/omamobile-llama/LICENSE.llama.cpp`.
