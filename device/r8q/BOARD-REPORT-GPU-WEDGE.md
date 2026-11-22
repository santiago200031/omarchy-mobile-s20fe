<!-- SPDX-License-Identifier: MIT -->
# Adreno 650 / GMU / Vulkan blocker on r8q (SM8250) —investigation report

Scope: laptop-side investigation only (no phone). Kernel mainline tree:
`/home/amd-omarchy/omamobile-kernel/linux` @ `2db2bc35b` ("Linux 7.3-rc6").
Worktree `wt-r8q`: no adreno/GMU changes (latest adreno-side commit on
`omamobile` is mainline `62b3c799e`; wt-r8q top commits are r8q MHI work).

## (A) The wedge itself

Primary record: [`board/failed/08-vulkan-llm-adreno-gpu-wedge.md`](board/failed/08-vulkan-llm-adreno-gpu-wedge.md).
Related class notes: [`board/failed/FAILED.md` L17], [`NOTES.md` §17] (2026-11-18),
[`CAMERA-PLAN.md` ~250], [`MISSING.md` P3].

**What was tried:**
- Mesa 26.2.4 Turnip (`vulkan-freedreno`), `llama-server --list-devices` reports
  `Vulkan0: Turnip Adreno (TM) 650 (4160 MiB)`; `llama.cpp` built with
  `-DGGML_VULKAN=ON`, `LFM3-1.7B-Q4_K_M`, pinned **305 MHz**.
- During `llama_model_load` (first matmul of the 1.7B Q4_K_M, not at a trivial render):
  - `ggml_vulkan: device lost on Vulkan0` → `vk::Device::waitSemaphores: ErrorDeviceLost`.
  - Kernel: `msm_dpu … hangcheck detected gpu lockup rb 0!` (submitted fence stuck,
    e.g. completed fence 5917 / submitted fence 5923).
  - `adreno 3d00000.gpu: [drm:a6xx_recover [msm]] *ERROR* cx gdsc didn't collapse`
    — the GMU **CX power-domain** recovery fails to cycle; `recover_worker` loops.
- The wedge persisted: `vkcube` then **segfaulted** (`TU: warning: Failed to set BO
  name with DRM_MSM_GEM_INFO: -2`), and hangcheck started flagging **`Hyprland`** as
  the offending task, taking `hyprland` + `omamobile-shell` down.
  **Recovery required a reboot.**

**Root-cause read (from the card):** not a frequency problem (it faulted at the
lowest OPP, 305 MHz, during the first real Vulkan compute/memory pass). It is
"the Vulkan compute/LLM path that wedges" while "the Vulkan render path
(vkcube / Hyprland compositing) is stable at 587 MHz" (from `NOTES.md` §17 and
`AI-BENCHMARKS.md`: Vulkan 1.3.354 measured, 245 GFLOP/s at 587 MHz, 127.6 GFLOP/s
at 305 MHz, zero GMU/RPMh/GPU errors).

**Permanent rules (card 08):**
- Keep `GPU_LAYERS=0` (CPU) as the default; don't set GPU layers until a Vulkan LLM
  run is clean.
- Treat `hangcheck gpu lockup` + `cx gdsc didn't collapse` as **reboot-required**,
  not live-recoverable.

**Evidence:** `/root/llama/bench305.txt` (on the phone), dmesg
hangcheck/`a6xx_recover` lines, `vkcube` segfault on the same boot.

## (B) Mainline status of Adreno 650 / GMU / Vulkan on SM8250

**Is Adreno 650 (a650) in mainline on SM8250? Yes.**
- `arch/arm64/boot/dts/qcom/sm8250.dtsi`: `gpu: gpu@3d00000` —
  `compatible = "qcom,adreno-650.2", "qcom,adreno";` (line 2941); `gmu: gmu@3d6a000` —
  `compatible = "qcom,adreno-gmu-650.2", "qcom,adreno-gmu";` (line 3012).
  (Both `status = "disabled"` in mainline sm8250.dtsi — see below.)
- Driver: `drivers/gpu/drm/msm/adreno/a6xx_gpu.c` / `a6xx_gmu.c` (plus
  `a6xx_gpu_state.c`, `a6xx_hfi.c` — `a650_build_bw_table` at `a6xx_hfi.c:821`;
  `adreno_is_a650` = `ADRENO_6XX_GEN3` at `adreno_gpu.h:464`). Adreno 405/640/650
  landed in mainline 5.8 (Phoronix, "Adreno-405-640-650-Linux-5.8").
