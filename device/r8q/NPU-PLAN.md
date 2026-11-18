<!-- SPDX-License-Identifier: MIT -->

# NPU Plan — Hexagon CDSP (HTP v66) for llama.cpp (r8q / SM8250)

**Status (2026-11-18): G0 PASSED.** The v66 toolchain is working: Hexagon SDK **5.4.2.0** at `/local/mnt/workspace/Qualcomm/Hexagon_SDK/5.4.2.0`, `HEXAGON_Tools/8.7.06`; `hexagon-clang -mv66` compiles (`__HEXAGON_ARCH__ 66`, with `libtinfo.so.5` symlinked to `libtinfo.so.6`); `rtos/qurt/computev66` and the glibc aarch64 `libcdsprpc.so` are present. P0.2 is done; P1/P2 (fastrpc + v66 skel) are next.
The Hexagon SDK 6.7 in the public toolchain image **cannot target v66**
(`hexagon-clang -mv66` is rejected; its arch table is `v68..v85`; QURT libs are
`computev68+`). **The public 6.4.0.2 SDK is also v68+ (verified), so the v66
build needs Hexagon SDK 5.4.0** (QPM3, owner-provided). The host-side
`libcdsprpc.so` for glibc aarch64 *is* in the image and
is clean (only libc/libm/libstdc++), and the kernel side is already in place.
**Upstream contribution is off (decision 2026-11-18):** the port stays in a
private fork, so llama.cpp's issue-first and AI-disclosure rules no longer gate
the work; the technical gates (G0-G5) and the private-fork hygiene rules in §10
still apply.
No phone write has been made for this plan. The phone runs the stable kernel
`7.3.0-rc6-omamobile-push1-g3667ab368ef9`, the `omamobile-ai` supervisor serves
`LFM3-1.7B` on the CPU (15.47 prompt / 10.6-11.0 generation tok/s), and the
Adreno 650 is on Turnip for render.
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
2. **Private fork only (decision 2026-11-18).** No issue, no upstream PR, no
   llama.cpp contribution. The port lives in `santiago200031/llama.cpp` (or a
   local clone) and is packaged for this phone; there is no maintainer gate.
3. **Keep the fork small and testable.** Commits stay separable: build v66,
   guard HMX, fix HVX assumptions, docs. The v66 build stays opt-in so the
   fork can still build a normal v68+ tree with SDK 6.7.
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

**P0.2 A v66-capable SDK (gate G0) — PASSED (2026-11-18).**

- The QSC license was activated with
  `qsc-cli sdk license activate --name HexagonSDK5.x` (ACTIVE to 2030).
- `qsc-cli sdk install` still returned `RebootRequired` after a reboot; the
  `.qik` was extracted with `qik extract` as root (`pkexec`), which installed
  HexagonSDK5.x.Core.
- SDK **5.4.2.0** at `/local/mnt/workspace/Qualcomm/Hexagon_SDK/5.4.2.0`,
  `HEXAGON_Tools/8.7.06`; `hexagon-clang -mv66` compiles
  (`__HEXAGON_ARCH__ 66`) with `libtinfo.so.5` → `/usr/lib/libtinfo.so.6`
  symlinked in a local compat dir; `rtos/qurt/computev66`;
  `ipc/fastrpc/remote/ship/UbuntuARM_aarch64/libcdsprpc.so`.
- The `.qik` download is `~/omamobile-kernel/hexagon-sdk-5.4/`; the extracted
  SDK is under `/local/mnt/workspace` (the QIK default). Neither is committed.

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

**P0.5 Upstream issue — dropped (2026-11-18).** The port stays private, so no
issue and no PR are opened with `ggml-org/llama.cpp`. The P0 facts above become
the fork's README and commit evidence instead. If that decision ever changes,
the issue-first, AI-disclosure and human-written-text rules in the original
analysis apply.

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

### P2 — Build the v66 HTP skel (laptop, container; private fork)

**Status (2026-11-18): configured; the v66 compiler stops at the v68 float
ISA, and a probe shows v66 HVX is integer-only.** The v66 skel configures with
SDK 5.4.2 once `qaic` is symlinked to `ipc/fastrpc/qaic/bin/qaic` (the SDK
ships it under `qaic/Ubuntu20/qaic`), and the Hexagon toolchain starts
compiling. It then stops at the QFloat/HMX code: `hvx-base.h` uses `Q6_Vqf32*`,
`Q6_Vsf_equals_Vqf32` and `Q6_Wqf32_vmpy_VhfVhf`, and the kernels carry **664
QFloat uses and 860 HMX references** (`matmul`, `flash-attn`,
`gated-delta-net`, `conv2d`).

