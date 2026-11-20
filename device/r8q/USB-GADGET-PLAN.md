<!-- SPDX-License-Identifier: MIT -->

# USB Gadget Plan — r8q (Galaxy S20 FE, SM8250Q)

Status: **investigating the gadget-missing delta** (2026-11-20) — the new
image's gadget goes missing; the stable baseline brings it up. Phone is
down (gadget missing; user reads "no usb host" on the phone).

Chapters:
- [A. R8q gadget architecture](#a-r8q-gadget-architecture)
- [B. Gadget-delta candidates (stable vs new)](#b-gadget-delta-candidates-stable-vs-new)
- [C. Phone-side diagnostics (E3 — for when SSH is up)](#c-phone-side-diagnostics-e3)
- [D. Decision tree + stable escape (E4)](#d-decision-tree--stable-escape-e4)
- [E. Automation scripts](#e-automation-scripts)
- [F. Investigation status](#f-investigation-status)

---

## A. R8q gadget architecture

### Where the gadget comes up during boot (initramfs step order)

The r8q gadget runs entirely in the initramfs (Alpine + busybox `ip`/dropbear),
before `switch_root` to the Arch root. Shared `initramfs/init` flow:

1. Storage guard: UFS marked read-only; `load_modules` (initramfs/init:268–282)
   reads `/lib/modules/omamobile/order` (the descriptor's `INITRAMFS_MODULES`).
   On r8q nothing is needed: the USB/PHY drivers are built-in
   (`device/r8q/kernel.config:31–35`: `CONFIG_PHY_QCOM_USB_SNPS_FEMTO_V2=y`,
   `CONFIG_PHY_QCOM_QMP_USB=y`, `CONFIG_USB_DWC3=y`, `CONFIG_USB_DWC3_QCOM=y`).
2. Gadget setup (initramfs/init:324–343): configfs gadget `g1` —
   `idVendor 0x1d6b` (Linux Foundation), `idProduct 0x0104` (Multifunction
   composite), manufacturer "Omarchy Mobile", product string from
   `/etc/omamobile-usb-product` (packed by `bin/omamobile` from the
   descriptor's `USB_PRODUCT` or the device NAME — "Samsung Galaxy S20 FE"),
   serialnumber "omamobile"; config `c.1` "NCM + ACM"; functions
   `ncm.usb0` + `acm.GS0` linked in.
3. UDC wait (initramfs/init:345–355): `udc=$(ls /sys/class/udc | head -1)`,
   up to 30×1 s. If non-empty: `echo "$udc" > $g/UDC` →
   `say "USB gadget up on $udc"`. If empty after 30 s:
   **`say "no USB device controller found"`** (initramfs/init:354).
4. `maybe_usb_switch` (initramfs/init:386–393): reads `/etc/omamobile-usb-switch`
   + `board_matches`. **Not packed on r8q** — `bin/omamobile:597–598` only
   writes it when the descriptor sets `DEV_USB_SWITCH`, and `device/r8q/`
   sets none (grep: no match).
5. Network (initramfs/init:397–399): `ip link set lo up`; wait ≤10 s for
   `ip link show usb0`; then `ip addr add 172.16.42.1/24 dev usb0 &&
   ip link set usb0 up` (phone side 172.16.42.1; host sets 172.16.42.2).
   If the interface never appears: busybox prints **`can't find device usb0`**
   (the `ip` at line 399).
6. `dropbear -R -E -B -p 22` → `root@172.16.42.1` (line 405); `getty` on
   `ttyGS0` (line 406) — the serial rescue shell.
7. `switch_to_arch` (initramfs/init:520–543): `switch_root` to the Arch root
   on `sda24` (userdata). "The USB gadget and its address are already up by
   the time this runs, and the kernel keeps them across the switch"
   (lines 520–521).
8. Post-boot self-recovery (initramfs/init:455–482): if no host configures
   within 45 s, prints "USB never configured; dmesg follows" — but **on r8q
   it stays alive** ("On non-A/B devices like r8q, panicking loops in
   recovery; stay alive instead", lines 471–473; `device.conf`:
   `BOOT_METHOD=ram-only` — no ABL/stock fallback on this device).

### Who registers the UDC on a healthy r8q

The mainline dwc3 core driver (built into the r8q kernel), via this chain
(vendor SM8250 tree, `~/omamobile-kernel/wt-r8q/arch/arm64/boot/dts/qcom/`):

- `usb_1: usb@a6f8800` — "qcom,sm8250-dwc3" (sm8250.dtsi:4183): wrapper at
  0xa6f8800 with clocks `cfg_noc/core/iface/sleep/mock_utmi/xo`
  (GCC_USB30_PRIM_*), assigned `mock_utmi=19.2 MHz`, `core=200 MHz`,
  power-domain `USB30_PRIM_GDSC`, reset `GCC_USB30_PRIM_BCR`, interconnects.
- `usb_1_dwc3: usb@a600000` — "snps,dwc3" (sm8250.dtsi:4229): DWC3 core at
  0xa600000, interrupt `GIC_SPI 133`, `iommus` (apps_smmu),
  `phys = <&usb_1_hsphy>` (the samsung common removed the SS/DP phy).
- The samsung common forces `dr_mode = "peripheral"` + `maximum-speed =
  "high-speed"` on `&usb_1_dwc3` (sm8250-samsung-common.dtsi:303–309).
- On probe with `dr_mode == USB_DR_MODE_PERIPHERAL`, mainline
  `dwc3_core_init()` runs `phy_set_mode(..., PHY_MODE_USB_DEVICE)` and
  `dwc3_gadget_init(dwc)` (drivers/usb/dwc3/core.c:1634+).
  `dwc3_gadget_init` → `usb_add_gadget(dwc->gadget)`
  (drivers/usb/dwc3/gadget.c:4934, "failed to add gadget" on error) →
  mainline `usb_add_gadget()` (drivers/usb/gadget/udc/core.c:1410) →
  **the UDC class entry `/sys/class/udc/a600000.usb`** (named after the
  gadget's parent device, udc/core.c:1424).

So on a healthy r8q: `ls /sys/class/udc/` shows `a600000.usb`;
`cat /sys/class/udc/*/state` reads one of the mainline strings
("not attached", "attached", "powered", "default", "configured", … —
drivers/usb/common/common.c:160–179; "configured" is what the initramfs
recovery loop waits for, initramfs/init:460). The gadget is USB 2.0
HS-only NCM + ACM (success notes,
`device/r8q/board/success/05-usb-power-battery-zram.md`:
"dwc3 USB 2.0 NCM gadget (172.16.42.1)").

### The two messages, exactly

- **"no USB device controller found"** — initramfs/init:354. After 30 s,
  `ls /sys/class/udc` is empty: the dwc3 gadget never registered a UDC
  (dwc3/hsphy/wrapper chain failed to come up, or the driver isn't probed —
  unlikely on r8q, built-in).
- **"can't find device usb0"** — the busybox `ip` error at
  initramfs/init:399 after the ≤10 s wait loop (line 398) for
  `ip link show usb0` failed: the `usb0` interface node doesn't exist when
  the initramfs tries to give the phone its 172.16.42.1/24 address.
- Together: no UDC was found → nothing wrote `$g/UDC` → the gadget was
  never bound → `usb0` never appeared. (A late-registering UDC after the
  30 s check produces the same pair.)

---

## B. Gadget-delta candidates (stable vs new)

Measured (2026-11-20): the tree delta between the stable branch
(`omamobile-r8q-signed` @ 3667ab368) and the reverted branch tip
(`r8q-modem-nom3` @ 0fd2c260d) — the branch base is the stable commit, so
the delta is the modem work:

- DTS (`sm8250-samsung-r8q.dts` +60): the modem PCIe MSI bank
  (`pcie2_msi: msi-controller@17a10040`, `&pcie2 { msi-parent = <&pcie2_msi>; }`)
  — PCIe2 stays disabled; enabled by the modem overlay.
- MHI host: `boot.c` +36, `init.c` +88, `main.c` +57, `pci_generic.c` +197,
  `pm.c` +139, `common.h` +7, `internal.h` +14 — the modem driver changes
  (event-ring polling, no_m2, wake_db, doorbell, keep-devices, SFR,
  M3 cycle [removed]).
- **No drivers/usb, no drivers/phy changes** — the gadget (dwc3/hsphy)
  drivers are identical between the trees. The DTB diff (out-r8q-uw vs
  out-r8q) is phandle renumbering only (benign). The modules trees are
  identical. The Image hashes differ.

The user observed:
- Stable (`out-r8q-uw/recovery-cam-uw.img`, 66fe127f…): gadget UP — SSH
  to 172.16.42.1 answered; phone-side `ls /sys/class/udc` = `a600000.usb`.
- New image (g634…, `recovery-omamobile.img` b60fde52…): gadget MISSING —
  laptop sees nothing; SSH times out; the phone booted the new image and
  read "no USB device controller found" + "can't find device usb0".
- Reverted (`out-r8q/recovery-omamobile.img`, d87d5999…) flashed by
  heimdall: the user reads "no usb host, crashing in 20s" — the gadget
  still missing.

Ranked candidates (to verify — general subagent investigation running):
1. **The mainline base moved.** The stable tree's mainline = 3667ab368;
   the branch is on the same base... — hmm, actually: `merge-base HEAD
   3667ab368` = 3667ab368 — the branch is a DESCENDANT of the stable
   commit — the mainline is identical, the delta is only the modem work.
   So: not the mainline delta.
2. **The modem DTS (`pcie2_msi`, `msi-parent` on pcie2)** — the PCIe MSI
   bank — hmm, the gadget is USB, not PCIe — but the modem overlay
   enables PCIe2 — how could that affect the USB gadget? Unclear.
3. **The native-vs-SD-root image type.** The new image is the native
   (userdata-root) image; the stable is the SD-root image. The gadget is
   registered by the kernel (dwc3) — the rootfs shouldn't matter... but
   the DTB differs? (diffed: benign). The rootfs trees: the native
   rootfs (`out/r8q/omamobile-rootfs-r8q-2026.11.13-aarch64.img`) vs the
   SD root (`/omarchy-r8q.img` on microSD) — the gadget is in the
   initramfs (the kernel registers the dwc3 UDC) — the rootfs difference
   is about where the root is — the gadget should be the same. Check the
   rootfs diffs for anything gadget-ish (the r8q packages — the USB
   gadget package files — `20-usb-gadget.network`, `10-omamobile-usb.conf`).
4. **The phone state: the user reads "crashing in 20s" on the reverted
   image** — the phone crashes early — the gadget is missing because the
   phone crashes before/at gadget registration — what crashes? The
   modem? The PCIe2/MSI changes? A new commit to test — see the "two
   crashes" rule: if the phone crashes on the reverted image but not on
   the stable, bisect the modem commits (checkout halves, pack, flash).
5. **The "crashing" evidence may be misread** — the user reads the phone's
   multi-screen logs ("I see the linux logs but in multi virtual screens") —
   "no usb host, crashing in 20s so abl falls back to slot a" might be
   about the MODEM (AER — "abl" vs "AER"! — the modem's AER slot reset,
   "falls back to slot a" — on the S20 FE (non-A/B) — hmm — or the phone
   is "staying alive" (r8q non-ab device, staying alive — the watchdog).

Phone-side diagnostics (when SSH is up — it's down now — documented for the
user):

### E1 — Confirm current state (on the phone)

```sh
echo "== UDC (healthy: a600000.usb) =="
ls -l /sys/class/udc/
cat /sys/class/udc/*/state 2>/dev/null

echo "== g1 gadget bind =="
ls -l /sys/kernel/config/usb_gadget/ 2>/dev/null
cat /sys/kernel/config/usb_gadget/g1/UDC 2>/dev/null

echo "== usb0 (healthy: UP, 172.16.42.1/24) =="
ip link show usb0 2>&1
ip -4 addr show dev usb0 2>&1

echo "== dwc3/udc dmesg =="
dmesg | grep -iE 'dwc3|a6f8800|a600000|88e3000|udc|gadget' | tail -40
```

### E2 — Fastest fix

```sh
reboot
```
(Power off + on. The initramfs re-runs the gadget flow early in boot. Re-run E1 after.)

### E3 — Deeper diagnostics (on the phone, if E2 didn't fix it)

```sh
echo "== dwc3/phy/regulator dmesg =="
dmesg | grep -iE 'failed to initialize gadget|failed to add gadget|a6f8800\.usb|a600000\.usb|88e3000|phy_common|failed to enable' | tail -30

echo "== probe state of the dwc3 wrapper + core =="
ls -l /sys/bus/platform/devices/ 2>/dev/null | grep -E 'a6f8800|a600000'
ls /sys/bus/platform/devices/a600000.usb/ 2>/dev/null
cat /sys/bus/platform/devices/a600000.usb/status 2>/dev/null
cat /sys/bus/platform/devices/a600000.usb/power/runtime_status 2>/dev/null
grep -iE 'a600000' /proc/interrupts | head -3

echo "== hsphy regulator supplies =="
ls -l /sys/class/regulator/ 2>/dev/null | grep -E 'vreg_l5a_0p88|vreg_l12a_1p8|vreg_l2a_3p1'

echo "== phy instances =="
ls /sys/class/phy 2>/dev/null
```

### E4 — Hardware escape (on the laptop)

The phone must be in Samsung Download Mode (power off, then hold
Volume Up + Volume Down + cable) — heimdall flashes then:

```sh
heimdall detect                                   # expect: "Device detected"
heimdall flash --RECOVERY ~/omamobile-kernel/out-r8q-uw/recovery-cam-uw.img
```

---

## C. Decision tree + stable escape (E4)

1. **Fastest: restart the phone.** `reboot` — the gadget re-inits early in boot (initramfs flow, section A).
2. **Restart without result → deeper diagnostics** (E3, on the phone).
3. **If the gadget never registers — the USB-C port is dead** — the only way
   back in is the Download Mode + Heimdall re-flash (E4).
4. **Stable escape: flash the stable baseline.**
   `heimdall flash --RECOVERY ~/omamobile-kernel/out-r8q-uw/recovery-cam-uw.img`
   (sha256 66fe127ffd9345e7913aec5eac8a02de0da44bfdbe15987abff1cb61b26d8503)
   — the well-proven image from 2026-11-17 — the "way back" per
   MODEM-PLAN.md §342 ("The old SD-root dual-boot image stays the way
   back...").

---

## D. Automation scripts

- `tools/r8q-gadget-status` — is the gadget (`1d6b:0104`/Omarchy) enumerated
  on the laptop? (usb-lint grep — the gadget's VID is 0x1d6b, not the
  download-mode 04e8.)
- `tools/r8q-gadget-wait [timeout]` — poll for the gadget + probe SSH.
- `tools/r8q-wait-download [timeout]` — poll for the phone in Download Mode
  (heimdall detect / lsusb 04e8:685d), flash the M3-cycle-free image
  (d87d5999…, out-r8q/recovery-omamobile.img), return.
- `tools/r8q-wait-gadget [timeout]` — wait for the gadget + probe SSH.
- `tools/r8q-m3-cycle-revert` — the experiment flow: wait for Download
  Mode → flash the reverted image → wait for gadget + SSH.

---

## E. Investigation status

- **The gadget-delta investigation**: general subagent running
  (session e0940af77ffezlQM11AA7czoav, "R8q gadget-breaking delta").
  Inputs: the out-tree diffs (DTB phandles-only; modules identical; Image
  hashes differ), the tree delta stable→reverted (the modem work + the M3
  cycle revert), the tree delta of the DTS (+60 modem PCIe MSI lines).
  Tasks: (1) diff the trees; (2) research the mainline gadget path;
  (3) ranked candidates; (4) phone-side diagnostic plan.
- **The "main-branch check" (2026-11-20):** the kernel repo has no "main"
  branch; the stable version (3667ab368ef97ce2b3e5d8632bbf1099f3f0eaf0) is
  the tip of the branch **`omamobile-r8q-signed`** (tip 3667ab368).
  The branch `r8q-modem-nom3` (current) is a descendant (base = the stable
  commit). The project repo's `main` (facdbca9) is the original base
  (pre-modem work).
- **The experiment record:** the flow's "reverted image" experiment —
  "flash ok at attempt 1" + "gadget not up within 180s" — INCONCLUSIVE
  (the phone stayed in download mode — the heimdall flash could not
  complete: "Protocol initialisation failed!" ×8 — the USB path was
  flaky). Later: heimdall flashed the reverted image successfully
  ("Rebooting device...") — the phone is on the reverted image — the
  gadget is missing (the user reads the phone's logs).
- **Phone state:** the user reads "r8q non ab device, staying alive,
  and also no usb host, crashing in 20s so abl falls back to slot a"
  (the phone's logs). Gadget missing, SSH to 172.16.42.1 times out.
  (Laptop: no device on the USB bus, heimdall can't detect — the physical
  USB path is currently broken; the user works around with different
  cables/ports.)
- **Stable image available to flash:** `~/omamobile-kernel/out-r8q-uw/
  recovery-cam-uw.img` (sha256 66fe127f…). The phone is "left there"
  (reverted image, gadget missing).
