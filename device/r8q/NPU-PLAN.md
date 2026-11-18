<!-- SPDX-License-Identifier: MIT -->

# NPU Plan — Hexagon CDSP (HTP v66) for llama.cpp (r8q / SM8250)

**Status (2026-11-18):** planning, and **P0 has already found the real blocker.**
The Hexagon SDK 6.7 in the public toolchain image **cannot target v66**
(`hexagon-clang -mv66` is rejected; its arch table is `v68..v85`; QURT libs are
`computev68+`). **The public 6.4.0.2 SDK is also v68+ (verified), so the v66
build needs Hexagon SDK 5.4.0** (QPM3, owner-provided). The host-side
`libcdsprpc.so` for glibc aarch64 *is* in the image and
is clean (only libc/libm/libstdc++), and the kernel side is already in place.
No phone write has been made for this plan. The phone runs the stable kernel
`7.3.0-rc6-omamobile-push1-g3667ab368ef9`, the `omamobile-ai` supervisor serves
`LFM3-1.7B` on the CPU (15.47 prompt / 10.6-11.0 generation tok/s), and the
Adreno 650 is on Turnip for render. Section 10 records the contribution rules
this plan must satisfy (llama.cpp issue-first and AI disclosure; this repo's
package and kernel checks).
**Target:** run `LFM3-1.7B-Q4_K_M` on the Hexagon CDSP through llama.cpp's
Hexagon backend, then decide whether it beats or complements the CPU.
**Kernel worktree:** `~/omamobile-kernel/wt-r8q` (only if a kernel/DT patch is
needed; the plan starts read-only). **llama.cpp worktree:** `~/Work/llama.cpp-r8q`
(to be created). **Outputs:** `~/omamobile-kernel/out-r8q-npu` (logs, builds).
**Last Updated:** 2026-11-18

---

## 1. Objective and non-goals

**Objective.** Give `llama.cpp` a working Hexagon backend on this phone's CDSP
(`Hexagon v66`), integrated with `omamobile-ai`, with a benchmark against the
CPU baseline and a one-command rollback to CPU.

**Non-goals, for now:**

- The dedicated NPU block (`npu@9910000`, the HTA) as the LLM engine. It has no
  mainline driver, no firmware in the firmware set we pulled, and no open
  compiler/runtime; llama.cpp has no HTA backend. See §7.
- Vulkan/GPU inference. It is blocked by the GMU `cx gdsc` wedge (Failed 08);
  unrelated to this plan.
- Rewriting the Hexagon math stack. The existing HVX kernels are the base.

---

## 2. What the hardware and software actually are

| Block | Where | State on this phone |
|---|---|---|
| **Hexagon CDSP / HTP** | `remoteproc@8300000` (`qcom,sm8250-cdsp-pas`), `glink-edge` → `fastrpc`, 8 `compute-cb` (SMMU SIDs `0x1001`-`0x1008`), firmware `qcom/sm8250/samsung/r8q/cdsp.mbn` | Running (`remoteproc` name `cdsp`). `/dev/fastrpc-cdsp` (10:261), `/dev/fastrpc-cdsp-secure` (10:260), `/dev/fastrpc-adsp`, `/dev/fastrpc-sdsp`. `GET_DSP_INFO`: **Hexagon v66 (`0x8f66`)**, `UNSIGNED_PD_SUPPORT=1`, **4×128 B HVX**, **256 KiB VTCM**, **HMX=0**. |
| **Dedicated NPU / HTA** | downstream `npu@9910000`; mainline keeps only the reserved `npu_mem` (5 MiB at `0x8d400000`, in `sm8250-samsung-common.dtsi`) | **No mainline device node or driver** (mainline `sm8250.dtsi` has no `npu@9910000`; the Samsung common DT re-adds only the memory). No `npu.mbn` in the firmware set we pulled. No open userspace for LLM. |
| **Kernel fastrpc** | `drivers/misc/fastrpc.c`, `CONFIG_QCOM_FASTRPC=m` | Probes all four domains. UAPI has `FASTRPC_IOCTL_INIT_CREATE` (unsigned module), `MMAP`, `MEM_MAP`, `SET_OPTION` and `GET_DSP_INFO` (13). |
| **Userspace fastrpc** | — | **Missing.** No `libcdsprpc.so` on the Arch root. This is the first bring-up item. |
| **llama.cpp Hexagon backend** | upstream `ggml/src/ggml-hexagon` | Supports HTP **v73/v75/v79/v81/v85** only (`build_htp_skel()` list). Has a **HVX-only mode**: `GGML_HEXAGON_NHMX=0`, and `n_hmx` is set from the hardware's `hw_n_hmx` (0 on v66). Linux target exists (commit `81069a8`, `arm64-linux-snapdragon-release`, `PREBUILT_LIB_DIR=linux_aarch64`). |
| **Toolchain** | `ghcr.io/snapdragon-toolchain/arm64-linux:v0.8` | Debian trixie + Hexagon SDK **6.7.0.0**, Hexagon Tools **21.0.04**, Android NDK r29, OpenCL SDK. **v68+ only** (see P0). The public **6.4.0.2** image is also v68+ (verified). The v66 build needs **SDK 5.4.0** (QPM3, owner-provided). `libcdsprpc.so` for glibc aarch64 is in both 6.x images and needs only `libc`, `libm`, `libstdc++`. |

