<!-- SPDX-License-Identifier: MIT -->
# UPSTREAM-KERNEL-PLAN: r8q topic branch split from r8q-base-dts-fold

**Date:** 2026-11-26 (rebased onto `cf22a9090`, legal/DCO cleanup, checkpatch & DT schema pass)
**Status:** 8 topic branches rebuilt cleanly on `origin/omamobile` (`cf22a9090`); merged cleanly in `r8q-merge-test2`; CI builds, `dt_binding_check` (5/5), & `CHECK_DTBS=y` (`0` `r8q` schema warnings) pass; awaiting human `Signed-off-by:` on 60 unique unsigned commits + attended flash touch check of `ts-int-default-state output-disable;`
**Source:** `r8q-base-dts-fold` → 8 topic branches (`68` unique commits / `74` branch sum incl. 3 shared base DTS dependencies on `touch` & `cameras`, after dropping 3 unmerged Qualcomm `cmd-db`/RSC diagnostic commits, replacing them with 2 native lock-safe `rpmh-rsc` patches, splitting `hynix,hi847.yaml`, and excluding 2 revert pairs)
**Base:** `origin/omamobile` (`cf22a9090`)

## 1. Overview

This plan documents the audit and split of `~/omamobile-kernel/wt-r8q` branch `r8q-base-dts-fold` into clean per-subsystem topic branches against `origin/omamobile` (`cf22a9090`).

**Shared-code & DCO guarantees verified:**
- `drivers/soc/qcom/cmd-db.c`, `include/soc/qcom/cmd-db.h`, and `arch/arm64/boot/dts/qcom/sm8250-samsung-common.dtsi` (`x1q`) have **0 bytes diff** against `origin/omamobile` (`cf22a9090`).
- Upstream author `Akhil P Oommen`'s 6 commits on `topic/r8q-gpu` and existing human-signed commits (`d9b8b35d5` on `topic/r8q-audio`, `WWAN_PORT_SAHARA` on `topic/r8q-modem`) are preserved verbatim.
- `Assisted-by: Gemini 4 Hydrogen` is added cleanly to rebuilt unsigned commits (`DeepSeek V4.1 Flash (OpenCode)` preserved alongside on `18a37fa19`); **0** AI `Signed-off-by:` trailers.

**Revert pairs excluded** (as specified in [krn-03]/[krn-04]):
- `6085e0349` / `f7f3096b1`: Revert + original for "pci: dwc: prepare PCIe2 MSI in the endpoint IOMMU domain"
- `0fd2c260d` / `6234db2d0`: Revert + original for "bus: mhi: host: force one M3->M0 state cycle at mission-mode entry"

## 2. Topic Branches

### topic/r8q-qcom-core (9 commits)
**Subsystem:** max77705 pass2, native rpmh-rsc timeout dump & reclaim, ufs writebooster, sdhci 64-bit DMA, gpi QUP0/r8q EE offsets, geni serial, gcc_gp2 clk_rcg2_gp_ops (`cmd-db` untouched)

| # | Subject | Status |
|---|---------|--------|
| 1 | mfd: max77705: accept PASS2 silicon revision (0x02) | Rebuilt cleanly (`0` warn) |
| 2 | soc: qcom: rpmh-rsc: dump active TCS state without holding lock on timeout | Native replacement (`cmd-db` untouched) |
| 3 | soc: qcom: rpmh-rsc: bound TCS claim wait and reclaim stuck TCSes | Native lock/race-safe reclaim (`0` warn) |
| 4 | scsi: ufs: core: disable WriteBooster when dCurWriteBoosterBufferSize returns -EINVAL | Rebuilt cleanly (`0` warn) |
| 5 | mmc: sdhci-msm: disable 64-bit DMA on SM8250 | Rebuilt cleanly (`0` warn) |
| 6 | dmaengine: qcom: gpi: set QUP0 ee_offset to 0x1000 on SM8250 | Rebuilt cleanly (`0` warn) |
| 7 | tty: serial: qcom-geni: clamp negative of_alias_get_highest_id() in get_port_from_line() | Rebuilt cleanly (`0` warn) |
| 8 | clk: qcom: gcc-sm8250: switch gcc_gp2_clk_src to clk_rcg2_gp_ops | Rebuilt cleanly (`0` warn) |
| 9 | dmaengine: qcom: gpi: set QUP EE offsets for samsung,r8q | Restored (`0` warn) |

