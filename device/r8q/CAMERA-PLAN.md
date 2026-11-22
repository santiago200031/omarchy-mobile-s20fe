<!-- SPDX-License-Identifier: MIT -->
# Camera plan (r8q / SM8250): the mainline V4L2 stack, telephoto first

- **Status (2026-11-17):** **HI-847 telephoto camera verified live in `omacam` (`7.3.0-rc6-omamobile-push1-g7ca90963c426` on kernel `camera-wip`, `libcamera 0.7.2-4.24`, `omamobile-device-r8q 0.1.24-1`).** `omacam` streams `3256x2448` (`max_fps=30.02`, `exposure_time=(46, 33269)` µs, `gain=(1.0, 16.0)`) upright with both UI and viewfinder orientation confirmed normal by the user.

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
  - Built `7.3.0-rc6-omamobile-push1-gf3be9fb6de08` with explicit `DEVICE=r8q`, `1821` modules in
    `modules-7.3.0-rc6-omamobile-push1-gf3be9fb6de08.tar.zst`, and packed 7-overlay
    `recovery-omamobile-camera.img` (sha256 `4ce468555bf345299a54119685514fbd33a1a07190ca35d591b7abb1ab5ff10b`).

### 6.3 Gated step: install modules, flash `sda24`, reboot, capture raw frame (VERIFIED PASS — 2026-11-16)

- **Tested on `7.3.0-rc6-omamobile-push1-gf3be9fb6de08` (`recovery-omamobile-camera.img`, sha256 `4ce468555bf345299a54119685514fbd33a1a07190ca35d591b7abb1ab5ff10b`):**
  - Build note: `omamobile.conf` defaults `DEVICE=cheetah`, so always invoke `DEVICE=r8q bin/omamobile kernel|modules`; building without `DEVICE=r8q` skipped `device/r8q/kernel.config` (`CONFIG_SCSI_UFS_QCOM=m`) and failed to mount `userdata`.
  - Flashed `RECOVERY` (`sda24`) with `heimdall flash --resume --RECOVERY .../recovery-omamobile-camera.img`, refreshed `/usr/lib/modules/7.3.0-rc6-omamobile-push1-gf3be9fb6de08` (`1821` modules, `depmod -a`), loaded `hi847`, and confirmed live DT `csiphy2_ep` has `clock-lanes = 7`, `data-lanes = 0 1 2 3`, `bus-type = 4`.
  - **Raw capture (`3264x2448`, `pgAA`):** after enabling `media-ctl` links (`"msm_csiphy2":1->"msm_csid0":0[1],"msm_csid0":1->"msm_vfe0_rdi0":0[1]`), `v4l2-ctl --set-fmt-video=width=3264,height=2448,pixelformat=pgAA --stream-mmap=4 --stream-count=2 --stream-to=/tmp/hi847.raw` returned `rc=0` with `19,975,680` bytes (exact 2 packed RAW10 frames; first 1 MiB stats: `226` unique byte values, `min=0`, `max=255`, `mean=21.96`), `vfe0` `2 -> 9` IRQs, `csid0` `0 -> 1` IRQs, zero kernel/I2C errors.
  - **Raw capture (`1632x1224`, `pgAA`):** returned `5,013,504` bytes (`rc=0`).
  - **libcamera probe:** `libcamera v0.7.2` (`SimplePipeline` + `EGL GL_RENDERER: FD650`) discovers `/base/soc@0/geniqup@ac0000/i2c@a88000/camera@21` and drives `vfe0` (`9 -> 42` IRQs) and `csid0` (`1 -> 4` IRQs); `gst-launch-1.0 libcamerasrc ! fakesink` still times out at latency negotiation and `hi847` logs missing `get_selection` / `orientation` / `rotation` props (tracked in §7 item 3).
- **Rollback backup preserved:** `backup/r8q-camera/recovery-before-camera.img` (sha256 `ebac7774…`).

### 6.4 `omacam` viewfinder, SoftISP, timings & orientation (VERIFIED PASS — 2026-11-17)

- **DMA-BUF heap & SoftISP (`omamobile-device-r8q 0.1.22`–`0.1.23`):**
  - `/dev/dma_heap` defaulted to `0600 root:root`, causing `libcamera` `DmaBufAllocator` failures and unsupported format errors in `omacam`; fixed via `70-omamobile-dma-heap.rules` (`GROUP="video", MODE="0660", TAG+="uaccess"`).
  - Adreno `FD650` `debayer_egl` hit GPU `hangcheck` lockups in `SWIspWorker`; fixed for now by defaulting `LIBCAMERA_SOFTISP_MODE=cpu` in `70-r8q-libcamera.conf` (GPU `debayer_egl` stays a follow-up).
