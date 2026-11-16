# Camera plan (r8q / SM8250): the mainline V4L2 stack, telephoto first

- **Status (2026-11-16):** **`i2c@a88000` (QUP10) timeout SOLVED (`56ed977c2`);**
  missing `MCLK3` (`GPIO 97`) pinctrl fix built and staged on kernel branch
  `camera-wip` (`b7e537fa4`, `~/omamobile-kernel/out-r8q`, release
  `7.3.0-rc6-omamobile-push1-gb7e537fa47a8`). Pending gated flash/boot test.

  - **Tested live on `56ed977c2` (`7.3.0-rc6-omamobile-push1-g56ed977c2b7f`):**
    switching `vddio-supply` from `vreg_l7a_1p8` to **`vreg_l7c_1p8` (`L7C`)**
    completely eliminated `geni_i2c a88000.i2c: Timeout cancel_m_cmd`.
    `hi847_identify_module()` completed I2C10 transfers in 8 ms and returned `-5`
    (`-EIO`, NACK). Live `/sys/kernel/debug/gpio` + `pinmux-pins` showed
    `GPIO 97` (`MCLK3`) was still `UNCLAIMED` (`func0`, `in low pull down`)
    because `camera@21` lacked `cam_mclk3_default`.
  - **Fixed in `b7e537fa4`:** added `pinctrl-0 = <&cam_mclk3_default &cam_tele_active>`
    (`gpio97` `cam_mclk`, `gpio78`/`gpio135` `bias-disable`) on `hi847: camera@21`.

  Staged (not flashed yet):
  `~/omamobile-kernel/out-r8q/recovery-omamobile-camera.img` (`display`,
  `remoteprocs`, `audio-amps`, `audio-sound`, `slpi-sensors`, `vol-up`,
  `camera`; sha256 `02b11e65…`) and
  `~/omamobile-kernel/out-r8q/modules-7.3.0-rc6-omamobile-push1-gb7e537fa47a8.tar.zst`
  (1821 modules).

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

## 6. The QUP10 I2C timeout: what was tested and what is left

Every transfer on `i2c@a88000` (QUP10, vendor `qupv3_se10`, GPIO129/130) ends
in `geni_i2c a88000.i2c: Timeout cancel_m_cmd`, so `hi847` cannot read its chip
ID (`failed to find sensor: -5`). Ruled out, each verified on the phone:

| Test | Result |
| --- | --- |
| Pin mux | `a88000.i2c` claims GPIO129/130 with function `qup10`, exactly the stock DT's `qupv3_se10_i2c_active` (drive 2, `bias-disable`) |
| CAM_VIO rail | **Fixed in `56ed977c2`:** `0a1227989` enabled `vreg_l7a_1p8` (`L7A`), whereas stock `live.dts` (`rpmh-regulator-ldoc7` → `pm8150a_l7: L7C`, phandle `0x2d1`) is **`vreg_l7c_1p8` (`L7C`)** under `regulators-1` |
| MCLK3 | `cam_cc_mclk3_clk` enable_count 1, 19.2 MHz (after the `TITAN_TOP_GDSC` power-domain fix) |
| Reset / LDO-enable polarity | all four combinations, via module parameters: identical `-EIO` |
| I2C address | `i2cdetect` on the powered bus: no ACK at any address; the stock DT's `slave-addr = 0x42` is the 8-bit form of 0x21 |
| Interrupt | the SE IRQ does fire (409 → 411 per probe), so it is not a missing/routed-wrong IRQ; the stock DT's SPI 355 is what mainline uses |
| GPI EE offset | `0x6000` for `0xa00000` (`gpi_dma1`, `00cfa5c21`) matches stock `live.dts` (`qcom,gpi-ee-offset = <0x6000>`) |
| Bench bus + QUP2 control | **Verified working (2026-11-16):** `&i2c11` (`a8c000.i2c`, `qupv3_se11`, dual `cs35l41` amps) on the **same `gpi_dma1` (`0xa00000`)** probes cleanly, ruling out QUP2/GPI |

Next gated test step (once human approves flashing `sda24` + reboot):

1. Push `~/omamobile-kernel/out-r8q/modules-7.3.0-rc6-omamobile-push1-gb7e537fa47a8.tar.zst`
   and flash `~/omamobile-kernel/out-r8q/recovery-omamobile-camera.img` (sha256
   `02b11e65…`) to `sda24` (`tools/r8q-flash-recovery`).
2. Check if `hi847 10-0021` probes cleanly with `cam_mclk3_default` +
   `vreg_l7c_1p8`, and capture raw frames via `media-ctl` + `v4l2-ctl`.

Note: **kexec cannot change the DTB on this phone** (`--dtb` ignored / panics),
and `tools/r8q-flash-recovery` clears both LUN and partition `ro` flags before
writing `sda24`. Byte-exact pre-camera backup of `sda24` is in
`backup/r8q-camera/recovery-before-camera.img` (sha256 `ebac7774…`).