### topic/r8q-dts (3 commits)
**Note:** Shared 3-commit base DTS dependency for `topic/r8q-touch` and `topic/r8q-cameras` (`sm8250-samsung-common.dtsi` untouched; XPU `no-map` and `&framebuffer` overrides kept in `sm8250-samsung-r8q.dts`; `ts_int_default` updated from `input-enable;` to `output-disable;`).
**Subsystem:** sm8250-samsung-r8q.dts base hardware & folded verified overlays

| # | Subject | Status |
|---|---------|--------|
| 1 | arm64: dts: qcom: sm8250-samsung-r8q: flesh out hardware nodes and fix pm8150l gpio11 | Clean (`0` `r8q` schema warnings) |
| 2 | arm64: dts: qcom: sm8250-samsung-r8q: fold verified overlay DT fixes into base DTS | Clean (`output-disable;` on `ts_int_default`) |
| 3 | arm64: dts: qcom: sm8250-samsung-r8q: fold verified subsystem overlays | Clean (multi-blank lines normalized) |

### topic/r8q-touch (4 commits; 1 touch + 3 shared base DTS)
**Subsystem:** stmfts 16-byte Type-B support

| # | Subject | Status |
|---|---------|--------|
| 1–3 | `topic/r8q-dts` base DTS dependency (`e897496c3..9d598fa30`) | Shared ancestor commits |
| 4 | Input: stmfts - support Samsung FTS5CU56A 16-byte Type-B events | Commit log wrapped; awaiting human `Signed-off-by:` |

### topic/r8q-panel (2 commits)
**Subsystem:** samsung,s6e3fc3 binding + parameterized panel driver

| # | Subject | Status |
|---|---------|--------|
| 1 | dt-bindings: display: panel: samsung,s6e3fc3-p10: add r8q support | Applied cleanly |
| 2 | drm/panel: samsung-s6e3fc3: parameterize timings and regulators | Applied cleanly |

### topic/r8q-audio (1 commit)
**Subsystem:** sm8250.c TDM slots

| # | Subject | Status |
|---|---------|--------|
| 1 | ASoC: qcom: sm8250: add TDM slot, clock and channel-map ops | Applied cleanly |

### topic/r8q-gpu (12 commits)
**Subsystem:** gpucc/gdsc reset, a6xx recovery, adreno smmu quiesce, hangcheck progress retries

| # | Subject | Status |
|---|---------|--------|
| 1 | clk: qcom: Allow custom reset ops | Applied cleanly |
| 2 | clk: qcom: gdsc: Add a reset op to poll gdsc collapse | Applied cleanly |
| 3 | dt-bindings: clk: qcom,gpucc-sm8250: Support gpu cx gdsc reset | Applied cleanly |
| 4 | clk: qcom: gpucc-sm8250: Add cx collapse reset support | Applied cleanly |
| 5 | dt-bindings: display: msm: Document the gpu cx_collapse reset | Applied cleanly |
| 6 | arm64: dts: qcom: sm8250: Add Reset support for gpu | Applied cleanly |
| 7 | drm/msm/a6xx: Ensure CX collapse during gpu recovery | Applied cleanly |
| 8 | drm/msm: track the adreno smmu device in struct msm_gpu | Applied cleanly |
| 9 | drm/msm/a6xx: force the adreno smmu's runtime suspend during recovery | Applied cleanly |
| 10 | iommu/arm-smmu-qcom: gate the adreno fault path for gpu recovery | Applied cleanly |
| 11 | drm/msm/a6xx: quiesce the adreno smmu across the CX collapse window | Applied with conflict resolution |
| 12 | drm/msm: raise hangcheck progress retry cap for long compute submits | Applied cleanly |

### topic/r8q-cameras (11 commits; 8 camera + 3 shared base DTS)
**Note:** Shares the 3 `topic/r8q-dts` base commits (`e897496c3..9d598fa30`), plus `hynix,hi847.yaml` split into its own binding patch before `hi847.c`.
**Subsystem:** hi847 binding + driver, s5k3l6xx binding + driver

