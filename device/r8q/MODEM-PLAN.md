<!-- SPDX-License-Identifier: MIT -->

# Modem & Cellular — Samsung Galaxy S20 FE (r8q)

**Status:** **THE MODEM REACHES AMSS ON MAINLINE (2026-11-18).** The SDX55M runs its full flashless boot: PCIe2 enumerates, the kernel BHI-loads `sbl1`, the SBL's complete 14-image SAHARA set is served over one open MHI channel, and the modem jumps to mission mode by itself (`Mission mode doorbell offsets: CHDBOFF=0x300 ERDBOFF=0x700`, `EE: MISSION MODE`), exposing `mhi0_QMI/MBIM/IPCR/IP_HW0/IP_SW0` and `/dev/wwan0{qmi,mbim,qcdm,at}0`. Three fixes got there: the `qcom,pci-msi` doorbell with GIC-hwirq message data; the ported MHI SBL/AMSS support (SBL power-up, context-register zeroing, M0 at SBL, mission-mode doorbell re-read); and AP2MDM_STATUS (gpio56) driven high with one SAHARA channel per boot. Rich history: §8 (Stage 3A failure), §9 (design), §10 (Stage 4 runs, the AMSS result and the exact image). Next for daily use: SIM/QMI through userspace on the Arch root (qrtr-ns, ModemManager), and a kernel release with the three commits signed off.
**Safety:** All 77 UFS partitions read-only (`ro=1`); single-slot recovery boot (`sda24`) leaves LineageOS untouched; 15/15 NV partitions match the full off-repo backup; no NV partition has ever been written by this work
**Last Updated:** 2026-11-18

**Normal boot (2026-11-18): the modem reaches SBL, but the modem power-on
removes the SD card.** With the normal `recovery-modem.img` (kernel
`g70a925aeee09`, modem overlay), PCIe2 comes up and the SDX55 enumerates
(`0002:01:00.0 [17cb:0306]`); `mhi1` powers on and reports `Device already in
SBL`. When `mhi_pci_generic` binds, `mmc0: card 0001 removed` fires 0.3 s later,
the root goes `emergency_ro`, `wwan.ko` becomes unreadable and
`mhi_wwan_ctrl` cannot create `/dev/wwan*`. The stock DT explains it: `sdhc_2`
uses `cd-gpios = gpio77` (not AP2MDM 56/57), has no `vmmc-supply`, and `pcie2`
has no 3.3 V rail; the modem's power-on browns out the always-on 3.3 V rail.
The rescue image (no SD root) is the only working modem configuration. See
§12. The stable image was restored with heimdall; the SD card came back and the
root is read-write again.

---

## 1. Safety Rules (Mandatory)

1. **Verify NV backup & read-only guard before any modem work (`AGENTS.md` §1):**
   - Device descriptor `device/r8q/device.conf` defines:
     - `NV_PARTS="efs sec_efs persist modemst1 modemst2 fsg fsc mdm1m9kefs1 mdm1m9kefs2 mdm1m9kefsc mdm1m9kefs3 mdmddr"` (commit `839eb96`)
     - `NV_BACKUP_EXTRA="param steady devinfo"`
   - Check storage guard on phone: `tools/phone-ssh 'r8q-preflight --json'` (expected `"ufs_guard":{"ok":true,"detail":"77 UFS partitions read-only"}`).
   - Verify off-repo backup with `-d r8q`:
     `tools/nv-backup -d r8q --check ~/omamobile-kernel/reference-r8q/nv-backup-20261117`
   - Take a fresh RO backup after any stock/LineageOS boot before touching Linux modem services.
2. **Never write NV partitions from Linux.** All NV partitions stay `ro=1`. Only `omamobile-modem`'s per-phone overlay on `userdata` (`/var/lib/omamobile/modem-nv-overlay`) may persist allowed modem writes once explicitly enabled.
3. **One change per boot & human approval required:** Ask the human before packing/flashing any new overlay onto `sda24` or rebooting.

---

## 2. Verified Hardware Topology (`live.dts` + Mainline Kernel)

SM8250 has no on-die `remoteproc_mpss`; stock `r8q` (`androidboot.baseband=mdm`) uses an **external Snapdragon X55M modem (`qcom,ext-sdx55m`) over PCIe2 + MHI**:

- **Root complex & PHY:** `&pcie2` (`pcie@1c10000`) and `&pcie2_phy` (`phy@1c16000`, `compatible = "qcom,sm8250-qmp-modem-pcie-phy"`), already defined (`disabled`) in `sm8250.dtsi`, matching upstream `sm8250-sony-xperia-edo.dtsi`:
  - `perst-gpios = <&tlmm 85 GPIO_ACTIVE_LOW>`, `wake-gpios = <&tlmm 87 GPIO_ACTIVE_HIGH>` (`clkreq` GPIO 86)
  - `pcie2_phy` supplies: `vdda-phy-supply = <&vreg_l5a_0p88>`, `vdda-pll-supply = <&vreg_l9a_1p2>`
- **Modem handshake GPIOs (stock `mdm0: qcom,mdm0` & `ap2mdm`/`mdm2ap`):**
  - `mdm2ap`: `tlmm` **GPIO 1** (`err_fatal`), **GPIO 3** (`status`)
  - `ap2mdm`: `tlmm` **GPIO 56** (`status`), **GPIO 57** (`err_fatal`), plus PMIC GPIO 7 (`ap2mdm_errfatal2`)
- **Kernel modules already built (`=m`) in `7.3.0-rc6-omamobile-push1-gf3a5609d121b`:**
  - `phy-qcom-qmp-pcie.ko`, `mhi.ko`, `mhi_pci_generic.ko`, `mhi_net.ko`, `mhi_wwan_ctrl.ko`, `mhi_wwan_mbim.ko`, `qrtr-mhi.ko`.
- **NV Backup & Firmware State (verified 2026-11-17):**
  - Full off-repo RO backup `~/omamobile-kernel/reference-r8q/nv-backup-20261117-full`: **15/15 `SAME`** on 2026-11-17 with `tools/nv-backup -d r8q --check` (the original 10, plus `mdm1m9kefs1/2/c/3` and `mdmddr`). This is the backup to check before and after every Stage 2 run.
  - The five external SDX55M UFS partitions are `sda4` (`mdm1m9kefs1`), `sda5` (`mdm1m9kefs2`), `sda6` (`mdm1m9kefsc`), `sdd4` (`mdm1m9kefs3`), `sdd2` (`mdmddr`), all `ro=1` and included in the backup above.
  - Firmware staged read-only from `/dev/sda20` (`modem`, VFAT, `ro=1`) into `/lib/firmware/qcom/sdx55m/` on the phone (and `qdsp6m.qdb` into `/lib/firmware/qcom/sdx55/`), with `sbl1.mbn` and `qdsp6sw.mbn` diffed against the source. The full image set is present (`apps`, `aop`, `acdb`, `apdp`, `devcfg`, `efs1/2/3`, `hyp`, `mdmddr`, `multi_image_qti`, `qdsp6sw`, `sbl1`, `sec`, `tz`, `xbl_cfg`).
  - The vendor loader's `sahara_map_arr` (observed in `lib64/libmdmimgload.so` on this device) gives the SDX55M image IDs: 6 apps, 8 qdsp6sw, 9/34 mdmddr, 16/17/20 mdm1m9kefs1/2/3, 21 sbl1, 23 aop, 25 tz, 33 hyp, 36 multi_image_qti, 38 xbl_cfg, 40 apdp, 41 devcfg, 42 sec. IDs 28/30/31/37 belong to that library's other platforms and are not staged.

---

## 3. Phased Bring-Up Plan

### 3.1 Phase 1: Isolated `pcie2` / `pcie2_phy` Overlay Probe (PASSED — 2026-11-17)

1. Added `device/r8q/overlays/modem.dtso` enabling `&pcie2_phy` (`vreg_l5a_0p88`, `vreg_l9a_1p2`) and `&pcie2` (with `ap2mdm`/`mdm2ap` pinmuxes).
2. Packed `recovery-modem-test.img` (`7.3.0-rc6-omamobile-push1-gf3a5609d121b` + `--dual-boot --overlay modem`) and booted on `sda24`.
3. **Observed results (`2026-11-17`):**
   - `qcom-pcie 1c10000.pcie: PCIe Gen.3 x2 link up` (`15.752 Gb/s` at `8.0 GT/s x2`), zero IOMMU/SMMU or RPMh faults.
   - `0002:01:00.0 [17cb:0306]` (`Qualcomm Technologies, Inc SDX55 [Snapdragon X55 5G]`, Subsystem `[17cb:010c]`) enumerated cleanly.
   - `mhi-pci-generic 0002:01:00.0: MHI PCI device found: foxconn-sdx55`; `mhi0` powered ON (`Power on setup success`), and BHI `oem_pk_hash` + `serial_number` sysfs nodes populated.
   - Loaded `mhi0` stopped cleanly before channel creation with:
     `mhi mhi0: No firmware image defined or !sbl_size || !seg_len`
     because `drivers/bus/mhi/host/pci_generic.c` matches `PCI_DEVICE_SUB(0x17cb, 0x0306, 0x17cb, 0x010c)` to `mhi_foxconn_sdx55_info` (`.fw = NULL`) instead of `mhi_qcom_sdx55_info` (`.fw = "qcom/sdx55m/sbl1.mbn"`).

### 3.2 Phase 2: Firmware Staging, `pci_generic` BHI Quirk, Sahara, `rmtfs` & QRTR

**Done offline (2026-11-17), no phone writes:**

1. Full 15-partition RO NV backup and check: `nv-backup-20261117-full`, 15/15 `SAME`.
2. Firmware staged RO from `/dev/sda20` into `/lib/firmware/qcom/sdx55m/` (plus `qdsp6m.qdb`), source-verified.
3. Kernel branch `modem-stage2` on `omamobile-r8q-signed` (`wt-r8q`), commits:
   - `7896cfb4b` `net: wwan: add WWAN_PORT_SAHARA and MHI SAHARA channel mapping` — exposes MHI SBL SAHARA channels as `/dev/wwan*sahara*` through `mhi_wwan_ctrl`.
   - `4d5ae30a8` `bus: mhi: pci_generic: load qcom/sdx55m/sbl1.mbn on Samsung r8q` — on `samsung,r8q`, use a dedicated profile with `.fw = "qcom/sdx55m/sbl1.mbn"` (BHI) and SAHARA/DIAG/MBIM/QMI/IPCR/DUN/FIREHOSE/IP_SW0/IP_HW0 channels.
   - `CONFIG_MHI_BUS_DEBUG=y` for state/channel inspection.
   - `tools/kernel-pr-check` clean; built in `~/omamobile-kernel/out-r8q-modem`.
4. Artifacts (all at `~/omamobile-kernel/out-r8q-modem/`):
   - `modules-7.3.0-rc6-omamobile-push1-g4d5ae30a8318.tar.zst` — sha256 `780a1a0da41abea2a102b5999a3dc94daa29a351eb64d677bf4611b9104d063b`.
   - `recovery-modem-s2.img` — `--dual-boot --overlay camera-uw --overlay modem`, sha256 `c1d99560fa0f7e90a3083f28fe164bddd967a3175e1fce2d9c72ff8ab8233cc7` (`recovery-modem-s2.tar.md5` is the same image for Odin/Heimdall).
5. `tools/r8q-sahara` — read-only userspace Sahara v2 loader. `python3 - --dry-run` on the phone reports all r8q sources present and readable; it logs every requested image ID and refuses to guess a missing one.

**Still pending:** human-approved writes and the first attended Sahara/BHI run — see §4. After that, adapt/build `packages/omamobile-modem` (`rmtfs`, `tqftpserv`, `pd-mapper`) for `r8q` + `qrtr-mhi`.

### 3.3 Phase 3: ModemManager & WWAN Data (With SIM)

1. Test `mhi_wwan_ctrl` / `mhi_wwan_mbim` / `mhi_net` + `ModemManager` (`mmcli -L`).
2. Test SIM detection, registration, and mobile data.

---

## 4. Stage 2 gated runbook (2026-11-17)

All commands are run from the repo root on the laptop. `OMAMOBILE_SSH_KEY` points at the r8q key. **Steps 1-4 are writes and need the human's explicit go.** Read-only steps come first.

### Step 0 — read-only preflight (no approval needed)

```sh
cd ~/Work/omarchy-mobile-s20fe
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh 'r8q-preflight --json'
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/nv-backup -d r8q --check \
  ~/omamobile-kernel/reference-r8q/nv-backup-20261117-full
python3 -m py_compile tools/r8q-sahara
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh 'python3 - --dry-run' < tools/r8q-sahara
sha256sum ~/omamobile-kernel/out-r8q-modem/recovery-modem-s2.img
```

- **Expect / Pass:** `"ufs_guard":{"ok":true,"detail":"77 UFS partitions read-only"}`; `15/15 SAME`; `all r8q image sources present and readable`; image hash `c1d99560fa0f7e90a3083f28fe164bddd967a3175e1fce2d9c72ff8ab8233cc7`.
- **Stop condition:** any backup mismatch, missing source, or hash mismatch — do not continue.
- **Rollback:** none. **Risk:** none (read-only).

### Step 1 — install modules for the new kernel (WRITE — needs go)

```sh
DEVICE=r8q KERNEL_DIR=~/omamobile-kernel/wt-r8q KERNEL_OUT=~/omamobile-kernel/out-r8q-modem \
  OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./bin/omamobile modules-push
```

- **Expect / Pass:** installs `/usr/lib/modules/7.3.0-rc6-omamobile-push1-g4d5ae30a8318` and runs `depmod`; the phone may still report its running kernel as `...-gf3a5609d121b`, which is fine — these modules are for the next boot.
- **Stop condition:** `modules-push` refuses because the folder holds hand-installed modules. Stop and ask; do **not** set `OMAMOBILE_MODULES_REPLACE=1` without a separate decision.
- **Rollback:** the old module tree stays in place; nothing is deleted. **Risk:** low (rootfs write).

### Step 2 — stage the Sahara loader and delay MHI binding (WRITE — needs go)

```sh
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh 'cat > /usr/local/bin/r8q-sahara' < tools/r8q-sahara
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh \
  'chmod 755 /usr/local/bin/r8q-sahara
   printf "blacklist mhi_pci_generic\n" > /etc/modprobe.d/omamobile-r8q-sahara-bringup.conf
   depmod -a'
```

The blacklist keeps `0002:01:00.0` unbound until the loader is listening, so the SBL's Sahara HELLO (20 s timeout) is not missed. `qrtr-mhi` and Wi-Fi use the MHI core, not this driver, so they are unaffected.