### 6.2 HI-847 probe and CSIPHY2 0-IRQ fix (2026-11-16)

- **Tested on `7.3.0-rc6-omamobile-push1-gb7e537fa47a8`:**
  - `hi847 10-0021` probed (`0x0847`), `/dev/media0` topology linked
    `hi847 10-0021:0 -> msm_csiphy2 -> msm_csid0 -> msm_vfe0_rdi0 -> /dev/video0`.
  - `VIDIOC_STREAMON` at `3264x2448` (`pgAA`), with `test_pattern=1`, and at
    `1632x1224` (`pgAA`) incremented `vfe0` (`3 -> 8` IRQs) and `csid0` (`2 -> 5` IRQs),
    but `msm_csiphy2` stayed at `0` IRQs and timed out with `0`-byte raw files.
- **Root cause:** `&camss` `port@2` (`csiphy2_ep`) used 1-based
  `data-lanes = <1 2 3 4>;` and omitted `clock-lanes = <7>;` and
  `bus-type = <MEDIA_BUS_TYPE_CSI2_DPHY>;`. `camss-csiphy-3ph-1-0.c` computes
  `BIT(c->data[i].pos * 2)` and lane offsets `c->data[i].pos * 2` on SM8250, so
  `<1 2 3 4>` skipped physical lane 0 and overflowed 8-bit `CTRL5`.
- **Fix staged (offline build, kernel `camera-wip` @ `f3be9fb6d`):**
  - Updated `csiphy2_ep` to `clock-lanes = <7>;`, `data-lanes = <0 1 2 3>;`,
    `bus-type = <MEDIA_BUS_TYPE_CSI2_DPHY>;` (keeping `data-lanes = <1 2 3 4>;` and
    `link-frequencies = /bits/ 64 <400000000>;` on `hi847_out`).
  - Built `7.3.0-rc6-omamobile-push1-gf3be9fb6de08`, `1827` modules in
    `modules-7.3.0-rc6-omamobile-push1-gf3be9fb6de08.tar.zst`, and packed 7-overlay
    `recovery-omamobile-camera.img` (sha256 `64f7aad8b42c0b7bb751e2ae1268b13dc59f637be26f4fdddbabf343dc3577cc`).

### 6.3 Gated step: install modules, flash `sda24`, reboot, capture raw frame

- **Risk:** Writes `/usr/lib/modules/7.3.0-rc6-omamobile-push1-gf3be9fb6de08` on `userdata`,
  writes `recovery` (`sda24`), and reboots into Omarchy. Require human go-ahead (`AGENTS.md` §1).
- **Commands (once approved):**
  1. Stream & unpack `modules-7.3.0-rc6-omamobile-push1-gf3be9fb6de08.tar.zst` to
     `/usr/lib/modules/` and run `depmod -a 7.3.0-rc6-omamobile-push1-gf3be9fb6de08`.
  2. Write `recovery-omamobile-camera.img` to `/dev/sda24`, read back sha256 (`64f7aad8…`),
     then `reboot-to recovery` (keeping LineageOS `boot` untouched).
  3. Over fresh SSH: set `media-ctl` `3264x2448` / `SGRBG10_1X10`, capture 2 frames with
     `timeout 5 v4l2-ctl -d /dev/video0 --set-fmt-video=width=3264,height=2448,pixelformat=pgAA --stream-mmap=4 --stream-count=2 --stream-to=/tmp/hi847.raw`,
     and verify `/proc/interrupts` (`csiphy2`, `csid0`, `vfe0`).
- **Pass:** Non-empty `/tmp/hi847.raw` (expected `~19.9 MB` for 2 packed `pgAA` `3264x2448`
  frames) and increasing `csiphy2`/`csid0`/`vfe0` IRQs without oops or I2C timeouts.
- **Stop / Rollback:** If USB fails to return, button-boot LineageOS (`boot` untouched) and
  restore `backup/r8q-camera/recovery-before-camera.img` (sha256 `ebac7774…`) to `sda24`.

## 7. Follow-ups for the other three sensors

1. **Sony IMX555** (rear main, CCI0 master 0, CSIPHY0, MCLK0 19.2 MHz) and
   **Sony IMX616** (front, CCI0 master 1, CSIPHY3, MCLK2 24 MHz): no mainline
   driver; write one each, with the actuator (`qcom,actuator` 0x18) and OIS for
   the main camera.
2. **Samsung S5K3L6** (ultrawide, QUP I2C2, 7-bit 0x2d, CSIPHY1, MCLK1): no
   driver either, and it is on the same kind of QUP I2C bus, so the QUP10
   finding above applies to it too.
3. libcamera and PipeWire once a sensor streams: the media graph is CAMSS's,
   so apps need a libcamera pipeline (`cam`, `qcam`) and then
   `pipewire-libcamera`.