| # | Subject | Status |
|---|---------|--------|
| 1–3 | `topic/r8q-dts` base DTS dependency (`e897496c3..9d598fa30`) | Shared ancestor commits |
| 4 | dt-bindings: media: i2c: add Samsung S5K3L6 / S5K3L6XX sensor binding | Applied cleanly |
| 5 | media: i2c: add Samsung S5K3L6 / S5K3L6XX image sensor driver | Applied cleanly |
| 6 | dt-bindings: media: i2c: add Hynix HI-847 sensor | Split from `hi847` DT patch (`DT_SPLIT_BINDING_PATCH` fix) |
| 7 | media: i2c: hi847: support the sensor from device tree | Rebuilt cleanly (`0` checkpatch warnings) |
| 8 | media: i2c: hi847: accept a board that lists one link frequency | Applied cleanly |
| 9 | media: i2c: hi847: power the sensor through runtime PM | Applied cleanly |
| 10 | arm64: dts: qcom: sm8250-samsung-r8q: add HI-847 camera clock power domain | Subject shortened <=75 chars (`0` warn) |
| 11 | media: i2c: hi847: fix HBLANK pixel-rate timing and selection | Subject shortened <=75 chars (`0` warn) |

### topic/r8q-modem (32 commits)
**Subsystem:** qcom,pci-msi binding + driver, wwan SAHARA/EFS/IP_CTRL, mhi host/pci_generic, af_qrtr

| # | Subject | Status |
|---|---------|--------|
| 1 | dt-bindings: interrupt-controller: add qcom,pci-msi doorbell | Applied cleanly |
| 2 | irqchip: add qcom,pci-msi doorbell for Qualcomm PCIe root complexes | Applied with conflict resolution |
| 3 | irqchip: qcom-pci-msi: compose on the parent irq_data | Applied cleanly |
| 4 | irqchip: qcom-pci-msi: program the bank offset, not the SPI number | Applied cleanly |
| 5 | irqchip: qcom-pci-msi: OR the bank base into the message data | Applied cleanly |
| 6 | irqchip: qcom-pci-msi: add a debugfs doorbell trigger for offline tests | Applied cleanly |
| 7 | irqchip: qcom-pci-msi: add a GICD SETSPI control knob | Applied cleanly |
| 8 | irqchip: qcom-pci-msi: program the GIC SPI hwirq as message data | Applied cleanly |
| 9 | bus: mhi: host: handle SBL and the AMSS doorbell change of embedded modems | Applied cleanly |
| 10 | MAINTAINERS: add the Qualcomm PCIe MSI controller entries | Applied cleanly |
| 11 | bus: mhi: pci_generic: keep the Samsung r8q SDX55M out of M3 | Applied cleanly |
| 12 | bus: mhi: host: update wake_db before mission-mode __mhi_device_get_sync() | Applied cleanly |
| 13 | bus: mhi: pci_generic: align Samsung r8q SDX55M IPCR ring to 64/er2 | Applied with conflict resolution |
| 14 | bus: mhi: pci_generic: align Samsung r8q SDX55M event and QMI rings | Applied cleanly |
| 15 | bus: mhi: pci_generic: drop auto-started IP channels on Samsung r8q | Applied cleanly |
| 16 | bus: mhi: pci_generic: disable ASPM/L1ss on Samsung r8q SDX55M | Applied cleanly |
| 17 | bus: mhi: pci_generic: cap Samsung r8q SDX55M DMA width to 29 bits | Applied cleanly |
| 18 | bus: mhi: host: allow keeping device wake asserted via no_m2 | Applied cleanly |
| 19 | bus: mhi: pci_generic: keep Samsung r8q SDX55M out of M1/M2 | Applied cleanly |
| 20 | bus: mhi: host: add Subsystem Failure Reason (SFR) capture | Applied cleanly |
| 21 | net: qrtr: let a node with an assigned ID receive before our HELLO | Applied cleanly |
| 22 | bus: mhi: pci_generic: power up asynchronously in recovery and reset | Applied cleanly |
| 23 | bus: mhi: pci_generic: keep the r8q SAHARA channels valid in AMSS | Applied cleanly |
| 24 | bus: mhi: host: keep devices whose channels survive an EE transition | Applied cleanly |
| 25 | bus: mhi: host: poll event rings when the device cannot signal MSI | Applied cleanly |
| 26 | net: wwan: add WWAN_PORT_SAHARA and MHI SAHARA channel mapping | Applied cleanly |
| 27 | irqchip: qcom-pci-msi: the doorbell is address-as-data — never map or read it | Applied with conflict resolution |
| 28 | bus: mhi: host: check mhi_tryset_pm_state() result in SBL transition | Applied cleanly (`-Wunused-result` fix) |
| 29 | net: qrtr: route control replies and broadcasts to every bridged node | Cherry-picked (`[mdm-05]`) |
| 30 | net: wwan: mhi_wwan_ctrl: add r8q EFS and IP_CTRL DTR support | Cherry-picked (`[mdm-05]`) |
| 31 | bus: mhi: pci_generic: reschedule poll_work across recovery | Cherry-picked (`[mdm-05]`) |
| 32 | bus: mhi: pci_generic: align Samsung r8q SDX55M channels with stock | Cherry-picked (`[mdm-05]`) |

