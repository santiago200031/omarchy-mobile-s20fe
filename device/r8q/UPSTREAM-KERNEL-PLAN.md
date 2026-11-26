<!-- SPDX-License-Identifier: MIT -->
# UPSTREAM-KERNEL-PLAN: r8q topic branch split from r8q-base-dts-fold

**Date:** 2026-11-26 (updated after `[mdm-05]` cherry-pick & `dt_binding_check` / `CHECK_DTBS` verification)
**Status:** 8 topic branches built cleanly on `origin/omamobile` (`82e3cf0b7`); merged cleanly in `r8q-merge-test2`; CI builds & binding checks pass; awaiting human `Signed-off-by:` + `ts-int-default-state` DT schema cleanup
**Source:** `r8q-base-dts-fold` → 8 topic branches (`74` branch commits incl. 3 shared base DTS dependencies on `touch`/`cameras`, plus `[mdm-05]` + `pm.c` fixes; 2 revert pairs excluded)
**Base:** `origin/omamobile` (82e3cf0b7)

## 1. Overview

This plan documents the audit and split of `~/omamobile-kernel/wt-r8q` branch `r8q-base-dts-fold` into clean per-subsystem topic branches against `origin/omamobile` (82e3cf0b7).

**Revert pairs excluded** (as specified in [krn-03]/[krn-04]):
- `6085e0349` / `f7f3096b1`: Revert + original for "pci: dwc: prepare PCIe2 MSI in the endpoint IOMMU domain"
- `0fd2c260d` / `6234db2d0`: Revert + original for "bus: mhi: host: force one M3->M0 state cycle at mission-mode entry"

**83 non-revert commits** from `r8q-base-dts-fold` were distributed across 8 topic branches.

## 2. Topic Branches

### topic/r8q-qcom-core (11 commits)
**Subsystem:** rpmh-rsc, cmd-db, gpi EE offsets, geni serial, max77705 pass2, ufs writebooster, sdhci 64-bit DMA, gcc_gp2 clk_rcg2_gp_ops

| # | Subject | Status |
|---|---------|--------|
| 1 | mfd: max77705: accept PASS2 silicon revision (0x02) | Applied cleanly |
| 2 | soc: qcom: rpmh: dump active TCS state on write timeout | Applied cleanly |
| 3 | scsi: ufs: core: disable WriteBooster when dCurWriteBoosterBufferSize returns -EINVAL | Applied cleanly |
| 4 | mmc: sdhci-msm: disable 64-bit DMA on SM8250 | Applied cleanly |
| 5 | dmaengine: qcom: gpi: set QUP0 ee_offset to 0x1000 on SM8250 | Applied cleanly |
| 6 | soc: qcom: cmd-db: export RPMh accelerator type string helper | Applied cleanly |
| 7 | soc: qcom: cmd-db: add reverse address-to-name lookup | Applied cleanly |
| 8 | soc: qcom: rpmh-rsc: Output debug information from RSC | Applied cleanly |
| 9 | soc: qcom: rpmh-rsc: bound the TCS claim wait and reclaim stuck TCSes | Applied cleanly |
| 10 | tty: serial: qcom-geni: clamp negative of_alias_get_highest_id() in get_port_from_line() | Applied cleanly |
| 11 | clk: qcom: gcc-sm8250: switch gcc_gp2_clk_src to clk_rcg2_gp_ops | Applied cleanly |

### topic/r8q-dts (3 commits)
**Note:** Also includes the 3 dts commits from r8q-touch and r8q-cameras as dependencies (the r8q.dts file is created by these commits).
**Subsystem:** sm8250-samsung-common.dtsi + sm8250-samsung-r8q.dts clean base fold, Makefile

| # | Subject | Status |
|---|---------|--------|
| 1 | arm64: dts: qcom: sm8250-samsung-r8q: flesh out hardware nodes and fix pm8150l gpio11 | Applied cleanly |
| 2 | arm64: dts: qcom: sm8250-samsung-r8q: fold verified overlay DT fixes into base DTS | Applied with conflict resolution |
| 3 | arm64: dts: qcom: sm8250-samsung-r8q: fold verified subsystem overlays | Applied with conflict resolution |

### topic/r8q-touch (3 commits)
**Subsystem:** stmfts 16-byte Type-B support

| # | Subject | Status |
|---|---------|--------|
| 1 | arm64: dts: qcom: sm8250-samsung-r8q: flesh out hardware nodes and fix pm8150l gpio11 | Applied with conflict resolution |
| 2 | arm64: dts: qcom: sm8250-samsung-r8q: fold verified overlay DT fixes into base DTS | Applied with conflict resolution |
| 3 | arm64: dts: qcom: sm8250-samsung-r8q: fold verified subsystem overlays | Applied with conflict resolution |
| 4 | Input: stmfts - support Samsung FTS5CU56A 16-byte Type-B events | Applied cleanly

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

