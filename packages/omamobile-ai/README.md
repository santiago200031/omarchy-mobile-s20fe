<!-- SPDX-License-Identifier: MIT -->
# omamobile-ai

On-device AI supervisor for Omarchy Mobile.

It exposes a single OpenAI-compatible HTTP endpoint on `127.0.0.1:8080` and
manages the llama.cpp inference server lifecycle so the model only runs when
a client actually asks for it.

## How it works

The daemon listens on port 8080 and serves three endpoints:

* `GET /health` — device status, battery, memory, load average
* `GET /v1/models` — model list (OpenAI-compatible)
* `POST /v1/chat/completions` — chat with the local model

When the first chat request arrives the supervisor spawns `llama-server` on a
private Unix socket, proxies the streaming response back to the caller, and
starts an idle timer.  After 180 seconds of no requests the server is killed
and the model is unloaded from RAM.

## Configuration

Device-specific tuning lives in `/etc/omamobile-ai.conf` (shipped with a
default that works on most hardware).  A device package can drop a drop-in
that overrides model path, thread count, or CPU affinity.

## License

MIT — see `LICENSE` in the package.
