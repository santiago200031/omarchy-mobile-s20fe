# Camera plan (r8q / SM8250): the mainline V4L2 stack, telephoto first

- **Status (2026-11-15, evening):** diagnosis done; the kernel side is
  committed, built and staged, and the CAMCC probe has been verified on the
  phone. What is left is one flash of the recovery image and the live camera
  checks (§5). Kernel `omamobile-r8q` worktree `~/omamobile-kernel/wt-r8q`
  (HEAD `bd65a535b`), output `~/omamobile-kernel/out-r8q`, build with
  `DEVICE=r8q`. Release `7.3.0-rc6-omamobile-push1-gbd65a535b90b`.

  Staged artifacts:

  | Artifact | Value |
  | --- | --- |
  | `out-r8q/recovery-omamobile.img` (camera overlay applied) | sha256 `a858c7a85ccdb7cda6978a8c908270a0db0907026674b9546b14b94d6b6f1b19` |
  | `out-r8q/modules-7.3.0-rc6-omamobile-push1-gbd65a535b90b.tar.zst` | installed on the phone under `/usr/lib/modules/` (depmod run) |
  | Kernel commits | `4824e58ad` (hi847 DT support), `bd65a535b` (r8q camera DT) |

  Verified on the phone so far: `modprobe camcc-sm8250` returns 0 and registers
  the whole camera clock tree (111 `cam_cc_*` clocks: `cam_cc_mclk0..5`,
  `cam_cc_csiphy0..5`, the CSID/VFE clocks), the phone stays up, and the
  `blacklist camcc_sm8250` line is gone from
  `/etc/modprobe.d/omamobile-r8q-bringup.conf` (original saved at
  `/root/omamobile-r8q-bringup.conf.pre-camera`). So the blacklist was indeed
  only a bring-up workaround and CAMCC is safe; it was the missing DT nodes,
  not the clock controller, that kept the camera dark.
- **Back:** [PORT-PLAN.md](PORT-PLAN.md) · [NOTES.md](NOTES.md) ·
  [board/missing/04-sensors-cameras-dpms-and-installer.md](board/missing/04-sensors-cameras-dpms-and-installer.md)

## 1. What is actually wrong (corrected)

An earlier reading said "zero camera drivers built, loaded or wired". Three of
those four are wrong, and the difference decides the work. Verified on the
phone over USB SSH on 2026-11-15 (`7.3.0-rc6-omamobile-push1-gd469bd3bf2a9`):

| Claim | Reality |
| --- | --- |
| no camera modules exist | **Wrong.** `/lib/modules/$(uname -r)/kernel/drivers/` already holds `clk/qcom/camcc-sm8250.ko`, `i2c/busses/i2c-qcom-cci.ko`, `media/platform/qcom/camss/qcom-camss.ko`, `media/i2c/s5kgn1.ko` and `s5kjn1.ko`. They are unmodified `arm64 defconfig` modules, built but never used |
| no camera nodes in the DT | **Half right.** `camss@ac6a000`, `cci@ac4f000` and `cci@ac50000` are `status = "disabled"`, there is no image-sensor node anywhere, and `/dev/media*` and `/dev/video*` do not exist. But `clock-controller@ad00000` (CAMCC) has **no** `status` property, i.e. it is enabled and merely blacklisted |
| `blacklist camcc_sm8250` blocks the clock controller | **Right**, and it is the one deliberate block: it was added during bring-up with `icc_bwmon` to quiet boot-time clock/RPMh churn ([06-boot-and-session-hazards.md](board/failed/06-boot-and-session-hazards.md) item 6). It is a boot-stability workaround, not a hardware verdict |
| one sensor driver is enough | **Wrong.** No mainline driver exists for the three sensors that matter most (see §2); the only in-tree register-level driver for this board is the **HI-847 telephoto**, and it was ACPI-only |

So the work is not "enable some config options": it is a sensor driver, a board
device tree, and then the CAMSS media graph.

## 2. The sensors, verified from the vendor image

Taken from the phone's own vendor partition
(`lib64/camera/com.samsung.sensor.*.so` and
`com.samsung.sensormodule.*.bin`, extracted in
`~/omamobile-kernel/reference-r8q/extracted/vendor_root/lib64/camera/`) and
cross-checked against the vendor DT
([board/missing/r8q-camera-upstream-reference.dtsi](board/missing/r8q-camera-upstream-reference.dtsi),
`azrim/android_kernel_samsung_sm8250` branch `16`,
`camera_sensor_r8q_common.dtsi`):

