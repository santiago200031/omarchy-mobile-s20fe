<!-- SPDX-License-Identifier: 0BSD -->

# qrtr — QRTR userspace for the r8q (SDX55M over MHI)

Upstream [linux-msm/qrtr](https://github.com/linux-msm/qrtr) v1.2, built
with `-Dqrtr-ns=disabled`: **the QRTR name service moved into the kernel**
(upstream commits `0c2204a4ad71` and `a365023a76f2`), so the userspace
name server and its unit are not built.

Ships:

- `libqrtr.so` + `qrtr.pc` + `libqrtr.h` — what `rmtfs`, `pd-mapper` and
  `tqftpserv` link against;
- `qrtr-lookup` — **the first command on a mission-mode modem**
  ([MODEM-USERSPACE.md](../../device/r8q/MODEM-USERSPACE.md)): expect the
  modem's DMS/NAS/UIM/WDS services;
- `qrtr-cfg` — address configuration.

No unit: nothing here needs to run at boot by itself.
