<!-- SPDX-License-Identifier: 0BSD -->

# rmtfs — remotefs service for the r8q's flashless SDX55M

Upstream [linux-msm/rmtfs](https://github.com/linux-msm/rmtfs) v1.3. The
r8q's SDX55M is flashless: its AMSS boots from RAM and then asks the host,
over QRTR, for the file system it has no flash for. Without an answer it
reboots — the modem cycles SBL → AMSS → SBL forever
([MODEM-PLAN](../../device/r8q/MODEM-PLAN.md) run 6).

## Flags (safety invariants, enforced by test/all)

- **`-r` always.** Every modem write lands in a RAM shadow and
  `storage_sync()` is a no-op; the storage files are only ever read.
- **Never `-s`.** That is the MPSS remoteproc sync path; the r8q's modem
  is an external MHI endpoint with no rproc, and `rproc_init()` fails.
- **Never `-S`.** A/B slot suffixes; these partitions are not slotted.
- `-P -o /var/lib/rmtfs`: serve by-partlabel-named files from that
  directory (the phone's Arch root has no `/dev/disk/by-partlabel`).
  Symlink the modem's partitions there by their `PARTNAME`, found via
  `/sys/class/block/*/uevent`. Upstream's table maps
  `modem_fs1→modemst1`, `modem_fs2→modemst2`, `modem_fsc→fsc`,
  `modem_fsg→fsg`, `modem_study→study`, `modem_tunning→tunning`.

## Known gap (discovery step, not a failure to hide)

Upstream's table does not cover `mdm1m9kefs1/2/3`, which this SDX55M has
in its NV set. A request for an unmapped name is logged
(`[RMTFS storage] request for unknown partition '…', rejecting`) and
rejected. Run the first session in the foreground
(`rmtfs -r -v -P -o /var/lib/rmtfs`) and record every requested path
before deciding whether the table or the directory layout needs to grow.

The unit is shipped **unenabled**: the first run is manual, in the
foreground, per the read-only test order in
[MODEM-USERSPACE.md](../../device/r8q/MODEM-USERSPACE.md).