## 3. Verification Status

### Checkpatch (2026-11-26, inside omamobile-build:24.04)
**Result: ALL PASS** — `tools/kernel-pr-check --build --merge origin/omamobile` (`scripts/checkpatch.pl --strict --no-signoff`) ran across all 8 topic branches (`clang-r596125` on base `cf22a9090`):

| Branch | Branch Commits | Unique Unsigned (`Signed-off-by:` needed) | checkpatch errors |
|--------|----------------|-------------------------------------------|-------------------|
| topic/r8q-qcom-core | 9 | 9 | 0 errors (0 warnings/checks) |
| topic/r8q-dts | 3 | 3 | 0 errors |
| topic/r8q-touch | 4 (1 touch + 3 shared DTS) | 1 touch (+ 3 shared DTS) | 0 errors |
| topic/r8q-panel | 2 | 2 | 0 errors (0 warnings/checks) |
| topic/r8q-audio | 1 | 0 (`EXIT=0`) | 0 errors |
| topic/r8q-gpu | 12 (6 Akhil + 6 Santiago) | 6 | 0 errors |
| topic/r8q-cameras | 11 (8 cam + 3 shared DTS) | 8 cam (+ 3 shared DTS) | 0 errors |
| topic/r8q-modem | 32 (31 Santiago + 1 signed) | 31 | 0 errors (only 2 `FILE_PATH_CHANGES`) |

**Unique commits across all 8 branches:** **68** (`6` Akhil `Signed-off-by:`, `2` Santiago `Signed-off-by:`, **`60` unique unsigned commits**).

### kernel-pr-check, dt_binding_check & dtbs_check (2026-11-26)
**Result:**
1. With pinned Android clang (`clang-r596125`) inside `omamobile-build:24.04` and `--build ... --merge origin/omamobile`, all 8 branches merge cleanly onto `cf22a9090`, pass `checkpatch --strict` (0 errors), pass policy checks, pass `W=1` C builds (0 warnings across all touched C files), and build all 347 DTBs with `W=1` (0 new warnings). Only `no human Signed-off-by` fails on the 7 unsigned branches (`topic/r8q-audio` exits `0`).
2. With `dtschema` (`dt-validate 2026.9`) and `yamllint 1.38.0` on `r8q-merge-test2` (all 8 branches merged cleanly onto `cf22a9090`), **`dt_binding_check` passed cleanly (`0` errors, `0` warnings)** across all 5 touched/new binding schemas (`display/msm/gpu.yaml`, `display/panel/samsung,s6e3fc3-p10.yaml`, `interrupt-controller/qcom,pci-msi.yaml`, `media/i2c/hynix,hi847.yaml`, `media/i2c/samsung,s5k3l6xx.yaml`), and `CHECK_DTBS=y W=1` on `qcom/sm8250-samsung-r8q.dtb` reported **0 `r8q` schema warnings** (`ts-int-default-state` `input-enable;` replaced with `output-disable;`; only the pre-existing `sm8250.dtsi` `dsi@ae96000` `avoid_unnecessary_addr_size` warning also present on `sm8250-samsung-x1q.dtb`).

