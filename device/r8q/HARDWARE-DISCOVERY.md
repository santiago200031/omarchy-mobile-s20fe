<!-- SPDX-License-Identifier: MIT -->

# `r8q` Hardware Discovery & Unported Subsystem Map (`2026-11-21`)

Exact hardware findings extracted from the stock reference capture (`~/omamobile-kernel/reference-r8q/live.dts`, `dmesg.txt`, `interrupts.txt`, `lineage-kernel.config`), mainline Linux 7.3 kernel drivers in `~/omamobile-kernel/wt-r8q`, and non-destructive probes on the connected phone (`7.3.0-rc6-omamobile-push1-gae195e6deaac`).

---

## 1. Camera `camss` `port@1` Deadlock (`[cam-03]`)
- **Live proof (`/sys/kernel/debug/v4l2-async/pending_async_subdevices`):**
  `qcom-camss ac6a000.camss:` permanently waits on `/soc@0/geniqup@9c0000/i2c@988000/camera@2d/port/endpoint` (`&s5k3l6_out`) while `hi847 10-0021` is already bound (`14 /dev/video*` registered, `/dev/media0` present, `cam -l` returns 0).
- **Root cause (`camss.c:4799`):** `camss_parse_ports()` iterates `fwnode_graph_for_each_endpoint(fwnode, ep)` without checking `fwnode_device_is_available()`, so `port@1 { status = "disabled"; ... remote-endpoint = <&s5k3l6_out>; }` in `sm8250-samsung-r8q.dts` still registers `&s5k3l6_out` in `v4l2_async_nf`.
- **Clean fix for next kernel pass:** Either remove `port@1` (or its `remote-endpoint`) from base `sm8250-samsung-r8q.dts` until `camera-uw` (`&i2c2`/`s5k3l6`) is enabled, or check `fwnode_device_is_available(port)` in `camss_parse_ports()`.

## 2. Audio Microphones, Earpiece & Speakers (`[aud-04..06]`)
- **No 3.5 mm jack & no `wcd938x`:** Stock `live.dts` marks `wcd938x-codec` `status = "disabled"`, and `sec-audio-sysfs` specifies `audio,no-earjack` (`audio,num-amp = <2>`).
- **Top earpiece & bottom speaker (`[aud-06]` verified):** Both speakers are driven by dual `cs35l41` (`11-0041` `Left`, `11-0040` `Right`). PipeWire/UCM2 (`omamobile-device-r8q 0.1.29-1`) already exposes both `HiFi (Speaker)` (balanced L/R `Analog=10`, `Digital=657` -> `-9.50 dB`) and `HiFi (Earpiece)` (mutes `Right Digital PCM` to `0`); profile switching verified silently on phone.
- **DMICs (`[aud-04..05]`):** `arecord -D hw:0,0` fails with `Invalid argument` and PipeWire shows zero `Sources` because `sm8250-samsung-r8q.dts` only wires `PRIMARY_TDM_RX_0`. Stock uses LPI `cdc_dmic01_pinctrl` / `cdc_dmic23_pinctrl` routed into `vamacro@3370000` (`qcom,va-dmic-sample-rate = <600000>`, solving live `va_macro 3370000.codec: qcom,dmic-sample-rate dt entry missing`) and `txmacro@3220000` (`qcom,tx-dmic-sample-rate = <2400000>`).