| Role | Sensor | Control bus | CSIPHY | MCLK | 7-bit addr | Mainline driver |
| --- | --- | --- | --- | --- | --- | --- |
| Rear main (cell 0) | **Sony IMX555** 12 MP | CCI0 master 0 | 0 | MCLK0 19.2 MHz | — | none |
| Rear ultrawide (cell 2) | **Samsung S5K3L6** 12 MP | QUP I2C2 (vendor `qupv3_se2`, GPIO115/116) | 1 | MCLK1 19.2 MHz | `0x2d` | none |
| Rear tele (cell 3) | **SK Hynix HI-847** 8 MP | QUP I2C10 (vendor `qupv3_se10`, GPIO129/130) | 2 | MCLK3 19.2 MHz | `0x21` | `hi847.c` (was ACPI-only) |
| Front (cells 1/12) | **Sony IMX616** 32 MP | CCI0 master 1 | 3 | MCLK2 24 MHz | — | none |

Two conversions matter:

- Samsung's downstream DT lists **8-bit** `slave-addr` values (the eeproms at
  `0xA0`/`0xA2` are the same parts at 7-bit `0x50`/`0x51`), so the tele's
  `0x42` is `0x21` and the ultrawide's `0x5A` is `0x2D`.
- The vendor tree puts the ultrawide and telephoto on **QUP I2C**, not on the
  CCI: only the rear main and the front sensor are CCI devices. That is why
  this plan does not need `i2c-qcom-cci` at all for its first camera.

Note also what this tree does **not** contain: `s5kjn1.c` is a 50 MP driver for
other boards, not the S20 FE's ultrawide, and `s5kgn1.c` is the GN1/J1/GM5
family, none of which is on `r8q`. The Pixel 7 Pro camera plan
([device/cheetah/CAMERA-PLAN.md](../cheetah/CAMERA-PLAN.md)) is a different
SoC, ISP and firmware model and does not apply here.

## 3. What the kernel side now has

**`drivers/media/i2c/hi847.c`** — the in-tree HI-847 driver (Intel, ACPI) now
also works as a device-tree sensor on a normal I2C bus: `of_device_id`
`hynix,hi847`, a `vddio` regulator, `reset-gpios`, an `ldo-enable-gpios` for
the board's switched VDDA/VDDD regulator, power-on/off and runtime-PM
callbacks, and the ACPI path kept for Intel machines. New binding:
`Documentation/devicetree/bindings/media/i2c/hynix,hi847.yaml`.

**`arch/arm64/boot/dts/qcom/sm8250-samsung-r8q.dts`** — three additions, all
kept `disabled` so the phone boots exactly as before:

- `vreg_l7a_1p8` (ldo7 of PM8150A): the 1.8 V CAM_VIO rail;
- `&camss`: `vdda-phy-supply`/`vdda-pll-supply` (the existing `vreg_l5a_0p88`
  and `vreg_l9a_1p2` rails) and a labelled CSIPHY2 endpoint, 4 lanes,
  400 MHz link frequency;
- `&i2c10` + `camera@21`: the HI-847 with MCLK3 at 19.2 MHz, its GPIOs, the
  `vddio` rail, and its graph endpoint into CSIPHY2.

**`device/r8q/kernel.config`** — camera block: `MEDIA_SUPPORT`,
`MEDIA_CAMERA_SUPPORT`, `MEDIA_CONTROLLER`, `MEDIA_PLATFORM_SUPPORT`,
`VIDEO_DEV`, `VIDEO_V4L2_SUBDEV_API`, `V4L_PLATFORM_DRIVERS`, `V4L2_FWNODE`,
`V4L2_ASYNC`, `SM_CAMCC_8250`, `VIDEO_QCOM_CAMSS` and `VIDEO_HI847`, all `=m`.
Modules mean a bad probe cannot stop the boot.

**`device/r8q/overlays/camera.dtso`** — enables `camss@ac6a000`, `i2c@a88000`
and `camera@21` on top of the base DTB. Applied at boot by
`tools/r8q-boot --overlay camera`, which is ram-only: no flash, no slot write.

## 4. Why the telephoto is the first camera

HI-847 is the only sensor on this board with a working register-level driver in
the tree (mode tables, exposure/gain/flip controls, the 0x0847 chip ID), so it
is the shortest honest path to a `/dev/video*` that carries real frames. The
other three need new V4L2 drivers written from the vendor register settings;
that is the follow-up, not a prerequisite.

The CAMSS pipeline is the same for whichever sensor comes first:

```
sensor ──MIPI CSI-2──▶ CSIPHY2 ──▶ CSID ──▶ VFE (RDI raw capture)
  │                                              │
  └── I2C10 control                              └── /dev/video*
```

`camcc-sm8250` feeds the CSIPHY clocks and the TITAN/IFE power domains; CAMSS
is what creates the media graph (`/dev/media0`) and the video nodes.

