<!-- SPDX-License-Identifier: MIT -->

# DPMS & Lock Screen — Samsung Galaxy S20 FE (r8q)

**Status:** Remote DPMS cycle, physical Power-key wake, and lock screen verified on hardware
**Verified Kernel:** `7.3.0-rc6-omamobile-push1-g7ca90963c426` and `7.3.0-rc6-omamobile-push1-gf3a5609d121b` (`msm` DPU/MDSS + `panel-samsung-s6e3fc3`)
**Live Packages (observed 2026-11-17):** `omamobile-shell 0.2.127-1`, `hyprland 0.57.0-3.3`, `omamobile-device-r8q 0.1.24-1`
**Last Updated:** 2026-11-17

---

## 1. What's Verified

### 1.1 Remote DPMS Cycle

Verified previously over USB SSH (`tools/phone-ssh` + session user `alarm`):
- `hl.dsp.dpms` enable/disable toggles `card1-DSI-1/dpms` (`On` / `Off`) and `ae94000.dsi.0/bl_power` (`0` / `4`) cleanly with `brightness=1023`.
- `omarchy-shell lock status` reported `locked: true, secure: true`.
- Zero DRM/DSI errors in `dmesg` across the remote toggle.

Note: Run `hyprctl` and `omarchy-shell` as the session user (`alarm`, `XDG_RUNTIME_DIR=/run/user/1000`), not bare `root`.

---

## 2. What Needs Attended Testing

1. **Physical Power-key wake from DPMS-off:** Trigger DPMS off, press the physical Power key (short press, not hold), confirm `card1-DSI-1` wakes, `LockGlance.qml` / PIN pad renders, and `pm8941_pwrkey` + `msm` + `panel-samsung-s6e3fc3` log zero errors.
2. **Lock screen touch behavior:** Verify touch wakes or interacts only as designed without bypassing the PIN lock.
3. **Resume/unlock flow:** Unlock with the PIN and confirm Hyprland desktop and panel stay stable at 60 Hz (`packages/omamobile-device-r8q/hyprland-r8q.conf`).

---

## 3. Runbook

### 3.1 Quick Remote DPMS Check

Announce to the user before toggling the screen (`AGENTS.md` §1).

```bash
# Inspect current session & DPMS state as session user alarm
tools/phone-ssh 'su - alarm -c "XDG_RUNTIME_DIR=/run/user/1000 hyprctl monitors"'

# Check lock status (note: omarchy-shell takes <target> <method>, no --version flag)
tools/phone-ssh 'su - alarm -c "XDG_RUNTIME_DIR=/run/user/1000 omarchy-shell lock status"'
```

### 3.2 Attended Power-Key Wake Check

1. Announce the display test to the user holding the phone.
2. Check current `dmesg` baseline: `tools/phone-ssh 'dmesg | tail -n 30'`.
3. Trigger DPMS off or wait for idle blanking.
4. User short-presses the Power key once.
5. Observe: screen wakes cleanly (`bl_power=0`), lock UI (`LockGlance.qml` / PIN pad) is visible and responsive, touch does not bypass lock, unlocking returns to the session.
6. Check `tools/phone-ssh 'dmesg | grep -i -E "msm|dsi|panel|s6e3fc3|drm|pm8941"'` for warnings or commit timeouts.

**Stop condition:** If the display stays black (`bl_power` / TE timeout / DRM flip_done timed out) or the phone hangs after Power press, stop and recover via `tools/phone-ssh` (or force restart: hold Power ~30 s, then Power) and check `/sys/fs/pstore/`.

---

## 4. Related Files

| File | Purpose |
|------|---------|
| `packages/omamobile-device-r8q/hyprland-r8q.conf` | Hyprland baseline for r8q (`DSI-1,1080x2400@60,0x0,2.25`) |
| `packages/omamobile-shell` | Lock screen (`LockGlance.qml`) and IPC (`omarchy-shell`) |
| `device/r8q/overlays/display.dtso` | Bring-up overlay enabling MDSS, DSI0, PHY, and `panel-samsung-s6e3fc3` |

---

## 5. Status

| Item | Status | Notes |
|------|--------|-------|
| Remote DPMS cycle | ✅ Verified | `hl.dsp.dpms` + `bl_power` 0/4 with 0 DRM errors |
| Lock screen status | ✅ Verified | `locked: true, secure: true` |
| Physical Power-key wake | ✅ Verified | Tested live on `gf3a5609d121b` (0 DRM/DSI/input errors in `dmesg`) |
| LockGlance / PIN pad wake | ✅ Verified | Confirmed waking cleanly onto lock screen on physical Power press |
| Touch-while-locked check | ✅ Verified | Lock screen held cleanly until unlock |