**Key code facts that make v66 plausible (verified in the upstream tree):**

- `ggml-hexagon.cpp`: `opt_nhmx` (env `GGML_HEXAGON_NHMX`) defaults to 1, but
  `n_hmx = (opt_nhmx != 0) ? hw_n_hmx : 0`, and `hw_n_hmx` comes from
  `htp_iface_hwinfo()`. On v66 it is 0, so the backend already selects a
  no-HMX (HVX) path at runtime.
- `matmul-ops.c`: `if (kparams->n_hmx) { ... }` guards the HMX-specific setup;
  the HVX kernels (`vec_dot_*`, `tiled_vec_dot_*`) are the other path.
- The HTP sources already branch on `__HVX_ARCH__` (`< 75`, `> 75`, `> 79`),
  so older architectures were considered.
- The HTP skel list is the main hardcode: `build_htp_skel(v73/v75/v79/v81/v85)`.
  Adding `v66` is a small CMake change, plus compile guards for HMX.

---

## 3. Decision

1. **Primary track: CDSP HTP v66.** Port the existing `ggml-hexagon` backend to
   v66 and run `LFM3-1.7B` on it. This is the only block with a live driver, a
   running firmware, a supported RPC path and an existing llama.cpp backend.
2. **Upstream first, but issue first.** The human opens an issue in
   `ggml-org/llama.cpp` with the v66 evidence and asks whether v66 is in scope
   before any upstream code is written. AI must not write or post that issue.
   If the maintainers say no (or prefer a private fork/example), the fork is the
   path and no upstream PR is attempted.
3. **Fork fallback is planned from day one.** The first commits are small and
   separable: build v66, guard HMX, fix HVX assumptions, docs. If the PR is
   rejected, the same commits become the fork's `r8q-v66` branch and a package.
4. **Dedicated `npu@9910000` is a research track with a go/no-go gate (§7).**
   It is not on the critical path and cannot run llama.cpp today.

---

## 4. Risks and unknowns (ranked)

