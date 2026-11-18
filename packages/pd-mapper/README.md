<!-- SPDX-License-Identifier: 0BSD -->

# pd-mapper — protection-domain mapper for the r8q's SDX55M

Upstream [linux-msm/pd-mapper](https://github.com/linux-msm/pd-mapper)
v1.1, answering the modem's servreg location queries over QRTR. The
in-kernel `qcom_pd_mapper` is marked broken upstream and is not in this
kernel's config; this userspace daemon is the one to run.

## r8q caveat

v1.1 scans the firmware class paths (and `/vendor/firmware/`) for
`*.jsn` PD maps. With no modem remoteproc bound, there may be no maps to
find, and the daemon exits `no pd maps available`. That is expected on a
first run: try QMI without it, and if the modem asks for a PD map, add
its `modem*.jsn` to a scanned firmware directory
([MODEM-USERSPACE.md](../../device/r8q/MODEM-USERSPACE.md)).

The unit is shipped **unenabled**; start it by hand in the read-only test
order and record what it reports.