- Mainline freedreno works on SM8250 devices: e.g. Xiaomi Mi 10 mainline
  (`colorgarden/umi-mainline`: "GPU (Adreno 650) **works**", devfreq 305–587 MHz,
  `a650_*` firmware); postmarketOS `sm8250-mainline` exists for SM8250.

**GMU/CX-gdsc handling in mainline** (what the wedge is about):
- Mainline `a6xx_recover` (`a6xx_gpu.c:1569`): halts SQE, `bus_halt` + RBBM SW
  reset, then `reinit_completion(&gmu->pd_gate);
  dev_pm_genpd_add_notifier(gmu->cxpd, &gmu->pd_nb);
  dev_pm_genpd_synced_poweroff(gmu->cxpd);` drops the adreno-gpu device pm refs
  (`pm_runtime_put`/`pm_runtime_put_sync` on `gpu->pdev->dev`) and waits **1 s**
  for the genpd's `GENPD_NOTIFY_OFF` — `cxpd_notifier_cb` (`a6xx_gmu.c:2163`)
  completes `pd_gate` on `GENPD_NOTIFY_OFF`. On timeout:
  `adreno 3d00000.gpu: *ERROR* cx gdsc didn't collapse` (`a6xx_gpu.c:1627`).
  Mainline has **no** polling-based cx-collapse via gpucc reset control
  (`cx_collapse` — only `drivers/soc/qcom/qcom_aoss.c:502 "prevent_cx_collapse"`
  exists; `dev_pm_genpd_synced_poweroff` in `drivers/pmdomain/core.c:661` is just
  a flag).
- Mainline SM8250 `gmu` node: `power-domains = <&gpucc GPU_CX_GDSC>,
  <&gpucc GPU_GX_GDSC>` ("cx"/"gx"). `dev_pm_domain_attach_by_name(dev, "cx")`
  (`drivers/pmdomain/core.c:3539`) matches the gmu node's `power-domain-names`.
- Mainline SM8250 GDSCs (`drivers/clk/qcom/gpucc-sm8250.c`):
  - `gpu_cx_gdsc` (0x106c, `gds_hw_ctrl = 0x1540`): `.flags = VOTABLE`, `PWRSTS_OFF_ON`.
    Mainline VOTABLE semantics (`drivers/clk/qcom/gdsc.c:167,480`): off is
    fire-and-forget; **ON when any voter votes**. Other CX voters on SM8250:
    `adreno_smmu` (sm8250.dtsi ~3087: `power-domains = <&gpucc GPU_CX_GDSC>`) and
    the GMU itself.
  - `gpu_gx_gdsc` (0x100c, `CLAMP_IO | AON_RESET | POLL_CFG_GDSCR`,
    `power_on = gdsc_gx_do_nothing_enable`) — on A6x/A7x the GX rail is voted by
    the GMU (`a6xx_gmu_gxpd_get`), not the CPU.