| Branch | W=1 C files | 347 DTBs (W=1) | Result (`--build --merge origin/omamobile`) |
|--------|-------------|----------------|----------------------------------------------|
| topic/r8q-qcom-core | 8 ok (0 warn) | skip (no DTS) | ok except missing Signed-off-by (9/9) |
| topic/r8q-dts | skip (no C) | ok (0 new warn) | ok except missing Signed-off-by (3/3); `CHECK_DTBS`: 0 `r8q` warn |
| topic/r8q-touch | 1 ok (0 warn) | ok (0 new warn) | ok except missing Signed-off-by (4/4); `CHECK_DTBS`: 0 `r8q` warn |
| topic/r8q-panel | 1 ok (0 warn) | skip (no DTS) | ok except missing Signed-off-by (2/2); `dt_binding_check`: pass |
| topic/r8q-audio | 1 ok (0 warn) | skip (no DTS) | `EXIT=0` (all active checks pass, 0/1 missing SoB) |
| topic/r8q-gpu | 7 ok (0 warn) | ok (0 new warn) | ok except missing Signed-off-by (6/12); `dt_binding_check`: pass |
| topic/r8q-cameras | 2 ok (0 warn) | ok (0 new warn) | ok except missing Signed-off-by (11/11); `dt_binding_check`: pass |
| topic/r8q-modem | 9 ok (0 warn) | skip (no DTS) | ok except missing Signed-off-by (31/32); `dt_binding_check`: pass |

Notes: `topic/r8q-qcom-core` and `topic/r8q-panel` now have **0** checkpatch warnings/checks; `topic/r8q-modem` has only 2 `FILE_PATH_CHANGES` notices (covered by `MAINTAINERS` commit `b651cdb1d`); `topic/r8q-audio` keeps its 7 required ASoC `-ENOTSUPP` returns; `topic/r8q-gpu` keeps Akhil's 6 upstream patches verbatim; `topic/r8q-dts` retains transient `UNDOCUMENTED_DT_STRING` warnings on standalone base DTS commits whose bindings live in `topic/r8q-{panel,cameras,modem}` (`dt_binding_check` and `CHECK_DTBS` pass cleanly on the merged tree). Note that switching `ts-int-default-state` from `input-enable;` to `output-disable;` awaits an attended phone flash/boot touch re-check (`AGENTS.md` §1 gate).

### Tree Equivalence Verification
**Result: VERIFIED** — Merging all 8 topic branches onto `origin/omamobile` produces a tree that matches `r8q-base-dts-fold` for all r8q-specific files:
- `arch/arm64/boot/dts/qcom/sm8250-samsung-r8q.dts`: MATCH
- `drivers/media/i2c/hi847.c`: MATCH
- `drivers/media/i2c/s5k3l6xx.c`: MATCH
- `drivers/irqchip/irq-qcom-pci-msi.c`: MATCH
- `drivers/bus/mhi/host/pci_generic.c`: MATCH
- `sound/soc/qcom/sm8250.c`: MATCH

### Commit Message Format
- **RESOLVED:** All `[PATCH XX/87]` prefix artifacts have been stripped. Verified: 0 `[PATCH]` prefixes across all 8 topic branches.
- **NOTE:** No `Signed-off-by:` trailers were added (per AGENTS.md rules). Human sign-off is required before PR.

## 4. Shared Driver Audit (CONTRIBUTING.md §6.2)

Per CONTRIBUTING.md §6.2: "Shared drivers leave Pixel 7 Pro / cheetah untouched."

