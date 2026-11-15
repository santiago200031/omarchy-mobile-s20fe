<!-- SPDX-License-Identifier: MIT -->

# Failed 03: Built-in `max77705` / `max17042` & Unloading `max77705_charger`

- **Symptom:** Built-in (`=y`) `MAX77705`/`MAX17042` hung early boot at `0.95 s`; unloading `max77705_charger` at runtime deleted `/sys/class/power_supply/max170xx_battery`.
- **Back:** [FAILED.md](../FAILED.md) · [Board Overview](../README.md)

## Root Cause
- Probing `max77705` / `max17042` before late-init subsystems settles freezes early boot on SM8250 Samsung PMIC/I2C1.
- When kept as `=m`, letting `980000.i2c` and `9c0000.geniqup` runtime-suspend triggers continuous `BCM`/`QUP0` RPMh churn; unloading `max77705_charger` (`0-0069`) to stop it also unbinds `max17042_battery` (`0-0036`) because of `fw_devlink` (`supplier:i2c:0-0069--i2c:0-0036`).

## Permanent Rule
- Keep `MAX77705` and `MAX17042` as kernel modules (`=m`), loaded post-boot via `omamobile-r8q-battery.service` (`r8q-battery-modprobe`).
- Keep `max77705_charger` loaded and pin `9c0000.geniqup` and `980000.i2c` to `ATTR{power/control}="on"` in `80-r8q-gpu.rules`.
