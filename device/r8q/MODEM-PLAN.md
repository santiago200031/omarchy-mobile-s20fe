<!-- SPDX-License-Identifier: MIT -->

# Modem & Cellular — Samsung Galaxy S20 FE (r8q)

**Status:** Off-repo RO backup taken (`~/omamobile-kernel/reference-r8q/nv-backup/`); modem stack not yet enabled
**Safety:** All 77 UFS partitions read-only (`ro=1`); single-slot recovery boot (`sda24`) leaves LineageOS untouched
**Last Updated:** 2026-11-17

---

## 1. Safety Rules (Mandatory)

1. **Verify NV backup & read-only guard before any modem work (`AGENTS.md` §1):**
   - Device descriptor `device/r8q/device.conf` defines:
     - `NV_PARTS="efs sec_efs persist modemst1 modemst2 fsg fsc"`
     - `NV_BACKUP_EXTRA="param steady devinfo"`
   - Check storage guard on phone: `tools/phone-ssh 'r8q-preflight --json'` (expected `"ufs_guard":{"ok":true,"detail":"77 UFS partitions read-only"}`).
   - Verify off-repo backup with `-d r8q`: `tools/nv-backup -d r8q --check <backup-dir>`.
   - Take a fresh RO backup after any stock/LineageOS boot before touching Linux modem services.
2. **Never write NV partitions from Linux.** All NV partitions stay `ro=1`. Only `omamobile-modem`'s per-phone overlay on `userdata` (`/var/lib/omamobile/modem-nv-overlay`) may persist allowed modem writes once explicitly enabled.
3. **One change per boot & human approval required:** Ask the human before packing/flashing any new overlay onto `sda24` or rebooting.

---

## 2. Current Verified State

- Off-repo backup directory `~/omamobile-kernel/reference-r8q/nv-backup/` contains `SHA256SUMS` and read-only partition captures (`efs.img`, `fsc.img`, `mdm1m9kefs1.img`, `mdm1m9kefs2.img`, `mdm1m9kefsc.img`, `modem.img`, `modemst1.img`, `modemst2.img`, `persist.img`, `sec_efs.img`).
- Live remoteprocs on `7.3.0-rc6-omamobile-push1-g7ca90963c426`:
  - `remoteproc0` (`slpi`) = `running`
  - `remoteproc1` (`cdsp`) = `running`
  - `remoteproc2` (`adsp`) = `running`
- `omamobile-modem` is not installed on the phone yet, and `device/r8q/overlays/modem.dtso` does not exist yet.

---

## 3. Phased Bring-Up Plan (SM8250)

### 3.1 Phase 0: Pre-Flight & Firmware Staging (Read-Only)

1. Confirm all 77 UFS partitions are `ro=1` via `r8q-preflight --json`.
2. Verify NV backup integrity against the phone via `PARTNAME` matching `device/r8q/device.conf` (`NV_PARTS`).
3. Verify extracted modem firmware files (`modem.mbn` / MBN splits) match what `qcom_q6v5_mss` (`&remoteproc_mpss`) expects in `/lib/firmware/qcom/sm8250/samsung/r8q/`.

### 3.2 Phase 1: Remoteproc MPSS (`&remoteproc_mpss`) Isolated Overlay

1. Inspect `sm8250-samsung-r8q.dts` / `sm8250.dtsi` for `&remoteproc_mpss` firmware path, memory regions, and clocks/rails.
2. Create an isolated `device/r8q/overlays/modem.dtso` overlay enabling MPSS (do not fold into base DTS until tested).
3. With human approval, pack an image with `--dual-boot --overlay modem`, stream `dmesg -w` over USB before probing, and flash `sda24`.
4. **Stop condition:** If MPSS fails to boot, crashes RPMh, or resets UFS/USB, immediately drop `--overlay modem` and roll back `sda24`.

### 3.3 Phase 2: QRTR / RMTFS / PD-Mapper (`omamobile-modem`)

1. Adapt and build `packages/omamobile-modem` (`rmtfs`, `tqftpserv`, `pd-mapper`) for SM8250 QMI/QRTR + `r8q` NV rules.
2. Verify `rmtfs` serves read-only EFS copies without writing raw UFS NV partitions.

### 3.4 Phase 3: ModemManager & Data (With SIM)

1. Install/test `modemmanager` once MPSS + RMTFS stay stable across boot/idle.
2. Test SIM detection, registration, and mobile data (`mmcli`).

---

## 4. Rollback

If an attended overlay test breaks boot or stability:
1. Hold Power + Volume Down (or use SSH if up) to enter recovery/download/bootloader as applicable, or re-flash `sda24` without `--overlay modem`:
   ```bash
   tools/r8q-boot pack --dual-boot --overlay ...   # previous verified overlay list only
   tools/r8q-flash-recovery
   ```

---

## 5. Status

| Item | Status | Notes |
|------|--------|-------|
| UFS storage guard (`77` parts `ro=1`) | ✅ Verified | Checked live via `r8q-preflight --json` |
| Off-repo NV backup (`nv-backup/`) | ✅ Present | Keep off-repo; re-verify with `-d r8q` before MPSS probe |
| `device/r8q/overlays/modem.dtso` | ⏳ Pending | Not created yet |
| MPSS remoteproc (`qcom_q6v5_mss`) | ⏳ Pending | Needs firmware path check + attended overlay test |
| `omamobile-modem` (`rmtfs`/`pd-mapper`) | ⏳ Pending | Package not installed on phone |
| `ModemManager` / cellular data | ⏳ Pending | After MPSS + RMTFS |