| Driver | Files Changed | cheetah Impact | Notes |
|--------|--------------|----------------|-------|
| `drivers/clk/qcom/gdsc.c` | gdsc.c, gdsc.h | None | Adds reset op for gdsc collapse - per-SoC match data only |
| `drivers/clk/qcom/gpucc-sm8250.c` | gpucc-sm8250.c | None | Adds cx collapse reset - SM8250-specific compatible |
| `drivers/clk/qcom/reset.c` | reset.c, reset.h | None | Adds custom reset ops - generic infrastructure |
| `drivers/gpu/drm/msm/adreno/a6xx_gpu.c` | a6xx_gpu.c, a6xx_gpu.h | None | GPU recovery changes - per-GPU quirk flags |
| `drivers/soc/qcom/rpmh-rsc.c` | rpmh-rsc.c (`cmd-db.c`/`.h` untouched) | None | Native lock-safe TCS state dump & timeout reclaim; zero `cmd-db` or non-SM8250 impact |
| `drivers/gpu/drm/msm/msm_gpu.c` | msm_gpu.c, msm_gpu.h | None (cheetah uses panthor) | Hangcheck progress - raises retry cap; Pixel cheetah (gs201) uses panthor so is unaffected |
| `drivers/iommu/arm/arm-smmu/arm-smmu-qcom.c` | arm-smmu-qcom.c, arm-smmu-qcom.h | None | Adreno fault path gating - per-device quirk |
| `drivers/bus/mhi/host/pci_generic.c` | pci_generic.c | None | Samsung r8q SDX55M quirks - match data only |
| `drivers/bus/mhi/host/main.c` | main.c, internal.h | None | MHI host improvements - generic changes (`mhi_power_up_capable()`) |
| `drivers/bus/mhi/host/pm.c` | pm.c | None | MHI power management - generic changes |
| `drivers/irqchip/irq-qcom-pci-msi.c` | NEW FILE | None | New driver - no cheetah impact |
| `drivers/media/i2c/hi847.c` | hi847.c | None | Adds OF regulator/GPIO power path; ACPI path preserved |
| `drivers/media/i2c/s5k3l6xx.c` | NEW FILE | None | New sensor driver - no cheetah impact |
| `drivers/input/touchscreen/stmfts.c` | stmfts.c | None | 16-byte Type-B support - per-device compatible |
| `drivers/gpu/drm/panel/panel-samsung-s6e3fc3.c` | panel-samsung-s6e3fc3.c | None | Parameterized timings - per-panel match data |
| `sound/soc/qcom/sm8250.c` | sm8250.c | None | TDM slot ops - SM8250-specific (`-ENOTSUPP` required by ASoC core) |
| `drivers/dma/qcom/gpi.c` | gpi.c | None | QUP EE offsets - per-device match data (`samsung,r8q` + SM8250 QUP0) |
| `drivers/tty/serial/qcom-geni.c` | qcom-geni.c | None | Clamp fix - generic bugfix |
| `drivers/mfd/max77705.c` | max77705.c | None | PASS2 silicon support - per-device compatible |
| `drivers/scsi/ufs/ufshcd.c` | ufshcd.c | None | WriteBooster fix - generic bugfix |
| `drivers/mmc/host/sdhci-msm.c` | sdhci-msm.c | None | 64-bit DMA disable - per-device quirk |
| `net/qrtr/af_qrtr.c` | af_qrtr.c | None | QRTR fix - generic bugfix |
| `drivers/net/wwan/wwan_core.c` | wwan_core.c | None | WWAN_PORT_SAHARA - new enum value |
| `include/linux/mhi.h` | mhi.h | None | no_m2 field - generic MHI change |
| `include/linux/wwan.h` | wwan.h | None | WWAN_PORT_SAHARA - new enum value |
| `include/linux/adreno-smmu-priv.h` | adreno-smmu-priv.h | None | Optional `set_recovery_gate` callback |
| `arch/arm64/boot/dts/qcom/sm8250-samsung-r8q.dts` | sm8250-samsung-r8q.dts | None (`x1q` / `sm8250-samsung-common.dtsi` untouched) | XPU `no-map` and `&framebuffer` overrides scoped to `r8q` only |

**Audit Result:** All shared driver changes are properly guarded with per-SoC/per-device match data or compatible strings. No unguarded global changes detected.

## 5. Commands for Human Sign-off ([krn-02])

### 5.1 Verify branch contents
```bash
cd ~/omamobile-kernel/linux
for branch in topic/r8q-qcom-core topic/r8q-dts topic/r8q-touch topic/r8q-panel topic/r8q-audio topic/r8q-gpu topic/r8q-cameras topic/r8q-modem; do
  echo "=== $branch ==="
  git log --oneline origin/omamobile..$branch
  echo ""
done
```

