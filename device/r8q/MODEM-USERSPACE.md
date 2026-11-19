<!-- SPDX-License-Identifier: MIT -->

# Modem userspace for the r8q (SDX55M over MHI/QRTR)

- **Status (2026-11-18):** the four packages (`qrtr`, `rmtfs`, `pd-mapper`,
  `tqftpserv`) are packaged, tested and installed — unenabled — on the phone.
  Run 7 ([MODEM-PLAN](MODEM-PLAN.md) §12) showed the stack working for the
  first time: a cold boot with `tqftpserv` up and one clean serve reached
  mission mode, and `qrtr-lookup` listed the modem's QRTR services, node 3
  advertising the *Modem embedded file system service* that `rmtfs` answers.
  The modem still drops back to SBL within a minute because `rmtfs` has no
  shared-memory window, so run 8 ([MODEM-PLAN](MODEM-PLAN.md) §12) is the
  `qcom,rmtfs-mem` + `rmtfs -r -v -P -o /var/lib/rmtfs` test with a
  6-minute hold. Nothing here writes the modem's NV partitions.
- **Why:** the kernel side is ready (`qcom_mhi_qrtr` on `mhi1_IPCR`,
  `mhi_wwan_ctrl` ports, ModemManager 1.24.2 installed); the userspace
  daemons are now packaged and installed, and run 8 tests whether `rmtfs`
  can answer the modem's QRTR/QMI requests with a shared-memory window.

## Components

All four are linux-msm projects, BSD-3-Clause, no vendor blobs; none is in the
official Arch Linux ARM repositories. The AUR carries all four
(`qrtr` 1.2-1, `rmtfs` 1.2-1, `pd-mapper` 1.1-1, `tqftpserv` 1.2-1); the project
packages should be adapted from upstream tags, not copied blindly.

| Package | Upstream | Tag | Build | Binary | Unit |
| --- | --- | --- | --- | --- | --- |
| `qrtr` | github.com/linux-msm/qrtr | v1.2 | meson | `libqrtr.so`, `qrtr-lookup`, `qrtr-cfg` | none |
| `rmtfs` | github.com/linux-msm/rmtfs | v1.3 | make | `rmtfs` | `rmtfs -r -P -v` |
| `pd-mapper` | github.com/linux-msm/pd-mapper | v1.1 | make (libqrtr, liblzma) | `pd-mapper` | `pd-mapper.service` |
| `tqftpserv` | github.com/linux-msm/tqftpserv | v1.2 | meson (libqrtr, liblzma, libzstd) | `tqftpserv` | `tqftpserv.service` |

**`qrtr-ns` is not needed:** the name service moved into the kernel (upstream
`0c2204a4ad71` and `a365023a76f2`), so the package only provides `libqrtr`,
`qrtr-lookup` and `qrtr-cfg`.

## r8q specifics and caveats

- **`rmtfs` must run read-only: `rmtfs -r -P -v`.** `-r` keeps modem writes in
  a RAM shadow and makes `storage_sync()` a no-op; never drop `-r`, never run
  it without it. **Do not use `-s`:** that looks for an MPSS remoteproc, and
  the r8q's modem is an external MHI endpoint with no rproc; the service exits
  and AGENTS forbids modem power-cycling. Do not use `-S` (non-A/B NV).
- **`-P` and `/dev/disk/by-partlabel`:** the r8q's Arch root has no
  `/dev/disk/by-partlabel` (AGENTS), so upstream `-P` can fail. Use `-o DIR`
  with a directory of symlinks to the read-only partitions, or with read-only
  copies from the off-repo NV backup, and name them as the modem requests.
- **`storage.c` table gap:** upstream maps `modem_fs1→modemst1`,
  `modem_fs2→modemst2`, `modem_fsc→fsc`, `modem_fsg→fsg`, `study`, `tunning`
  and some OEM names, but **not `mdm1m9kefs1/2/3`**, which this SDX55M has in
  its NV set. The first `rmtfs -r -P -v` session logs the requested paths;
  unknown names are rejected until the table (or the `-o` layout) covers them.
  That log is the discovery step, not a failure to hide.
- **`pd-mapper`:** v1.1 scans `/sys/class/remoteproc/*/firmware` for `*.jsn`.
  With no modem remoteproc bound it exits "no pd maps available". Try QMI
  without it first; if the modem asks for a PD map, add its `modem*.jsn` to a
  scanned firmware directory (or patch a static directory).
- **`tqftpserv`:** serves `/readonly/firmware/image/*` from
  `/lib/firmware/…` (the staged `qcom/sdx55m` set) and writes WRQs only under
  `/tmp/tqftpserv`.
- **ModemManager 1.24.2:** QRTR discovery needs libqmi built with QRTR and
  libqrtr-glib ≥ 1.0 (`ldd /usr/lib/libqmi-glib.so*`). For an external MHI
  modem, MBIM over `wwan0mbim0` is the reliable fallback if the qcom-soc MHI
  routing is not in this ModemManager.
- **Keep the in-kernel `qcom_pd_mapper` off** (marked broken upstream); use the
  userspace daemon.

## Read-only test order (after the link is stable)

1. `r8q-preflight --json`, `tools/nv-backup -d r8q --check <backup>`, and
   `dmesg`/MHI state `EE: MISSION MODE`; ports exist. **Stop:** guard not ok,
   NV mismatch, or a link-down/AER event.
2. `qrtr-lookup` — expect the modem's DMS/NAS/UIM/WDS services. **Stop:** no
   node/services.
3. `qmicli -d qrtr://<node> --uim-get-card-status` (read-only SIM status).
4. `tqftpserv`; watch the journal for RRQ/WRQ.
5. `pd-mapper`; record whether it reports "no pd maps available".
6. `rmtfs -r -P -v` in the foreground; log every requested partition path.
   **Stop** on any read-write open, oops, UFS error, or NV mismatch.
7. Only then ModemManager (`mmcli -L`, `mmcli -m 0`); registration and data are
   the owner's call.

**Smallest set:** `qrtr`, `rmtfs`, `pd-mapper`, `tqftpserv`. **First command on
a mission-mode modem:** `qrtr-lookup` (read-only, touches nothing).

## Packaging plan

Adopt the four upstream tags above under `packages/omamobile-{qrtr,rmtfs,
pd-mapper,tqftpserv}` with the project's PKGBUILD/test/version layout. The
r8q package (or a drop-in) must:

- run `rmtfs` as `-r -P -v`, never `-s`;
- drop any `qrtr-ns.service` dependency (the kernel owns the name service);
- install the udev rule for `qcom_rmtfs_mem*` only if the kernel exposes it
  (the external MHI modem may not);
- ship the read-only `-o` directory/symlink helper if `-P` cannot be made to
  work on this root.

The AUR PKGBUILDs are the starting point for source/checksum layout; the
systemd units are from upstream.
