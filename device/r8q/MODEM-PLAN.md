<!-- SPDX-License-Identifier: MIT -->

# Modem & Cellular — Samsung Galaxy S20 FE (r8q)

**Status:** Phase 1 PASS; Phase 2 prepared offline (kernel `modem-stage2` built + `kernel-pr-check` clean, modules/recovery image packed, Sahara loader written and dry-run verified). Awaiting the human's go for the writes (modules install, `sda24` flash, reboot) and the first attended Sahara run.
**Safety:** All 77 UFS partitions read-only (`ro=1`); single-slot recovery boot (`sda24`) leaves LineageOS untouched; 15/15 NV partitions match the full off-repo backup
**Last Updated:** 2026-11-17

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
