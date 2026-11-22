<!-- SPDX-License-Identifier: MIT -->

# `omamobile-device-r8q`

Device support package for the Samsung Galaxy S20 FE (`r8q`, SM8250).

## Layout

| File(s) | Destination | Purpose |
|---|---|---|
| `reboot-to.c`, `reboot` | `/usr/local/bin/` | Reboot reason helper and wrapper for `r8q` |
| `rust/r8q-preflight` | `/usr/local/bin/r8q-preflight` | Read-only boot/DT/guard/SQE preflight check |
| `r8q-liveness-heartbeat.c`, `r8q-liveness-monitor`, `omamobile-r8q-liveness.service` | `/usr/lib/omamobile/`, systemd | Per-CPU heartbeat monitor, I2C runtime-PM pins, BT address/audio startup safety |
| `r8q-battery-modprobe`, `omamobile-r8q-battery.service` | `/usr/lib/omamobile/`, systemd | Post-boot `max77705`/`max17042` module load |
| `20-usb-gadget.network`, `10-omamobile-usb.conf` | systemd-networkd, NetworkManager | USB NCM gadget (`172.16.42.1/24`), kept unmanaged by NM |
| `etc/NetworkManager/conf.d/zz-omamobile-wifi-powersave.conf` | `/etc/NetworkManager/conf.d/` | Enables Wi-Fi power save (`wifi.powersave=3`) after Omarchy's laptop override (`backup=`) |
| `sshd-omamobile.conf`, `sshd.sysusers`, `zz-r8q-relogin.conf` | `/etc/ssh/sshd_config.d/`, sysusers, PAM | SSH key-only login and session environment |
| `10-r8q-no-suspend.conf`, `10-r8q-no-idle.conf`, `10-r8q-watchdog.conf`, `ufs-no-autohibern8`, `cpufreq-no-dvfs`, `cpuidle-no-deep-sleep` | systemd, `/usr/lib/omamobile/` | Safety gates until s2idle/DVFS/cpuidle/UFS-PM stabilize (see [README-suspend.md](README-suspend.md)) |
| `80-r8q-gpu.rules`, `20-r8q-torch.rules`, `79-r8q-iio-sensor-proxy.rules`, `70-omamobile-dma-heap.rules` | `/usr/lib/udev/rules.d/` | GPU devfreq `simple_ondemand` (305-587 MHz), RT8547 `white:torch`, SLPI sensor tags, DMA-BUF heaps |
| `ucm2/`, `50-r8q-audio.conf`, `10-r8q-hexagonrpcd.conf` | ALSA UCM2, WirePlumber, systemd | Dual CS35L41 speakers/earpiece (-9.5 dB), DMIC3 S16LE/48 kHz capture, SLPI sensor tunnel |
| `70-r8q-libcamera.conf`, `omacam-lenses.toml`, `../../device/r8q/overlays/*.dtso` | environment.d, skel/share | SoftISP CPU mode, 0.5x/3x `omacam` lenses, bring-up DT overlays |
| `etc/omamobile-ai.conf.d/10-sm8250.conf` | `/etc/omamobile-ai.conf.d/` | Kyro 585 affinity (`4-7`, 4 threads), 180 s idle TTL, USB serving performance boost |
| `80-r8q-mglru.conf`, `90-r8q-vm.conf`, `91-r8q-zram.conf`, `50-r8q-gpu-blacklist.conf` | sysctl, zram, modprobe | 6 GB RAM tuning (`2.7G` zram + MGLRU), boot-time module blacklists |
