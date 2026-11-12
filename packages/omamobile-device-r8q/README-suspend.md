# r8q suspend state

The kernel's s2idle on this port never wakes: the phone freezes and only a
PMIC hard reset (Power held ~20 s) recovers it. Until that is fixed:

- `/usr/lib/systemd/sleep.conf.d/10-r8q-no-suspend.conf` makes logind refuse
  suspend/hibernate, the gate every caller uses.
- `/usr/lib/systemd/logind.conf.d/10-r8q-no-idle.conf` keeps the seat out of
  idle actions and leaves the power key to the shell.
- `/etc/systemd/system/{sleep,suspend,hibernate,hybrid-sleep,suspend-then-hibernate}.target`
  are masked, for anything that calls the targets directly.
- The kernel command line also carries `systemd.mask=` for the same targets,
  so a boot with a broken userspace still cannot reach the freezer.

`tools/r8q-stabilize` applies the same policy to a live phone and captures
bring-up facts; it is the repair path when a phone was installed before this
package carried the files.

The console on the USB serial port (`serial-getty@ttyGS0`) is enabled as the
rescue shell for the same reason: it works when the USB network stack or the
session does not.