**Default-enablement quirk (matters for "mainline status"):**
- Mainline `sm8250.dtsi`: adreno-650.2 + gmu are `status = "disabled"` by default
  (patch by Konrad Dybcio, 2021, "[PATCH] arm64: dts: qcom: sm8250: Disable
  Adreno and Venus by default").
- Mainline SoC dts files that **enable** them: `sm8250-mtp.dts` (`&gmu`/`&gpu` at
  lines 481/485), `sm8250-hdk.dts` (368/372). Mainline `sm8250-samsung-r8q.dts`
  does **not** enable them, and mainline `sm8250-samsung-common.dtsi` has no
  adreno/gpu references — so on mainline, the S20 FE's Adreno is off unless the
  SoC dts enables it. Our port kernel has the Adreno enabled (success card 04).

## (C) Why the wedge breaks Vulkan and renderD128

**renderD128 on mainline SM8250: yes.** Mainline `msm_drm` probes `gpu@3d00000`
(Adreno-650.2) → DRM card + render node `renderD128`. On the S20 FE with Adreno
enabled, `/dev/dri/card0` + `/dev/dri/renderD128` both exist; the Adreno is the
card's GPU (success card 04 measured this on the port: `Vulkan0: Turnip Adreno
(TM) 650`, `msm_dri.so` loaded, `Vulkan 1.3.354`).

**The wedge, in order (wedge-card sequence on the port kernel):**
1. A Vulkan **compute** submit (llama.cpp matmul during `llama_model_load`)
   trips the Adreno lockup → hangcheck fires (`msm_gpu`: "gpu lockup rb 0");
   Turnip reports `ErrorDeviceLost` during `llama_model_load`; `vkcube`
   segfaults; the session (Hyprland + omamobile-shell) goes down — hangcheck
   blames the first task touching the GPU after the hang ("offending task:
   Hyprland").
2. Kernel recovery: `a6xx_recover` resets the RBBM, then tries to cycle the
   GMU CX gdsc — `cx gdsc didn't collapse` within 1 s. The **CX rail never
   collapses** → the in-place GPU reset can't complete → `recover_worker`
   loops → GPU stays wedged → **reboot required** (card 08 permanent rule).
3. Why CX can't collapse on mainline (mainline structures, per the wedge card +
   the upstream `adreno_smmu`/`GPU_CX_GDSC` setup): the `adreno_smmu` holds a
   runtime-pm ref on the "cx" genpd (sm8250.dtsi ~3087). Mainline's recover
   drops the **adreno gpu device's** pm refs but nothing visibly suspends the
   SMMU/gpu device during recovery, so the SMMU's vote keeps `GPU_CX_GDSC` ON —
   the genpd never emits `GENPD_NOTIFY_OFF` — the 1 s wait in `a6xx_recover`
   fails. This exact class is named upstream (see D-H1: kernelthread series
   "Currently, cx doesn't collapse due to a devlink between gpu and its smmu").
4. Why *render* is fine at 587 MHz but *compute* wedges (wedge-card root read +
   NOTES §17 + AI-BENCHMARKS): the render path (vkcube, Hyprland compositing)
   never trips the lockup on this GPU; the first real compute/memory pass (LFM3
   matmuls at low clocks) does. Keep the two repros separate when triaging.

## (D) The plan

### Ranked hypotheses for the wedge (port kernel 7.3.0-rc6-omamobile, = mainline)

**D-H1 — SMMU devlink vote keeps the "cx" gdsc ON during recovery (most likely,
upstream-named).** Mainline's `a6xx_recover` drops the adreno gpu device's pm
refs and waits 1 s for the genpd's `GENPD_NOTIFY_OFF`, but nothing drops the
**SMMU's** vote: `adreno_smmu` is a consumer of `GPU_CX_GDSC`
(sm8250.dtsi ~3087). Upstream kernelthread v4 0/7 "Improve GPU Recovery"
(Akhil P Oommen, 2022) says verbatim: *"Currently, cx doesn't collapse due to a
devlink between gpu and its smmu. So the *struct gpu device* needs to be
runtime suspended to ensure that the iommu driver removes its vote on cx
gdsc."* Mainline matches the "cx doesn't collapse" premise. Fix (mainline or
port): in `a6xx_recover`, suspend/retire the SMMU (or adreno gpu) device so the
SMMU removes its cx vote before/with the collapse wait — e.g.,
`pm_runtime_suspend(&gpu->pdev->dev)` during the pd_gate wait, or a port patch
mirroring v4 4/7 on `wt-r8q`.

**D-H2 — Adreno 650 GX domain not fully off / GEMNoC race on SM8250.**
Mainline has `gpu_gx_gdsc` (`CLAMP_IO | AON_RESET | POLL_CFG_GDSCR`,
`power_on = gdsc_gx_do_nothing_enable`) and `a6xx_gmu_gxpd_get/put` keeps GX
voted via the GMU on A6x. A race where GX stays up during recovery is a weaker
variant of the collapse failure; checkable via the same dmesg grep for
`status stuck` / GDSC polls.