| # | Risk | Why it matters | Mitigation / gate |
|---|---|---|---|
| 1 | **Hexagon SDK 6.7 and 6.4 cannot target v66 (verified)** | Without a v66 target and `computev66` QURT libs, the skel cannot be built or linked | G0: owner downloads **SDK 5.4.0** (QPM3); verify `hexagon-clang -mv66` + `computev66`. If unavailable, own minimal skel or park the track. |
| 2 | **HMX code is compiled unconditionally** | v66 has no HMX; HMX intrinsics may not compile for `-mv66` | G2: wrap HMX sources/headers in `#if __HVX_ARCH__ >= 68`; verify with `hexagon-objdump` that no HMX instructions remain. |
| 3 | **VTCM is 256 KiB, not 8 MiB** | Limits tile buffers and pipelining; some kernels assume more | Start with the smallest tiles and `GGML_HEXAGON_VTCM_SIZE=256K`; measure; add a v66-specific buffering path only if needed. |
| 4 | **`libcdsprpc.so` licensing and ABI** | It is Qualcomm's, glibc aarch64; cannot be committed | Stage it locally from the SDK image; ship our code only; package fetches/builds the SDK or uses the user's own copy. Check the SDK license in P0.2. |
| 5 | **Mainline fastrpc vs the SDK's lib** | The lib may expect a remote heap, ION or an ioctl the mainline driver lacks | G1: run the SDK's smallest unsigned-PD test before touching llama.cpp; if it fails, trace the ioctl and add a DT `memory-region` or a driver patch. |
| 6 | **Performance without HMX** | v66 may not beat 3×A77 CPU at 10.6 tok/s | G4 measures; the win may be offload/power, not raw tok/s. Decide the default on the numbers. |
| 7 | **Upstream may reject v66 as out of scope** | Qualcomm's backend targets v68+ | The fork is the fallback; keep the v66 changes clean and evidence-backed. |
| 8 | **CDSP crash / firmware fault** | It could take the DSP (and unrelated services) down | Unsigned PD is isolated; stop on any `fastrpc` fault; a CDSP reset needs a reboot (ask the human). |

---

## 5. Phases, gates and commands

Each step is read-only first; the phone writes are marked. The stable phone stays
on its current kernel and root; no flashing is planned in this document.

### P0 — Feasibility (laptop + phone, read-only) — **started 2026-11-18**

**P0.1 Toolchain image (done).** Pulled `arm64-linux:v0.8`
(digest `sha256:1cd251b4a1b1b62bafd52857b948daf3d41064934adbd01ab9fa6f0760d69456`)
and inspected it:

- Hexagon SDK 6.7.0.0, Tools 21.0.04. `hexagon-clang -mv66` →
  *"unknown argument '-mv66'; did you mean '-mv68'?"*
- `build/cmake/hexagon_arch.cmake` lists `v85 v83 v81 v79 v75 v73 v69 v68`
  only; `rtos/qurt/` has `computev68`, `computev69`, … (no `computev66`).
- `ipc/fastrpc/remote/ship/UbuntuARM_aarch64/libcdsprpc.so` exists, is
  aarch64, and needs only `libc`, `libm`, `libstdc++`.

**Result:** SDK 6.7 **cannot build the v66 skel**. The host `libcdsprpc` is
usable; the DSP build is not. This is the plan's real pivot.

**P0.2 A v66-capable SDK (gate G0).** The 6.x line is out (6.7 and 6.4 were
both verified: `-mv66` rejected, arch table `v68..v85`, QURT `computev68+`).
The v66 build needs the **5.4 line**: Qualcomm's QAIRT release notes name
`hexagon-sdk-5.4.0` and "securepd on v66 target", and the SDK 5.x docs list
V65/V66/V68.

1. **SDK 5.4.0** — owner-provided via Qualcomm Software Center (QPM3; free
   account + license acceptance; Linux package). Put it under
   `~/omamobile-kernel/hexagon-sdk-5.4/` (gitignored, never committed).
   Verify `hexagon-clang -mv66` and `rtos/qurt/computev66`.
2. **Own minimal v66 skel** — if the 6.x CMake helpers (`hexagon_fun.cmake`,
   `build_idl`) are absent in 5.4, write a small CMake/Make shim that calls
   `hexagon-clang -mv66` on the `htp/` sources and links `libqurt.a` for v66;
   only the ops LFM needs. This is the fallback the user allowed.

**Expect/pass (G0):** `hexagon-clang -mv66` accepts the target and
`rtos/qurt/computev66` exists; record the SDK path and version.
**Stop:** if the QPM3 download is unavailable, park the track (do not commit
anything vendor-owned).