- **SLPI orientation (`omamobile-device-r8q 0.1.24`):**
  - Firmware returned an all-zero mount matrix so `iio-sensor-proxy` reported `bottom-up` (inverting `omacam`'s UI buttons). Added `ACCEL_MOUNT_MATRIX="-1, 0, 0; 0, -1, 0; 0, 0, 1"` in `79-r8q-iio-sensor-proxy.rules`; UI confirmed upright.
- **HI-847 sensor helper (`libcamera 0.7.2-4.24`, patch `0058`):**
  - Without a helper, `IPASoft` treated `V4L2_CID_ANALOGUE_GAIN` min code `0` as `0x` gain and pinned exposure at `4` lines (pure black-level read noise). Added `CameraSensorHelperHi847` (`blackLevel_ = 4096`, `AnalogueGainLinear{ 1, 16, 0, 16 }` verified from `com.samsung.sensor.hi847.so`), `hi847.yaml`, and `CameraSensorProperties`.
- **HI-847 timings, crop selection & DT rotation (`wt-r8q` `63cbf400a` + `7ca90963c`):**
  - Scaled `.llp` from `0x033C` (828 72 MHz SCLKs) to MIPI pixel clocks (`3680` at 320 MHz, `1840` at 160 MHz), fixing negative `HBLANK` (`-2436` → `+416` / `+208`) and 30/60 fps timing (`max_fps=30.02`, `exposure_time=(46, 33269)` µs).
  - Added `hi847_get_selection()`, `v4l2_fwnode_device_parse()`, and DT `orientation = <1>; rotation = <270>;` (at `90` preview was upside-down; at `270` confirmed upright in `omacam` on `7.3.0-rc6-omamobile-push1-g7ca90963c426`).

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


### 7.1 Where the IMX555/IMX616 register sequences live (`2026-11-18`)

The vendor kernel `azrim/android_kernel_samsung_sm8250` (branch 16, cloned to
`~/omamobile-kernel/vendor-r8q`) **does not ship IMX555/IMX616 sensor
drivers**. The only match is
`techpack/sec-camera/drivers/cam_sensor_module/cam_sensor/cam_sensor_adaptive_mipi_imx555.h`
(the adaptive-MIPI table). The sensor programming is in the **vendor HAL**
(`com.samsung.sensor.imx555.so` / `imx616.so`, in
`reference-r8q/extracted/vendor_root/lib64/camera`), which is proprietary; the
AGENTS forbids decompiling it. The main/front drivers therefore need **public
register sequences/datasheets** (Sony) or a mainline/postmarketOS driver. The
tele (`hi847`, in-tree) and ultrawide (`s5k3l6xx`) are not blocked because
their drivers already exist.

### 6.5 Regression on `gae195e6deaac`: disabled `port@1` blocks `camss_subdev_notifier_complete()` (2026-11-21)

- **Observed live on `7.3.0-rc6-omamobile-push1-gae195e6deaac`:** `cam -l` reports `No sensor found for /dev/media0`, even though `hi847 10-0021` is bound (entity 329 in `media-ctl -p -d /dev/media0`) and `port@2` (`[102]`) ↔ `camera@21` (`[229]`) phandles match.
- **Root cause:** `/sys/kernel/debug/v4l2-async/pending_async_subdevices` shows `qcom-camss ac6a000.camss` permanently waiting for `/soc@0/geniqup@9c0000/i2c@988000/camera@2d/port/endpoint` (`s5k3l6_out`). Kernel commit `3667ab368` added `port@1 { status = "disabled"; csiphy1_ep: endpoint { remote-endpoint = <&s5k3l6_out>; }; };` in `sm8250-samsung-r8q.dts`, but `fwnode_graph_for_each_endpoint()` in `camss_parse_ports()` (`drivers/media/platform/qcom/camss/camss.c`) does not check `fwnode_device_is_available()` on `port` or the remote device, so `camss->notifier` never completes and `hi847` receives 0 pad links.
- **Next gated kernel step (requires human approval before flashing `sda24`):**
  1. Either move `csiphy1_ep` out of disabled `port@1` into `camera-uw.dtso` or skip unavailable ports/endpoints in `camss_parse_ports()`.
  2. Build kernel + `W=1` + `dt-validate` offline, then flash `sda24` with human approval and verify `cam -l` lists `hi847` and `omacam` streams.
  3. **Stop condition:** if `pending_async_subdevices` still shows `qcom-camss` waiting, inspect `camss_parse_ports()` endpoints; **Rollback:** restore `stable-image.conf` (`adeddb31…660a6`) via `tools/r8q-flash-recovery`.