### topic/r8q-cameras (10 commits)
**Note:** Also includes the 3 dts commits from r8q-dts as dependencies (the r8q.dts file is created by these commits). 5 commits were applied during conflict resolution (no separate commit needed as content was already present).
**Subsystem:** hi847 binding + driver, s5k3l6xx binding + driver

| # | Subject | Status |
|---|---------|--------|
| 1 | arm64: dts: qcom: sm8250-samsung-r8q: flesh out hardware nodes and fix pm8150l gpio11 | Applied with conflict resolution |
| 2 | arm64: dts: qcom: sm8250-samsung-r8q: fold verified overlay DT fixes into base DTS | Applied with conflict resolution |
| 3 | arm64: dts: qcom: sm8250-samsung-r8q: fold verified subsystem overlays | Applied with conflict resolution |
| 4 | dt-bindings: media: i2c: add Samsung S5K3L6 / S5K3L6XX sensor binding | Applied cleanly |
| 5 | media: i2c: add Samsung S5K3L6 / S5K3L6XX image sensor driver | Applied cleanly |
| 6 | media: i2c: hi847: support the sensor from device tree | Applied cleanly |
| 7 | media: i2c: hi847: accept a board that lists one link frequency | Applied cleanly |
| 8 | media: i2c: hi847: power the sensor through runtime PM | Applied cleanly |
| 9 | arm64: dts: qcom: sm8250-samsung-r8q: give the HI-847 its camera clock power domain | Applied with conflict resolution |
| 10 | media: i2c: hi847: fix HBLANK pixel-rate timing, selection, and fwnode props | Applied cleanly

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
**Result: ALL PASS** — `tools/kernel-pr-check --build --merge origin/omamobile` (`scripts/checkpatch.pl --strict --no-signoff`) ran across all 8 topic branches (`clang-r596125`):

| Branch | Commits | Missing `Signed-off-by:` | checkpatch errors |
|--------|---------|--------------------------|-------------------|
| topic/r8q-qcom-core | 11 | 11 | 0 errors |
| topic/r8q-dts | 3 | 3 | 0 errors |
| topic/r8q-touch | 3 | 1 | 0 errors |
| topic/r8q-panel | 2 | 2 | 0 errors |
| topic/r8q-audio | 1 | 0 | 0 errors |
| topic/r8q-gpu | 12 | 6 | 0 errors |
| topic/r8q-cameras | 10 | 10 | 0 errors |
| topic/r8q-modem | 32 | 31 | 0 errors |

### kernel-pr-check, dt_binding_check & dtbs_check (2026-11-26)
**Result:**
1. With pinned Android clang (`clang-r596125`) inside `omamobile-build:24.04` and `--build ... --merge origin/omamobile`, all 8 branches merge cleanly onto `82e3cf0b7`, pass `checkpatch --strict` (0 errors), pass policy checks, pass `W=1` C builds (0 warnings across all touched C files), and build all 347 DTBs with `W=1` (0 new warnings). Only `no human Signed-off-by` fails on 7 branches (`topic/r8q-audio` passes 100%).
2. With `dtschema` (`dt-validate 2026.9`) and `yamllint 1.38.0` on `r8q-merge-test2` (after merging `topic/r8q-modem` `ebee87d80`), **`dt_binding_check` passed cleanly (`0` errors, `0` warnings)** across all 5 touched/new binding schemas (`display/msm/gpu.yaml`, `display/panel/samsung,s6e3fc3-p10.yaml`, `interrupt-controller/qcom,pci-msi.yaml`, `media/i2c/hynix,hi847.yaml`, `media/i2c/samsung,s5k3l6xx.yaml`), and `CHECK_DTBS=y W=1` on `qcom/sm8250-samsung-r8q.dtb` reported **1 r8q schema warning** (`pinctrl@f100000: ts-int-default-state: 'oneOf' conditional failed` due to `input-enable` forbidden by `qcom,tlmm-common.yaml:84` / `qcom,sm8250-pinctrl.yaml`, plus the pre-existing `sm8250.dtsi` `dsi@ae96000` `avoid_unnecessary_addr_size` warning also present on `sm8250-mtp.dtb`).