**D-H3 — Turnip/Mesa compute bug on Adreno 650 (user-space).** Would *not*
produce the kernel line `adreno 3d00000.gpu: [drm:a6xx_recover [msm]] *ERROR*
cx gdsc didn't collapse`. The wedge card's root read names the kernel GMU class
("the `cx gdsc didn't collapse` points at the GMU CX power domain refusing to
cycle"), so kernel-side is the prime suspect. Verify by checking whether the
cx-gdsc line is present in the wedged dmesg.

**D-H4 — SM8250-specific Adreno 650-1341 quirk.** Adreno 650 is used on SM8250
mainline without documented compute-on-mainline hangs; qualitative, last.

### Phone-side GPU investigation plan (the human runs this on the S20 FE when
it's up; all read-only unless noted)

Baseline (before any wedge, no reboot yet):
1. `ls -l /dev/dri/ && ls /sys/class/drm/` — confirm `card0` + `renderD128` exist
   (Adreno).
2. `dmesg | grep -E "adreno|msm_gpu|hangcheck|a6xx|recover|cx gdsc|gpu lockup"` —
   confirm Adreno 650 + GMU probe cleanly (no earlier wedge lines).
3. `lsmod | grep -iE "adreno|msm"`.
4. Re-run the render path (stable at 587 MHz per card 08): `vkcube` (a few
   frames); record the result.

Wedge repro + capture (ask human first; wedge needs reboot to clear):
5. Run the failing repro: `llama-bench -m LFM3-1.7B-Q4_K_M.gguf -ngl 99 -p 512 -n 128`
   at the pinned 305 MHz (as in Failed 08); expect `device lost on Vulkan0` /
   `ErrorDeviceLost`. (On the port: `llama-bench` is at `/usr/bin/llama-bench`
   from `omamobile-llama`.)
6. `dmesg | grep -E "gpu lockup|cx gdsc|hangcheck|a6xx_recover|offending task"`
   — confirm the wedge signature: `hangcheck detected gpu lockup rb 0`,
   `cx gdsc didn't collapse`, the recovery-worker loop.
7. `ls /dev/dri/` + `ls /sys/class/drm/` after the wedge — GPU still present.
8. Note GPU freq if observable: the port pins 305 MHz via
   `80-r8q-gpu.rules` (`simple_ondemand`, min+max on `3d00000.gpu`); on
   mainline the adreno is in `power/control` + devfreq if enabled.
9. Reboot (permanent rule). Repeat baseline steps 1–4 — confirm render path
   still works (it should).

Fix attempt + re-verify:
10. Install the upstream-or-port fix (see D-H1) when ready; on the phone:
    - `ls -l /dev/dri/` (renderD128 still there),
    - re-run repro from step 5 — expect: model loads, benchmark completes,
    - `vkcube` again (render stable),
    - `dmesg` — no new `cx gdsc didn't collapse` lines.
11. Capture `llama-server --list-devices` output + `vulkaninfo --summary`
    (if available) as evidence of Vulkan-on-Adreno-650.

### Recommended next steps (for when a fix is ready)

1. If mainline has the fix (watch: kernelthread "Improve GPU Recovery" v4/v5,
   any 2026 mainline patches on `a6xx_recover`/`adreno_smmu`/`GPU_CX_GDSC`
   collapse during recovery): rebase/import it; verify on the phone per
   D-10–11.
2. Else, write the port fix on `wt-r8q` (target D-H1): suspend the adreno gpu
   device/SMMU during `a6xx_recover`'s collapse wait, add a trace log of the
   GDSC status (`gpu_read(REG_A6XX_GPU_GMU_CX_GMU_RPMH_POWER_STATE)`? main
   — mainline has `REG_A6XX_GPU_GMU_CX_GMU_RPMH_POWER_STATE` reads in
   `a6xx_gmu_notify_slumber`), test on the phone.
3. If the wedge only reproduces via a *specific* Turnip/Mesa path (26.2.4 vs
   26.2.7): bisect user-space (Mesa 26.2.7 worked on desktop GLX), confirm the
   kernel-side signature (cx-gdsc line) before blaming the GPU.
4. Update the board card (new date + kernel release via `uname -r`), then lift
   the `GPU_LAYERS=0` default once a clean Vulkan LLM run is recorded.
5. Related-but-separate follow-up (context): the `debayer_egl` Adreno hangchecks
   from libcamera soft-ISP (CAMERA-PLAN ~250, AI-BENCHMARKS ~156) are a
   different issue (`SWIspWorker` lockups on the Adreno).

Evidence files on the phone: `/root/llama/bench305.txt` (pre-existing, Failed 08),
`dmesg` wedges, `vkcube` segfault, and (new) the repro/fix-run dmesg above.


