<!-- SPDX-License-Identifier: MIT -->
# omamobile-ai

On-device AI supervisor for Omarchy Mobile.

It exposes an OpenAI-compatible HTTP endpoint on `127.0.0.1:8080` by default
(`0.0.0.0:8080` on the r8q drop-in, so the laptop reaches it over the USB
link) and manages the `llama.cpp` inference server lifecycle, so the model
only runs when a client actually asks for it.

## How it works

The daemon serves:

* `GET /health` — device status, battery, memory, load average, model state
* `GET /v1/models` — model list (OpenAI-compatible)
* `POST /v1/chat/completions` — chat with the local model, streaming or JSON
* `POST /v1/unload` — free the model's RAM now; the supervisor stays up
* the llama.cpp router surface (`/models`, `/props`, `/models/load`,
  `/models/unload`), so a client that validates a llama.cpp server accepts
  the phone

When the first chat request arrives the supervisor starts `llama-server` on
`127.0.0.1:LLAMA_PORT` (default `18080`), waits for its `/health`, proxies the
response back untouched (SSE included), and starts an idle timer. After
`IDLE_TIMEOUT` seconds (default 180) with no requests the server is stopped and
the model is unloaded from RAM. The supervisor itself is a small Rust process
and keeps running.

## Configuration

Base defaults live in `/etc/omamobile-ai.conf`; a device package drops
overrides into `/etc/omamobile-ai.conf.d/`. Keys: `MODEL_PATH`, `MODEL_ID`,
`LLAMA_BIN`, `LISTEN_HOST`, `LISTEN_PORT`, `LLAMA_PORT`, `CPU_AFFINITY`,
`THREADS`, `CTX_SIZE`, `GPU_LAYERS`, `IDLE_TIMEOUT`.

The r8q drop-in (`omamobile-device-r8q`) pins inference to the three Gold
cores (`CPU_AFFINITY=4-6`, `THREADS=3`) and opens the USB listener.

## Status and control

From the laptop, `tools/r8q-ai` drives the API at `http://172.16.42.1:8080`
(with an SSH fallback) and can `unload`, `stop`, `start` and `push-model`.

## License

MIT — see `LICENSE` in the package.
