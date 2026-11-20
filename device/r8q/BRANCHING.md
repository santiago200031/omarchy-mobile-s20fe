<!-- SPDX-License-Identifier: MIT -->

# Branching and stability policy — r8q port

The r8q port's repositories are public, and the phone is real hardware:
wrong builds cost a flash cycle and a dead phone in the pocket. This
policy keeps every experiment cheap and every stable state verifiable.

## The rule

**Work lands on topic branches. `main` takes only hardware-verified
stable states.** Nothing merges onto `main` without a recorded
on-device PASS of an image built from that tree.

## What makes a state stable

1. The kernel builds through `tools/r8q-kernel` (the DEVICE=r8q config
   assertions pass — a wrong-config build dies on the laptop, not the
   phone; NOTES.md 30).
2. An image packed from that tree (`tools/r8q-boot pack --dual-boot
   [--overlay NAME]`) passed `tools/r8q-test-image` on the phone:
   UFS enumerated, gadget up, kernel banner matching, boot log saved.
3. The PASS is recorded in `device/r8q/NOTES.md` with the image sha256
   (the pattern of NOTES.md 188: the stable baseline is a checked
   record, `device/r8q/stable-image.conf`).

## The gate

`tools/r8q-merge-main --pass-sha <sha256>` merges the current topic
branch into `main` and refuses unless the sha is recorded as a PASS in
`NOTES.md`. There is no override flag: a human records the PASS (the
flash is the human's call, AGENTS.md section 1), and the tool only
checks that the record exists.

## The kernel repository

The same model one level down: `omamobile-r8q-signed` is the signed
stable record (`stable-image.conf` pins it); modem/GPU/driver work
lives on `r8q-*` topic branches (`r8q-gmu-recover-fix`,
`r8q-modem-nom3`, …) and only a hardware-passed state moves the stable
record. The in-flight exception is documented in NOTES.md: a failed
build on a topic branch costs one flash; a failed build on the stable
record costs the daily phone.

## Pushing

Topic branches push freely — they are the public record of the work
(NOTES.md, plans, tools). `main` pushes only through the gate. Never
rewrite a branch someone else merges from.