### 5.2 Run checkpatch and builds (inside Docker)
```bash
# Start the build container
docker run --rm -it -v ~/omamobile-kernel:/kernel -v /tmp/r8q-checks:/checks omamobile-build:24.04 bash

# Inside container:
cd /kernel
git checkout topic/r8q-qcom-core
for branch in topic/r8q-qcom-core topic/r8q-dts topic/r8q-touch topic/r8q-panel topic/r8q-audio topic/r8q-gpu topic/r8q-cameras topic/r8q-modem; do
  git checkout $branch
  echo "=== Checking $branch ==="
  # Run checkpatch on each commit
  git log --oneline origin/omamobile..$branch | while read sha msg; do
    git format-patch -1 --stdout $sha | perl scripts/checkpatch.pl --strict --no-signoff --terse -
  done
done

# Run W=1 builds
make defconfig
scripts/kconfig/merge_config.sh -m -O .config kernel/configs/omamobile.config
make W=1 -j$(nproc) 2>&1 | tee /checks/w1-build.log

# Run dtbs_check
make CHECK_DTBS=y W=1 exynos/google/gs201-cheetah.dtb 2>&1 | tee /checks/dtbs.log
make CHECK_DTBS=y W=1 qcom/sm8250-samsung-r8q.dtb 2>&1 | tee /checks/dtbs-r8q.log

# Run dt_binding_check
make dt_binding_check DT_SCHEMA_FILES=Documentation/devicetree/bindings/display/panel/samsung,s6e3fc3-p10.yaml 2>&1 | tee /checks/binding.log
make dt_binding_check DT_SCHEMA_FILES=Documentation/devicetree/bindings/interrupt-controller/qcom,pci-msi.yaml 2>&1 | tee /checks/binding.log
make dt_binding_check DT_SCHEMA_FILES=Documentation/devicetree/bindings/media/i2c/hynix,hi847.yaml 2>&1 | tee /checks/binding.log
make dt_binding_check DT_SCHEMA_FILES=Documentation/devicetree/bindings/media/i2c/samsung,s5k3l6xx.yaml 2>&1 | tee /checks/binding.log
```

### 5.3 Run kernel-pr-check
```bash
cd ~/omamobile-kernel/linux
for branch in topic/r8q-qcom-core topic/r8q-dts topic/r8q-touch topic/r8q-panel topic/r8q-audio topic/r8q-gpu topic/r8q-cameras topic/r8q-modem; do
  echo "=== Running kernel-pr-check on $branch ==="
  ~/omamobile-kernel/linux/../tests/kernel-pr-check --build /tmp/r8q-checks/$branch origin/omamobile $branch 2>&1 | tee /tmp/r8q-checks/$branch.log
done
```

### 5.4 Commit message, DTS schema & post-signoff checks
- `ts_int_default` (`arch/arm64/boot/dts/qcom/sm8250-samsung-r8q.dts`) now uses `output-disable;` (`CHECK_DTBS=y` passes with `0` `r8q` schema warnings); verify touchscreen input on an attended flash/boot after `[krn-02]` sign-off (`AGENTS.md` §1 gate) before marking `[krn-04]` done.
- Re-run `tools/kernel-pr-check --build --merge origin/omamobile` on all signed topic branches after `git rebase --signoff` so every branch exits `0`.

## 6. PR Creation ([krn-05])

### 6.1 Push branches to fork
```bash
cd ~/omamobile-kernel/linux
for branch in topic/r8q-qcom-core topic/r8q-dts topic/r8q-touch topic/r8q-panel topic/r8q-audio topic/r8q-gpu topic/r8q-cameras topic/r8q-modem; do
  git push origin $branch:$branch 2>&1
done
```

### 6.2 Create pull requests
Create one PR per topic branch against `derekross/linux-omarchy-mobile:omamobile`:
- `topic/r8q-qcom-core` → `omamobile`
- `topic/r8q-dts` → `omamobile`
- `topic/r8q-touch` → `omamobile`
- `topic/r8q-panel` → `omamobile`
- `topic/r8q-audio` → `omamobile`
- `topic/r8q-gpu` → `omamobile`
- `topic/r8q-cameras` → `omamobile`
- `topic/r8q-modem` → `omamobile`

Each PR should include:
- Link to this plan (§2 for commit list, §4 for shared driver audit)
- checkpatch --strict results
- W=1 build results
- dtbs_check and dt_binding_check results
- Phone smoke test results (if available)

## 7. Notes

- The `r8q-base-dts-fold` branch was based on a different commit than `origin/omamobile`, so some patches had conflicts during application. All conflicts were resolved by accepting the r8q version.
- All `[PATCH XX/87]` prefixes were stripped when rebuilding the 8 topic branches.
- No `Signed-off-by:` trailers were added by AI agents. Each commit on the 7 unsigned branches needs human `Signed-off-by:` (`[krn-02]`) before PR submission.
- The existing branches `omamobile`, `r8q-base-dts-fold`, `omamobile-r8q-signed`, and `r8q-usbc-max77705` were NOT modified per safety rules.