> **Owner action for G0:** install Qualcomm Package Manager 3 (QPM3), sign in,
> accept the Hexagon SDK license, download **Hexagon SDK 5.4.0 (Linux)** and
> unpack it to `~/omamobile-kernel/hexagon-sdk-5.4/`. It is gitignored and
> never committed; only its path and checksums go in this plan.

**P0.3 Phone surface (read-only).**

```sh
OMAMOBILE_SSH_KEY=~/.ssh/omamobile-r8q tools/phone-ssh '
  dmesg | grep -iE "fastrpc|cdsp" | tail -20
  cat /sys/class/remoteproc/remoteproc*/name
  ls -l /dev/fastrpc-* /dev/dma_heap
  cat /sys/kernel/debug/remoteproc/remoteproc*/name 2>/dev/null'
```

**Expect/pass:** `cdsp` running; `fastrpc-cdsp` present; no fastrpc faults.
**Risk:** read-only.

**P0.4 License (read-only).** Read the SDK license for `libcdsprpc.so` and the
skels. Record whether a package may redistribute them. Default: ship only our
binaries; the package fetches the SDK at build time or the user supplies it;
never commit the `.so`, the SDK or vendor firmware.

**P0.5 Upstream issue (the human writes it; gate for P2/P6).**

llama.cpp requires a feature to start as an issue, and it forbids AI-written
issue/PR text. With the P0 facts in hand, the human opens an issue in
`ggml-org/llama.cpp` that states:

- the device (SM8250, CDSP HTP v66, 4x128B HVX, 256 KiB VTCM, no HMX) and the
  working mainline fastrpc path;
- that the backend already has an HVX-only mode (`GGML_HEXAGON_NHMX=0`);
- the build dependency problem: SDK 6.7/6.4 target v68+, so v66 needs the
  licensed SDK 5.4, and upstream CI has no such hardware;
- the question: is a v66 build + HMX guards in scope, or is a private
  fork/example the preferred path?

**Gate G0.5:** the maintainers answer. If in scope, P2/P6 proceed. If not, P2
is done only in the fork (`santiago200031/llama.cpp`) and P6 is dropped; the
plan records their reason. The agent may prepare private technical notes for
the human, but **never writes or posts the issue, a comment, or a PR**. Use
ASCII in the issue (no em dash or arrows), as llama.cpp's style guide asks.

### P1 — Userspace fastrpc on the phone (phone write: staged libs only; needs G0)

1. Copy the SDK's `libcdsprpc.so` and its dependencies to
   `/usr/local/lib/llama-hexagon/` (not a package yet). Add that directory to
   `LD_LIBRARY_PATH` for the test only.
2. Build/run the smallest **unsigned PD** test from the SDK (or a 20-line
   `fastrpc` stub+skel) against `/dev/fastrpc-cdsp`.
3. Watch `dmesg | grep -i fastrpc` and `/sys/kernel/debug/remoteproc`.

**Expect/pass (G1):** a remote call returns a value computed on the CDSP; the
DSP stays up; no `fastrpc` error in dmesg.
**Stop:** any CDSP crash, `-EIO` or `FASTRPC_*` timeout. Do not proceed to
llama.cpp; collect the ioctl trace and fix the kernel/DT first.
**Rollback:** remove `/usr/local/lib/llama-hexagon/`; nothing else changed.
**Risk:** low (staged files, no boot change). A DSP user-PD crash should be
isolated; if the CDSP itself faults, a reboot is the recovery (ask first).

### P2 — Build the v66 HTP skel (laptop, container; only after G0.5 says in scope, or in the fork)

1. Clone llama.cpp to `~/Work/llama.cpp-r8q`, branch `r8q-hexagon-v66`.
2. Prepare a build image with the v66-capable SDK from G0 (**SDK 5.4.0**).
   If 5.4 has the 6.x `hexagon_fun.cmake`/`build_idl` helpers, map it into the
   v0.8 image; if not, add the shim (direct `hexagon-clang -mv66` + `qaic`).