## 5. Runbook (gated; ask the human before every reboot)

**Where we are:** R0 and R2 below are done. R1's artifacts are built and its
modules are already installed on the phone; what remains is the **flash of
`sda24` and one reboot**, which is the human's step (AGENTS.md section 1: never
flash without them), then R3/R4 run live over USB SSH.

**R0 — read-only baseline (done 2026-11-15).** Modules present for the running
release, `camss@ac6a000` and both CCI buses `disabled`, no `/dev/media*` or
`/dev/video*`, `blacklist camcc_sm8250` present. Matched §1.

**R1 — build and stage (done).**

```sh
KERNEL_DIR=~/omamobile-kernel/wt-r8q KERNEL_OUT=~/omamobile-kernel/out-r8q \
  DEVICE=r8q bin/omamobile kernel        # release ...-gbd65a535b90b
KERNEL_OUT=~/omamobile-kernel/out-r8q tools/r8q-boot pack --overlay camera
KERNEL_DIR=~/omamobile-kernel/wt-r8q KERNEL_OUT=~/omamobile-kernel/out-r8q \
  DEVICE=r8q bin/omamobile modules
```

The modules for that release are already unpacked on the phone
(`/usr/lib/modules/7.3.0-rc6-omamobile-push1-gbd65a535b90b`, `depmod` run), and
the camera overlay is compiled into the packed image's DTB (checked with
`fdtoverlay`: `camss@ac6a000`, `i2c@a88000` and `camera@21` all read `okay`).

**R2 — CAMCC alone (done 2026-11-15, live).** `modprobe camcc-sm8250` returned
0, 111 `cam_cc_*` clocks appeared under `/sys/kernel/debug/clk/`, no stall or
RPMh error, and the blacklist line is removed (backup at
`/root/omamobile-r8q-bringup.conf.pre-camera`). Pass condition met, so the
clock controller is not what broke the camera.

**R3 — flash `sda24` with the packed image and boot it (human step).**

```sh
# phone in Download Mode; RECOVERY only (sda24). Never boot/param/super.
tools/r8q-boot verify ~/omamobile-kernel/out-r8q/recovery-omamobile.img   # sha256 a858c7a8...
# then Heimdall/Odin-flash recovery-omamobile.tar.md5's RECOVERY, or
tools/r8q-flash-recovery ~/omamobile-kernel/out-r8q/recovery-omamobile.img
```

Expect on that boot: the desktop comes up as before, and in `dmesg` `camcc`
probes, `qcom-camss` binds, `hi847 0-0021` (or `21-0021`) reports the chip id,
`/dev/media0` exists with `/dev/video*` nodes.
Pass: `uname -r` is `7.3.0-rc6-omamobile-push1-gbd65a535b90b` and `/dev/media0`
exists. Stop: if no `hi847` line appears, the sensor did not bind — check the
I2C address (`0x21`), MCLK3 and GPIO78/135 before changing anything (do not
change two things at once). Rollback: the source tree is untouched and
LineageOS on `sda23` is untouched; restore the previous recovery image
(`tools/r8q-boot restore-recovery`) and boot that.

**R4 — first raw frames (live).**

```sh
modprobe hi847; modprobe qcom-camss
dmesg | grep -iE "hi847|camss|csiphy"
media-ctl -p -d /dev/media0          # hi847 -> msm_csiphy2 -> msm_csid -> msm_vfe
media-ctl -d /dev/media0 --set-v4l2 '"hi847 0-0021":0[fmt:SBGGR10_1X10/3864x2192]'
v4l2-ctl -d /dev/video0 --stream-mmap --stream-count=10 --stream-to=/tmp/f.raw
```

Pass: `/tmp/f.raw` is 3864*2192*2 bytes (about 17 MB) and a raw Bayer viewer
shows an image. That is the camera working on this port. The pipeline's exact
video node number comes from `media-ctl -p`; link the graph step by step
(`media-ctl -l`) if the pipeline handler has not linked it.

## 6. Follow-ups, in order

1. The three remaining sensors (IMX555, S5K3L6, IMX616): port the vendor mode
   tables into `drivers/media/i2c/` drivers of the same shape as `hi847`; the
   rear main also needs its actuator (`qcom,actuator` at 0x18 → a mainline VCM
   driver) and OIS, and its eeprom for calibration.
2. libcamera and PipeWire: CAMSS exposes a media graph, so apps need a
   libcamera pipeline (raw capture first via `cam`/`qcam`), then
   `pipewire-libcamera` so Wayland apps see it through the portal.
3. `i2c-qcom-cci` is only needed for the rear main and the front sensor; wire
   it in the same way when those drivers land.
