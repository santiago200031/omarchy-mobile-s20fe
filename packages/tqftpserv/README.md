<!-- SPDX-License-Identifier: 0BSD -->

# tqftpserv — QRTR TFTP service for the r8q's SDX55M

Upstream [linux-msm/tqftpserv](https://github.com/linux-msm/tqftpserv)
v1.2. The modem requests files over QRTR-TFTP once it is in mission
mode: reads are served from the firmware class paths (the staged
`/lib/firmware/qcom/sdx55m` set among them), and writes land only under
its state directory (`/var/lib/tqftpserv`).

Run it in the foreground during the read-only test order and watch the
journal for RRQ/WRQ to learn which files the modem asks for
([MODEM-USERSPACE.md](../../device/r8q/MODEM-USERSPACE.md)).

The unit is shipped **unenabled**: the first run is manual.