3. Add `build_htp_skel(v66)` to `ggml/src/ggml-hexagon/CMakeLists.txt`,
   **behind an opt-in** (e.g. `-DGGML_HEXAGON_V66=ON`), so the default build
   with SDK 6.7 (v68+) is unchanged and CI keeps working. The v66 skel is
   additive; nothing in the v68+ path may change.
4. Guard HMX code with `#if __HVX_ARCH__ >= 68` (sources/headers that use HMX
   intrinsics), keeping the HVX path intact.
5. Build `libggml-htp-v66.so`:
   `cmake --preset arm64-linux-snapdragon-release -B build-v66` with
   `GGML_HEXAGON=ON` (or the shim's direct `hexagon-clang` rules).

**Expect/pass (G2):** the v66 skel compiles and links; `hexagon-objdump` shows
no HMX instructions; the host `libggml-hexagon.so` loads.
**Stop:** an intrinsic the v66 toolchain does not have that cannot be guarded
with a sensible HVX fallback. Record it and reassess (older kernel sources or a
different op).
**Risk:** none on the phone; all in the container.

### P3 — First ops on the HTP (phone)

1. Deploy the build (`libggml-hexagon.so`, `libggml-htp-v66.so`,
   `libggml-base/cpu`, `llama-bench`, `llama-cli`).
2. `GGML_HEXAGON_NHMX=0 ADSP_LIBRARY_PATH=... LD_LIBRARY_PATH=... \
   llama-bench --device HTP0 -ngl 99 -p 16 -n 8 -m <small model>`.
3. `test-backend-ops -b HTP0 -o MUL_MAT` (the backend's own gate).
4. A small GGUF first (SmolLM2-135M or Qwen2.5-0.5B Q4_0), then LFM.

**Expect/pass (G3):** `MUL_MAT` OK; a small model produces coherent text; no
`fastrpc` fault.
**Stop:** any wrong result, DSP fault or hang. Do not run LFM on a failing op.
**Rollback:** delete the staged build; the CPU service is untouched.
**Risk:** medium (sustained DSP load; announce heat). The phone stays on the
stable kernel.

### P4 — LFM benchmark (phone)

```sh
GGML_HEXAGON_NHMX=0 ... llama-bench -m LFM3-1.7B-Q4_K_M.gguf \
  --device HTP0 -ngl 99 -p 128 -n 64 -r 3
```

Compare with the CPU baseline (15.47 prompt / 10.6-11.0 generation tok/s) and
the CPU at the same threads. Record prompt/gen tok/s, wall time, SoC/zone
temperatures, and battery current if available; run a 30-minute soak.

**Expect/pass (G4):** correct answers, no faults, stable for 30 minutes.
**Decision:** use HTP as the default only if it is stable and either faster or
clearly better on power/thermals; otherwise keep CPU and document.
**Risk:** medium (heat; announce).

### P5 — Integration and packaging

- Add a backend selector to `omamobile-ai` (CPU vs HTP) with CPU as the default
  until G4 passes; one config drop-in switches it.
- Package our build as `omamobile-llama-hexagon` (our binaries and skel; no
  Qualcomm blobs in git; `libcdsprpc` from the SDK or the user's phone), with
  the repo's `PKGBUILD`/`version`/`README.md`/`test/all` layout.
- Update `AI-BENCHMARKS.md`, the board card, NOTES and the r8q row of
  `docs/DEVICES.md` with only what was verified.
- Run `tools/pr-check origin/main`, `tests/all`, `tests/packages` and
  `tools/phone-smoke`; put the smoke report in the PR; run the
  `omamobile-reviewer` agent before posting.

**Expect/pass (G5):** `omamobile-ai` serves LFM on the chosen backend after a
restart; `systemctl restart omamobile-ai` + config rollback returns to CPU; all
the checks above pass.

### P6 — Upstream PR (only if G0.5 said yes; the human owns it)

- Rebase onto `ggml-org/llama.cpp` master; split commits:
  `hexagon: build the v66 skel`, `hexagon: guard HMX for pre-v68`,
  `hexagon: HVX fixes for v66`, `docs: v66 on SM8250`.
- **The human writes the commit messages, the PR description and every reply.**
  The agent does not push or open the PR; if it commits at the human's explicit
  request, the message carries `Assisted-by: <assistant>` (never
  `Co-authored-by:`). The human also creates the GitHub fork. Fill the PR
  template's **AI usage disclosure** honestly.
- Run `skills/code-review/SKILL.md` over the diff (private notes only), then
  `ci/run.sh`, `test-backend-ops` (CPU vs HTP), `llama-perplexity` and
  `llama-bench` before/after; include the v66 benchmark and the exact phone.
- If the issue said no, or the PR is rejected: publish the fork
  (`santiago200031/llama.cpp`) and keep the package as the deliverable. No
  duplicate PRs.

**Expect/pass (G6):** a human-submitted PR with the disclosure and the evidence,
or a tagged fork documented as the supported path.

---

## 6. What "kernel and all needed" means here

- **Kernel:** the mainline `fastrpc` driver already binds the CDSP and exposes
  the ioctls the SDK uses (`INIT_CREATE`, `MMAP`, `MEM_MAP`, `SET_OPTION`,
  `GET_DSP_INFO`). No patch is expected for P0-P3. If P1 shows a missing piece:
  - a `memory-region` for a CDSP remote heap (the ADSP has
    `adsp-rpc-remote-heap`; the CDSP has none), or
  - a missing ioctl/flag (add to `drivers/misc/fastrpc.c` with a DT property),
  then a kernel branch `r8q-npu` is opened in `~/omamobile-kernel/wt-r8q` and
  follows the project's kernel rules (one change per boot, `W=1`,
  `dt-validate`, a gated runbook, no flash without approval).
- **Firmware:** `cdsp.mbn` is the stock Samsung firmware already running; no
  change. No vendor firmware is committed.
- **Userspace:** the SDK's `libcdsprpc.so` + our `libggml-htp-v66.so`; staged
  locally, packaged later without blobs.
- **llama.cpp:** the v66 skel and the HMX guards; the host backend is unchanged.

---

## 7. Track B — the dedicated `npu@9910000` (go/no-go only)

**What it is.** The SM8250's separate NPU/HTA block at `0x9910000`, with a
5 MiB reserved `npu_mem` at `0x8d400000` on r8q. Downstream Android drives it
with Qualcomm's `msm_npu` driver and the HTA/QNN userspace; mainline has no
node, no driver and no firmware in the set we pulled.

**B0 research (read-only):**
- Find the downstream `msm_npu` source (Qualcomm's GPL kernel) and its
  firmware/`npu.mbn`; check whether the r8q stock vendor image even ships it.
- Check the userspace: QNN "HTA" backend / SNPE HTA, its license and whether it
  can target this block on Linux.
- Check whether llama.cpp has any HTA path (it does not today).

**B1 go/no-go.** Default **no-go for LLM**: no open compiler/runtime, no
llama.cpp backend, and the HTA is a CNN-oriented fixed-function block. Only if
B0 finds an open, usable runtime does this become a separate driver plan. The
CDSP track is not blocked by it.

---

## 8. Tracking

| Phase | Gate | Status | Evidence (log/commit) | Date |
|---|---|---|---|---|
| P0.1 toolchain 6.7 | G0 | **done — no v66** | `-mv66` rejected; arch table `v68..v85`; `libcdsprpc` OK | 2026-11-18 |
| P0.2 v66-capable SDK (5.4) | G0 | **blocked on owner download (QPM3)** | verify `-mv66` + `computev66` | |
| P0.3 phone surface | G0 | planned | `logs/r8q/npu-p0-phone.log` | |
| P0.4 license | G0 | planned | `NPU-PLAN.md` §5 note | |
| P0.5 upstream issue | G0.5 | planned (human) | issue URL | |
| P1 libcdsprpc + hello PD | G1 | planned | `logs/r8q/npu-p1-fastrpc.log` | |
| P2 v66 skel build | G2 | planned | `logs/r8q/npu-p2-build.log` | |
| P3 MUL_MAT + small model | G3 | planned | `logs/r8q/npu-p3-ops.log` | |
| P4 LFM benchmark | G4 | planned | `logs/r8q/npu-p4-lfm.log` | |
| P5 integration | G5 | planned | package + `AI-BENCHMARKS.md` | |
| P6 upstream | G6 | planned | PR URL / fork tag | |
| B0/B1 dedicated NPU | go/no-go | planned | `logs/r8q/npu-b0-research.md` | |

---

## 9. Safety

- **No NV, boot or flash writes in P0-P4.** The stable kernel, the SD-image
  root and the 77/77 read-only guard stay as they are.
- **Ask before a reboot.** A CDSP user-PD crash should be isolated; a CDSP
  firmware fault may need a reboot. The human is asked first.
- **Announce sustained NPU load.** It heats the SoC like a long CPU benchmark.
- **No vendor blobs in git.** `libcdsprpc.so`, `cdsp.mbn` and the SDK are never
  committed; only paths, checksums and build instructions are.
- **One change per boot** for any kernel/DT step, with a log stream and a
  rollback, as in `AGENTS.md` §1.

---

## 10. Contribution rules and checklist

This work can produce code in three places, each with its own rules. The
agent implements, tests and prepares **private** notes; the human owns every
public word and every submission. llama.cpp forbids fully autonomous
submissions: the agent works only under human supervision, and before any
submission it verifies the human can explain the change and has reviewed every
line.

### 10.1 llama.cpp (P6)

Source of truth: `CONTRIBUTING.md` and `AGENTS.md` in `ggml-org/llama.cpp`.

| Rule | What it means here |
|---|---|
| **Issue before PR** | P0.5 is that issue. No upstream code before the maintainers answer. |
| **AI disclosure** | The PR template has an `AI usage disclosure: YES/NO` line; fill it honestly (agent-assisted design, code, tests). |
| **Human-written public text** | The human writes the issue, commit messages, PR description and reviewer replies. AI-written text is prohibited and can close the PR. |
| **No agent submission** | The agent does not run `git push` or `gh pr create` for llama.cpp. If it commits at the human's explicit request, the message carries `Assisted-by: <assistant>`, never `Co-authored-by:`. |
| **Understand every line** | The human must explain and debug the change without the agent; keep the v66 change small and reviewable. |
| **One PR** | A new contributor keeps one open PR; no unrelated cleanup. |
| **Local review** | Run `skills/code-review/SKILL.md` over the diff and fix the findings. The output is private notes, never posted. |
| **CI and tests** | `ci/run.sh` locally; `test-backend-ops` (CPU vs HTP) for ggml changes; `llama-perplexity` and `llama-bench` before/after. |
| **Style** | ASCII only (no em dash, arrows or unicode), 4 spaces, `void * ptr`, sized ints, concise comments, clang-format v15+, existing patterns. |
| **Scope/maintenance** | v66 is niche: the issue must say who maintains it. Search issues/PRs first; no duplicates. The v66 skel must be **opt-in** so default v68+ builds and CI are untouched. |
| **Fork** | Private forks are exempt from the project's merge standards; the fork is the fallback. |
| **Comprehension** | Before submission, the human reads every line and can explain and debug each commit; the agent walks through the design and the diffs privately. |

### 10.2 This repository (the package and docs)

Source of truth: `CONTRIBUTING.md` §6, `AGENTS.md` §2, the PR template.

- **Package layout:** `packages/omamobile-llama-hexagon/` with `PKGBUILD`,
  `install`, `version`, `README.md` and `test/all`; bump `version` and put it in
  the commit subject.
- **SPDX:** every file; MIT for ours, GPL-2.0 for kernel/DTS.
- **No vendor blobs:** `libcdsprpc.so`, `cdsp.mbn`, the SDK and the skels built
  from Qualcomm headers are never committed; the package points at the user's
  SDK or a build-time fetch.
- **Checks:** `tools/pr-check origin/main`, `tests/all`, `tests/packages` and
  `tools/phone-smoke`; paste the smoke report into the PR.
- **PR content:** phone, `uname -r`, package versions, the plan steps that
  passed, and what was not tested.
- **Device matrix:** update the r8q row of `docs/DEVICES.md` only for what is
  verified.
- **Review:** run the `omamobile-reviewer` agent before posting; add confirmed
  lessons to `docs/review/lessons/` in a `docs/review:` commit.
- **AI trailers:** `Co-Authored-By:`/`Assisted-by:`, never `Signed-off-by:`.

### 10.3 Kernel (only if a fastrpc/DT patch is needed)

Source of truth: `CONTRIBUTING.md` §6.2 and
`Documentation/process/coding-assistants.rst`.

- One subsystem per branch (`omamobile-<subsystem>`);
  `dt-bindings:` -> driver -> `arm64: dts:`; each commit builds.
- Prefixes from `git log --oneline <file>`; 72-column messages that say why and
  cite the evidence.
- `scripts/checkpatch.pl --strict`, `W=1`, `dt_binding_check`, `CHECK_DTBS`,
  and `tools/kernel-pr-check --build OUT --merge BASE`.
- The human author's `Signed-off-by:`; AI help is `Assisted-by:`, never
  `Signed-off-by:`.
- New hardware stays `disabled` in the DTS until it passes on the phone;
  overlays only switch it on for a test boot.
- `fastrpc` is a shared driver: gate any change by DT/compatible data and show
  the other boards (cheetah first) are unchanged, as `CONTRIBUTING.md` §6.2
  requires.

### 10.4 Pre-submission checklist

- [ ] G0: SDK 5.4 `-mv66` + `computev66` verified
- [ ] G0.5: upstream issue answered (in scope, or fork)
- [ ] G1: unsigned-PD hello on the CDSP
- [ ] G2: v66 skel builds, no HMX instructions
- [ ] G3: `MUL_MAT` + small model correct
- [ ] G4: LFM benchmark + 30-minute soak
- [ ] G5: package + one-command CPU rollback
- [ ] `skills/code-review` (llama.cpp) / `omamobile-reviewer` (this repo)
- [ ] `ci/run.sh` + `test-backend-ops` + perplexity/bench
- [ ] `tools/pr-check`, `tests/all`, `tests/packages`, `tools/phone-smoke`
- [ ] AI disclosure filled in the llama.cpp PR template
- [ ] The human wrote the issue, commit messages, PR description and replies

---

## 11. References

- llama.cpp Snapdragon/Hexagon backend:
  `docs/backend/snapdragon/README.md`, `developer.md`, `linux.md`;
  `ggml/src/ggml-hexagon/` (CMake skel list, `GGML_HEXAGON_NHMX`, HVX/HMX
  kernels).
- Linux support commit: `81069a8` ("hexagon: add support for linux on
  snapdragon", `cmake/arm64-linux-clang.cmake`, `arm64-linux-snapdragon-release`).
- Toolchain: `ghcr.io/snapdragon-toolchain/arm64-linux:v0.8` (Hexagon SDK
  6.7.0.0, Tools 21.0.04); `github.com/snapdragon-toolchain/docker`.
- Kernel: `drivers/misc/fastrpc.c`, `include/uapi/misc/fastrpc.h`;
  `arch/arm64/boot/dts/qcom/sm8250.dtsi` (CDSP + fastrpc).
- FastRPC userspace: `github.com/quic/fastrpc`; SDK `ipc/fastrpc`.
- Project records: [`AI-BENCHMARKS.md`](AI-BENCHMARKS.md) (CPU baseline),
  [`board/failed/08-vulkan-llm-adreno-gpu-wedge.md`](board/failed/08-vulkan-llm-adreno-gpu-wedge.md)
  (the GPU path's blocker), `AGENTS.md` §1 and §2.