| Branch | W=1 C files | 347 DTBs (W=1) | Result (`--build --merge`) |
|--------|-------------|----------------|----------------------------|
| topic/r8q-qcom-core | 9 ok (0 warn) | skip (no DTS) | ok except missing Signed-off-by (11/11) |
| topic/r8q-dts | skip (no C) | ok (0 new warn) | ok except missing Signed-off-by (3/3); `CHECK_DTBS`: 1 warn (`ts-int-default-state` `input-enable`) |
| topic/r8q-touch | 1 ok (0 warn) | ok (0 new warn) | ok except missing Signed-off-by (1/3); `CHECK_DTBS`: 1 warn (`ts-int-default-state` `input-enable`) |
| topic/r8q-panel | 1 ok (0 warn) | skip (no DTS) | ok except missing Signed-off-by (2/2); `dt_binding_check`: pass |
| topic/r8q-audio | 1 ok (0 warn) | skip (no DTS) | `EXIT:0` (all active checks pass, 0/1 missing SoB) |
| topic/r8q-gpu | 7 ok (0 warn) | ok (0 new warn) | ok except missing Signed-off-by (6/12); `dt_binding_check`: pass |
| topic/r8q-cameras | 2 ok (0 warn) | ok (0 new warn) | ok except missing Signed-off-by (10/10); `dt_binding_check`: pass |
| topic/r8q-modem | 9 ok (0 warn) | ok (0 new warn) | ok except missing Signed-off-by (31/32); `dt_binding_check`: pass |

Notes: `kernel-pr-check` still reports non-blocking checkpatch warnings (`COMMIT_MESSAGE`, `UNDOCUMENTED_DT_STRING`, `FILE_PATH_CHANGES`, `PREFER_LORE_ARCHIVE`, etc.) with zero checkpatch errors. Removing `input-enable;` from `ts-int-default-state` resolves the remaining `dtbs_check` warning before `[krn-04]` sign-off.

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
| `drivers/gpu/drm/msm/msm_gpu.c` | msm_gpu.c, msm_gpu.h | None (cheetah uses panthor) | Hangcheck progress - changes `DRM_MSM_HANGCHECK_PROGRESS_RETRIES` macro (3→31, 8000 ms cap). Affects msm/Adreno drivers only; Pixel cheetah (gs201) uses panthor so is unaffected. SM8250/r8q uses Adreno 650 (a6xx) which is affected.
| `drivers/iommu/arm/arm-smmu/arm-smmu-qcom.c` | arm-smmu-qcom.c, arm-smmu-qcom.h | None | Adreno fault path gating - per-device quirk |
| `drivers/bus/mhi/host/pci_generic.c` | pci_generic.c | None | Samsung r8q SDX55M quirks - match data only |
| `drivers/bus/mhi/host/main.c` | main.c, internal.h | None | MHI host improvements - generic changes |
| `drivers/bus/mhi/host/pm.c` | pm.c | None | MHI power management - generic changes |
| `drivers/irqchip/irq-qcom-pci-msi.c` | NEW FILE | None | New driver - no cheetah impact |
| `drivers/media/i2c/hi847.c` | NEW FILE | None | New sensor driver - no cheetah impact |
| `drivers/media/i2c/s5k3l6xx.c` | NEW FILE | None | New sensor driver - no cheetah impact |
| `drivers/input/touchscreen/stmfts.c` | stmfts.c | None | 16-byte Type-B support - per-device compatible |
| `drivers/panel/panel-samsung-s6e3fc3.c` | panel-samsung-s6e3fc3.c | None | Parameterized timings - per-panel match data |
| `sound/soc/qcom/sm8250.c` | sm8250.c | None | TDM slot ops - SM8250-specific |
| `drivers/dma/qcom/gpi.c` | gpi.c | None | QUP EE offsets - per-device match data |
| `drivers/tty/serial/qcom-geni.c` | qcom-geni.c | None | Clamp fix - generic bugfix |
| `drivers/mfd/max77705.c` | max77705.c | None | PASS2 silicon support - per-device compatible |
| `drivers/scsi/ufs/ufshcd.c` | ufshcd.c | None | WriteBooster fix - generic bugfix |
| `drivers/mmc/host/sdhci-msm.c` | sdhci-msm.c | None | 64-bit DMA disable - per-device quirk |
| `net/qrtr/af_qrtr.c` | af_qrtr.c | None | QRTR fix - generic bugfix |
| `drivers/net/wwan/wwan_core.c` | wwan_core.c | None | WWAN_PORT_SAHARA - new enum value |
| `include/linux/mhi.h` | mhi.h | None | no_m2 field - generic MHI change |
| `include/linux/wwan.h` | wwan.h | None | WWAN_PORT_SAHARA - new enum value |
| `include/linux/adreno-smmu-priv.h` | NEW FILE | None | New header - no cheetah impact |

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

### 5.4 Commit message & DTS schema items
- `[PATCH XX/87]` prefix stripping is already complete (`0` remaining across all 8 topic branches).
- Before opening `[krn-05]` PRs, drop `input-enable;` from `ts_int_default` (`arch/arm64/boot/dts/qcom/sm8250-samsung-r8q.dts:558`, flagged by `CHECK_DTBS=y`) during human `[krn-02]` sign-off, re-test touch on phone, and re-run `tools/kernel-pr-check`.

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
