<!-- SPDX-License-Identifier: MIT -->

# Runbook: move the r8q root from the SD card to `userdata`

- **Status:** **executed successfully 2026-11-18** (steps 1-5; see Result at the end).
- **Why:** the root image lives on the microSD (`mmcblk0p1`, `/omarchy-r8q.img`). Powering the SDX55M modem browns out the SD card, and the root goes read-only within 0.3 s ([MODEM-PLAN](MODEM-PLAN.md) §12). A root on `userdata` (`sda36`) does not depend on the SD card, so the modem can be brought up in a normal boot.
- **Cost:** formatting `sda36` **destroys LineageOS's userdata** (encrypted app data, photos, accounts). The Android system (`boot` sda23, `super` sda29) is untouched; a future Android boot would factory-reset its data. The Omarchy root data was backed up to `~/omamobile-kernel/reference-r8q/root-data-20261118.tar.zst`.
- **Safety:** the storage guard allows exactly `userdata` writable in these images; all other UFS partitions stay `ro=1`. No NV/modem partition is touched. Slot A / bootloader / vbmeta are not touched.

## Images (built 2026-11-18, stable kernel `7.3.0-rc6-omamobile-push1-g3667ab368ef9`)

| Image | Use | SHA256 |
| --- | --- | --- |
| `~/omamobile-kernel/out-r8q-uw/recovery-userdata-setup.img` | boot the initramfs rescue shell with `userdata` writable, `rsync` and `mkfs.ext4` staged | `e6069c1bae302b7ebf2fc33f7364b60ff42a1ac1517c135d8369d8475f50f3b7` |
| `~/omamobile-kernel/out-r8q-uw/recovery-userdata-native.img` | normal boot with the root on `userdata` (no `omamobile.loop=`, label `omamobile-root`) | `1ac311d54b8a05ff507f14e5bbe3c2b51d734c2d152bafc0ccc4541d5c7af561` |
| `device/r8q/stable-image.conf` | rollback to the SD-root dual boot | `66fe127ffd9345e7913aec5eac8a02de0da44bfdbe15987abff1cb61b26d8503` |

Both new images carry `/etc/omamobile-ufs-rw` = `userdata` and `/etc/omamobile-root-label` = `omamobile-root`; the pack tool writes them for a non-`--dual-boot` image (`tools/r8q-boot`).

## Step 1 — flash the setup image (write; needs the owner's go)

```sh
tools/r8q-flash-recovery ~/omamobile-kernel/out-r8q-uw/recovery-userdata-setup.img
```

**Expect:** `Recovery partition now holds e6069c1b…` and `ro: sda=1 /dev/sda24=1`.
**Pass:** the readback SHA256 equals the image's.
**Stop:** any SHA mismatch, a partition name other than `recovery`, or an answer that the phone did not come back. Do not retry blindly; the current system is still the stable SD-root one.
**Rollback:** none needed yet; the stable image is unchanged on the laptop.

## Step 2 — boot the setup image

```sh
tools/phone-ssh 'sync; reboot'
```

If the phone ever comes up in Android instead of the initramfs, **stop and ask the owner before pressing anything**: since a plain power-on boots `sda24` ([NOTES §24](NOTES.md)), an Android boot would mean `sda24` changed under us, not that the wrong key was pressed. The initramfs attempts `boot_arch`, refuses the encrypted `userdata` (label check), and stays in its rescue shell on tty1 with SSH at `172.16.42.1`.

**Expect:** `tools/phone-ssh --fresh 'cat /proc/sys/kernel/hostname'` answers `omamobile`, and the init log on tty1 says `arch: userdata is not an ext4 root` (or `not labelled`). Announce the screen: it is a text console, no desktop.
**Pass:** SSH answers, and `cat /sys/class/block/sda/sda36/ro` is `0`.
**Stop:** SSH does not come back, or `sda36/ro` is `1`. Stay in the rescue shell; do not flash anything. The stable image is still the way back.
**Rollback:** power-cycle into the stable image (`tools/r8q-flash-recovery --stable` from any shell that answers).

## Step 3 — format `userdata` and copy the root (destructive)

Run on the phone from the rescue shell. The copy is detached and logged, so a dropped SSH does not kill it.

```sh
mkfs.ext4 -F -L omamobile-root -m 1 /dev/sda36
mkdir -p /mnt/new /mnt/sd /mnt/old
mount /dev/sda36 /mnt/new
mount -t exfat -o ro /dev/mmcblk0p1 /mnt/sd || mount -o ro /dev/mmcblk0p1 /mnt/sd
loop=$(losetup -f --show -r /mnt/sd/omarchy-r8q.img)
mount -t ext4 -o ro "$loop" /mnt/old
setsid sh -c 'rsync -aHAX --numeric-ids --bwlimit=80000 /mnt/old/ /mnt/new/ >/tmp/root-copy.log 2>&1; echo rc=$? >>/tmp/root-copy.log' &
```