That is not a missing include: `hvx_hexagon_protos.h` guards **all** `Q6_Vsf_*`
and `Q6_Vhf_*` arithmetic at `__HVX_ARCH__ >= 68`, and QFloat at 68 too. A
compile probe with `hexagon-clang -mv66 -mhvx -mhvx-length=128b`
(`__HVX_ARCH__ 66`) fails for `Q6_Vsf_vadd_VsfVsf`, `Q6_Vsf_vmpy_VsfVsf`,
`Q6_Vhf_vadd_VhfVhf`, `Q6_Vhf_vmpy_VhfVhf`, `Q6_Vqf32_*` and `Q6_Vqf16_*`; only
the integer HVX intrinsics remain. The v68 compiler (6.7) has no v66 target.
`GGML_HEXAGON_NHMX=0` only selects the runtime HMX path; it does not remove the
compile-time HMX/QFloat code.

**So the v66 port is an integer-only kernel rewrite**, not a build flag and not
just a "fallback set": the ggml-hexagon kernels represent activations and
accumulators as `sf`/`hf`/QFloat, and v66 has none of them. A real v66 matmul
would dequantize to integer and use `Q6_Vw_vrmpy*` (uint8×int8 dot), i.e. a new
kernel family beside the v68+ one. P2 therefore splits:

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

### P2a — FastRPC on the phone (gate G1): the stack is already up

Read-only check on the stable system (2026-11-18): `fastrpc` is loaded,
`remoteproc2` (`adsp`) and `remoteproc1` (`cdsp`) are **running**, the
`/dev/fastrpc-{adsp,cdsp,cdsp-secure,sdsp}` nodes exist, and `hexagonrpcd` is
installed. The packaged instance serves only `sdsp`
(`/usr/bin/hexagonrpcd -f /dev/fastrpc-sdsp -d sdsp -s -R /usr/share/qcom/sm8250/samsung/r8q`),
so a CDSP test needs a second instance on `/dev/fastrpc-cdsp -d cdsp` with the
same `-R` root (or a root that contains the test skel). Nothing here writes
boot or UFS; starting a CDSP instance and loading a user PD is a phone change
and asks for a go.

### P1 result (2026-11-18): the v66 skel builds; the host ABI is the blocker

**The v66 skel works.** The SDK 5.4.2 `examples/calculator` built for v66 after
two packaging fixes (`-DPREBUILT_LIB_DIR=toolv21_v66`, the `qaic` symlink):
`libcalculator_skel.so` is a valid Hexagon DSP6 ELF (14 KB). That closes the
skel half of G1.

**The host half is a userspace ABI mismatch, not the DSP.** The phone's
FastRPC userspace is **`hexagonrpc 0.5.0`** (`/usr/bin/hexagonrpcd`,
`libhexagonrpc.so`: the raw `fastrpc`/`fastrpc2` ioctl wrappers), not
Qualcomm's `libcdsprpc`. The SDK's `libcdsprpc.so` (UbuntuARM_aarch64) against
the mainline driver says:

```
ERROR 0xffffffff: FastRPC Capability API failed. Falling back to signed pd.
Overriding user request for unsigned PD. Only signed offload is allowed on domain 3.
```

The kernel driver's `fastrpc_get_info_from_kernel()` indexes only the DSP's
attribute array; the SDK library asks for the driver-owned attributes
`DOMAIN_SUPPORT (0)` and `UNSIGNED_PD_SUPPORT (1)` (`incs/remote.h`'s
`enum remote_dsp_attributes`). `cctx->unsigned_support` is `true` for CDSP and
GDSP in the mainline driver, but the capability ioctl never returns it, so the
library concludes "signed only" and refuses the unsigned skel.

**Next (kernel, laptop):** special-case `DOMAIN_SUPPORT` and
`UNSIGNED_PD_SUPPORT` in `fastrpc_get_info_from_kernel()` to answer from the
driver (`1` and `cctx->unsigned_support`), leaving the other attributes to the
DSP. Then retest the calculator. If the SDK library's next step, `rpcmem_alloc`
("Allocate 64 bytes from ION heap", `0x68`), still fails because there is no
`/dev/ion` (the phone has `/dev/dma_heap/{system,reserved}`), write a minimal
host that calls `calculator_sum` with a normal array and no rpcmem.

The v66 skel itself is not the problem, and no DSP fault appeared. The
`hexagonrpc` daemon runs on `/dev/fastrpc-cdsp`; the calculator opens domain 3.

### P1 second result (2026-11-18): the SDK Linux lib is a stub; the
### mainline path is `libhexagonrpc`

The capability fix was built into `7.3.0-rc6-omamobile-push1-gcd919933b72c`
and did not change the result. Reading the SDK library settled why:

