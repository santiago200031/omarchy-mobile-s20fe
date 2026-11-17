<!-- SPDX-License-Identifier: MIT -->

# Modem & Cellular — Samsung Galaxy S20 FE (r8q)

**Status:** Phase 1 PASS (`pcie2` Gen.3 x2 link up, SDX55M `[17cb:0306]`/`[17cb:010c]` bound to `mhi-pci-generic`, `mhi0` BHI ready); blocked at Stage 2 firmware load (`foxconn-sdx55` `.fw = NULL` + SDX55M firmware/Sahara staging)
**Safety:** All 77 UFS partitions read-only (`ro=1`); single-slot recovery boot (`sda24`) leaves LineageOS untouched
**Last Updated:** 2026-11-17

---

## 1. Safety Rules (Mandatory)

1. **Verify NV backup & read-only guard before any modem work (`AGENTS.md` §1):**
   - Device descriptor `device/r8q/device.conf` defines:
     - `NV_PARTS="efs sec_efs persist modemst1 modemst2 fsg fsc"`
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
  - Off-repo RO backup `~/omamobile-kernel/reference-r8q/nv-backup-20261117`: **10/10 `SAME`** (`efs`, `sec_efs`, `persist`, `modemst1`, `modemst2`, `fsg`, `fsc`, `param`, `steady`, `devinfo`).
  - Additional external SDX55M UFS partitions observed on the phone (all `ro=1`): `sda4` (`mdm1m9kefs1`), `sda5` (`mdm1m9kefs2`), `sda6` (`mdm1m9kefsc`), `sdd4` (`mdm1m9kefs3`), `sdd2` (`mdmddr`). Back up these five partitions off-repo read-only as well before any Stage 2 firmware/Sahara execution.
  - Read-only inspection of `/dev/sda20` (`modem`, VFAT, `ro=1`) confirmed `image/sdx55m/` contains `sbl1.mbn`, `qdsp6sw.mbn`, `aop.mbn`, `apps.mbn`, `devcfg.mbn`, `hyp.mbn`, `tz.mbn`, `xbl_cfg.elf`, `multi_image_qti.mbn`, `sec.elf`, `acdb.mbn`, `mdmddr.mbn`, `modemr.jsn`, and `efs1/2/3.bin` (plus `image/sdx55/qdsp6m.qdb`). `/lib/firmware/qcom/sdx55m/` is not yet staged on rootfs, and `omamobile-modem` is not installed yet.

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

### 3.2 Phase 2: Firmware Staging, `pci_generic` BHI Quirk, `rmtfs` & QRTR (`omamobile-modem`)

1. Back up `mdm1m9kefs1`, `mdm1m9kefs2`, `mdm1m9kefsc`, `mdm1m9kefs3`, and `mdmddr` read-only off-repo first.
2. Add an `of_machine_is_compatible("samsung,r8q")` (or DT firmware-name) quirk in `drivers/bus/mhi/host/pci_generic.c` so `0002:01:00.0` loads `qcom/sdx55m/sbl1.mbn` via BHI instead of skipping firmware load as `foxconn-sdx55`.
3. Extract/stage `/lib/firmware/qcom/sdx55m/` (from read-only `/dev/sda20` `/image/sdx55m/`) and verify Sahara/BHI handoff without writing raw UFS NV partitions (`efs`/`mdm1m9kefs*` stay `ro=1`).
4. Adapt/build `packages/omamobile-modem` (`rmtfs`, `tqftpserv`, `pd-mapper`, Sahara downloader if needed) for `r8q` + `qrtr-mhi`.

### 3.3 Phase 3: ModemManager & WWAN Data (With SIM)

1. Test `mhi_wwan_ctrl` / `mhi_wwan_mbim` / `mhi_net` + `ModemManager` (`mmcli -L`).
2. Test SIM detection, registration, and mobile data.

---

## 4. Rollback

If an attended overlay test breaks boot or stability:
1. Re-flash `sda24` without `--overlay modem` using the verified `out-r8q-fold/recovery-omamobile.img` (`--dual-boot`).

---

## 5. Status

| Item | Status | Notes |
|------|--------|-------|
| UFS storage guard (`77` parts `ro=1`) | ✅ Verified | Checked live via `r8q-preflight --json` |
| Fresh NV backup (`nv-backup-20261117`) | ✅ Verified | `10/10 SAME` (`tools/nv-backup -d r8q --check`) |
| Stock topology (`SDX55M` on `pcie2` + `mhi`) | ✅ Identified | Modules already built (`mhi_pci_generic`, `qrtr-mhi`, `mhi_wwan_*`) |
| `device/r8q/overlays/modem.dtso` | ✅ Verified | `pcie2` Gen.3 x2 link up; `0002:01:00.0` `[17cb:0306]`/`[17cb:010c]` probed cleanly |
| `pcie2` PCIe enumeration & `mhi` probe | ✅ Stage 1 / ⏳ Stage 2 | `mhi0` BHI ready; blocked on `foxconn-sdx55` `.fw = NULL` vs `qcom/sdx55m/sbl1.mbn` + Sahara |
| `omamobile-modem` (`rmtfs`/`pd-mapper`) & `ModemManager` | ⏳ Pending | Needs `mdm1m9kefs*` RO backup, `pci_generic.c` quirk, firmware staging & `omamobile-modem` |