- **Expect / Pass:** `r8q-sahara --dry-run` still passes from `/usr/local/bin`; the blacklist file exists.
- **Stop condition:** any existing `omamobile-r8q-sahara-bringup.conf` with other contents — stop and inspect rather than overwrite.
- **Rollback:** `rm /etc/modprobe.d/omamobile-r8q-sahara-bringup.conf && depmod -a`. **Risk:** low.

### Step 3 — flash the new recovery image to `sda24` (WRITE — needs go)

```sh
KERNEL_OUT=~/omamobile-kernel/out-r8q-modem OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" \
  ./tools/r8q-flash-recovery ~/omamobile-kernel/out-r8q-modem/recovery-modem-s2.img
```

- **Expect / Pass:** `PARTNAME=recovery`, image staged in `/tmp` and sha256-verified before the write, partition set back to `ro=1` afterwards; `boot`/`sda23` (LineageOS) untouched.
- **Stop condition:** partition name/size check fails, or the staged hash differs. The tool stops before writing.
- **Rollback:** re-flash the previous verified image (`KERNEL_OUT=~/omamobile-kernel/out-r8q-fold ./tools/r8q-flash-recovery out-r8q-fold/recovery-omamobile.img`). **Risk:** medium (recovery slot only; single-slot device, so the human must be ready to recover).

### Step 4 — reboot into the new image (needs go; affects the screen)

```sh
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh 'systemctl reboot'
```

- **Expect / Pass:** phone returns on `sda24` with `uname -r` = `7.3.0-rc6-omamobile-push1-g4d5ae30a8318`; `r8q-preflight --json` guard still `ok`.
- **Stop condition:** phone does not return — use the normal r8q recovery path, do not send more commands.
- **Rollback:** Step 3 rollback, then reboot. **Risk:** medium.

### Step 5 — attended first Sahara handshake (loader writes only to the modem port)

Start the log stream **first**, in a laptop terminal:

```sh
mkdir -p logs/r8q
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh \
  'python3 /usr/local/bin/r8q-sahara --device /dev/wwan0sahara0 --wait 180 --max-images 4' \
  2>&1 | tee logs/r8q/sahara-stage2-first.log
```

Then, in a second terminal, release the driver:

```sh
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh 'modprobe mhi_pci_generic'
```

- **Expect / Pass:** `MHI PCI device found: qcom-sdx55m` in `dmesg`; BHI loads `qcom/sdx55m/sbl1.mbn`; `/dev/wwan0sahara0` appears; the loader logs `HELLO` followed by `IMAGE ID <n> ...` for the first four requests, then exits at `--max-images`. No NV partition is written (all sources are opened `O_RDONLY`).
- **Stop condition:** no HELLO within the 50 s MHI ready timeout (capture `dmesg` and `/sys/kernel/debug/mhi/mhi0/`, then stop — do not repeatedly modprobe); a request for a cross-platform ID (28/30/31/37); any `SAME`→`DIFFERENT` in a fresh NV check.
- **Rollback:** unbind/remove the module and remove the bringup blacklist; re-check the 15-partition backup. **Risk:** low to the phone; the modem may simply fail to finish booting.

### Step 6 — full serve run and evidence

If Step 5 produced a clean ID sequence, repeat without `--max-images`, then capture evidence:

```sh
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh \
  'dmesg | grep -iE "mhi|sahara|sdx55" | tail -80; echo ===; ls /sys/kernel/debug/mhi/ 2>/dev/null; echo ===; cat /sys/bus/pci/drivers/mhi_pci_generic/*/uevent 2>/dev/null | head'
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/nv-backup -d r8q --check \
  ~/omamobile-kernel/reference-r8q/nv-backup-20261117-full
```

Record the results in this plan (§6, with the date and `uname -r`), quoting only the relevant lines, not whole logs. **Check for identifiers before quoting.** Remove the bringup blacklist once the modem path is stable, and decide whether the loader belongs in `packages/omamobile-device-r8q` or stays a bring-up tool.

---

## 5. Rollback

If an attended overlay test breaks boot or stability:
1. Re-flash `sda24` without `--overlay modem` using the verified `out-r8q-fold/recovery-omamobile.img` (`--dual-boot`).
2. Reboot; confirm `r8q-preflight --json` and the 15-partition NV check are clean.
3. Remove `/etc/modprobe.d/omamobile-r8q-sahara-bringup.conf` if it was installed.

---

## 6. Status

| Item | Status | Notes |
|------|--------|-------|
| UFS storage guard (`77` parts `ro=1`) | ✅ Verified | Checked live via `r8q-preflight --json` |
| Full NV backup (`nv-backup-20261117-full`) | ✅ Verified | `15/15 SAME` (2026-11-17, `tools/nv-backup -d r8q --check`) |
| Stock topology (`SDX55M` on `pcie2` + `mhi`) | ✅ Identified | Modules already built (`mhi_pci_generic`, `qrtr-mhi`, `mhi_wwan_*`) |
| `device/r8q/overlays/modem.dtso` | ✅ Verified | `pcie2` Gen.3 x2 link up; `0002:01:00.0` `[17cb:0306]`/`[17cb:010c]` probed cleanly |
| Stage 2 kernel (`modem-stage2`) | ✅ Built | `7896cfb4b` + `4d5ae30a8`; `kernel-pr-check` clean; `out-r8q-modem` |
| Stage 2 artifacts | ✅ Packed | `modules-...-g4d5ae30a8318.tar.zst`, `recovery-modem-s2.img` (hashes in §3.2) |
| `tools/r8q-sahara` | ✅ Dry-run passed | All r8q image sources present/readable on the phone |
| BHI `sbl1.mbn` load & Sahara handshake | ⏳ Runbook §4 | Needs human go for writes and an attended run |
| `omamobile-modem` (`rmtfs`/`pd-mapper`) & `ModemManager` | ⏳ Pending | Follows a successful Sahara transfer |

---

## 8. Stage 3A — PCIe2 MSI in the endpoint IOMMU domain (2026-11-17)