**Expect:** `/tmp/root-copy.log` ends with `rc=0` (about 3–5 minutes for 14 GiB).
**Pass:**
- `rsync -aHAX --numeric-ids --dry-run --itemize-changes /mnt/old/ /mnt/new/` prints nothing;
- `test -x /mnt/new/usr/lib/systemd/systemd` succeeds;
- `sync; df -h /mnt/new` shows about 14 GiB used and the full `userdata` size after step 5's grow.

**Stop:** any I/O error, a UFS error line in `dmesg`, `rc` other than 0, or a checksum difference from the dry run. Do not reboot into the native image; the SD root is still intact.
**Rollback:** `umount /mnt/new`; the SD root still boots with the stable image.

Then, before flashing the native image:

```sh
umount /mnt/new /mnt/old /mnt/sd
```

## Step 4 — flash the native image (write; needs the owner's go)

```sh
tools/r8q-flash-recovery ~/omamobile-kernel/out-r8q-uw/recovery-userdata-native.img
```

**Expect:** readback SHA256 `1ac311d5…`. **Stop/rollback:** as step 1; `--stable` restores the SD-root image.

## Step 5 — boot the native root

```sh
tools/phone-ssh 'sync; reboot'
```

**Expect:** the init log says `arch (/dev/sda36): switching to Arch Linux`; the first boot grows the filesystem to `userdata`'s ~107 GiB; then the normal Omarchy desktop.
**Pass:**
- `findmnt -no SOURCE,FSTYPE /` shows `/dev/sda36 ext4`;
- `cat /sys/class/block/sda/sda36/ro` is `0` and every other `sda*` is `1`;
- `tools/phone-smoke` is green and `tools/r8q-stable-check` still recognises the phone;
- the SD card can be pulled or left in; nothing on it is required at boot.

**Stop:** a fallback to the rescue shell, a missing label, or a root that mounts read-only. The serial console stays available; the stable image is the way back.
**Rollback:** `tools/r8q-flash-recovery --stable`, then restore the SD-root modules if the release changed (`bin/omamobile modules-push`); the SD root image was never written.

## After this

The modem test that failed on the SD root (§12) can be repeated with the native root: pack the SDX55M/MSI kernel (`out-r8q-msi`) as a **native** image, flash it, and follow [MODEM-PLAN](MODEM-PLAN.md) M2.5 with the SD card out or ignored. One change per boot: verify the native root on the stable kernel first (this runbook), then the modem kernel.

## Result (2026-11-18)

- Steps 1-5 ran as written. The copy took 265 s (`rsync --bwlimit=80000`, 13.9 GiB),
  the post-copy dry run reported **0 differences**, and the first native boot grew
  the filesystem from 24 GiB to **105 GiB**.
- Running system: `/dev/sda36 ext4 rw`, kernel
  `7.3.0-rc6-omamobile-push1-g3667ab368ef9`, `systemd` **running**, Hyprland and
  quickshell up, **exactly one writable UFS partition** (`sda36`; every other
  `sda*` is `ro=1`). One benign UFS attribute query (`idn 31`, `-22`) at 25 s;
  no ext4 errors, no I/O errors.
- Two boot-path facts to keep:
  - **`reboot` from the rescue initramfs returns 0 and does not reboot.** Use
    `sync; echo b > /proc/sysrq-trigger` (busybox `reboot` is a no-op here, as
    the old notes warned for other phones).
  - `tools/r8q-flash-recovery` needed its sysfs fallback fixed before it could
    write from the rescue shell: `/sys/block/sda/ro` is mode `0444` here, and
    the whole disk is already writable when its node is removed. It now
    asserts that instead of writing the flag (`44ca4352`).
- The SD card is no longer required at boot. The stable SD-root image is
  untouched on the laptop and the way back is unchanged
  (`tools/r8q-flash-recovery --stable`).

**Daily native image (2026-11-18).** With the SD-root `--dual-boot` image kept
as the way back, the daily configuration is now
`out-r8q-uw/recovery-stable-native.img` (sha256 `7799b1e2…`): stable kernel
`7.3.0-rc6-omamobile-push1-g3667ab368ef9`, the `camera-uw` overlay, and the
userdata root. Its first boot after flashing stopped before the USB gadget; a
forced restart (Power + Volume Down) booted normally, and the S5K3L6
(ultrawide) and HI-847 (tele) sensors are ENABLED in `/dev/media0`. Watch for
a repeat; if it recurs, capture the initramfs console before restarting.
