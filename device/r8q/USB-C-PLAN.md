<!-- SPDX-License-Identifier: MIT -->

# USB-C and charging plan: r8q (Galaxy S20 FE, SM8250Q)

Status (2026-11-22): **`[usb-03]` PASSED; `[usb-04]` controller role-switch PASSED (`xhci-hcd` host/device), extcon driver written and committed, 5 V VBUS boost still blocked (CNFG_00 on charger I2C 0x69).**
The gadget runs in fixed peripheral mode (HS-only, USB 2.0), the charger
reports `online=1, status=Full`. `tools/usbc-deadman` refuses r8q because
its I2C bus detection, TCPC address and PHY parameter are hardcoded for
cheetah (Exynos gs201). `tools/usbc-deadman-r8q` + `tools/usbc-deadman-r8q-phone`
now adapt the dead-man safety net for r8q and passed live `[usb-03]` drills
(`noop`, `noop --no-confirm`, `udc-rebind`, `replug`), and `[usb-04]`'s
`device/r8q/overlays/usbc-otg-test.dtso` passed DWC3 host/device role switching;
bus-powered USB peripherals (hub/mouse) do not receive 5 V VBUS yet because
the MAX77705 charger's CNFG_00 (0xB7, OTG_EN/BOOST_EN) lives on I2C slave
0x69 and requires coordination with the USBC block at 0x25.

**Kernel on phone:** 7.3.0-rc6-omamobile-push1-g4d235a595f28 (HEAD commit
4d235a595f28 in `~/omamobile-kernel/wt-r8q` on branch `r8q-base-dts-fold`).

**Branch `r8q-usbc-max77705`:** 3 commits (dt-bindings → driver → DTS),
builds with 0 warnings, 0 checkpatch errors. DT wiring verified: `usb-role-switch`
on dwc3 in base DTS, extcon + `dr_mode = "otg"` in overlay.

**Kernel on phone:** 7.3.0-rc6-omamobile-push1-g4d235a595f28 (HEAD commit
4d235a595f28 in `~/omamobile-kernel/wt-r8q` on branch `r8q-base-dts-fold`).

**Boot method:** `r8q` is single-slot non-A/B (`BOOT_METHOD=ram-only`, RECOVERY
at `/dev/sda24` is the power-on default — plain power-on/reboot boots
Omarchy). Because software recovery avoids losing a test boot state,
`usbc-deadman-r8q` never reboots unconditionally unless `--reboot-on-fail`
is explicitly passed.