- `ipc/fastrpc/remote/ship/UbuntuARM_aarch64/libcdsprpc.so` is a **link-time
  stub**: `remote_handle64_open`, `remote_handle64_invoke`,
  `remote_session_control` and `remote_handle64_control` all `mov w0, #-1;
  ret`, `rpcmem_alloc` returns NULL. The calculator never reached the DSP; its
  "capability API failed" message was the stub returning -1 for every call.
- The Android ship (`android_aarch64/libcdsprpc.so`) is the real library, but
  it is bionic and needs the Android runtime, not the Arch root.

The mainline path is the phone's **`hexagonrpc`** userspace
(`libhexagonrpc.so` + `hexagonrpcd`). Its repo master provides the client API
the SDK stub lacks: `hexagonrpc_open(fd, "libcalculator_skel.so", &hdl, ...)`,
a method definition (`HRPC_ARG_BLOB_SEQ` in, `HRPC_ARG_OUT_BLOB` out) and
`hexagonrpc()`. The installed `hexagonrpc 0.5.0` predates those symbols, so a
test binary built with the repo's `error.c handle.c hexagonrpc.c session.c
interface/remotectl.c` links and runs (it was built on the phone as
`/tmp/hrpc/calc_hrpc`).

What is not settled: `hexagonrpcd -f /dev/fastrpc-cdsp -d cdsp -R
/usr/share/qcom -p /tmp/hrpc/calc_hrpc` exits with no output. The `-p` mode
runs the client on the **DSP remotectl listener**; that listener exists on the
ADSP/SDSP (the sensors use it) but the CDSP may not expose one, or the installed
0.5.0 daemon does not speak the repo master's listener protocol. The next step
is to run the **repo master's** `hexagonrpcd` (or read its `-p`/listener code)
and confirm the CDSP listener; that is a focused session, not a quick test.

The kernel capability patch stays: it is the downstream behaviour and the
vendor/bionic library would use it, but it is not on the `hexagonrpc` path.

### P1 retest runbook (after the driver fix; needs the human's go for writes)

The kernel fix is `misc: fastrpc: answer the domain and unsigned-PD capability
attributes` on branch `r8q-npu-fastrpc` (off `r8q-modem-msi`). The retest uses
the userdata root; no modem overlay is needed.

0. **Read-only:** `uname -r`, `findmnt -no SOURCE /` (expect `/dev/sda36`),
   `cat /sys/class/thermal/thermal_zone*/type` for the temperature.
1. **Modules (write):** the tree is at the fix, so the release is
   `7.3.0-rc6-omamobile-push1-gcd919933b72c`. Install the tarball directly (the
   root already has hand-installed modem modules, which `modules-push`'s
   hand-edit guard would refuse):
   `zstd -dc ~/omamobile-kernel/out-r8q-npu/modules-7.3.0-rc6-omamobile-push1-gcd919933b72c.tar.zst | tools/phone-ssh 'tar -C /usr/lib/modules -xf - && depmod -a 7.3.0-rc6-omamobile-push1-gcd919933b72c'`.
   Keep sddm stopped (`systemctl disable --now sddm`) so the next boot stays
   on the console and cannot restart the login loop.
2. **Image (laptop):**
   `KERNEL_OUT=~/omamobile-kernel/out-r8q-npu tools/r8q-boot pack ~/omamobile-kernel/out-r8q-npu/recovery-npu-fastrpc.img`
   It must report `userdata` and `omamobile-root` in the ramdisk and no
   `omamobile.loop=` in the cmdline (the native image).
3. **Flash (write, needs go):** `tools/r8q-flash-recovery <image>`; readback sha
   must match.
4. **Reboot (needs go):** the native image boots `/dev/sda36` again.
5. **Test:** push the host tarball again (`/tmp` does not survive a reboot),
   build it with the malloc shim (the phone has no `/dev/ion`):
   `gcc -O2 -I. calculator_stub.c calculator_main.c calculator_test.c dsp_capabilities_utils.c pd_status_notification.c rpcmem_malloc.c -o calculator -L. -lcdsprpc -ldl -lpthread -Wl,-rpath,/tmp/calc-host`,
   start the CDSP daemon (`hexagonrpcd -f /dev/fastrpc-cdsp -d cdsp`), then
   `cd /tmp/calc-host && LD_LIBRARY_PATH=/tmp/calc-host ./calculator -d 3 -U 1 -r 0 -n 16`.
   **Expect/pass:** no "falling back to signed pd"; the unsigned PD opens and
   the sum returns. **Stop:** a DSP fault, a PD load failure, or a reset; the
   stable image is the way back.

If the only remaining error is the ION allocation (`0x68`), replace the test
with a minimal host that calls `calculator_sum` with a normal array and no
`rpcmem` (the IDL stub copies the sequence); record that as the G1 pass.

### P2b — v66 integer matmul probe (the decision gate)

The port only makes sense if integer HVX beats the CPU on the same quantized
matmul. Before touching ggml's kernels, build one small v66 shared object that
does a Q4_0×Q8_0-style `Q6_Vw_vrmpyacc_VwVubVb` dot product over a few MB of
weights, load it through `libcdsprpc.so` from the SDK
(`ipc/fastrpc/remote/ship/UbuntuARM_aarch64/`), and time it on the phone
against `llama-bench`'s CPU number for the same shape. If the DSP is not
clearly faster, park the NPU track with the measurement instead of starting
the rewrite.

**Expect/pass (probe):** the CDSP loads the unsigned PD, the checksum matches
the CPU reference, and the measured GMAC/s exceeds the CPU's Q4_0 matmul.
**Stop:** a DSP fault, a signature/secure-PD refusal, or throughput at or below
the CPU. Record it and park P3/P4.
**Risk:** low (a user PD on a running CDSP; no boot change).

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

### P6 — Upstream — dropped (private fork)

The private fork is the deliverable. No issue, no PR and no upstream text. Keep
the fork tagged (`r8q-v66`) with a README that records the build/test evidence,
the SDK 5.4 requirement and the phone's results; package it as
`omamobile-llama-hexagon` for this phone. If the decision ever changes, the
upstream rules from the original analysis apply again.

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
| P0.2 v66-capable SDK (5.4.2) | G0 | **done** | SDK 5.4.2; `-mv66`; `computev66`; `libcdsprpc` aarch64 | 2026-11-18 |
| P0.3 phone surface | G0 | **done** | fastrpc + cdsp running; `/dev/fastrpc-cdsp`; hexagonrpcd (sdsp) | 2026-11-18 |
| P0.4 license | G0 | **done** | HexagonSDK5.x ACTIVE to 2030 (qsc-cli) | 2026-11-18 |
| P1 libcdsprpc + hello PD | G1 | **in progress** | needs a cdsp `hexagonrpcd` + user PD | 2026-11-18 |
| P2 v66 skel build | G2 | **blocked: v66 is integer-only** | v68+ sf/hf/QFloat/HMX; probe fails all float intrinsics | 2026-11-18 |
| P2b integer matmul probe | G2b | **next — decision gate** | measure vrmpy GMAC/s vs CPU before the rewrite | |
| P3 MUL_MAT + small model | G3 | planned | `logs/r8q/npu-p3-ops.log` | |
| P4 LFM benchmark | G4 | planned | `logs/r8q/npu-p4-lfm.log` | |
| P5 integration | G5 | planned | package + `AI-BENCHMARKS.md` | |
| P6 upstream | — | **dropped (private fork)** | fork tag `r8q-v66` | 2026-11-18 |
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
public word and every submission. The llama.cpp port stays in a private fork,
so there is no upstream gate; the fork still keeps the project's license, its
notices and a reproducible build, and this repository keeps its own package and
kernel rules.

### 10.1 llama.cpp private fork (no upstream)

There is no issue, PR, AI disclosure or maintainer review. The fork still
keeps:

| Rule | What it means here |
|---|---|
| **License** | Keep llama.cpp's MIT license and notices. |
| **No vendor blobs** | `libcdsprpc.so`, `cdsp.mbn`, SDK files and prebuilt skels stay out of git; the build fetches the SDK or the user supplies it. |
| **Small commits** | Build v66, HMX guards, HVX fixes and docs, each explaining why and what was tested. |
| **Tests** | `test-backend-ops` (CPU vs HTP), a small model, LFM, `llama-perplexity` and `llama-bench`; record the results in the fork's README. |
| **Opt-in v66** | `GGML_HEXAGON_V66=ON`, so the same fork still builds v68+ with SDK 6.7. |
| **Reproducible** | Pin the llama.cpp commit, the SDK 5.4 path/version and the CMake command. |
| **Package** | `omamobile-llama-hexagon`, versioned, with `test/all`. |
| **Comprehension** | The human still reads every line and can explain the change; the agent walks through the design privately. |

If the upstream decision ever changes, the original rules apply again:
issue-first, AI disclosure, human-written public text, no agent push/PR, and
the llama.cpp code-review skill.

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
- [ ] G1: unsigned-PD hello on the CDSP
- [ ] G2: v66 skel builds, no HMX instructions
- [ ] G3: `MUL_MAT` + small model correct
- [ ] G4: LFM benchmark + 30-minute soak
- [ ] G5: package + one-command CPU rollback
- [ ] Fork README: pinned llama.cpp commit, SDK 5.4 path/version, CMake command
- [ ] No vendor blobs in the fork or the package; MIT license kept
- [ ] `test-backend-ops`, perplexity and bench results recorded
- [ ] `omamobile-reviewer` (this repo) and `tools/pr-check`/`tests/all`/`tests/packages`/`tools/phone-smoke` for the package

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