**Why the first Stage 2 run stalled.** *(The reading below was written before the logs were re-read; §9.2 corrects the one part it got wrong — READY is polled, so `mhi1` did reach `Wait for device to enter SBL`, and it is the SBL's later BHI interrupt that never arrived. The conclusion, that the MSI is what is missing, stands.)* `mhi1` powered on and BHI loaded `sbl1.mbn`, but the device stayed in `EE: PRIMARY BOOTLOADER` and no `/dev/wwan*sahara*` channel appeared. `pci 0002:01:00.0: Adding to iommu group 13` shows the endpoint is behind `apps_smmu`. The DWC host's iMSI-RX uses the RC's cfg0 address (`0x64100000`) as the MSI target, and that address is not mapped in the endpoint's IOMMU domain, so the endpoint's MSI writes fault. MHI event rings therefore never raise an interrupt, the SBL execution-environment transition never completes, and the SAHARA channels are never created. This matches the MHI documentation's BHI/SBL flow (the host must observe the device's state transitions) and the same failure has been reproduced on another SM8250 SDX55M device with these PCI IDs.

**Change (kernel `modem-stage3`, commit `5be4efd5e`):** `drivers/pci/controller/dwc/pcie-designware-host.c` calls `iommu_dma_prepare_msi()` when allocating the endpoint's MSI on PCIe2 (cfg0 `0x64100000`) and lets `msi_msg_set_addr()` place the allocated IOVA in the message. PCIe0 (the QCA6390 Wi-Fi card) keeps the raw iMSI-RX path it already works on. `CONFIG_IRQ_MSI_IOMMU=y` is already set in `device/r8q/kernel.config`'s build.

**Artifacts** (`~/omamobile-kernel/out-r8q-modem3/`):
- `modules-7.3.0-rc6-omamobile-push1-g5be4efd5e298.tar.zst`
- `recovery-modem-s3-msi.img` — sha256 `19a203d747df775d1b5583c0ee408fe2dd9f39697f7054d1ab098f93a0e89c52`

### Step 0 — read-only preflight (no approval needed)

```sh
cd ~/Work/omarchy-mobile-s20fe
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh 'r8q-preflight --json'
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/nv-backup -d r8q --check \
  ~/omamobile-kernel/reference-r8q/nv-backup-20261117-full
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh \
  'cat /etc/modprobe.d/omamobile-r8q-sahara-bringup.conf; mount | grep " / "'
sha256sum ~/omamobile-kernel/out-r8q-modem3/recovery-modem-s3-msi.img
```

- **Expect / Pass:** guard `77 UFS partitions read-only`; `15/15 SAME`; the blacklist still present; `/` mounted `rw`; image hash `19a203d7…`.
- **Stop condition:** backup mismatch, guard not ok, root not `rw`, or the blacklist missing.
- **Rollback:** none. **Risk:** none (read-only).

### Step 1 — install the new modules (WRITE — needs go)

```sh
DEVICE=r8q KERNEL_DIR=~/omamobile-kernel/wt-r8q KERNEL_OUT=~/omamobile-kernel/out-r8q-modem3 \
  OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./bin/omamobile modules-push
```

- **Expect / Pass:** installs `/usr/lib/modules/7.3.0-rc6-omamobile-push1-g5be4efd5e298` and runs `depmod`.
- **Stop condition:** `modules-push` refuses (hand-installed modules in the folder). Stop and ask; do not set `OMAMOBILE_MODULES_REPLACE=1`.
- **Rollback:** the old module tree stays. **Risk:** low.

### Step 2 — flash the Stage 3A image to `sda24` (WRITE — needs go)

```sh
KERNEL_OUT=~/omamobile-kernel/out-r8q-modem3 OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" \
  ./tools/r8q-flash-recovery ~/omamobile-kernel/out-r8q-modem3/recovery-modem-s3-msi.img
```

- **Expect / Pass:** `PARTNAME=recovery`, staged and sha256-verified, written, read back at `19a203d7…`, `sda24 ro=1` afterwards.
- **Stop condition:** partition name/size check fails or the staged hash differs (the tool stops before writing).
- **Rollback:** re-flash the previous verified image (`out-r8q-modem/recovery-modem-s2.img`). **Risk:** medium (recovery slot only).

### Step 3 — reboot (needs go; affects the screen)

```sh
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh 'systemctl reboot'
```

Start `tools/r8q-boot-watch --key ~/.ssh/omamobile-r8q --log logs/r8q/stage3a-reboot.log` first. USB re-enumeration after a reboot has taken several minutes before; allow time and check the screen before assuming a hang.

- **Expect / Pass:** phone returns with `uname -r` = `7.3.0-rc6-omamobile-push1-g5be4efd5e298`; guard ok; root `rw`.
- **Stop condition:** no return after several minutes and a dark screen — ask the human to force-restart (Power ~30 s, then Power; or Power + Volume Down ~10 s).
- **Rollback:** Step 2 rollback, then reboot. **Risk:** medium.

### Step 4 — attended SBL/MSI check (loader writes only to the modem port)

Start the log stream first:

```sh
mkdir -p logs/r8q
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh 'dmesg -w' \
  > logs/r8q/stage3a-dmesg.log 2>&1 &
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh \
  'python3 /usr/local/bin/r8q-sahara --device /dev/wwan0sahara0 --wait 180 --max-images 4' \
  2>&1 | tee logs/r8q/stage3a-sahara.log
```

Then release the driver in a second terminal:

```sh
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh 'modprobe mhi_pci_generic'
```

- **Expect / Pass:** `MHI PCI device found: qcom-sdx55m`; `mhi1` reaches `EE: SECONDARY_BOOTLOADER` (or beyond) instead of staying in PRIMARY BOOTLOADER; `/dev/wwan0sahara0` appears; the loader logs `HELLO` and the first `READ_DATA` image IDs. Check `dmesg | grep -iE "mhi|sahara|iommu"` and `/proc/interrupts | grep -i mhi` to confirm the MHI event IRQs now increment.
- **Stop condition:** the device still reads PRIMARY BOOTLOADER after the MSI fix, or no event interrupts appear; any `I/O error`/`emergency_ro` on the root filesystem; a request for a cross-platform image ID (28/30/31/37).
- **Rollback:** unbind/remove the module, remove the bringup blacklist, re-check the 15-partition backup. **Risk:** low to the phone; the root image is on the SD card, so keep journald quiet and stop on any storage error.

### Stage 3A results — 2026-11-17 (kernel `7.3.0-rc6-omamobile-push1-g5be4efd5e298`)

The runbook was run with the owner's go. Steps 0-2 passed:

- preflight: guard `77 UFS partitions read-only`, NV 15/15 `SAME`, root `rw`, image `19a203d7…`.
- modules installed for `g5be4efd5e298`.
- `sda24` written, read back `19a203d7…`, and left `ro=1` (`/sys/block/sda/sda24/ro`); the tool's own status echo then used `/sys/block/sda24/ro`, which is not a path, so it printed an empty value after the write (fixed 2026-11-17: the echo now uses `/sys/block/sda/sda24/ro`).

**Result — the phone reset.** With the loader waiting on `/dev/wwan0sahara0`, `modprobe mhi_pci_generic` printed `MHI PCI device found: qcom-sdx55m` and `mhi mhi1: Power on setup success`, and the SSH connection was reset at once. The host watcher saw the USB gadget disappear and the phone come back as a fresh boot; the device never printed `Wait for device to enter SBL or Mission mode` and no SAHARA port appeared. So the PCIe2 MSI change did not produce a usable SBL transition: the machine went down at (or immediately after) MHI power-on.

**Recovery.** The first boot after the reset did not detect the SD card that holds `/omarchy-r8q.img` (`omamobile: loop: /dev/mmcblk0p1 not present`; the initramfs held in its rescue shell). A host-side `sdhci_msm` unbind/bind attempt did not answer and the phone reset again; the next boot enumerated `/dev/mmcblk0p1` and mounted the root image cleanly (no ext4 recovery errors). After that: guard ok, NV 15/15 `SAME` (`param` differs as expected after a flash), modem driver not loaded, and `/` mounted `rw`.

**Reading.** The patch is line-for-line the approach used on another SM8250/SDX55M port, so the difference is likely in this base or this device's PCIe2/IOMMU wiring rather than in the idea. Two candidates to check before another attempt:

1. the MSI IOVA is not in the endpoint's SMMU context (different stream ID for MSI than for DMA), so the first MSI write faults and the abort takes the SoC down;
2. the write is translated to cfg0 but the RC does not terminate it there, with the same outcome.

A softdog/`softlockup_panic` reset after a hang fits the observed delay as well as an immediate abort does; the log ends at power-on either way, so the mechanism is not yet proven.

**Do not repeat this test as-is.** Any kernel carrying `5be4efd5e` resets the phone when `mhi_pci_generic` binds. Keep the bringup blacklist in place (`/etc/modprobe.d/omamobile-r8q-sahara-bringup.conf`) until a different approach is agreed. The next step is offline: compare the endpoint's SMMU stream IDs and the MSI/IOMMU domain flags against the reference port, and decide whether to (a) drop the MSI change and go back to the "no event interrupt" stall, or (b) build a diagnostic kernel that maps the MSI at probe and reads back the SMMU fault state, under a dead-man switch. **Done 2026-11-17: §9 records the comparison and the decision (the stock memory-doorbell MSI controller; no polling shortcut exists, and the diagnostic kernel stays optional).**

**Way back (2026-11-17).** `device/r8q/stable-image.conf` pins the image that was verified end-to-end before this work (`7.3.0-rc6-omamobile-push1-g3667ab368ef9`, sha256 `66fe127f…d8503`, packed `--dual-boot --overlay camera-uw`). `tools/r8q-flash-recovery --stable` writes those exact bytes to `sda24` and refuses a rebuild that no longer hashes to the record; `--dry-run` stops after the checks. The phone was restored with it after Stage 3A: `sda24` read back the same sha256 and stayed `ro=1`, the next boot reported the recorded release, the temporary blacklist and `/usr/local/bin/r8q-sahara` were removed, and `tools/r8q-stable-check` passed every check (release, `sda24`, 77/77 `ro=1`, root `rw`, PCIe2/PHY `disabled`, no SDX55M on `mhi_pci_generic`, modules present). Restore the same record after any further experiment; a rebuilt kernel needs its own hardware pass and a new record, never a silent replacement of this one.

---

## 9. Offline Design After Stage 3A — the PCIe2 MSI Path (2026-11-17)

**Design only.** Nothing in this section was run on the phone. The phone stays
on the stable image (§8, [`stable-image.conf`](stable-image.conf)); every write,
flash and reboot below needs the human's go, and the driver work in 9.5 is
laptop-only.

### 9.1 What the three trees actually do

| | Stock `qcom,pci-msm` | Mainline `qcom,pcie-sm8250` | `pcie-gs201-cp.c` (same tree) |
|---|---|---|---|
| MSI controller | `qcom,pci-msi` at doorbell `0x17a10040`, 32 GIC SPIs per root complex | DWC iMSI-RX; `PCIE_MSI_ADDR` = `cfg0_base` = `0x64100000` | DWC iMSI-RX; target = a `no-map` reserved page (`cp-msi@f6200000`) |
| Doorbell kind | memory register | address match inside the controller | address match inside the controller, RAM |
| Who translates the write | `apps_smmu` (RID → SID from `iommu-map`) | none expected: iMSI-RX is meant to terminate the TLP | none expected (Google's driver sets `pp->msi_data` to the reserved page, no IOMMU mapping) |
| MHI DMA domain | separate `qcom,iommu-group`, pool `0x20000000-0x3fffffff`, `fastmap`, coherent | the endpoint's default `dma-iommu` domain | the CPIF driver's own IOMMU |
| Evidence | `~/omamobile-kernel/reference-r8q/live.dts` `pcie2_msi`; stock `/proc/interrupts`: `gic_msm_pci_msi` `0x340+` named `mhi` | `sm8250.dtsi` `&pcie2`; `pcie-designware-host.c` | driver source in this tree |

Stock gives each root complex its own node at the **same** doorbell address,
with a distinct SPI bank:

| Node | Doorbell | SPIs |
|---|---|---|
| `pcie0_msi` | `0x17a10040` | `0x300-0x31f` |
| `pcie1_msi` | `0x17a10040` | `0x320-0x33f` |
| `pcie2_msi` | `0x17a10040` | `0x340-0x35f` |

`&pcie2` carries `msi-parent = <&pcie2_msi>`, and stock's interrupt list shows
the modem's ten MHI vectors on the `0x340` bank. The block therefore receives a
plain memory write from the endpoint and selects the bank by root complex.
Mainline has no driver for `qcom,pci-msi` (checked in this tree: no match in
`drivers/irqchip` or `drivers/pci`).

### 9.2 What the two runs actually show

The Stage 2 log (`logs/r8q/sahara-stage2-dmesg.log`) ends with:

```
mhi mhi1: Requested to power ON
mhi mhi1: Power on setup success
mhi mhi1: Wait for device to enter SBL or Mission mode
```

so `mhi_ready_state_transition()` **did** succeed. It polls `MHISTATUS` every
25 ms (`mhi_poll_reg_field()` in `drivers/bus/mhi/host/pm.c`), so READY needs no
interrupt. The message is printed after the event rings are set up and the MHI
state is set to M0. What never followed was the device's move to SBL: the SBL
changes `BHI_EE` and signals the host with the **BHI interrupt**, an MSI;
`mhi_intvec_threaded_handler()` reads the new EE and `mhi_sbl_transition()`
creates the Sahara channels. With no MSI the host never reads the new EE, never
creates `/dev/wwan*sahara*`, and the device waits in SBL. That is the Stage 2
stall: one missing interrupt, no fault on the phone.

Stage 3A then died about 0.3 s after the same power-on point, **before** the
`Wait for device to enter SBL or Mission mode` line — i.e. during or just after
the ready transition, at the moment the device first signals the host. The
change had mapped the doorbell with `iommu_dma_prepare_msi()` and programmed the
endpoint with a `dma-iommu` IOVA, while `PCIE_MSI_ADDR` still held `0x64100000`.
The iMSI-RX matches the address it is given; with a different address in the
message the write is no longer a doorbell, and after translation it is an
ordinary write to the PCIe2 config/DBI block. That is not a legal DMA target —
an abort there, or a corrupted controller block, fits a reset with no further
log. The recipe is wrong in principle: `iommu_dma_prepare_msi()` is for a
**memory** doorbell (GIC ITS, MBI, GICv2m), not for an address-matching doorbell
inside the DWC core.

Two details stay open because the phone died before they could be observed: the
exact tap point of the iMSI-RX (before or after the inbound TBU) and whether the
write aborted or landed in the config block. The design below does not depend on
either answer, which is why it is preferred over another iMSI-RX variant.

### 9.3 Decision

**Design A (adopted): give PCIe2 the stock MSI topology in mainline.** Add a
small platform MSI controller for `qcom,pci-msi` (a memory doorbell plus 32
parent SPIs), attach it to `&pcie2` with `msi-parent`, and leave iMSI-RX to
PCIe0 (the QCA6390 Wi-Fi, which works on it today).

- The endpoint's write is a normal translated memory write to `0x17a10040`,
  exactly as on stock.
- The generic MSI core maps the doorbell into the endpoint's domain with
  `iommu_dma_prepare_msi()`, and the message carries that IOVA — correct here,
  because the hardware doorbell really is memory.
- `dw_pcie_host_init()` sees `msi-parent` and turns `use_imsi_rx` off for PCIe2
  only; no DWC MSI registers are armed for that host.
- The driver is a near-copy of `drivers/irqchip/irq-gic-v3-mbi.c` (a doorbell
  register, a bitmap of SPIs, `iommu_dma_prepare_msi()` in `alloc`,
  `msi_msg_set_addr()` in `compose`, `msi_parent_ops` +
  `msi_create_parent_irq_domain()`), roughly 150 lines plus the binding.

**Design B (fallback, only if A cannot work):** keep iMSI-RX and point
`pp->msi_data` at a `no-map` reserved page, as `pcie-gs201-cp.c` does. It is
cheap, but it only works if the iMSI-RX tap sits before the SMMU — the exact
question Stage 2/3A left open — and it needs a device-tree `memory-region` plus
a way to override `msi_data` from the r8q side. It is a diagnostic step, not
the first choice.

**Rejected:** anything that programs an IOVA into `PCIE_MSI_ADDR` while
`use_imsi_rx` is true (that is `5be4efd5e`).

### 9.4 Why not just poll

READY is polled, but SBL is not: the device announces its move to SBL with the
BHI interrupt, and `mhi_intvec_threaded_handler()` is what reads the new EE and
creates the Sahara channels. The data path needs interrupts for the same reason
— every event ring and the BHI doorbell are MSIs. A polling variant could at
best observe the device's state, not run it. There is no "drop MSI" shortcut
that reaches a usable modem.

### 9.5 Implementation plan (offline, no phone)

1. **Kernel branch** `r8q-modem-msi` off `modem-stage2` (`4d5ae30a8`, the
   `pci_generic.c` `samsung,r8q` BHI quirk). Do **not** carry `5be4efd5e`.
2. **Driver** `drivers/irqchip/irq-qcom-pci-msi.c` (+ Kconfig, Makefile),
   modeled on `irq-gic-v3-mbi.c`:
   - parse the `reg` doorbell and the 32 parent interrupts; one bitmap per
     instance;
   - `alloc`: pick the next free SPI and call
     `iommu_dma_prepare_msi(info->desc, doorbell)`;
   - `compose`: `msg->data = vector index`, then
     `msi_msg_set_addr(desc, msg, doorbell)`;
   - `msi_parent_ops` with MBI's required/supported flags, created with
     `msi_create_parent_irq_domain()`.
3. **Binding** `Documentation/devicetree/bindings/interrupt-controller/qcom,pci-msi.yaml`:
   `compatible = "qcom,pci-msi"`, one `reg` (doorbell), `msi-controller`, the
   32 `interrupts`; `#msi-cells = <0>` is the default.
4. **r8q device tree** (base `sm8250-samsung-r8q.dts`, so the controller exists
   before the modem overlay enables the link):
   `pcie2_msi: msi-controller@17a10040 { compatible = "qcom,pci-msi"; reg =
   <0x17a10040 0x4>; msi-controller; interrupts = <GIC_SPI 832 ... 863>; }`
   and `&pcie2 { msi-parent = <&pcie2_msi>; };`. The stock DT uses a zero-sized
   `reg` and shares the address across three nodes; the r8q node declares the
   four bytes and uses only the `0x340` bank.
5. **Config:** `CONFIG_QCOM_PCI_MSI=y` in `device/r8q/kernel.config` (built in,
   because the MSI controller must exist before the PCI host probes).
6. **Gates:** `bin/omamobile kernel` with its own output tree,
   `checkpatch --strict`, `dt-validate` on the new binding and the r8q DTB, and
   `tools/kernel-pr-check --build` for the touched files.
7. **Runbook §10** (new, gated, attended): read-only preflight, modules, flash
   `sda24`, reboot, then the Sahara loader waiting while `mhi_pci_generic`
   binds, under a log stream and a dead-man switch. Stop on any SMMU context
   fault, `I/O error` on the root, or a reset. Rollback is
   `tools/r8q-flash-recovery --stable`.

### 9.6 What cannot be settled offline

1. **The SMMU context of the doorbell mapping.** `iommu-map` routes RID `0x100`
   to SID `0x1d01`, so the mapping should land in the endpoint's own domain.
   Stock's separate MHI group (pool `0x20000000-0x3fffffff`) is a downstream
   device; mainline has no per-MHI group. Confirm the IOVA appears in the right
   domain with the IOMMU debugfs before the first MHI bind.
2. **Whether Linux may touch `0x17a10040`.** It lies outside the GICD window
   (`0x17a00000-0x17a0ffff`) and the GICR window (`0x17a60000+`), and no
   mainline driver owns it. The stock MSI driver arguably never reads it (the
   endpoint writes the doorbell). A single read at probe, before any mapping,
   proves it; do that read first.
3. **How the block selects the bank.** All three stock nodes share the address,
   so the selection must be by root complex. r8q uses only the `0x340` bank;
   the first test should allocate one vector to keep the question small.
4. **The endpoint's MSI capability.** Stock shows ten MHI vectors, so MSI or
   MSI-X with ten vectors is enough; the first link-up should print the
   negotiated vector count (`pci_alloc_irq_vectors` and MHI both log it).
5. **Whether the modem's SBL needs a specific DMA IOVA range.** If the first MHI
   event still does not arrive with a working MSI, this is the next suspect,
   not the MSI controller.

### 9.7 Optional read-only diagnostic before the driver

If the mechanism should be proven first, a kernel with only

- a one-shot SMMU context-fault log (SID, SSID, address, FSR; no panic), and
- a readback of `PCIE_MSI_ADDR_LO/HI`, `PCIE_MSI_INTR0_ENABLE` and the
  endpoint's MSI/MSI-X message after `pci_alloc_irq_vectors()`

keeps the Stage 2 path and should behave like Stage 2 (no reset). It turns "no
MSI" into either "SMMU context fault at `0x64100000`" (confirms 9.2) or "the
message address is not `0x64100000`" (a different bug). It needs a flash and a
reboot, so it waits for a go; it is **not** required before Design A.

### 9.8 Rules for the next attempt

- The phone stays on the stable image until the new kernel has passed its own
  hardware run; every test starts from `tools/r8q-stable-check --phone`.
- One change per boot: the MSI controller first, then the MHI bind.
- Never bind `mhi_pci_generic` with an NV overlay on, and never leave the
  modules installed after the test: restore the blacklist from §8 or reflash the
  stable image.
- Any reset costs more than one boot before the SD card returns (Stage 3A
  showed one boot without `/dev/mmcblk0p1`). Do not power-cycle a phone that is
  still answering, and check the guard and the 15-partition NV checksums after
  any reset.

---

## 10. Stage 4 Runbook — the qcom,pci-msi doorbell (NOT RUN)

**Gated and attended. Every write, flash and reboot below needs the human's
go.** The kernel is `r8q-modem-msi` (binding + driver, off `modem-stage2`
`4d5ae30a8`, without `5be4efd5e`); the s20fe side is this branch
(`CONFIG_QCOM_PCI_MSI=y`). Pack from its own output tree
(`~/omamobile-kernel/out-r8q-msi`); never reuse a Stage 3A artifact.

**Artifacts (packed 2026-11-17, laptop-only):**
- `modules-7.3.0-rc6-omamobile-push1-gff311d19e68f.tar.zst` — sha256 `35a5c87a…be5599464ab8`
- `modules-7.3.0-rc6-omamobile-push1-gc3867f30962c.tar.zst` — sha256 `1e7c057d…0a705ef7b20a4a8`
- `recovery-modem-s4-msi.img` — sha256 `cbd7c657…584c09c33c` (79 MB, AVB-signed),
  packed with `--dual-boot --overlay modem`; MSI data is now the bank base
  `0x10080000` ORed with the offset (stock names this bank
  `0x10080000+i`).
  (`43f4c7b6…` lacked the pack flags; `2ca70c35…` oopsed in compose;
  `d12c1796…` used SPI numbers; `b2d18af5…` used bare offsets. Zero
  interrupts on each. All discarded. `cbd7c657…` was never flashed: the
  root went `emergency_ro` again before `modules-push`, so the phone was
  powered off with `sda24` still holding `b2d18af5…`.)

### Step 0 — read-only preflight (no approval needed)

Same as §8 Step 0, plus: `uname -r` is the recorded stable release and
`mhi_pci_generic` is not loaded. Rebuild the images only from
`KERNEL_OUT=~/omamobile-kernel/out-r8q-msi` and confirm the new sha256
matches the record above before flashing.

- **Expect / Pass:** guard `77 UFS partitions read-only`; NV 15/15 `SAME`;
  root `rw`; image hash matches the record.
- **Stop condition:** any mismatch. **Rollback:** none. **Risk:** none.

### Step 1 — install the new modules (WRITE — needs go)

`DEVICE=r8q KERNEL_DIR=~/omamobile-kernel/wt-r8q KERNEL_OUT=~/omamobile-kernel/out-r8q-msi ./bin/omamobile modules-push`
(plus the r8q Sahara bringup blacklist from §8, re-applied).

- **Expect / Pass:** modules for the new release install, `depmod` clean.
- **Stop condition:** `modules-push` refuses (hand-installed modules).
  Stop and ask. **Rollback:** the old module tree stays.

### Step 2 — flash the Stage 4 image to `sda24` (WRITE — needs go)

`tools/r8q-flash-recovery` with the Stage 4 image from the record above.

- **Expect / Pass:** staged and sha256-verified, written, read back
  identical, `sda24 ro=1` afterwards (check `/sys/block/sda/sda24/ro`).
- **Stop condition:** hash differs. The tool stops before writing.
  **Rollback:** re-flash the stable image (`tools/r8q-flash-recovery --stable`).
- **Fallback (2026-11-17):** if the phone is in the rescue shell, the
  LUN flag cannot be cleared there (no `/dev/sda` node, sysfs `ro` is
  read-only) and the tool aborts before writing anything. Flash from
  Download Mode instead, per `RUNBOOK-DISPLAY-GPU.md` Gate A:
  `heimdall flash --RECOVERY <image> --no-reboot`, boot with
  Volume Up + Power, then verify by reading `sda24` back (`dd` sha) and
  `uname -r` — a Heimdall flash once left a stale image behind
  (NOTES §2). Writes `sda24` only; `sda23`/LineageOS untouched.

### Step 3 — reboot (needs go; affects the screen)

`tools/phone-ssh 'systemctl reboot'` with `tools/r8q-boot-watch` running
first, as in §8 Step 3.

- **Expect / Pass:** `uname -r` is the new release; guard ok; root `rw`;
  `dmesg | grep -i "qcom-pci-msi"` shows the probe line with the doorbell
  readback and `32 SPIs from 832`; **no reset**.
- **Stop condition:** no return after several minutes (ask the human;
  Power ~30 s, never send commands to a silent phone); a reset at any
  point; an SMMU context fault naming `0x17a10040`.
  **Rollback:** Step 2 rollback, then reboot. **Risk:** medium.

### Step 4 — first MSI vector, no modem bind yet (attended)

With the log stream running (`dmesg -w` to `logs/r8q/stage4-dmesg.log`),
allocate **one** MSI vector without binding the modem: e.g.
`pci_alloc_irq_vectors()` on the PCIe2 port via debugfs if available, else
bind `mhi_pci_generic` only after Step 4b passes. (Detail to be filled
once the packed kernel's debugfs/port layout is known.)

- **Expect / Pass:** one parent SPI (832) is requested; `/proc/interrupts`
  shows the `QCOM-PCI-MSI` domain; no fault, no reset.
- **Stop condition:** SMMU fault, `I/O error` on the root, or a reset.
  **Rollback:** unbind, restore the blacklist, re-check the 15-partition
  backup.

### Step 4b — attended SBL/MSI check (loader writes only to the modem port)

Only after Step 4 passes. Same shape as §8 Step 4: the Sahara loader
waiting on `/dev/wwan0sahara0`, then `modprobe mhi_pci_generic` in a
second terminal, under the log stream and a dead-man switch.

- **Expect / Pass:** `MHI PCI device found: qcom-sdx55m`;
  `Wait for device to enter SBL or Mission mode`; then an SBL transition
  (`EE: SECONDARY_BOOTLOADER` or beyond), `/dev/wwan0sahara0`, and the
  loader's first `IMAGE ID`s — with the MHI event IRQs incrementing in
  `/proc/interrupts`.
- **Stop condition:** still PRIMARY BOOTLOADER after the MSI fix; no event
  interrupts; any `I/O error`/`emergency_ro`; a cross-platform image ID
  (28/30/31/37); a reset.
  **Rollback:** unbind/remove the module, remove the bringup blacklist,
  re-check the 15-partition backup — then reflash the stable image, as in §8.

### Results — Stage 4 (2026-11-17, HALTED on a storage stop-condition)

**What ran.** Image `2ca70c35…` (first correct pack) was flashed via
Download Mode/Heimdall after the SSH flasher proved unable to clear the
LUN flag from the rescue shell; read back `2ca70c35…`, `ro=1`. The phone
booted Arch, and the driver probed cleanly — doorbell reads `0x0`, 32
SPIs from 832, no SMMU fault — but the root port's async MSI allocation
oopsed on a NULL `chip_data` in compose: the per-device chip copy carries
the callback, so it runs on the leaf irq_data (MBI reads
`data->parent_data` for the same reason). The oops wedged async, hanging
the first `modprobe`. Fixed (`aafa78617`), rebuilt, modules pushed and
reflashed from Arch with the SSH tool (`d12c1796…`, read back verified),
rebooted — full story in §10 history.

**MSI allocation: WORKS.** On `gaafa78617e1a`, the root port holds a live
`QCOM-PCI-MSI` vector (`PCIe PME, aerdrv, PCIe bwctrl`, SPI 832), and the
modem bind allocated ten vectors (`bhi` + `mhi`, `0002:01:00.0`) with no
fault and no hang. But every count stayed at zero and the device never
left `PRIMARY BOOTLOADER`: the programmed data was the SPI number
(`0x348` = 840 in the endpoint's MSI capability), while the doorbell
selects 1-of-32 **by index** — classic MSI shares one data value with the
device ORing in the vector, so anything above 31 is silently dropped, no
fault, no interrupt. The middle-domain hwirq is now the bank offset and
the GIC parent alloc still takes `spi_start + offset` (`ff311d19e`,
committed, **not yet run on hardware**).

**STORAGE HALT.** During the stalled bind the loop root started failing
writes (`I/O error, dev loop0`, spread across sectors, no mmc errors),
the journal aborted, `/` went `emergency_ro`, and eventually even
executing `/usr/bin/dd` failed with EIO. The bind was undone (`rmmod`
clean), `modules-push` for the new release was correctly refused (ro
root), and the phone was synced + powered off via SysRq. Cause unknown:
media wear vs. a transient glitch. No NV partition was ever written
(NV 15/15 `SAME` on every check); `sda24` holds the untested `b2d18af5…`
image and the phone cannot return to Arch until the SD-backed
`/omarchy-r8q.img` is checked.

**Way back.** The SD card must be checked on a laptop (card reader):
`fsck` the partition and the ext4 image inside it; if the card is dying,
replace it — the rootfs is rebuildable (`tools/build-rootfs`) and no
phone-unique data lives in it beyond the demo/user layer. Until the root
is healthy, no modem bind, no flash, no reboot-to-test. The stable
`sda24` reflash (`--stable`) is also blocked on the same root (its
preflight needs a working phone); Download Mode + Heimdall stays the
available write path. The offset-data fix (`ff311d19e` + image) is ready
to test once storage is green.

---

## 11. SD-less Rescue Test — MSI Delivery Without the SD Card (2026-11-17)

**Why.** The SD-backed root went `emergency_ro` twice with block-layer
write errors, halting all Stage 4 work that needs Arch (`modules-push`,
NV re-checks). The modem bind itself needs no Arch: kernel + modules +
BHI firmware + a shell. `tools/r8q-boot pack --overlay modem
--rescue-test` stages the kernel modules tarball, `sbl1.mbn`/`sec.elf`
(read from the phone over SSH, hashes match) and `pciutils` into the
initramfs and packs **without** `--dual-boot`, so the phone stops at the
rescue shell (SSH works there, proven). The SD card stays out of the
phone: zero SD involvement, and the UFS storage guard still applies.

**Image (packed 2026-11-17, laptop-only):**
- `recovery-modem-rescue.img` — sha256 `452b3ed8…e69ca874b8fd`,
  kernel `7.3.0-rc6-omamobile-push1-g14d313c015f5` (GIC-hwirq message
  data + debugfs doorbell trigger + GICD SETSPI control knob),
  modem overlay, rescue modules + firmware verified inside the packed
  ramdisk (2913 entries: `mhi_pci_generic.ko`, `sdx55m/sbl1.mbn`,
  `sec.elf`, `lspci`, `dropbear`, Rust `r8q-sahara-rs`).

**Runbook (all phone writes need a go):**
0. Preflight: phone off, SD card OUT, USB plugged. (No NV check possible
   without Arch; the NV partitions stay untouched and read-only either
   way — the UFS guard marks them at boot and nothing in this test
   writes UFS.)
1. Download Mode + `heimdall flash --RECOVERY recovery-modem-rescue.img
   --no-reboot` (sda24 only, read back afterwards).
2. VolUp+Power boot to the rescue shell; SSH in.
3. `insmod /lib/modules/*/kernel/drivers/bus/mhi/host/mhi.ko` then
   `insmod .../mhi_pci_generic.ko`; watch `dmesg`, `/proc/interrupts`
   (`QCOM-PCI-MSI` counts), `/dev/wwan*`.
4. **Pass:** Sahara port nodes appear, MHI event counts increment from
   zero, `SECONDARY BOOTLOADER` in the log. That proves MSI delivery;
   the full Sahara download waits for healthy storage (no Python in
   rescue).
5. **Stop:** SMMU fault, reset, or still-zero counts (then the address
   mapping, not the data, is the suspect — §10 history).
6. Afterwards: power off via SysRq; reflash stable or the next test image
   the same way. No blacklist/modules cleanup needed (nothing installed).

**Status:** image packed and verified; flashing and the bind need the
human's go for Download Mode + the rescue SSH session.

### Results — Stage 4 rescue trigger test (2026-11-17, DELIVERY PROVEN)

**2026-11-18: the modem reaches AMSS.** See NOTES for the full record.
The remaining gate after delivery was the modem's own jump: the host was
not asserting AP2MDM_STATUS (gpio56), and the SAHARA walk was spread over
14 channel reopenings (each a STOP/RESET/START) where stock serves it over
one session (`ks -n`). With gpio56 high from the PCIe2 pinctrl state and
one open channel, the SBL completed the image set and entered mission
mode: `Mission mode doorbell offsets: CHDBOFF=0x300 ERDBOFF=0x700`,
`EE: MISSION MODE`, AMSS channels `QMI/MBIM/IPCR/IP_HW0/IP_SW0`, and
`/dev/wwan0{qmi,mbim,qcdm,at}0` all present.

**The image that achieved it:** `recovery-modem-rescue.img`, sha256
`0e3c74b5…77f1acef`, packed 2026-11-18 with `--overlay modem
--rescue-test` on kernel `gbd4cb8a49313` (sda24 held these exact
bytes when AMSS was reached).

The downstream `pci-msm-msi` source (winnerx tree) settled the data
question: its compose programs the **parent GIC hwirq** (`irqd_to_hwirq`
of the GIC irqdata). With the GIC's +32 SPI translation that is 864+i
for our bank (DT 832+i) — the stock `0x10080000+i` display numbers are
that driver's internal numbering, not doorbell data, and classic MSI
only carries 16 bits anyway.

Proven live from the rescue shell with a debugfs doorbell trigger:
- `SETSPI 864`/`872` increments the port/BHI counts — the GIC side
  (parent mapping, unmask, handler) is correct.
- `fire 872`/`873` increments the BHI/mhi1 counts — the doorbell
  decoder takes the GIC SPI hwirq as data. Values 0-33, 832, 840 and
  the old `0x10080008` stay silent.
- The fired BHI interrupts drove the real MHI stack: the host read
  `BHI_EXECENV=0x1` (SBL), transitioned to M0, created `mhi0_SAHARA`,
  and `/dev/wwan0sahara0` appeared after `mhi_wwan_ctrl` (+ its `wwan`,
  `gnss` deps) was insmodded.

The endpoint's own MSI never arrived only because the running driver
programmed data `0x0008` (truncated bank base) instead of `872+j`.
`irqchip: qcom-pci-msi: program the GIC SPI hwirq as message data`
programs exactly the proven value — and the retest with it (image
`bcb62616…`, rescue shell, no manual trigger) showed unassisted counts
on the BHI and event vectors, the host transition to SBL/M0, and
`/dev/wwan0sahara0` attached (`port wwan0sahara0 attached`). MSI
delivery is DONE. What remains is the Sahara firmware download, which
needs Python and the full firmware set — i.e. healthy storage, not a
rescue shell.

### Kernel branch state (2026-11-18)

The work is on the local branch `r8q-modem-msi` in
`~/omamobile-kernel/wt-r8q` (off `modem-stage2`, `4d5ae30a8`, which
carries the r8q `mhi_pci_generic` BHI quirk from the device package's
kernel config): the `qcom,pci-msi` binding and driver (doorbell, GIC
SPI hwirq as message data, debugfs fire/SETSPI bring-up knobs), the
MHI SBL/AMSS support (ported from the reference port, with the
channel-START skip removed because r8q's SBL answers it), and a
MAINTAINERS entry. `tools/kernel-pr-check` passes everything except
the human `Signed-off-by`, which the maintainer adds through
`tools/publish-kernel`'s flow; the two CHECKs it lists
(`MACRO_ARG_REUSE`, `PARENTHESIS_ALIGNMENT`) are inherited style in
the touched macro, not regressions. Build and gate were run with the
tree's own output directory (`out-r8q-msi`).


---

## 12. Normal Boot vs the SD Card (`2026-11-18`)

**What was run.** `recovery-modem.img` (sha256 `344eafe5…`, kernel
`7.3.0-rc6-omamobile-push1-g70a925aeee09`, `--dual-boot --overlay modem`,
normal initramfs + Arch root on the microSD image) was flashed to `sda24`
and booted. The loader and modules were staged on the root beforehand.

**What worked.**

- PCIe2 link up: `qcom-pcie 1c00000.pcie: PCIe Gen.2 x1 link up`.
- SDX55 enumerated: `0002:01:00.0 [17cb:0306] Qualcomm SDX55`.
- `mhi1: Requested to power ON`, `Power on setup success`,
  `Device already in SBL, queuing transition`.
- `qcom-pci-msi` is built in (`CONFIG_QCOM_PCI_MSI=y`); the PCI2 MSI works.

**What failed.**

- At the `mhi_pci_generic` bind (`43.7 s`), `mmc0: card 0001 removed` fired at
  `44.0 s`. The root became `rw,...,emergency_ro`; `blockdev`, `sha256sum` and
  `wwan.ko` returned EIO; `mhi_wwan_ctrl` could not load; no `/dev/wwan*`.
- The SD card came back after the stable image was restored (heimdall, image
  `66fe127f…`) and the root is read-write again; the journal aborted, so the
  root image is not pristine. A data backup was taken at
  `~/omamobile-kernel/reference-r8q/root-data-20261118.tar.zst`
  (sha256 `3762e761…`, 1.7 GiB).

**Why it is a board power issue, not the modem stack.** The stock `live.dts`
shows `sdhc_2: cd-gpios = <&tlmm 77>` (not the AP2MDM 56/57 pins), no
`vmmc-supply` on `sdhc_2`, and a `pcie2` node with `vreg-1p8`, `vreg-0p9`,
`vreg-cx` and `gdsc-vdd` but no 3.3 V supply. The mainline `pcie-qcom` logs
`supply vddpe-3v3 not found, using dummy regulator`. The modem's 3.3 V rail is
therefore always on and shared with the SD card; the modem's power-on current
browns the card out. The rescue image worked because it had no SD root.

**Run 2 on the native root (2026-11-18).** `recovery-modem-native.img` (kernel
`7.3.0-rc6-omamobile-push1-g70a925aeee09`, `--overlay modem`, no
`omamobile.loop=`, `UFS_RW=userdata`) booted with `/dev/sda36` as the root.
The SD card **still dropped** (`mmc0: card 0001 removed`) at modem power-on, and
the root stayed `rw` with no ext4/I/O errors: the blocker is gone.

The first attempt did not bring the modem up because the copied root still held
the rescue-test blacklist from §5 Step 2,
`/etc/modprobe.d/omamobile-r8q-sahara-bringup.conf` (`blacklist
mhi_pci_generic`). With it moved to `/root/bringup-backup/`, `modprobe
mhi_pci_generic` produced:

```
mhi-pci-generic 0002:01:00.0: MHI PCI device found: qcom-sdx55m
mhi mhi1: Requested to power ON
mhi mhi1: Power on setup success
mhi mhi1: Device already in SBL, queuing transition
wwan wwan0: port wwan0sahara0 attached
```

The SDX55's MSI vectors are live (`QCOM-PCI-MSI-PCI-MSI-0002:01:00.0` 1-4,
`mhi`), and `/dev/wwan0sahara0` is the firmware channel. USB and Wi-Fi both
survived the power-on. **Lesson:** a bring-up blacklist in `/etc/modprobe.d`
shadows the driver after the root moves; the plan's Step 3 removal is not
optional.

**Run 3 — AMSS reached on the userdata root (2026-11-18).** The same image
and root, with the one-channel loader (`--wait 200 --probe 3 --idle 90`)
started over Wi-Fi before the modem powered: the SBL's set was served in one
open channel (`acdb`, `qdsp6sw` 83 MB, `apps`, `apdp`, ...), the MHI entered
**`EE: MISSION MODE`**, and `wwan0qmi0`, `wwan0mbim0`, `wwan0at0` appeared.
That is the first mission-mode window on the userdata root.

About 1 s later the root port reported a **Completion Timeout**:

```
pcieport 0002:00:00.0: Recovering Root Port due to Link Down
AER: Multiple Uncorrectable (Non-Fatal) Error messages received
pcieport 0002:00:00.0: PCIe Bus Error: severity=Uncorrectable (Non-Fatal)
  device [17cb:010b] error status/mask=00004000/00400000
  [14] CmpltTO | Requester | Transaction Layer (First)
Root Port has been reset ... AER: device recovery successful
```

No SMMU context fault. The host's first AMSS transaction timed out, the link
dropped and the endpoint reset back to SBL. The mission-mode channels were
real but short-lived. The next questions: the shared 3.3 V rail under the
AMSS load (the same rail that browns the SD), or the AMSS init timing (the
host's transaction before the endpoint answers). The one-channel loader's
`unexpected cmd 9` after the `mode=2` HELLO is the final SBL command; it does
not affect the transition.

**Run 3 follow-up — the link dies after mission mode (2026-11-18).** The
first mission-mode window was followed by a link that never settled:

- `CmpltTO | Requester | Transaction Layer` (Uncorrectable Non-Fatal) with
  `Link Down`, root-port reset, AER recovery;
- then `RxErr | Receiver | Physical Layer` (Correctable) and more link-downs,
  plus a fatal "no details found" from the ghost `0002:ff:1f.7`;
- at 679 s: `Failed to reset Root Port: -5`, `broken device, retraining
  non-functional downstream link at 2.5GT/s`, `Data Link Layer Link Active not
  set in 100 msec`, `subordinate device reset failed`; `/dev/wwan*` gone.

The link trains at **2.5 GT/s x1** (capability 8 GT/s x2). **ASPM is disabled**
on the root port, and the L1 substates are off, so ASPM is not the trigger.
The modem's `1c00000.pcie-vddpe-3v3` and `-vdda` are **dummy regulators**
(0 mA, 0 mV): the 3.3 V rail is not modelled or monitored. The errors are
physical-layer (RxErr) plus a completion timeout, so the leading hypotheses
are the shared 3.3 V rail under the AMSS load and the PCIe PHY/link training,
with the `0002:ff:1f.7` fatal as a possible MSI/doorbell side effect.

A reboot is needed to power the endpoint again (the PCIe link reset no longer
recovers). The next run should capture the regulator/clk state and the AER
counters from the first second of mission mode.

**Research result (2026-11-18, independent review).** Two corrections and two
mechanisms:

- **Which controller is the modem:** `1c10000.pcie` (`linux,pci-domain = <2>`,
  the modem) trains **Gen3 x2** and carried the 83 MB SBL set at that rate;
  `1c00000.pcie` is the QCA6390 Wi-Fi and is the source of the "Gen.2 x1" line.
  The link degraded to 2.5 GT/s x1 only after the first CmpltTO.
- **MSI is not the cause:** a rejected MSI is a posted write and cannot produce
  `CmpltTO | Requester`; an unmapped endpoint write would raise an SMMU context
  fault, and none was logged. The `0002:ff:1f.7` fatal is AER decoding a
  garbage TLP header on the retraining link.
- **PHY is secondary:** 83 MB moved error-free at Gen3 x2 before the CmpltTO;
  RxErr appears only after it. **3.3 V** remains plausible (dummy
  `vddpe-3v3`/`vdda`, shared rail) but a brown-out should bite during the SBL
  transfer, not exactly at the AMSS jump.
- **Best fit: the modem resets itself.** The one-channel log ends with a
  `mode=2` HELLO and `unexpected cmd 9`; cmd 9 is **MEMORY_DEBUG**, the modem's
  EFS-dump offer. The reference port documents that without a response the
  modem times out (~38 s) and crashes via a **PS_HOLD warm reset**, which
  drops PCIe. Second: the MHI entry has no `.no_m3`, so the first M3 runtime
  suspend arms 2 s after the mission ports idle; the reference port sets
  `.no_m3 = true` for the same reason.

**Next experiment (ranked):** (1) `.no_m3 = true` in
`mhi_samsung_r8q_sdx55_info`; (2) the loader declines the `mode=2` HELLO
(`HELLO_RESP mode=0`) instead of accepting it and stopping on cmd 9; (3)
capture AER counters, `LnkSta` and MHI PM state from mission-mode entry; (4)
only if those fail, cap the link (`max-link-speed = <2>`) or model the 3.3 V
rail. The stock `live.dts` shows the modem PHY supplies as L5A 0.88 V and L9A
1.2 V and votes CX to max while pcie2 runs; the mainline overlay has the first
two and no CX vote.

### Run 4 — `.no_m3` + mode-2 decline (2026-11-18)

The two fixes from the research: the kernel sets `.no_m3 = true` on the r8q
MHI entry (`r8q-modem-nom3`, commit `05ae9a461`), and the loader replies
`HELLO_RESP mode=0` to the mode-2 MEMORY_DEBUG offer instead of accepting it.

0. **Read-only preflight.** The endpoint from run 3 is failed; a reboot is
   needed. `tools/nv-backup -d r8q --check <backup>` must be 15/15.
1. **Modules (write):** install `out-r8q-nom3/modules-<release>.tar.zst`, then
   `depmod -a <release>`.
2. **Image (laptop):** pack `--overlay modem` from `out-r8q-nom3` to
   `recovery-modem-nom3.img`; it must report `userdata`/`omamobile-root` and no
   `omamobile.loop=`.
3. **Flash (write):** write it to `sda24`; readback sha256 must match.
4. **Reboot (needs go):** as soon as SSH answers, start the loader before the
   modem powers: `r8q-sahara --wait 200 --probe 3 --idle 90`. The log should
   show `declined memory-debug` at the end of the set.
5. **Capture from mission-mode entry:** loader log; `dmesg` AER (no `CmpltTO`,
   `RxErr` or `Link Down`); `lspci -vv -s 0002:01:00.0` `LnkSta`;
   `cat /sys/bus/pci/devices/0002:01:00.0/power/runtime_status` (`active`, no
   M3); MHI `EE: MISSION MODE`; `/dev/wwan*`.

**Pass:** the mission window survives past 60 s with clean AER and
`runtime_status=active`. **Stop:** any `CmpltTO`, `RxErr`, link-down or reset;
capture the first second's AER/LnkSta/PM state and stop. Rollback: the stable
native image (`7799b1e2…`).

**Run 4 result (Gen2, 2026-11-18).** The DTB cap took effect
(`qcom-pcie 1c10000.pcie: PCIe Gen.2 x2 link up`), but the AMSS jump still
trips `RxErr` (Physical Layer) then `CmpltTO` (Transaction Layer), the link
resets, and the loader's loop repeats (apdp → mode-2 declined →
`multi_image_qti` → channel EIO). The mission mode is reached, but the link
does not hold. The pstore is empty and the restarts seen were clean Shutdown
targets (no stale-CPU liveness reboot). Next: **Gen1**
(`max-link-speed = <1>`, 2.5 GT/s), then the PHY/3.3 V suspects.

**Run 4 result (Gen1, 2026-11-18).** `LnkSta: Speed 2.5GT/s (downgraded), Width x2`
— the cap works, but the AMSS jump still reports `RxErr`, `Rollover`, `Timeout`
and `CmpltTO`, then link down. **The phone restarts because the AER recovery
hangs**: the last kernel event is `pci_reset_bus_function+0x1d0`, the pstore is
empty, and the hardware watchdog (`qcom_wdt`, 30 s) resets it; there is no
stale-CPU liveness line. Next: `pci=noaer` to remove the AER reset path and
see whether the phone survives the AMSS jump (Gen1 kept, `.no_m3` kept).

**Run 4 result (noaer, 2026-11-18).** `pci=noaer` did not change the
outcome. The pstore's last line is
`pcieport 0002:00:00.0: unlocked secondary bus reset via:
pci_reset_bus_function+0x1d0/0x1e8`, and the phone reboots via `qcom_wdt`
after the hang. The reset is **not AER**: the controller's link-down
interrupt calls `pci_host_handle_link_down()` →
`pcie_do_recovery(..., pci_host_reset_root_port)` →
`pci_bus_error_reset()` → `pci_reset_bus_function()`, which invokes
qcom's `reset_root_port` callback (`qcom_pcie_reset_root_port`: flush
wait, `qcom_pcie_host_deinit`, `qcom_pcie_host_init`, wait for link).
`pci=noaer` only disables AER error reporting; this path is compiled in
and runs regardless. So noaer is a dead end and is dropped from the next
image. The boot also shows the link re-trains at Gen1 x2
(`PCIe Gen.1 x2 link up`) both at boot and after the first reset.

Next: the CX vote. Stock has `vreg-cx-supply` +
`qcom,vreg-cx-voltage-level = <0xffff 0x100 0x00>` on all three PCIe
nodes; mainline `pcie2` has only `PCIE_2_GDSC`. The mainline idiom is
`required-opps` on the device node (as `sc8280xp` does): genpd parses it
(`genpd_set_required_opp`), and the GDSC domain's performance state
propagates to its parent RPMHPD CX. Test with
`required-opps = <&rpmhpd_opp_turbo>` on `pcie2`, DT-only.

**Run 5 preparation (CX vote, 2026-11-18).** Stock votes VDD_CX to max on
every PCIe node (`vreg-cx-supply` + `qcom,vreg-cx-voltage-level =
<0xffff 0x100 0x0>`); mainline `pcie2` has only `PCIE_2_GDSC`. The
mainline equivalent is the RPMHPD CX domain plus `required-opps`, as
sc8280xp does, but `dev_pm_domain_attach()` attaches only the first power
domain and `pcie-qcom` does not call `devm_pm_domain_attach_list()`, so CX
cannot be attached through `pcie2`. It is attached through GCC instead
(the gcc driver does attach all its domains), which makes every GDSC a CX
subdomain and lets `pcie2`'s GDSC performance state propagate up to CX.
The overlay therefore adds `power-domains = <&rpmhpd 0>` (RPMHPD_CX) to
`clock-controller@100000` and `required-opps = <&rpmhpd_opp_turbo_l1>` to
`pcie2`. Image `recovery-modem-cx.img` (`f2dfafa7…`), Gen1, no `noaer`.
Verified in the packed DTB: required-opps resolves to `opp10`
(`opp-level = <0x1a0>`) and the gcc domain to `rpmhpd` cell 0.

**Run 5 result (CX vote, 2026-11-18).** The vote applies on every boot
(`pm_genpd_summary`: `cx on 416`), and it **removed the AER-recovery
hang**: a boot now runs through several complete recoveries
(`Root Port has been reset` → `PCI slot reset done` →
`AER: device recovery successful` → `PCIe Gen.1 x2 link up`) where every
earlier boot hung in the first `pci_reset_bus_function` and was reset by
`qcom_wdt` at ~60 s.

**Correction (same day):** the `EE: MISSION MODE` read during these runs
was `mhi1`, the **Wi-Fi** (QCA6390 on `0000:01:00.0`, mission mode when
ath11k is up), not the modem. The modem is `mhi0` (SDX55 on
`0002:01:00.0`); today it only cycles **SBL → failed AMSS → SBL**, and
no `wwan0qmi0`/`wwan0mbim0`/`wwan0at0` appeared. Whether the modem
itself reached mission mode in any of today's boots is unverified. With
the autotest disabled, the modem sits in `EE: SECONDARY BOOTLOADER`,
reachable on `/dev/wwan0sahara0`, and the phone stays up.

The autotest's endless retry still ends each boot (~2-3.5 min): the fatal
cycle is `mhi_wwan_ctrl mhi0_SAHARA: Failed to queue buffer` (x16) →
`wwan0sahara0 disconnected` → `mhi-pci-generic: failed to power up MHI
controller` → `PCI error detected, state = 2` → `device recovery started`
→ then a silent wedge (one pstore shows `[dpu error] enc35 frame done
timeout` as the last line) and a watchdog reset with no panic recorded.
During the cycles the modem's internal bridges report
`AER: Multiple Correctable + Uncorrectable (Fatal) Error messages`
(from `0002:ff:1f.7`).

**Correction of an earlier reading:** the `Reached target Shutdown`
lines in the journal are the **user manager** (`systemd --user`, the
`Removed session` teardown after SSH logouts), not system shutdowns.
The boots die silently mid-recovery; they are not clean shutdowns.

The autotest is `enabled`, so every boot restarts the loop and the phone
power-cycles on its own; it is masked on the phone to hold a stable
state (`systemctl mask r8q-modem-autotest.service`, unmask to retest).

**Post-run 5 findings (2026-11-18).**

- **The MHI controller numbering is probe-order dependent and swaps
  between boots**: in earlier runs the modem was `mhi1` (see the quotes
  above), on 2026-11-18 the modem was `mhi0` (SDX55 at `0002:01:00.0`)
  and `mhi1` was the **Wi-Fi** (QCA6390 at `0000:01:00.0`, mission mode
  whenever ath11k is up). Never identify the modem's controller by
  name: resolve it from the PCI path
  (`/sys/devices/platform/soc@0/1c10000.pcie/pci0002:00/0002:01:00.0/mhi*`).
  The phone-side autotest script hardcodes `mhi1`, so its mission-mode
  gate has been reading the Wi-Fi all along; fix it before the next run.
- **Stability with the autotest disabled:** 8 samples over 6 minutes,
  phone alive throughout (uptime 133 s to 452 s, 43-44.8 °C). The reboot
  churn was entirely the autotest's endless retries.
- **A mid-serve stop wedges the modem.** Stopping the loader while the
  modem held a partial image set left it to make its own attempt
  (~216 s): the link dropped, the AER recovery succeeded, and the modem
  ended `EE: DISABLE / MHI state: RESET / Device: Inactive` with no
  sahara port. It does not wait in SBL for a loader that vanished.
- **`soc_reset` does not recover that state.**
  `echo 1 > /sys/bus/mhi/devices/mhi0/soc_reset` produced no kernel
  event, no port, and the state stayed DISABLE. A cold PMIC power cycle
  (full power-off, not a warm reboot) is the only way back to a fresh
  SBL from there.

**Run 6 (one-shot from a cold boot, 2026-11-18).** With the autotest
disabled and the phone cold-power-cycled, a single `r8q-sahara` serve
(apdp → acdb → **qdsp6sw 83 MB** → apps, `HELLO mode=1` after the big
image) **completed the AMSS jump cleanly**: no AER storm, no link-down
recovery, `mhi mhi0: Mission mode doorbell offsets`, and the
mission-mode ports attached (`wwan0qcdm0/mbim0/qmi0/at0`). The loader
exited when the sahara channel dropped (`could not reopen`) — no churn.
The earlier RxErr/CmpltTO storms were the autotest's reopen loop
re-serving a transitioning modem, not the jump itself.

The modem then left mission mode within ~60 s and now cycles on its
own: it retains the images in RAM, re-attempts AMSS, drops the link
each time, and the AP recovers every cycle (the CX vote) back to
`EE: SECONDARY BOOTLOADER`. The phone stays up through the cycles.
This is the flashless-modem pattern: AMSS boots from RAM, asks the host
for its remote services over QRTR, gets no answer (no
rmtfs/pd-mapper/tqftpserv installed) and reboots. **The remaining
blocker is the userspace, not the link.**

**Run 7 (userspace installed, 2026-11-18).** `qrtr`/`rmtfs`/`pd-mapper`/
`tqftpserv` built and installed on the phone. From a cold boot with
`tqftpserv` up and one clean serve, the modem reached mission mode and
**`qrtr-lookup` showed its QRTR services for the first time**: node 3
(the modem) advertising the *Modem embedded file system service* (the
EFS counterpart of `rmtfs`), thermal and time services; node 1 carrying
tqftpserv's TFTP service.

It still dropped back to SBL within a minute, and the cause is now
precise: **`rmtfs` has no shared-memory window**. It starts, fails to
open `/dev/qcom_rmtfs_mem1` (the `qcom,rmtfs-mem` region is not in the
DT and `CONFIG_QCOM_RMTFS_MEM` is not set), and its `/dev/mem` fallback
cannot mmap. The EFS handshake can never complete, so the flashless
modem reboots. `pd-mapper` also exited to its start limit (`no pd maps
available`, no remoteproc bound) — the known caveat; QMI is tried
without it first.

The 6-minute hold watch after the serve: the modem cycled SBL for
~2 minutes (30-90 s samples), then wedged into `EE: DISABLE` and stayed
there for the rest of the watch, with rmtfs never serving a request (its
log never grew past the startup errors).

Next: add `CONFIG_QCOM_RMTFS_MEM=y` and a dynamically allocated
`qcom,rmtfs-mem` reserved-memory region (the address is negotiated with
the modem over QMI, so `alloc-ranges` from free System RAM
(`0xc0000000`, per `/proc/iomem`) instead of a stock hole; stock's map
has none free).

### Run 8 — qcom,rmtfs-mem + rmtfs -r hold test (2026-11-18)

Run 7's precise blocker: the userspace is installed and the modem asks for
its file system service, but `rmtfs` has no shared-memory window to answer
with. Run 8 boots the first kernel that has one: `CONFIG_QCOM_RMTFS_MEM`
moves from `=m` to `=y` and `modem.dtso` reserves a dynamically allocated
2 MiB `qcom,rmtfs-mem` region from free System RAM (commit `467d26e3`). A
correction to run 7's record: the installed kernel was built `=m`, not "not
set" — the prior `g05ae9a4610fe` tarball carried an inert `rmtfs_mem.ko`.
With no region in the DT no device ever existed, so the observation stands
and only the config reading was wrong.

**The one honest caveat, so the result is read right.** The window is raw
physical RAM and `rmtfs` hands that *physical* address to the modem over
QMI. The modem then DMAs to it through `pcie2`, whose `iommu-map` routes the
endpoint to `apps_smmu` SID `0x1d01` (sm8250.dtsi), and this build has
`CONFIG_IOMMU_DEFAULT_DMA_STRICT=y`. That strict DMA domain maps only
DMA-API allocations — the MHI rings work because the host programs IOVAs —
and nothing in mainline maps the rmtfs window into it (`rmtfs_mem.c` does no
DMA mapping, and no in-tree sm8250/sc8280xp board pairs `qcom,rmtfs-mem`
with an IOMMU'd PCIe modem; the in-tree users are integrated-modem boards
with `qcom,vmid`). Stock never had this gap because its MHI DMA domain is a
fixed `0x20000000-0x3fffffff` pool (§9.1), a downstream mechanism. So the
modem's first EFS access to the window is *expected* to raise an `arm-smmu`
context fault naming `SID=0x1d01` at the region's `phys_addr`. That is a
recorded outcome of this run, not a surprise to hide: the dmesg check in
Step 8 exists to catch it, and the fix it points to (an identity/bypass
window for the endpoint, the stock pool's equivalent) is a one-change
follow-up, never a mid-run improvisation.

**Artifacts (laptop-only, rebuilt with `DEVICE=r8q`, packed and verified 2026-11-18):**

- `modules-7.3.0-rc6-omamobile-push1-g05ae9a4610fe.tar.zst` — sha256
  `6f5434e68a72f15cc521b34f7cf46dacb3c5ad9040fc5d6a110d8f97f6ac7647`
  (16,888,783 bytes). The kernel tree is unchanged (HEAD `05ae9a461`), so
  the release string is the same as runs 4-7 and the tarball keeps the
  same filename, without `rmtfs_mem.ko` (built in now via
  `CONFIG_QCOM_RMTFS_MEM=y`, alongside `CONFIG_QCOM_PCI_MSI=y` and
  `CONFIG_TOUCHSCREEN_STMFTS=y`).
- `recovery-modem-rmtfs.img` — sha256
  `a3c8b811a0eb02a1b260b29049fe63ce46c8d9e411dac4053fd524cfe2aa2185`
  (82,694,144 bytes, AVB-signed), packed from `out-r8q-nom3` with
  `tools/r8q-boot pack --overlay modem` and **without** `--dual-boot`:
  `userdata` in the packed `/etc/omamobile-ufs-rw`, the `omamobile-root`
  label gate, no `omamobile.loop=` on the cmdline.
- **`uname -r` cannot prove this boot.** The release string is identical to
  the image on `sda24` today, so NOTES §2's verify-with-uname rule is blind
  here. Proof is the `sda24` readback sha256
  (`a3c8b811a0eb02a1b260b29049fe63ce46c8d9e411dac4053fd524cfe2aa2185`) plus
  `/dev/qcom_rmtfs_mem1` existing after the flash.

Offline checks done 2026-11-18: `modem.dtso` compiles clean with the tree's
`dtc -@`, `fdtoverlay` merges cleanly onto `sm8250-samsung-r8q.dtb`, and
the packed image's embedded DTB contains `rmtfs-region` matching
`sm8250.dtsi`'s reserved-memory cells (`#address-cells = <2>`,
`#size-cells = <2>`; `alloc-ranges = <0x0 0xc0000000 0x0 0x40000000>`,
`size = <0x0 0x200000>`, `no-map`, `qcom,client-id = <1>`).

#### Step 0 — read-only preflight (no approval needed)

The phone must be up in the Arch root; if it is not after run 7's watch,
bring it there first.

```sh
cd ~/Work/omarchy-mobile-s20fe
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh 'r8q-preflight --json'
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/nv-backup -d r8q --check \
  ~/omamobile-kernel/reference-r8q/nv-backup-20261117-full
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh \
  'sha256sum /dev/sda24; systemctl is-enabled r8q-modem-autotest.service 2>&1 || true'
grep -E 'CONFIG_QCOM_RMTFS_MEM=|CONFIG_IOMMU_DEFAULT_DMA_STRICT=' \
  ~/omamobile-kernel/out-r8q-nom3/.config
python3 -m py_compile tools/r8q-sahara
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh 'python3 - --dry-run' < tools/r8q-sahara
```

- **Expect / Pass:** guard `77 UFS partitions read-only`; `15/15 SAME`; the
  current `sda24` sha printed for the record; the autotest reads `masked`
  (its mission-mode gate hardcodes `mhi1`, the Wi-Fi, so it stays masked);
  the finished build's `.config` shows `CONFIG_QCOM_RMTFS_MEM=y` and
  `CONFIG_IOMMU_DEFAULT_DMA_STRICT=y`; `all r8q image sources present and
  readable`.
- **Stop condition:** any NV `DIFFERENT`, guard not ok, missing image
  sources, or the autotest not masked.
- **Rollback:** none. **Risk:** none (read-only).

#### Step 1 — install the rebuilt modules (WRITE — needs go)

`CONFIG_QCOM_RMTFS_MEM` moved from `=m` to `=y`, so the modules tree changes
with the release string held fixed.

```sh
DEVICE=r8q KERNEL_DIR=~/omamobile-kernel/wt-r8q KERNEL_OUT=~/omamobile-kernel/out-r8q-nom3 \
  OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./bin/omamobile modules-push
```

- **Expect / Pass:** installs
  `/usr/lib/modules/7.3.0-rc6-omamobile-push1-g05ae9a4610fe` — the same
  name as the installed tree, new content without `rmtfs_mem.ko` — and runs
  `depmod`. The "phone runs ..." note will not appear (same release); that
  is expected, and the manifest check is built for exactly this ("a rebuilt
  module under the same release name is fine").
- **Stop condition:** `modules-push` refuses (a module changed by hand
  since the last push). Stop and ask; never set `OMAMOBILE_MODULES_REPLACE=1`
  without a separate decision.
- **Rollback:** the old tree is replaced wholesale and the previous tarball
  is gone under its filename; the way back is the stable image's own release
  tree (`g3667ab368ef9`, already installed on the root). **Risk:** low
  (rootfs write).

#### Step 2 — prepare `/var/lib/rmtfs` (WRITE — needs go; touches no partition)

No package creates this directory or its symlinks (the packaged helper is a
follow-up once the layout is proven), so it is prepared by hand, resolving
each partition by `PARTNAME` in `/sys/class/block/*/uevent`. Never mount,
write or resize these partitions: `rmtfs -r` only ever reads them, and every
modem write stays in its RAM shadow.

```sh
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh 'set -e
  mkdir -p /var/lib/rmtfs
  for n in modemst1 modemst2 fsc fsg; do
    u=$(grep -l "^PARTNAME=$n$" /sys/class/block/*/uevent | head -1)
    [ -n "$u" ] || { echo "$n: PARTNAME not found"; exit 1; }
    d=/dev/$(basename "$(dirname "$u")")
    [ "$(cat /sys/class/block/${d#/dev/}/ro)" = 1 ] || { echo "$n: $d is not ro=1"; exit 1; }
    ln -sfn "$d" /var/lib/rmtfs/"$n"
    echo "$n -> $d (ro=1)"
  done
  ls -l /var/lib/rmtfs'
```

- **Expect / Pass:** four symlinks, each target `ro=1`. Upstream's storage
  table maps `modem_fs1→modemst1`, `modem_fs2→modemst2`, `modem_fsc→fsc`,
  `modem_fsg→fsg`; `mdm1m9kefs1/2/3` and `mdmddr` are **not** in the table,
  so a request for them is logged and rejected ("request for unknown
  partition '…', rejecting") — that log is the discovery step, not a
  failure to hide.
- **The `-o` prefix is unverified.** rmtfs v1.3's `-o`-over-`-P` precedence
  could not be checked offline (the source is not cached on the laptop),
  and run 7 never got past the rmtfs-mem open to exercise it. The foreground
  `-v` run prints every path it opens: if it looks under
  `/dev/disk/by-partlabel/` instead of `/var/lib/rmtfs`, record it and stop
  — the unit or layout follows from that log.
- **Stop condition:** a `PARTNAME` not found, or any target not `ro=1`.
- **Rollback:** `rm -rf /var/lib/rmtfs` (it holds only symlinks).
  **Risk:** low.

#### Step 3 — pack and verify the image (DONE offline 2026-11-18; laptop only)

```sh
KERNEL_OUT=~/omamobile-kernel/out-r8q-nom3 tools/r8q-boot pack --overlay modem \
  ~/omamobile-kernel/out-r8q-nom3/recovery-modem-rmtfs.img
sha256sum ~/omamobile-kernel/out-r8q-nom3/recovery-modem-rmtfs.img
tmpd=$(mktemp -d)
~/omamobile-kernel/out-r8q-nom3/scripts/dtc/dtc -q -@ -I dts -O dtb \
  -o "$tmpd/modem-run8.dtbo" device/r8q/overlays/modem.dtso
~/omamobile-kernel/out-r8q-nom3/scripts/dtc/fdtoverlay \
  -i ~/omamobile-kernel/out-r8q-nom3/arch/arm64/boot/dts/qcom/sm8250-samsung-r8q.dtb \
  -o "$tmpd/r8q-run8.dtb" "$tmpd/modem-run8.dtbo"
~/omamobile-kernel/out-r8q-nom3/scripts/dtc/dtc -q -I dtb -O dts "$tmpd/r8q-run8.dtb" \
  | grep -A9 'rmtfs-region'
rm -rf "$tmpd"
```

- **Expect / Pass:** the pack stages the native root (`userdata`,
  `omamobile-root`, no `omamobile.loop=`) and signs with the AVB footer at
  82,694,144 bytes; the decompiled overlaid DTB shows the `rmtfs-region`
  node with `compatible = "qcom,rmtfs-mem"`, `size = <0x0 0x200000>`,
  `no-map`, `qcom,client-id = <1>`, `alloc-ranges = <0x0 0xc0000000 0x0 0x40000000>`.
  Verified 2026-11-18 (`a3c8b811…`, embedded DTB and ramdisk checked).
- **Stop condition:** pack or `fdtoverlay` errors; the node missing or its
  values wrong (a wrong cells count silently lands the region anywhere —
  the gs201 lesson, checked here against `sm8250.dtsi`'s 2/2 cells).
- **Rollback:** none (nothing written to the phone). **Risk:** none.

#### Step 4 — flash via Download Mode (WRITE — needs go)

Download Mode, not the SSH flasher: the flash takes a full power-off, and
only a cold PMIC cycle returns the modem from run 7's `EE: DISABLE` wedge
(a warm reboot does not, and `soc_reset` does not either). Writes `sda24`
only; `boot`/`sda23` (LineageOS) untouched.

1. Power the phone fully off; enter Download Mode (hold Volume Up + Volume
   Down, plug in USB; or `adb reboot download` if it is in LineageOS).
2. `heimdall detect`, then
   `heimdall flash --RECOVERY ~/omamobile-kernel/out-r8q-nom3/recovery-modem-rmtfs.img --no-reboot`.

- **Expect / Pass:** `Device detected`; `100%` transfer; a clean session
  end.
- **Stop condition:** any Heimdall protocol or write error — do not retry
  blind. A Heimdall flash once left a stale image behind (NOTES §2), so the
  readback in Step 5 is mandatory.
- **Rollback:** `tools/r8q-flash-recovery
  ~/omamobile-kernel/out-r8q-uw/recovery-stable-native.img` (sha256
  `7799b1e2…`, re-verified 2026-11-18 — check it again before flashing).
  **Risk:** medium (recovery slot; single-slot device, the human ready to
  recover).

#### Step 5 — cold boot and first verification (needs go; affects the screen)

Boot with Volume Up + Power (Gate A's combo); wait for the USB gadget at
`172.16.42.1`.

- **Expect / Pass:** SSH answers; `sha256sum /dev/sda24` equals the recorded
  image sha; `/dev/qcom_rmtfs_mem1` exists; `cat
  /sys/class/rmtfs/qcom_rmtfs_mem1/{phys_addr,size,client_id}` reads
  `client_id 1`, size `0x200000`, and a `phys_addr` inside
  `0xc0000000..0x100000000`; `r8q-preflight --json` guard ok; `/` mounted
  `rw`. That is the proof the new image booted (`uname -r` cannot show it).
- **Seen once before (NOTES §22):** the first boot after a Heimdall flash
  stopped before the USB gadget; one forced restart (Power + Volume Down)
  booted normally. Try that once before calling the image bad, and capture
  the initramfs console if it repeats.
- **Stop condition:** no enumeration after the restart, guard not ok, or no
  `qcom_rmtfs_mem1` device.
- **Rollback:** Step 4 rollback, then boot. **Risk:** medium.

#### Step 6 — userspace up (attended; the read-only test order)

Stream the log first, then start the daemons; `rmtfs` runs in the
foreground with its log on the laptop.

```sh
mkdir -p logs/r8q
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh 'dmesg -w' \
  > logs/r8q/run8-dmesg.log 2>&1 &
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh 'systemctl start tqftpserv pd-mapper'
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh \
  'rmtfs -r -v -P -o /var/lib/rmtfs' 2>&1 | tee logs/r8q/run8-rmtfs.log
```

- **Expect / Pass:** `tqftpserv` up; `pd-mapper` may exit to its start
  limit with `no pd maps available` (no modem remoteproc is bound — the
  known caveat: record it and try QMI without it); `rmtfs` stays up, maps
  `/dev/qcom_rmtfs_mem1` and waits for the modem's EFS service. Its log
  shows the paths it opens (the Step 2 `-o` check) and, once the modem asks,
  every requested partition path.
- **Stop condition:** `rmtfs` exits at startup; any read-write open of a
  partition in its log — stop at once; any oops or UFS error.
- **Rollback:** stop the three services; nothing is persisted (the RAM
  shadow). **Risk:** low.

#### Step 7 — one clean serve (attended; the loader writes only to the modem port)

```sh
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh \
  'python3 - --wait 200 --probe 3 --idle 90' < tools/r8q-sahara 2>&1 | tee logs/r8q/run8-sahara.log
```

- **Expect / Pass:** `HELLO`, the 14-image set over one open channel (`acdb`,
  `qdsp6sw` 83 MB, `apps`, `apdp`, ...), `declined memory-debug` on the
  mode-2 offer, then `EE: MISSION MODE` with the mission ports attached;
  the loader exits when the channel drops (no reopen loop — the autotest
  stays masked).
- **Stop condition:** no HELLO within the wait (the modem may still be
  wedged `DISABLE` if the power cycle did not happen — cold-boot again, do
  not re-serve a wedged modem); a cross-platform image ID (28/30/31/37); a
  `CmpltTO`/`RxErr`/link-down storm.
- **Rollback:** none beyond stopping; the modem may fail to finish
  booting. **Risk:** low.

#### Step 8 — mission-mode checks (read-only)

Resolve the modem's controller from the PCI path, never by name:
`mhi0`/`mhi1` swap between boots (the post-run-5 finding; the autotest's
hardcoded `mhi1` reads the Wi-Fi).

```sh
OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh '
  m=$(ls -d /sys/devices/platform/soc@0/1c10000.pcie/pci0002:00/0002:01:00.0/mhi* | head -1)
  echo "modem controller: $m"
  cat /sys/kernel/debug/mhi/$(basename "$m")/states
  ls /dev/wwan0*
  qrtr-lookup
  dmesg | grep -iE "arm-smmu|context fault"
  dmesg | grep -iE "CmpltTO|RxErr|Link Down"'
```

- **Expect / Pass:** the PCI-resolved controller in mission mode; the
  mission ports; run 7's QRTR services (node 3's *Modem embedded file
  system service*, thermal and time; node 1's TFTP). The SMMU grep coming
  back clean is the full pass. An `Unhandled context fault ...
  iova=<the Step 5 phys_addr>` with `SID=0x1d01` is the recorded expected
  failure mode — capture the whole two-line FSR report (fsr/iova/fsynr/
  cbfrsynra and the SID) and let the run continue to the hold watch; the
  modem will cycle, and that is the measurement.
- **Stop condition:** none of its own; the hold watch decides.
- **Rollback:** none. **Risk:** none (read-only).

#### Step 9 — six-minute mission-mode hold watch (attended)

Sample every ~30 s for 6 minutes (run 7's watch used 30-90 s samples over
the same span): the debugfs `states` of
the PCI-resolved controller, `ls /dev/wwan0*`, `qrtr-lookup`'s service list,
the rmtfs log's growth — every requested partition path,
`mdm1m9kefs*` included — and the dmesg tail for SMMU and link lines.

- **Pass (full):** the modem holds mission mode for the full 6 minutes,
  `rmtfs` answers every EFS request from the RAM shadow, no SMMU fault, no
  link drop. The userspace wins; SIM/QMI (the
  [MODEM-USERSPACE](MODEM-USERSPACE.md) order) is next.
- **Pass (partial, recorded):** the modem still cycles. With the SMMU fault
  at the window, the blocker is the strict DMA domain and the next change is
  one per boot: an identity/bypass window for the endpoint (the §9.1 stock
  pool's equivalent) in the overlay. With unknown-name rejections in the
  rmtfs log, the storage table or the `-o` layout grows next. Either way,
  the requested-path list is the take.
- **Stop condition:** any read-write open of a partition, oops, UFS error
  handler, the phone leaving the bus, or a watchdog reset — capture what
  exists and stop. After any reset, re-run `tools/nv-backup -d r8q --check`
  (15/15) before anything else.
- **Rollback:** `tools/r8q-flash-recovery
  ~/omamobile-kernel/out-r8q-uw/recovery-stable-native.img` — the daily
  native image, sha256 `7799b1e2…` (verify before flashing; its release's
  modules are already installed on the root).
  `tools/r8q-flash-recovery --stable` restores the older SD-loop image
  (`66fe127f…`, `--dual-boot`): that is the way back for a broken
  `userdata` root, not this run's rollback. **Risk:** low throughout:
  read-only services, RAM-shadow writes, no NV partition ever opened for
  writing.

Record the results in this section with the date, the filled-in sha256s
and the release, quoting only the relevant log lines.

**Run 8 results (2026-11-19, `7.3.0-rc6-omamobile-push1-g05ae9a4610fe`):**

- `r8q-sahara` served all 14 Sahara images cleanly (`END_OF_IMAGE image=40 status=0`)
  and `mhi0` transitioned to `EE: MISSION MODE` (`Mission mode doorbell offsets:
  CHDBOFF=0x300 ERDBOFF=0x700` at `802.372s`; `wwan0qmi0` attached at `802.376s`).
- `Node 3` advertised 12 early QMI services (`15, 17, 21 [EFS], 22 [Time], 23,
  24, 34, 36, 49, 66, 227, 232`; no `WDS/DMS/NAS/UIM` yet), and zero
  `arm-smmu` faults occurred, but `rmtfs -r -v -P -o /var/lib/rmtfs`
  (`logs/r8q/run8-rmtfs.log`) received zero packets from `Node 3`.
- At `818.832s` (+16.46 s after mission mode), `pcieport 0002:00:00.0: Recovering
  Root Port due to Link Down` fired and the modem returned to SBL (`HELLO
  mode=2`, declined memory-debug).
- Deep research identified 4 offline kernel/DT/config causes addressed before
  Run 9:
  1. `drivers/bus/mhi/host/pm.c` (`mhi_pm_mission_mode_transition()`): called
     `__mhi_device_get_sync()` before re-reading `CHDBOFF`/`ERDBOFF` and never
     updated `mhi_cntrl->wake_db` from SBL's offset (`0x3f8`, which landed on
     Channel 31 doorbell instead of `0x6f8`). Fixed by reading/bounds-checking
     `CHDBOFF`/`ERDBOFF` and updating `wake_db` (clearing `wake_set` if moved)
     plus ring doorbell pointers *before* `__mhi_device_get_sync()`.
  2. `drivers/bus/mhi/host/pci_generic.c` (`mhi_samsung_r8q_sdx55_channels`):
     matched stock `live.dts` (`MHI_EVENT_CONFIG_SW_DATA(2, 64)`) for `IPCR`
     channels 20/21 (`64` elements on event ring `2` instead of `8` on ring `0`).
  3. `device/r8q/kernel.config`: added `# CONFIG_QCOM_PD_MAPPER is not set` so
     in-kernel `qcom_pd_mapper` no longer hijacks QRTR `Service 64` without a
     `mdm/modem/root_pd` (`165`) domain.
  4. `device/r8q/overlays/modem.dtso`: added stock `ap2mdm_errfatal2_default`
     (`pm8150l_gpios` `gpio7`: `output-low; input-disable; bias-disable`) to
     `pcie2` `pinctrl-0`.

### Run 9 — Gated Runbook (`r8q-modem-nom3`, MHI `wake_db` + `IPCR` ring 2/64 + no `qcom_pd_mapper` + `ap2mdm_errfatal2`)

**Goal:** Boot `7.3.0-rc6-omamobile-push1-g561090d2799a` with
`recovery-modem-nom3.img` (`4e6d8cdfcbf9da6138b252c13613cfc11a9fb7e528c9b1e0d6257f6e39ff47ff`),
verify `mhi0` stays in `EE: MISSION MODE` with live `rmtfs` / `tftp` traffic or
captures exact in-window diagnostics (`mhi0` debugfs + `qrtr-lookup` +
`dmesg`) if `apps_smmu` (`SID 0x1d01`) faults next.

#### Step 0 — Read-only preflight on currently booted phone (LOW risk)
- **Commands:**
  ```sh
  OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh --fresh '
    uname -r
    r8q-preflight --json
  '
  ```
- **Expect / Pass:** SSH answers on `172.16.42.1`, NV checksums pass (`15/15`,
  zero `ro=0` NV blocks).
- **Stop condition:** Any NV checksum mismatch or unexpected `ro=0` block.
- **Rollback:** N/A (read-only).

#### Step 1 — Push Run 9 kernel modules over USB SSH (MEDIUM risk — WRITE)
- **Approval required:** Ask human before running `modules-push`.
- **Commands:**
  ```sh
  TAR=~/omamobile-kernel/out-r8q-nom3/modules-7.3.0-rc6-omamobile-push1-g561090d2799a.tar.zst
  sha256sum "$TAR"
  OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" OMAMOBILE_SSH_SHARE=1 \
    ./bin/omamobile modules-push "$TAR"
  ```
- **Expect / Pass:** `sha256sum` matches
  `89fcb4c717534d15c0fd183b367f834d3ce9572e7e09e3fae8468f28289e4e28`;
  `/usr/lib/modules/7.3.0-rc6-omamobile-push1-g561090d2799a` is installed
  without `qcom_pd_mapper.ko` or `rmtfs_mem.ko`. (New release dir, so
  `OMAMOBILE_MODULES_REPLACE=1` is not needed.)
- **Stop condition:** Non-zero exit or sha256 mismatch.
- **Rollback:** `rm -rf /usr/lib/modules/7.3.0-rc6-omamobile-push1-g561090d2799a`
  does not affect the currently booted kernel or daily `out-r8q-uw` modules.

#### Step 2 — Flash `recovery-modem-nom3.img` & cold/warm boot (MEDIUM risk — WRITE)
- **Approval required:** Ask human before flashing `sda24` and before rebooting.
- **Path A (Over USB SSH when already booted into Omarchy Recovery on `userdata`):**
  ```sh
  IMG=~/omamobile-kernel/out-r8q-nom3/recovery-modem-nom3.img
  sha256sum "$IMG"
  OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/r8q-flash-recovery "$IMG"
  OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh --fresh 'reboot-to recovery'
  ```
  *(Note: if `mhi0` was wedged in `EE: DISABLE` on that boot and warm `reboot-to recovery` does not power-cycle SDX55M `PON`, use Path B's full `poweroff`.)*
- **Path B (Download Mode cold flash — clears `EE: DISABLE`):**
  1. Power off cleanly so `userdata` (`sda36`) unmounts and SDX55M loses `PON`:
     ```sh
     OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh --fresh 'poweroff'
     ```
  2. Enter Download Mode (hold **Vol Up + Vol Down** while plugging USB), ensure **only one** `04e8:685d` device is present in `lsusb`, and run `heimdall flash` directly (do not run `heimdall detect` beforehand without `--resume`):
     ```sh
     IMG=~/omamobile-kernel/out-r8q-nom3/recovery-modem-nom3.img
     sha256sum "$IMG"
     heimdall flash --RECOVERY "$IMG" --no-reboot
     ```
  3. Exit Download Mode (**Vol Down + Power** ~7 s) then boot Recovery (**Vol Up + Power** with USB plugged).
- **Expect / Pass:** `sha256sum` matches
  `4e6d8cdfcbf9da6138b252c13613cfc11a9fb7e528c9b1e0d6257f6e39ff47ff`;
  write and readback succeed (`sda24 ro=1`).
- **Stop condition:** Non-zero write exit or readback sha256 mismatch.
- **Rollback:** Flash known-good daily native image:
  `tools/r8q-flash-recovery ~/omamobile-kernel/out-r8q-uw/recovery-stable-native.img`
  (`7799b1e2…`).

#### Step 3 — Verify Run 9 kernel & runtime environment (LOW risk)
- **Commands:**
  ```sh
  OMAMOBILE_SSH_KEY="$HOME/.ssh/omamobile-r8q" ./tools/phone-ssh --fresh '
    uname -r
    lsmod | grep -E "pd_mapper|rmtfs|mhi|qrtr" || true
    ls -la /dev/qcom_rmtfs_mem* /var/lib/rmtfs/
    dmesg | grep -iE "rmtfs|mhi|pcie|ap2mdm" | tail -n 30
    r8q-preflight --json
  '
  ```
- **Expect / Pass:**
  - `uname -r` prints `7.3.0-rc6-omamobile-push1-g561090d2799a`
  - `qcom_pd_mapper` and `rmtfs_mem` are **not** loaded
  - `/dev/qcom_rmtfs_mem1` exists, `/var/lib/rmtfs/modem_fs1..` symlinks intact
  - NV checksums match, all NV partitions `ro=1`
  - `mhi0` is in `SBL` (`wwan0sahara0` present)
- **Stop condition:** Wrong kernel release, missing `/dev/qcom_rmtfs_mem1`,
  `mhi0` already in `EE: DISABLE`, or any NV check failure.
- **Rollback:** `tools/r8q-flash-recovery ~/omamobile-kernel/out-r8q-uw/recovery-stable-native.img`.

**Run 9 results (2026-11-19, `7.3.0-rc6-omamobile-push1-g561090d2799a`):**

- Pushed `1820` modules via `bin/omamobile modules-push` (`89fcb4c7...`) and
  flashed `recovery-modem-nom3.img` (`4e6d8cdf...`) to `sda24` over USB SSH via
  `tools/r8q-flash-recovery`, then booted straight into it via `reboot-to recovery`
  (zero Download Mode needed).
- Step 3 passed: `uname -r` = `7.3.0-rc6-omamobile-push1-g561090d2799a`,
  `qcom_pd_mapper` and `rmtfs_mem` absent, `/dev/qcom_rmtfs_mem1` and
  `/var/lib/rmtfs/*` intact, all 12 NV partitions `ro=1`, `gpio56` (`out high`),
  `gpio57` (`out low`), `pm8150l gpio7` (`out low`), `mhi0` in `SBL`.
- Step 4 (`logs/r8q/run9-{dmesg,rmtfs,capture}.log`):
  - `r8q-sahara` served all 14 images (`END_OF_IMAGE status=0`), reaching
    `EE: MISSION MODE` (`[   92.270249] mhi mhi0: Mission mode doorbell offsets:
    CHDBOFF=0x300 ERDBOFF=0x700`, `wwan0qmi0` attached `[   92.283188]`).
  - `T+0s`, `T+2s`, `T+5s` debugfs captures confirmed `wake: false`,
    `IPCR(20)` TX advancing (`wp: 0xfffcd030 -> 0xfffcd040 -> 0xfffcd050`),
    and `IPCR(21)` (`er=2, len=0x400`) receiving `18` packets at `T+0s`
    (`wp: 0xfffcc120`, registering the 12 `Node 3` early QMI services), then
    `0` further packets from `Node 3` at `T+2s` and `T+5s`.
  - `rmtfs -r -v -P -o /var/lib/rmtfs` logged `registering services` and QRTR
    control messages (`from: 1:-2`, `bye from 3`), with **zero** `rmtfs` QMI
    requests from `Node 3`, and `qmicli -d /dev/wwan0qmi0 --dms-get-manufacturer`
    timed out.
  - At `[  106.080270]` (**+13.81 s** after mission mode), `pcieport
    0002:00:00.0: Recovering Root Port due to Link Down` fired with zero
    `arm-smmu` faults. (Second `reboot-to recovery` at `11:49:17` hit the
    warm-reboot USB re-enumeration quirk.)
- **Next (Run 10 diagnostic script ready in scratch `run10_diag.py`):**
  1. Monitor `gpio1` (`mdm2ap_errfatal`) and `gpio3` (`mdm2ap_status`) during the
     `0..15 s` window + bind QRTR `Service 64` listener to catch whether
     `Node 3` queries `SERVREG_LOC`.
  2. When SDX55M resets with Sahara `HELLO mode=2`, read the read-only
     `MEMORY_DEBUG` table and small `sfr`/`err`/`dmesg` regions to capture
     SDX55M's exact panic reason.

**Run 10–11 results (2026-11-19, `7.3.0-rc6-omamobile-push1-g561090d2799a`):**

- `run10_diag.py watch` (`logs/r8q/run10-{dmesg,rmtfs,watch,capture}.log`)
  captured live GPIO state transitions across the full Mission Mode window
  without `qmicli`:
  - `gpio56` (`AP2MDM_STATUS`) = `out:high`, `gpio57` (`AP2MDM_ERRFATAL`) =
    `out:low`, `pm8150l gpio7` (`AP2MDM_ERRFATAL2`) = `out:low`, `gpio85`
    (`PERST#`) = `out:high`.
  - At `1586.401`, `gpio3` (`mdm2ap_status`) transitioned `in:high`.
  - SDX55M remained in Mission Mode for **32.2 s** (`2.3x` longer without
    `qmicli`), then at `1618.624` `gpio1` (**`mdm2ap_errfatal`**) asserted
    **`in:high`** for ~0.9 s while `mdm2ap_status` was still `high`, then both
    dropped and `Link Down` fired at `[1619.506844]`. This proves the Mission
    Mode exit is an internal SDX55M software **`ERR_FATAL`** rather than a
    random PCIe electrical drop.
- `run10_diag.py memdump` observed `HELLO mode=0` (not `mode=2`) after `Link
  Down`, matching Run 8b (`logs/r8q/run8b-sahara.log:2-16`) where SBL asks for
  `mdmddr.mbn` (`34`) / `apdp.mbn` (`40`) before offering `HELLO mode=2`
  (`MEMORY_DEBUG`); closing `/dev/wwan0sahara0` on `mode != 2` tore down MHI to
  `EE: DISABLE`.
- Offline comparison against stock `live.dts` and `qdsp6sw.mbn` showed:
  - `qdsp6sw.mbn` uses `QMI RMTFS` (`Service 14`), `SERVREG_LOC` (`Service 64`),
    and `RFS TFTP` (`Service 4096`).
  - Stock `live.dts` dedicates `ER0` strictly to control events (`32` elements),
    puts `SW_DATA` (`256` elements, `intmod=0`) on `ER1..ER3`, `QMI0(14/15)` at
    `64` elements on `ER1` (`UL`) and `ER2` (`DL`), `SAHARA(2/3)` at `128` on
    `ER1`, `DIAG(4/5)` at `64` on `ER1`/`ER3`, and `DUN(32/33)` at `64` on `ER3`.

**Run 12 image prepared and flashed (2026-11-19, `7.3.0-rc6-omamobile-push1-g7e390b575f95`, pending USB replug):**

- Kernel commit `7e390b575` (`wt-r8q`, `r8q-modem-nom3`) adds
  `mhi_samsung_r8q_sdx55_events` (`ER0` `CTRL(32)`, `ER1..ER3` `SW_DATA(256)`)
  and moves all data channels off `ER0` (`QMI(14/15)` -> `64` on `ER1`/`ER2`,
  `SAHARA(2/3)` -> `128` on `ER1`, `DIAG(4/5)` -> `64` on `ER1`/`ER3`,
  `DUN(32/33)` -> `64` on `ER3`); `device/r8q/kernel.config` adds
  `CONFIG_DYNAMIC_DEBUG=y`.
- Pushed `1820` modules (`modules-7.3.0-rc6-omamobile-push1-g7e390b575f95.tar.zst`)
  over SSH and flashed `recovery-modem-nom3.img` (`137b5f431d97f723f2696fdce5edb4689df082388616dc39bd0ae054242347ab`)
  to `/dev/sda24` with all 12 NV partitions verified `ro=1`.
- `reboot-to recovery` hit the warm-reboot USB-C re-enumeration quirk (`NOTES.md:151`):
  phone requires one USB-C replug before uploading `run10_diag.py` (now updated
  with `QMI_SERVREG_LOC` responder for `tms/servreg` -> `mdm/modem/root_pd:165`
  & `avs/audio` -> `msm/adsp/audio_pd:74`, plus `mode=0 -> mdmddr/apdp -> mode=2`
  `MEMORY_DEBUG` crash-table capture) and running Run 12.

#### Step 4 — Attended Run 9 modem bring-up + in-window capture (MEDIUM risk — modem RAM-only, NV `ro=1`)
- **Approval required:** Ask human before starting `r8q-sahara` / modem bring-up.
- **Commands:**
  1. Start laptop-side `dmesg -w`, `tqftpserv`, and line-buffered
     `rmtfs -r -v -P -o /var/lib/rmtfs` logging into `logs/r8q/run9-*.log`.
  2. Verify `/sys/kernel/debug/mhi/mhi0/{states,channels,events}` exists.
  3. Trigger `r8q-sahara` and immediately poll/snapshot `mhi0` debugfs,
     `qrtr-lookup`, and `qmicli -d /dev/wwan0qmi0 --dms-get-manufacturer` within
     the `0..20 s` window after `EE: MISSION MODE`:
     ```sh
     # On phone (scripted): start r8q-sahara -> on MISSION MODE dump states/channels/events,
     # qrtr-lookup, qmicli --dms-get-manufacturer, and capture dmesg for arm-smmu / Link Down.
     ```
- **Expect / Pass:**
  - `rmtfs -r -v -P -o /var/lib/rmtfs` logs real `rmtfs` requests from `Node 3`
  - `qmicli` returns `QUALCOMM INCORPORATED` (or `WDS/DMS/NAS/UIM` appear in
    `qrtr-lookup`)
  - No `pcieport 0002:00:00.0: Recovering Root Port due to Link Down`
  - **Next diagnostic if `apps_smmu` faults:** `dmesg` shows `arm-smmu`
    unhandled context fault (`SID 0x1d01`) — expected next step per Run 8 /
    SMMU plan (`MODEM-PLAN.md` lines 1130–1146), not an NV risk.
- **Stop condition:** `Link Down` repeats, unexpected kernel oops/panic, or
  `PCIE` fails to recover.
- **Rollback:** Power off phone, then (if needed) flash `out-r8q-uw/recovery-stable-native.img` in Download Mode.

**Resolution (2026-11-18).** The root now lives on `userdata` (`sda36`, ext4
label `omamobile-root`, 105 GiB), installed with
[RUNBOOK-USERDATA-ROOT](RUNBOOK-USERDATA-ROOT.md) and verified: `systemd`
running, Hyprland up, exactly one writable UFS partition. The SD card is no
longer in the boot path, so its drop at modem power-on is now harmless. The
normal-boot test can be repeated with the `out-r8q-msi` kernel (native image +
its modules in the root) — that is the next run, and it is still attended.

**Next, in order.**

1. ~~**Move the root to `userdata` (UFS).**~~ **Done** (2026-11-18): the root is
   on `userdata`; the descriptor's `UFS_RW="userdata"` and the label-gated
   native image are in place. Repeat the modem test against it.
2. **Or model a modem 3.3 V load switch/rail** from the schematic, if the
   hardware has one; the stock DT does not describe one, so this needs the
   board schematic, not just the DT.
3. **Then the userspace.** `ModemManager 1.24.2-2` is installed; the kernel QRTR
   name service is in `qrtr.ko`. `qrtr`, `rmtfs`, `pd-mapper` and `tqftpserv`
   are researched in [MODEM-USERSPACE](MODEM-USERSPACE.md): upstream tags,
   the r8q-specific `rmtfs -r -P -v` (never `-s`), the by-partlabel and
   `mdm1m9kefs*` caveats, and the read-only test order. The SIM/QMI test waits
   for a link that survives the AMSS.