Chapters:
- [1. Current mainline r8q USB-C state](#1-current-mainline-r8q-usb-c-state)
- [2. Hardware & DT delta: stock vs mainline](#2-hardware--dt-delta-stock-vs-mainline)
- [3. [usb-03] Dead-man safety net adaptation](#3-usb-03-dead-man-safety-net-adaptation)
- [4. [usb-04] OTG / dual-role switching](#4-usb-04-otg--dual-role-switching)
- [5. Risks](#5-risks)

---

## 1. Current mainline r8q USB-C state

### 1.1 dwc3 gadget: working, fixed peripheral

| Node | Address | Compatible | Mode | Speed |
|---|---|---|---|---|
| `usb_1` (wrapper) | `0xa6f8800` | `qcom,sm8250-dwc3` | — | — |
| `usb_1_dwc3` (core) | `0xa600000` | `snps,dwc3` | `peripheral` | `high-speed` |
| `usb_1_hsphy` | `0x88e3000` | `qcom,usb-snps-femto-v2` | USB device | HS |

The DT node path is `/soc@0/usb@a6f8800/usb@a600000` (the wrapper is
`usb@a6f8800`, the core is `usb@a600000`).

The samsung-common overlay (`sm8250-samsung-common.dtsi`) sets:

```dts
&usb_1_dwc3 {
    dr_mode = "peripheral";
    maximum-speed = "high-speed";
    phys = <&usb_1_hsphy>;
    phy-names = "usb2-phy";
};
```

No `usb-role-switch` property. The SS PHY is removed from the `phys` list —
only HS (USB 2.0) is available.

The gadget registers as `/sys/class/udc/a600000.usb` and the initramfs
successfully binds the configfs NCM+ACM gadget (verified on stable builds:
`172.16.42.1` SSH over USB).

**Phone state (read-only, 2026-11-22):**

```
/sys/class/udc/a600000.usb/state  → configured
/sys/kernel/config/usb_gadget/g1/UDC → a600000.usb
/sys/kernel/debug/usb/a600000.usb/mode → device (dwc3_mode_write ignores writes unless dr_mode == otg)
/sys/class/extcon → (empty)
/sys/class/typec → (empty)
/sys/class/usb_role → (empty)
```

### 1.2 max77705 MFD: charger only, no MUIC/TCPC

Mainline `drivers/mfd/max77705.c` instantiates only three sub-devices:

```c
static struct mfd_cell max77705_devs[] = {
    MFD_CELL_OF("max77705-rgb", …),
    MFD_CELL_OF("max77705-charger", …),
    MFD_CELL_OF("max77705-haptic", …),
};
```

No MUIC (USB orientation / cable type detector), no CCIC (USB Type-C
connector / TCPC), and no extcon provider cell.

**I2C bus 0 (980000.i2c under 9c0000.geniqup):**

| Addr | Device | Notes |
|---|---|---|
| `0-0036` | `maxim,max77705-battery` | `power_supply/max170xx_battery` |
| `0-0066` | `maxim,max77705` | PMIC MFD: haptic + rgb; `max77705.c` does NOT register USBC/MUIC yet |
| `0-0069` | `maxim,max77705-charger` | `power_supply/max77705-charger`, register base 0xB0 |
| `0-0025` | (unbound) | MAX77705 internal USBC/CCIC block; responds to `i2cdetect` on i2c-0 |

**Charger registers (0x69, base 0xB0):**

| Reg | Name | Bits |
|---|---|---|
| `CNFG_00` (0xB7) | `bit0=CHG_EN, bit1=OTG_EN, bit2=BUCK_EN, bit3=BOOST_EN, bit4=WDTEN` |
| `MAX77705_OTG_CTRL` (0x0A) | OTG control register |

**Phone state (read-only, 2026-11-22):**

```
/sys/class/power_supply/max77705-charger/online → 1
/sys/class/power_supply/max77705-charger/status → Full
/sys/class/power_supply/max77705-charger/input_current_limit → 500000
```

### 1.3 Watchdog state on r8q

| Device | Compatible | Owner | Notes |
|---|---|---|---|
| `/dev/watchdog0` | `17c10000.watchdog` (qcom-wdt) | PID 1 (systemd), `RuntimeWatchdogSec=30s` via `10-r8q-watchdog.conf` | **ALREADY held busy** — never touch |
| `/dev/watchdog1` | Software Watchdog (softdog) | Free | Timeout 100s, `SETTIMEOUT=0`, `MAGICCLOSE=0` |

### 1.4 Why `tools/usbc-deadman` refuses r8q

`tools/usbc-deadman-phone` hardcodes the cheetah (Exynos gs201) platform:

| Hardcoded value | cheetah (gs201) | r8q (SM8250) |
|---|---|---|
| I2C bus search | `10d60000.i2c` (Exynos geni) | `i2c@980000` (Geni-I2C, bus `i2c-0`) |
| Charger address | `0x69` (same) | `0x69` (same) |
| TCPC / MUIC address | `0x25` (max77759) | **integrated into max77705, unbound** |
| PHY parameter | `phy_exynos5_usbdrd/force_vbus_valid` | **does not exist** (QCOM PHY) |
| Watchdog | `s3c2410` at `/dev/watchdog0` | **qcom-wdt, already held by systemd** |

The `i2c_bus()` function scans `/sys/bus/i2c/devices/*/name` and
`readlink -f` for `10d60000.i2c`. r8q's bus reports `Geni-I2C` and is
under `i2c-0`, so the function returns failure and the dead-man refuses
with `"no i2c bus for 10d60000.i2c"`.

Even if the I2C bus were found, the `rescue()` function writes to TCPC
address `0x25` register `0x93` (USBSW_CTRL) — this register does not
exist on max77705. The PHY `force_vbus_valid` parameter does not exist on
SM8250.

### 1.5 Packaging & boot workflow for r8q

r8q is a **single-slot, non-A/B** device. The standard `bin/omamobile push`
and `bin/omamobile flash` (which target A/B slots) do not apply.

**Image packing:** `tools/r8q-boot pack --overlay <NAME> ...` builds a
header-v2 Android recovery image with optional DT overlays from
`device/r8q/overlays/*.dtso`.

**Flashing:** `tools/r8q-flash-recovery [IMAGE]` writes a packed header-v2 image
directly to `RECOVERY` (`/dev/sda24`) over the live USB SSH link (handling the
storage guard cleanly and verifying sha256; `--stable` restores the image from
`device/r8q/stable-image.conf`), and Heimdall in Samsung Download Mode (`04e8:685d`,
from power-off hold Volume Up + Volume Down + plug USB) provides cold recovery.

**Testing overlays:** `tools/r8q-boot` (verify / info) or
`tools/r8q-flash-recovery` for live testing.

---

## 2. Hardware & DT delta: stock vs mainline

### 2.1 Stock SM8250 r8q (from `reference-r8q/live.dts`)

**max77705 at `i2c@980000` (0x66):**

```dts
max77705@66 {
    compatible = "maxim,max77705";
    reg = <0x66>;
    interrupt-parent = <&pm8150l_gpios>;
    interrupts = <0x04 0xca 0x00 0x08>;
    interrupt-names = "max77705_irq";
    interrupt-controller;
    max77705,irq-gpio = <0x5ed 0x0b 0x01>;
    max77705,wakeup;
    support_pd_role_swap;
    muic-universal,irq-gpio = <0x5ed 0x0b 0x01>;
    muic-universal,chip_name = "maxim,max77705";
    muic,undefined_range;
    max77705,detect-alalog-audio;
    max77705,bc1p2_retry_count = <0x02>;
};
```

**Stock MUIC/CCIC log snippet (from `reference-r8q/dmesg.txt`):**

```
USBC1:0x27, USBC2:0x05, BC:0x81, UICINTM:0x0, attached_dev:1
```

Notice these hex numbers (`0x27`, `0x05`, `0x81`) are **runtime register values**
logged by Samsung's `muic-max77705` driver (`attached_dev:1` = USB device attached),
not register addresses. Note also that in `reference-r8q/live.dts`:
- `pm8150l qcom,qpnp-smb5` has `status = "disabled";` (`smb5-vbus`, `smb5-vconn`,
  and `qcom,typec@1500` are **not used** on `r8q` — Samsung delegates Type-C,
  BC1.2, and charger/OTG VBUS boost to the MAX77705 PMIC/USBC/charger IC on `i2c-0`).
- `halo,hl6111r@25` in `live.dts` is on a separate I2C bus alongside `sec-nfc@27`,
  whereas on `i2c-0` (`980000.i2c`) address `0x25` responds as MAX77705's own
  internal USBC/CCIC I2C slave.

**MUIC support-list (from `live.dts`):**

```
+OTG:GND, -MHL:1K, -VZW Accessory:28.7K, -VZW Incompatible:34K,
+RDU TA:40.2K, +HMT:49.9K, -Audiodock:64.9K, -USB LANHUB:80.07K,
-Charging Cable:102K, +Game Pad:121K, +Jig USB Off:255K, +Jig USB On:301K,
+Deskdock:365K, -TYPE2 Charger:442K, +Jig UART Off:523K, +Jig UART On:619K,
+TA:OPEN, +USB:OPEN, +CDP:OPEN, +Undefined Charging:XXX
```

This is the Samsung `muic-universal` ADC table: resistance values on the
MUIC's ADC input identify cable type (OTG, USB host, charger, audio dock,
etc.).

### 2.2 Mainline r8q (current `sm8250-samsung-r8q.dts`)

```dts
&i2c0 {
    max77705: pmic@66 {
        compatible = "maxim,max77705";
        reg = <0x66>;
        interrupt-parent = <&pm8150l_gpios>;
        interrupts = <11 IRQ_TYPE_LEVEL_LOW>;
        #interrupt-cells = <1>;
        interrupt-controller;
        pinctrl-names = "default";
        pinctrl-0 = <&chg_int_default>;

        haptic {
            compatible = "maxim,max77705-haptic";
            haptic-supply = <&vreg_l10c_3p2>;
            pwms = <&vib_pwm 0 50000>;
            status = "okay";
        };
    };

    max77705_charger: charger@69 {
        compatible = "maxim,max77705-charger";
        reg = <0x69>;
        monitored-battery = <&battery>;
        interrupt-parent = <&max77705>;
        interrupts = <0>;
    };

    fuel-gauge@36 {
        compatible = "maxim,max77705-battery";
        reg = <0x36>;
        power-supplies = <&max77705_charger>;
        maxim,rsns-microohm = <5000>;
        interrupt-parent = <&max77705>;
        interrupts = <2>;
        status = "okay";
    };
};
```

**Delta from stock:**

| Feature | Stock | Mainline | Missing |
|---|---|---|---|
| max77705 MFD cells | MUIC + charger + haptic + rgb | charger + haptic + rgb | **MUIC** |
| `muic-universal,irq-gpio` | present | absent | **MUIC IRQ → no cable detection** |
| `muic-universal,chip_name` | `"maxim,max77705"` | absent | **no MUIC sub-device** |
| `muic,undefined_range` | ADC table present | absent | **no cable type detection** |
| `support_pd_role_swap` | present | absent | **no PD role swap** |
| `max77705,detect-alalog-audio` | present | absent | **no audio jack detection** |
| `max77705,bc1p2_retry_count` | `<0x02>` | absent | **no BC1.2 retry** |
| `PM8150L qcom,qpnp-smb5` | disabled (`status = "disabled"`) | absent | unused on `r8q` (MAX77705 handles Type-C/VBUS) |
| `dwc3 dr_mode` | — | `"peripheral"` | **no OTG / DRD** |
| `dwc3 usb-role-switch` | absent | absent | **no role switch** |
| `dwc3 extcon` | absent | absent | **no extcon provider** |

### 2.3 Mainline driver landscape for max77705

**`drivers/mfd/max77705.c`:** Only creates `rgb`, `charger`, and `haptic`
cells. No MUIC cell. The MUIC functionality (CC orientation, cable type,
BC1.2 detection, audio jack) is entirely missing from mainline.

**`drivers/power/supply/max77705_charger.c`:** Works as-is. Reports
`online`, `status`, `input_current_limit`, `constant_charge_voltage`,
`constant_charge_current`. No USB-C role awareness (no extcon, no typec).

**`drivers/extcon/extcon-max77843.c`:** This is the **MAX77843** extcon
driver (a different Maxim chip, also a MUIC). It supports:
- `EXTCON_USB`, `EXTCON_USB_HOST`, `EXTCON_JIG`, `EXTCON_CHG_USB_*`
- ADC-based cable type detection (same approach as Samsung's MUIC)
- OTG VBUS control via `max77843_charger_set_otg_vbus()`
- USB path switching via `max77843_muic_set_path()`

**Key insight:** The MAX77843 extcon driver is a **reference for how a
MAX77705 MUIC driver should work**. The register layouts differ, but the
architecture (ADC-based cable detection → extcon state → OTG VBUS control)
is the same. The MAX77705 MUIC registers are documented in the Maxim
datasheet and used by the stock kernel driver.

**`drivers/usb/dwc3/dwc3-qcom.c`:** Supports `usb-role-switch` and
`dr_mode = "otg"`. The `pre_set_role` notifier
(`dwc3_qcom_run_stop_notifier`) re-enables QSCRATCH VBUS_VALID when the
UDC is rebound (write empty string then `a600000.usb` to `g1/UDC`).

**`drivers/usb/dwc3/debugfs.c`:** `dwc3_mode_write` silently ignores
writes to `/sys/kernel/debug/usb/a600000.usb/mode` unless `dr_mode == otg`.

**`drivers/usb/dwc3/drd.c`:** The DRD (dual-role) subsystem:
- If `usb-role-switch` is present: registers a `usb_role_switch` with
  `dwc3_usb_role_switch_set/get` and creates platform devices from the
  connector DT node.
- If `extcon` is present: registers an extcon notifier.
- If neither: uses the OTG block with an ID pin IRQ.

**`drivers/phy/samsung/phy-exynos5-usbdrd.c`:** Samsung USBDRD PHY with
orientation-switch support. **Not applicable to r8q** — SM8250 uses
QCOM's `phy-qcom-usb-snps-femto-v2` and `phy-qcom-usb-qmp`.

---

## 3. [usb-03] Dead-man safety net adaptation

### 3.1 Problem statement

`tools/usbc-deadman` and `tools/usbc-deadman-phone` are hardcoded for
cheetah (Exynos gs201):
- I2C bus detection: `10d60000.i2c` → r8q needs `i2c@980000` (Geni-I2C,
  bus number `i2c-0`).
- TCPC address: `0x25` (max77759 separate TCPC) → r8q has no separate
  TCPC; the CC/MUIC functionality is integrated into the max77705 itself.
- PHY parameter: `phy_exynos5_usbdrd/force_vbus_valid` → does not exist
  on SM8250 (QCOM PHYs don't have this parameter).
- Watchdog: `s3c2410` → r8q uses qcom-wdt (`/dev/watchdog0` already held
  by systemd PID 1; `/dev/watchdog1` is softdog, free to use).

### 3.2 r8q-specific USB-C failure mode

On cheetah, the MAX77759 TCPC opens its USB2 data switch (reg `0x93`) on
unplug and Linux never closes it. On r8q, the failure mode is different:

**There is no separate TCPC that controls a data switch.** The max77705's
MUIC handles CC detection, but the USB2 data path is controlled by the
dwc3 controller itself (the QCOM DWC3 glue handles VBUS override). On
r8q, the primary risk is not a stuck data switch but **the USB gadget
disappearing after a cable unplug** because:

1. The dwc3 in peripheral mode does not receive a detach event without an
   extcon provider or orientation-switch.
2. The PHY (QCOM femto) does not have a `force_vbus_valid` parameter to
   keep VBUS valid after detach.
3. Without Type-C awareness, the gadget may not re-enumerate after replug.

### 3.3 Engineering plan: r8q dead-man tool

**Option A: Fork a dedicated r8q deadman helper** (recommended for clarity)

Create `tools/usbc-deadman-r8q` (laptop) and `tools/usbc-deadman-r8q-phone`
(phone), mirroring the cheetah tool's structure but adapted for r8q:

**Phone half (`usbc-deadman-r8q-phone`):**

```bash
# I2C bus detection: look for "Geni-I2C" or i2c@980000
i2c_bus() {
    local d
    for d in /sys/bus/i2c/devices/i2c-*; do
        if grep -qs "Geni-I2C" "$d/name" || \
           readlink -f "$d" | grep -q "i2c@980000"; then
            echo "${d##*-}"; return 0
        fi
    done
    return 1
}
```

**Key differences from cheetah:**

| Feature | Cheetah | r8q |
|---|---|---|
| I2C bus | `10d60000.i2c` → `i2c-0` | `Geni-I2C` → `i2c-0` |
| Charger addr | `0x69` | `0x69` (same) |
| TCPC addr | `0x25` (max77759) | `0x25` on `i2c-0` (MAX77705 USBC slave, unbound) |
| USBSW_CTRL | `0x25/0x93` | **N/A** (no max77759 switch register) |
| PHY param | `phy_exynos5_usbdrd` | **N/A** (QCOM PHY) |
| Rescue | Close USBSW_CTRL | Re-enumerate gadget (`g1/UDC` unbind/rebind) |
| Watchdog | `s3c2410` at `/dev/watchdog0` | qcom-wdt (`/dev/watchdog0` held by systemd); use `/dev/watchdog1` (softdog) only with `--reboot-on-fail` |

**Option B: Make the cheetah tool multi-platform**

Extend `usbc-deadman-phone` with a `--platform r8q` flag that changes the
I2C bus search, removes the TCPC rescue, and uses the QCOM WDT. This
keeps a single codebase but adds complexity to the cheetah tool.

**Recommendation: Option A** — a dedicated r8q tool is cleaner, easier to
test independently, and doesn't risk breaking the cheetah tool.

### 3.4 Gated runbook: [usb-03]

**Prerequisites:**
- Battery ≥ 50 %, phone at room temperature.
- Wi-Fi SSH available as a second connection (in case USB drops).
- `tools/phone-ssh 'pacman -S --needed --noconfirm i2c-tools'` (if not
  already installed).

**Step U3.0 — Read-only: verify I2C bus and charger registers**

```sh
# On the laptop:
tools/phone-ssh --fresh '
    echo "=== I2C bus ==="
    for d in /sys/bus/i2c/devices/i2c-*; do
        echo "$d: $(cat $d/name 2>/dev/null) ← $(readlink -f $d 2>/dev/null)"
    done
    echo "=== Charger ==="
    for r in 0xb0 0xb1 0xb4 0xb5 0xb6 0xb9 0xbb 0xbd 0xc2; do
        echo "chg 0x$(printf %02x $r)=$(i2cget -f -y 0 0x69 $r 2>/dev/null || echo ERR)"
    done
    echo "=== UDC ==="
    cat /sys/class/udc/a600000.usb/state 2>/dev/null
    echo "=== lsmod ==="
    lsmod | grep -E "max77705|dwc3"
'
```

**Expected:** I2C bus `i2c-0` with name `Geni-I2C`. Charger registers
readable. UDC state `configured` or `attached`.

**Step U3.1 — Install the r8q dead-man tool**

```sh
tools/usbc-deadman-r8q install   # copies phone half to /usr/local/bin/
```

**Step U3.2 — Dead-man drill: noop (verify the framework works)**

```sh
tools/usbc-deadman-r8q noop
# Expected: CONFIRMED within ~10s, exit 0

tools/usbc-deadman-r8q noop --no-confirm
# Expected: UNDONE after ~100s, link stays up, exit 0
```

**Pass criteria:** Both results as stated. No reset. If the phone resets
during the second run, the confirm path or watchdog is broken.

**Step U3.3 — Live unplugging test (the real test)**

```sh
# This tests: unplug cable → gadget drops → replug → gadget recovers
tools/usbc-deadman-r8q replug
# The human unplugs the cable, waits 5 s, replugs it
```

**Expected pass:**
- The link drops when unplugged (SSH to 172.16.42.1 fails).
- After replug: SSH to 172.16.42.1 comes back within 10 s.
- UDC state returns to `configured`.
- dmesg shows the gadget reconnecting (no errors).

**If the gadget does not recover after replug:**
1. Check dmesg for dwc3 errors.
2. Try manual re-enumeration:
   ```sh
   tools/phone-ssh 'echo "" > /sys/kernel/config/usb_gadget/g1/UDC;
     sleep 1; echo a600000.usb > /sys/kernel/config/usb_gadget/g1/UDC'
   ```
3. If that works, the dead-man's rescue should do the same automatically.

**Rollback:** The dead-man's undo path is `noop` (no-op for this step).
If the gadget is stuck, a reboot (`reboot-to omamobile`) restores it.

### 3.5 Watchdog handling on r8q

`/dev/watchdog0` (qcom-wdt, `17c10000.watchdog`) is **already held busy**
by PID 1 systemd (`RuntimeWatchdogSec=30s` via `10-r8q-watchdog.conf`).
The r8q dead-man tool must **never touch `/dev/watchdog0`**.

`/dev/watchdog1` is the Software Watchdog (`softdog`, timeout 100s,
`SETTIMEOUT=0`, `MAGICCLOSE=0`). It may be used as the dead-man feeder
**only when `--reboot-on-fail` is explicitly requested**.

Default recovery (no `--reboot-on-fail`): undo via UDC rebind and role
restore. Note that on `r8q`, `/dev/sda24` (RECOVERY) is the power-on default.

### 3.6 Results (`[usb-03]`, 2026-11-22, `7.3.0-rc6-omamobile-push1-g4d235a595f28`)

| Step | Result | Observed |
|---|---|---|
| `tools/usbc-deadman-r8q install && dump` | **PASS** | `i2c-0`, `0x69` `CNFG_00=0x05`, `0x25` responds on `i2c-0`, `g1/UDC=a600000.usb` (`configured`) |
| `tools/usbc-deadman-r8q noop --confirm 15 --grace 5` | **PASS** (`exit 0`) | Confirmed over fresh SSH in `5 s` |
| `tools/usbc-deadman-r8q noop --confirm 5 --grace 5 --no-confirm` | **PASS** (`exit 0`) | Timed out `5 s` confirm, reverted cleanly in grace (`8 s`), zero unexpected reset |
| `tools/usbc-deadman-r8q udc-rebind --confirm 30 --grace 20` | **PASS** (`exit 0`) | Unbound & rebound `a600000.usb`; fresh SSH confirmed in `5 s` |
| `tools/usbc-deadman-r8q replug --confirm 120 --grace 30` | **PASS** (`exit 0`) | Physical unplug (~5 s) dropped USB link; replug automatically re-enumerated gadget and fresh SSH confirmed at `27 s` (`state=configured`, `CNFG_00=0x05`) |

---

## 4. [usb-04] OTG / dual-role switching

### 4.1 Problem statement

r8q's dwc3 is fixed to `dr_mode = "peripheral"`. There is no USB-C
connector node, no extcon provider, no role switch, and no VBUS boost
wired for host mode. To support OTG (host mode for external USB peripherals):

1. The dwc3 must support dual-role (`dr_mode = "otg"` and `usb-role-switch`).
2. Long-term, `max77705` needs a MUIC/USBC/extcon driver for cable-type detection
   and automatic OTG VBUS boost.
3. For manual bring-up (`Path B`), VBUS can be sourced either by a powered USB hub/Y-cable
   or carefully via MAX77705 `CNFG_00` (`0xB7`) bit1 (`MAX77705_OTG_EN`) / bit3 (`BOOST_EN`).

### 4.2 Two paths to OTG

#### Path A: Full USB-C with extcon (long-term, requires driver work)

This is the proper solution: add a MUIC cell to the max77705 MFD driver,
create an extcon provider, and wire it to the dwc3 via `extcon` property
or `usb-role-switch`.

**Kernel changes needed:**

1. **`drivers/mfd/max77705.c`:** Add a MUIC sub-device cell:
   ```c
   MFD_CELL_OF("max77705-muic", NULL, NULL, 0, 0, "maxim,max77705-muic"),
   ```
2. **`drivers/extcon/extcon-max77705.c` (new):** A new extcon driver for
   the max77705's integrated MUIC. It reads the MUIC registers (CC_STATUS,
   BC_STATUS, ADC values) to determine cable type and orientation, and
   sets extcon states (`EXTCON_USB`, `EXTCON_USB_HOST`, `EXTCON_CHG_USB_*`).

   This is modeled after `extcon-max77843.c` but uses the max77705's
   MUIC register map (documented in the Maxim MAX77705 datasheet).

3. **DTS overlay:** Wire the extcon to the dwc3:
   ```dts
   &usb_1_dwc3 {
       extcon = <&max77705_muic>;
       // or: usb-role-switch;
   };
   ```

4. **MAX77705 OTG boost control:** Wire `CNFG_00` (`MAX77705_OTG_CTRL`)
   from `max77705-charger` / Type-C source-VBUS callbacks when host mode is requested
   (`pm8150l qpnp-smb5` is `status = "disabled"` on `r8q`).

**This path is significant driver work** and is not part of the immediate
[usb-04] gate. It should be a separate plan.

#### Path B: Manual OTG via debugfs / role-switch (short-term, testable now)

This path uses the existing `usb-role-switch` framework in the dwc3 core
and the dwc3-qcom glue, without requiring a MUIC/extcon driver. The role
is switched manually via sysfs, and VBUS sourcing is handled by the
PM8150L's VBUS regulator (if available) or by the max77705's internal
OTG boost.

**What mainline already supports:**

1. **dwc3 `usb-role-switch`:** The core driver (`drivers/usb/dwc3/drd.c`)
   registers a `usb_role_switch` when `usb-role-switch` is present in the
   DT. The role can be changed via:
   ```sh
   echo host > /sys/class/usb_role/a600000.usb-role-switch/role
   echo device > /sys/class/usb_role/a600000.usb-role-switch/role
   ```

2. **dwc3-qcom VBUS override:** The `dwc3_qcom_run_stop_notifier`
   (`dwc3-qcom.c`) re-enables QSCRATCH VBUS_VALID when the UDC is rebound
   (write empty string then `a600000.usb` to `g1/UDC`).

3. **dr_mode = "otg":** If the DT sets `dr_mode = "otg"`, the dwc3 core
   enters dual-role mode and uses the extcon or OTG block for role
   detection. Without extcon, it falls back to manual role switching.

### 4.3 Engineering plan for [usb-04]

**Phase 1: Enable OTG via DT overlay (no driver changes)**

Create a minimal DT overlay (`device/r8q/overlays/usbc-otg-test.dtso`) that sets
`dr_mode = "otg"`, `usb-role-switch;`, and `role-switch-default-mode = "peripheral";`
on `&usb_1_dwc3`:

```dts
// SPDX-License-Identifier: MIT
/dts-v1/;
/plugin/;

&usb_1_dwc3 {
    dr_mode = "otg";
    usb-role-switch;
    role-switch-default-mode = "peripheral";
};
```

**Build and test (r8q workflow):**

```sh
tools/r8q-boot pack --overlay usbc-otg-test
tools/r8q-flash-recovery
```

**Phase 2: Test role switching (manual)**

```sh
# After boot with the overlay:
tools/phone-ssh --fresh '
    echo "=== Current state ==="
    cat /sys/class/udc/*/state 2>/dev/null
    cat /sys/kernel/debug/usb/a600000.usb/mode 2>/dev/null
    ls /sys/class/usb_role/ 2>/dev/null
    ls /sys/class/typec/ 2>/dev/null
'

# Switch to host mode:
tools/phone-ssh 'echo host > /sys/class/usb_role/a600000.usb-role-switch/role'

# Plug in a USB peripheral (keyboard, flash drive, etc.)
# Check dmesg for enumeration:
tools/phone-ssh 'dmesg | tail -30'

# Switch back to device mode:
tools/phone-ssh 'echo device > /sys/class/usb_role/a600000.usb-role-switch/role'
```

**Phase 3: VBUS sourcing for host mode**

For bus-powered USB peripherals (keyboard, flash drive without external hub power),
the phone must source 5 V VBUS via the **MAX77705 internal OTG boost** (`0-0069`,
`CNFG_00` register `0xB7`, bit1 `MAX77705_OTG_EN` / bit3 `BOOST_EN`), or use an
externally powered USB-C hub during initial role-switch testing. Note that PM8150L
`qcom,smb5-vbus` is disabled on `r8q` (`qcom,qpnp-smb5` has `status = "disabled"`).

**Recommendation:** Start with Path B Phase 1 and 2 (role switching
without VBUS sourcing). This tests the dwc3 role-switch framework.
Then investigate VBUS sourcing in Phase 3.

### 4.4 Gated runbook: [usb-04]

**Prerequisites:**
- [usb-03] dead-man adapted and tested (the phone can recover from USB drops).
- Battery ≥ 50 %.
- External USB peripheral available (keyboard, flash drive, or USB hub).
- Wi-Fi SSH as backup.

**Step U4.0 — Read-only: verify current state**

```sh
tools/phone-ssh --fresh '
    echo "=== dr_mode ==="
    cat /sys/firmware/devicetree/base/soc@0/usb@a6f8800/usb@a600000/dr_mode 2>/dev/null
    echo "=== role switch ==="
    ls /sys/class/usb_role/ 2>/dev/null
    echo "=== typec ==="
    ls /sys/class/typec/ 2>/dev/null
    echo "=== extcon ==="
    ls /sys/class/extcon/ 2>/dev/null
    echo "=== dmesg dwc3 ==="
    dmesg | grep -iE "dwc3|a600000|usb" | tail -20
'
```

**Expected:** `dr_mode = peripheral`, no `usb_role` or `typec` or `extcon`
directories.

**Step U4.1 — Apply the OTG overlay (one reboot)**

```sh
tools/r8q-boot pack --overlay usbc-otg-test
tools/r8q-flash-recovery
```

**Verify after boot:**

```sh
tools/phone-ssh --fresh '
    echo "=== role switch ==="
    ls -la /sys/class/usb_role/ 2>/dev/null
    cat /sys/class/usb_role/*/role 2>/dev/null
    echo "=== debugfs mode ==="
    cat /sys/kernel/debug/usb/a600000.usb/mode 2>/dev/null
    echo "=== UDC ==="
    cat /sys/class/udc/*/state 2>/dev/null
'
```

**Pass:** `/sys/class/usb_role/a600000.usb-role-switch/role` exists and reads
`device`. UDC state is `configured` or `attached`. (`/sys/class/typec` remains
empty because `usbc-otg-test.dtso` enables only `usb-role-switch`, not a Type-C
port driver.)

**Fail (no USB at boot):** Enter Download Mode, rebuild the stable image without
the overlay (`tools/r8q-boot pack`), and flash with `tools/r8q-flash-recovery`.

**Step U4.2 — Manual role switch to host under dead-man (risk: medium)**

```sh
# Uses tools/usbc-deadman-r8q otg-host (refuses with exit 5 unless dr_mode == otg
# or /sys/class/usb_role/*/role exists): switches to host for --duration 15s,
# then automatically switches back to device and rebinds g1/UDC so SSH returns.
tools/usbc-deadman-r8q otg-host --duration 15
```

**Expected:** dmesg shows the dwc3 switching to host mode. The gadget
disappears from the laptop.

**Step U4.3 — Plug in a USB peripheral (risk: medium, human's go)**

Plug in a USB keyboard or flash drive into the phone's USB-C port (using
a USB-C OTG adapter).

**Expected:**
- dmesg shows USB enumeration (new device, new interface).
- `lsusb` on the phone shows the device.
- If it's a flash drive, `/dev/sdX` appears and `blkid` shows the partition.

**If no enumeration:**
1. Check dmesg for dwc3 host errors.
2. Check if VBUS is present (the PM8150L VBUS regulator may not be
   enabled — this is expected if Phase 3 is not done).
3. Try a different peripheral.

**Step U4.4 — Return to device mode**

```sh
tools/phone-ssh 'echo device > /sys/class/usb_role/a600000.usb-role-switch/role'
```

**Expected:** The gadget reappears on the laptop. SSH to 172.16.42.1 works.

**Step U4.5 — Replug test (risk: medium, human's go)**

```sh
tools/usbc-deadman-r8q replug
```

**Expected:** Same as [usb-03] — the link drops and recovers after replug.

### 4.5 Rollback

If the overlay breaks USB at boot:

1. Rebuild the image without the overlay and flash via Samsung Download Mode:
   ```sh
   tools/r8q-boot pack   # no --overlay flag
   tools/r8q-flash-recovery
   ```

2. Or flash the known-good stable recovery image listed in
   `device/r8q/stable-image.conf` using `tools/r8q-flash-recovery`.

### 4.6 Results (`[usb-04]`, 2026-11-22, `7.3.0-rc6-omamobile-push1-g4d235a595f28` + `usbc-otg-test.dtso`)

| Check | Result | Observed |
|---|---|---|
| Boot with `usbc-otg-test.dtso` (`5e46b6e9...`) | **PASS** | `dr_mode=otg`, `/sys/class/usb_role/a600000.usb-role-switch/role = device`, `g1/UDC = a600000.usb` |
| `tools/usbc-deadman-r8q otg-host --duration 15 --confirm 30 --grace 20` | **PASS** (`exit 0`) | `xhci-hcd.3.auto` registered `usb1` (USB 2.0) + `usb2` (USB 3.1) root hubs at `0x0a600000` IRQ `180`; held `15 s`; switched back to `device`, deregistered `usb1`/`usb2`, rebound `a600000.usb`, fresh SSH confirmed in `3 s` |
| `otg-host --delay 15 --duration 35 --otg-vbus` + unpowered USB-C hub + mouse | **BLOCKED (no 5 V VBUS / reset)** | Writing raw `CNFG_00=0x0e` directly at `0-0069` without coordinating MAX77705's USBC/CCIC block (`i2c-0` `0x25`) did not power the USB-C hub/mouse and caused an immediate PMIC/system reset; `--otg-vbus` is now refused (`exit 5`) until MAX77705 `0x25` USBC/CCIC opcode support is wired in kernel. |

**Conclusion for `[usb-04]`:** DWC3 OTG role switching (`a600000.usb-role-switch`,
`xhci-hcd` <-> gadget) works cleanly on `r8q`. Full bus-powered peripheral
enumeration remains open until mainline `max77705` drives the `0x25` USBC/CCIC
block (Type-C CC/VBUS source control) — or an externally powered USB-C PD hub
is tested without `--otg-vbus`. Stable baseline `recovery-stable-20261122.img`
restored afterward via `tools/r8q-flash-recovery --stable`.

---

## 5. Risks

| Risk | Mitigation |
|---|---|
| OTG overlay breaks USB at boot | Keep the default DTB intact; overlay is applied on top. Rollback via Download Mode + `tools/r8q-flash-recovery` with `device/r8q/stable-image.conf`. |
| VBUS sourcing / MAX77705 OTG boost | Test `otg-host` first without `--otg-vbus` (self-powered hub), then opt in `--otg-vbus` carefully with multimeter/battery checks. |
| dwc3 host mode hangs | `otg-host` automatically returns to `device` mode and rebinds `g1/UDC` after `--duration` seconds; `--reboot-on-fail` adds `/dev/watchdog1` fallback. |
| No extcon = no auto role detection | Path B uses manual role switching. The phone does not auto-switch; the user or a userspace daemon must trigger it. |
| QCOM WDT held by systemd | `/dev/watchdog0` is held by PID 1; `usbc-deadman-r8q` leaves it untouched and uses `/dev/watchdog1` (softdog) only with `--reboot-on-fail`. |
| Rebooting goes back to `sda24` RECOVERY | `r8q` is `BOOT_METHOD=ram-only` with `sda24` as power-on default; `usbc-deadman-r8q` never reboots unconditionally without `--reboot-on-fail`. |

---

## 6. [usb-04] Feature 1 (`[usb-04]`) fixup results (2026-11-22)

Branch `r8q-usbc-max77705` on `~/omamobile-kernel/wt-r8q-usbc` contains 3
commits applied in order: **dt-bindings → driver → DTS**.

### 6.1 Commits

| # | Commit | Files |
|---|---|---|
| 1 | `c59ff5614` dt-bindings: extcon: Add maxim,max77705-muic DT schema | `Documentation/devicetree/bindings/extcon/maxim,max77705-muic.yaml` (new) |
| 2 | `14e718746` extcon: Add MAX77705 integrated MUIC/USBC extcon driver | `drivers/extcon/extcon-max77705.c` (new), `Kconfig`, `Makefile`, `drivers/mfd/max77705.c` |
| 3 | `61efacc62` arm64: dts: qcom: sm8250-samsung: Enable usb-role-switch on dwc3 | `sm8250-samsung-common.dtsi`, `sm8250-samsung-r8q.dts` |

### 6.2 CRITICAL BUGS FIXED

| # | Bug | Fix | Status |
|---|---|---|---|
| 1 | **Sleeping-in-atomic**: `spinlock_t` + `spin_lock_irqsave()` around I2C transfers and `extcon_set_state_sync()` | Replaced with `struct mutex` + `mutex_lock()`/`mutex_unlock()` | ✅ Fixed |
| 2 | **Deprecated `flush_workqueue(system_wq)`** in `.remove()` | Removed — `cancel_delayed_work_sync()` is sufficient | ✅ Fixed |
| 3 | **OTG detection bug**: `EXTCON_NONE` returned immediately when `!(bc_status & VBUSDET)`, preventing unpowered OTG hub detection | Check CC_STATUS0/1 and UIDADC regardless of VBUSDET; CC_SOURCE with no VBUS = OTG host mode | ✅ Fixed |
| 4 | **Wrong regmap for VBUS OTG boost**: `max77705_muic_set_otg_vbus()` wrote to CNFG_00 (0xB7) via parent regmap (0x66), but CNFG_00 lives on I2C slave 0x69 (charger) | Removed `max77705_muic_set_otg_vbus()` entirely; no unused static functions remain; OTG VBUS boost to be handled in `max77705_charger.c` (0x69 regmap) | ✅ Fixed |
| 5 | **Log order bug**: `dev_info` printed new → old because `prev_cable_type` was overwritten before the log | Save `old_cable = info->prev_cable_type` before updating; log reads old → new | ✅ Fixed |
| 6 | **dwc3 extcon wiring**: Verified `drivers/usb/dwc3/drd.c` — `usb-role-switch` property registers `usb_role_switch` (userspace control); `extcon` property auto-detects role from cable state. Base DTS gets `usb-role-switch`; overlay adds `extcon = <&max77705_muic>` and `dr_mode = "otg"` | ✅ Verified and wired |
| 7 | **DT binding schema**: Added `Documentation/devicetree/bindings/extcon/maxim,max77705-muic.yaml` | ✅ Added |
| 8 | **Clean commits**: dt-bindings → driver → DTS, no Signed-off-by trailer | ✅ Done |

### 6.3 Build and check results

| Test | Result |
|---|---|
| `KERNEL_DIR=$PWD KERNEL_OUT=... bin/omamobile kernel` (W=1, -Werror) | **0 warnings, 0 errors** on extcon-max77705.c, Kconfig, Makefile, DTS |
| `scripts/checkpatch.pl --strict` on all 3 commits | **0 errors**, 2 warnings per commit (MAINTAINERS update for new file, SPDX `#include` false positive) |
| `./tests/device-overlays` | **90 overlays compile**, every descriptor parses |
| `./tools/pr-check` | **No commits** to check against origin/main (branch based on r8q-base-dts-fold) |
| DT overlay syntax (`dtc -@`) | **usbc-otg-test.dtso compiles cleanly** |

### 6.4 DT wiring verified (`5cd87a8f5` + `usbc-otg-test.dtso`)

```dts
/* Base DTS (sm8250-samsung-common.dtsi) */
&usb_1_dwc3 {
    dr_mode = "peripheral";
    usb-role-switch;
    role-switch-default-mode = "peripheral";
};

/* Base DTS (sm8250-samsung-r8q.dts) */
max77705_muic: muic {
    compatible = "maxim,max77705-muic";
    interrupt-parent = <&max77705>;
    interrupts = <3>;
    status = "disabled";
};

/* Overlay (usbc-otg-test.dtso): OF graph port avoids initramfs -EPROBE_DEFER */
&max77705_muic {
    status = "okay";
    port {
        max77705_muic_ep: endpoint {
            remote-endpoint = <&usb_1_dwc3_hs_out>;
        };
    };
};
&usb_1_dwc3_hs_out { remote-endpoint = <&max77705_muic_ep>; };
&usb_1_dwc3 { dr_mode = "otg"; };
```

### 6.5 Live hardware test (`2026-11-22`, `7.3.0-rc6-omamobile-push1-g61efacc62a48-dirty`)

- Built cleanly (`wt-r8q-usbc` -> `out-r8q-usbc`), `checkpatch.pl` `0` errors/`0` warnings on `extcon-max77705.c`, packed `--overlay usbc-otg-test` (`sha256: 03ff23110da88bc7253e647c61d14c422889dd91e78e32af97a5eba6836d8ce3`), flashed `/dev/sda24`, pushed modules (`extcon-max77705.ko`, `msm.ko`).
- **Live probe:** `extcon0` (`max77705-muic`) registered, `i2cdetect 0x25 = UU`, `role_sw=yes`, `USB=1`, `role=device`, `/dev/dri/renderD128` active, `tools/phone-smoke` all hardware checks `ok`.
- **OTG switch test:** `tools/usbc-deadman-r8q otg-host --duration 12 --confirm 30 --grace 15` switched `device -> host` (`xhci-hcd.3.auto` registered `usb1` + `usb2`) and restored `device` + SSH login cleanly in `16 s`. (Physical external USB peripheral attach/unplug remains open.)
- **Rollback:** Restored pinned `recovery-stable-20261122.img` (`sha256: f84a754ad5d28935998b396ab08e4b6051abcf0e3969e40897c611bd4fb3464e`) and confirmed `tools/r8q-stable-check --phone`.

### 6.5 Safe gated test runbook (online, requires phone)

**Prerequisites:** Battery ≥ 50 %, Wi-Fi SSH backup, USB-C OTG adapter.

**Step U4.0 — Read-only: verify base state**
```sh
tools/phone-ssh --fresh '
    echo "=== role switch ==="
    ls /sys/class/usb_role/ 2>/dev/null
    echo "=== extcon ==="
    ls /sys/class/extcon/ 2>/dev/null
'
```
**Expected:** `/sys/class/usb_role/a600000.usb-role-switch/role` exists, reads `device`.

**Step U4.1 — Build and flash the overlay**
```sh
# On laptop:
tools/r8q-boot pack --overlay usbc-otg-test
tools/r8q-flash-recovery
```

**Step U4.2 — Verify overlay after boot**
```sh
tools/phone-ssh --fresh '
    echo "=== dr_mode ==="
    cat /sys/firmware/devicetree/base/soc@0/usb@a6f8800/usb@a600000/dr_mode 2>/dev/null
    echo "=== extcon ==="
    ls /sys/class/extcon/ 2>/dev/null
    echo "=== extcon state ==="
    cat /sys/class/extcon/*/state 2>/dev/null
'
```
**Expected:** `dr_mode = otg`, `/sys/class/extcon/extcon-X/` exists with cable state.

**Step U4.3 — Manual role switch (risk: medium)**
```sh
tools/usbc-deadman-r8q otg-host --duration 15 --confirm 30 --grace 20
```
**Expected:** xhci-hcd registers root hubs, switches back to device, gadget rebinds.

**Step U4.4 — OTG peripheral test (risk: medium, human's go)**
Plug in a bus-powered USB-C hub + mouse. **Expected:** dmesg shows enumeration.
**Note:** Bus-powered hub needs 5 V VBUS from MAX77705 CNFG_00 (0xB7) — this
requires OTG boost wiring in `max77705_charger.c` (0x69 regmap), which is the
next step after this feature is merged.

**Rollback:** `tools/r8q-flash-recovery --stable` (no overlay).

For comparison, the cheetah tool's `i2c_bus()` function:

```bash
i2c_bus() {
    local d
    for d in /sys/bus/i2c/devices/i2c-*; do
        if grep -qs 10d60000 "$d/name" || \
           readlink -f "$d" | grep -q 10d60000.i2c; then
            echo "${d##*-}"; return 0
        fi
    done
    return 1
}
```

This looks for `10d60000.i2c` (cheetah's Exynos I2C controller). On r8q,
the I2C controller is `i2c@980000` (Geni-I2C), and the bus name is
`Geni-I2C`. The replacement for r8q:

```bash
i2c_bus_r8q() {
    local d
    for d in /sys/bus/i2c/devices/i2c-*; do
        if grep -qs "Geni-I2C" "$d/name" || \
           readlink -f "$d" | grep -q "i2c@980000"; then
            echo "${d##*-}"; return 0
        fi
    done
    return 1
}
```

## Appendix B: Stock MUIC runtime log values (from `reference-r8q/dmesg.txt`)

During USB device connection on stock Android (`dmesg.txt`), Samsung's
`muic-max77705` driver logs:

```
muic-max77705:max77705_muic_print_reg_log USBC1:0x27, USBC2:0x05, BC:0x81, UICINTM:0x0, attached_dev:1
```

`USBC1`, `USBC2`, `BC`, and `UICINTM` are the current register **values** read
from MAX77705's internal USBC/CCIC I2C slave at `0x25` (`attached_dev:1` = USB
peripheral attached). Mainline does not bind `0x25` yet.

## Appendix C: dwc3-qcom VBUS override

The `dwc3_qcom_run_stop_notifier` function (`dwc3-qcom.c`) re-enables
QSCRATCH VBUS_VALID when the UDC is rebound (write empty string then
`a600000.usb` to `g1/UDC`). This is critical for OTG because the QCOM
PHY needs to know VBUS is present even when the controller is in host
mode (no external VBUS sense pin on SM8250).

The `dwc3_mode_write` function (`debugfs.c`) silently ignores writes to
`/sys/kernel/debug/usb/a600000.usb/mode` unless `dr_mode == otg`.

## Appendix D: r8q watchdog layout

| Device | Compatible | Owner | Timeout | Notes |
|---|---|---|---|---|
| `/dev/watchdog0` | `17c10000.watchdog` (qcom-wdt) | PID 1 systemd | 30s (`RuntimeWatchdogSec`) | **DO NOT TOUCH** — held busy |
| `/dev/watchdog1` | Software Watchdog (softdog) | Free | 100s, SETTIMEOUT=0, MAGICCLOSE=0 | Available for `--reboot-on-fail` feeder |