## 3. Haptics (`[hap-01]`) & Torch (`[hap-03]`)
- **Haptics (`[hap-01]`):** Stock uses `samsung,vibrator` (`LINEAR_0832RC` LRA) on `tlmm` GPIO `137` (`gcc_gp2` PWM clock), powered by `pm8150a_l10` (`L10C`, `3.0 V`), alongside `max77705` (`haptic,mode = <1>`, `haptic,divisor = <128>`) and `CONFIG_SLPI_MOTOR`. On mainline, `max77705` MFD already creates `/sys/bus/platform/devices/platform:max77705-haptic` (matched by `drivers/input/misc/max77693-haptic.c`, currently `# CONFIG_INPUT_MAX77693_HAPTIC is not set`), and `pwm-vibra.ko` (`CONFIG_INPUT_PWM_VIBRA=m`) is already built. Note PM8150B's `qcom,haptics@c000` is `disabled` in stock.
- **Torch / Flash (`[hap-03]`):** Stock uses `qcom,qpnp-flash-led-v2` on **`pm8150l@5` (`reg = <0xd300>`)**, with `qcom,torch_0` and `qcom,torch_1` capped at `500 mA` max (`300 mA` default). Mainline `pm8150l.dtsi` (already included by `sm8250-samsung-r8q.dts`) already defines disabled `pm8150l_flash: led-controller@d300` (`compatible = "qcom,pm8150l-flash-led", "qcom,spmi-flash-led"`), and **`leds-qcom-flash.ko` (`CONFIG_LEDS_QCOM_FLASH=m`) is already compiled and installed on the phone**. Only DT child `led-0`/`led-1` subnodes + overlay enablement remain.

## 4. USB-C Role Switch / MUIC (`[usb-02]`)
- Stock `max77705@66` (`interrupt-parent = <&pmic_arb>`, IRQ `max77705,irq-gpio` on `pm8150l_gpio11` / `0xca`, triggering `max77705-irq`) handles BC1.2/MUIC (`attached_dev`), CC orientation (`CC_STATUS0/1`), and USB-PD (`PD_STATUS0/1`, `support_pd_role_swap`), while PM8150B `qcom,usb-pdphy@1700` is `disabled`.
- Mainline `drivers/mfd/max77705.c` currently instantiates only `max77705-rgb`, `max77705-charger`, and `max77705-haptic`; `/sys/class/typec` and `/sys/class/extcon` are empty. Full plug/replug role-switch and PD negotiation require extending `max77705` with MUIC/TCPM/extcon support (or wiring `dwc3` OTG/extcon manually under `tools/usbc-deadman`).

## 5. Suspend Wake IRQs (`[slp-01]`) & Power Baseline (`[pwr-02]`)
- **Live WFI baseline (`[pwr-02]`):** `max170xx_battery`: `status=Full`, `capacity=99%`, `4.372 V`, `current_now=-7..-96 mA` (on USB), `27.5 °C`, `health=Good`; CPU governors `performance`, `cpuidle` `state0=enabled` (`WFI`), `state1=disabled`; GPU devfreq `305..587 MHz` idling at `305 MHz`; thermals `38.0–42.3 °C`.
- **Wake IRQs (`[slp-01]`):** Stock enables `pm8xxx_rtc_alarm`, `gpio_keys` (`gpio-key,wakeup_enable`, Volume Up `pm8150l_gpios 3`), `pm8941_pwrkey`, and `max77705-irq` (`wakeup`) alongside `s2idle` wakeup-source annotations.

## 6. NFC, GNSS (`[mdm-06]`) & Fingerprint (`[fp-01]`)
- **NFC:** Stock active node is **`sec-nfc@27`** on `qupv3_se15_i2c` (`i2c@884000`, `nxp,sn100u` / `CONFIG_NFC_FEATURE_SN100U=y`): `irq` `gpio31`, `ven` `gpio8`, `firm` `gpio9`, `clk_req` `gpio7`, supply `pm8150a_l1` (`L1C`, `1.8 V`). `nxp-nci_i2c.ko` (`CONFIG_NFC_NXP_NCI_I2C=m`) is already built on the phone.
- **GNSS:** No standalone GNSS chip in `live.dts` — position fixes come from the external `SDX55M` modem via QMI PDS/LOC once the modem (`[mdm-03..05]`) is up on userspace.
- **Fingerprint (`[fp-01]`):** Optical under-display **`goodix,gw9558x`** (`GW9558`) over SPI (`cs 0`, `goodix,gpio_pwr`, `goodix,gpio_reset`) gated by a proprietary TrustZone TA (`tz: 1 type: goodixopt` in `dmesg.txt`) — no mainline driver exists; future support would require Android TZ/HAL chroot or protocol documentation.
