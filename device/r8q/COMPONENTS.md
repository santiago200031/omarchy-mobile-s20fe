<!-- SPDX-License-Identifier: MIT -->

# r8q component map — what each part is, does, and how it is wired

The reference for every component of the S20 FE port: what it is, what
it does, and the technical parts (driver, DT node, kernel config, the
tool that drives it, its state today, and the gotcha that bit us).
Companion docs: [PORT-PLAN.md](PORT-PLAN.md) (the objectives),
[BRANCHING.md](BRANCHING.md) (the policy), [NOTES.md](NOTES.md) (the
dated findings, newest last), [FILE-DASHBOARD.md](FILE-DASHBOARD.md)
(the file index), [MODEM-PLAN.md](MODEM-PLAN.md) (the cellular detail),
[BOARD-REPORT-GPU-WEDGE.md](BOARD-REPORT-GPU-WEDGE.md) (the GPU wedge).

## 1. Boot chain and dual boot

**What:** S-Boot (Samsung's bootloader, untouched) loads the recovery
partition (`sda24`) as the Linux entry; the stock Android/LineageOS
partitions (`boot` sda23, `super`, and the bootloader) are never
written, so the phone always has its way back.

**Does:** power-on → S-Boot → the recovery kernel (mainline 7.3-rc6
`7.3.0-rc6-omamobile-push1-g<sha>`) → the initramfs → the Arch root on
userdata. A normal reboot without our recovery selected boots
LineageOS. Writes to the phone: `sda24` only, via `tools/r8q-test-image`
(SSH) or heimdall (Download Mode: power off, Volume Down + Power).

**Parts:** Android boot image v2 + AVB hash footer (`tools/r8q-boot
pack --dual-boot`); the cmdline (`console=tty0 console=ttyGS0 …
omamobile.loop=/omarchy-r8q.img` with `--dual-boot`); `bin/omamobile
kernel` builds, `tools/r8q-kernel` gates the build.

**Gotchas:** every image must be packed `--dual-boot` for the loop-root
flow; a kernel built without `DEVICE=r8q` silently loses UFS and
power-does nothing (NOTES §30 — now caught at build and pack time).

## 2. Initramfs (Alpine) and the storage guard

**What:** a small busybox+dropbear root that runs before Arch.

**Does:** marks the UFS partitions read-only (the dual-boot guard;
only the allowlisted partitions stay writable), brings up the USB
gadget + SSH at 172.16.42.1, waits for the host, then `switch_root`s
to the Arch loop image (`/omarchy-r8q.img`) or the native root on
userdata. On a failed boot it stays in the rescue shell (r8q is
non-A/B: it never panics to a fallback).

**Parts:** `initramfs/init` (the guard + probe flow), packed by
`tools/r8q-boot`; **it ships NO kernel modules** — everything
boot-critical must be built-in (`CONFIG_SCSI_UFS_QCOM=y`,
`CONFIG_PHY_QCOM_QMP_UFS=y`, dwc3 + its PHYs `=y`: enforced by
`tools/r8q-kernel` and at pack time).

## 3. Storage — UFS

**What:** the internal 128 GB UFS (`/dev/sda`, partitions
`PARTNAME=userdata`, the modem `mdm*` partitions, and LineageOS's
partitions).

**Does:** the whole-disk node is removed by the guard; everything but
the allowlist gets `blockdev --setro`. The Arch root lives on the
`userdata` partition (the loop image with `--dual-boot`).

**Parts:** `ufshcd-qcom` built-in + the QMP UFS PHY built-in; the
HC↔PHY dependency cycle in the DT is expected (fw_devlink relaxes it).

**State:** enumerates every boot on a correct-config build (0 disks was
never a UFS bug — it was the §30 config).

## 4. USB — the dwc3 gadget (the laptop link)

**What:** the SoC's USB2 controller as a device: one NCM network +
one ACM serial gadget.

**Does:** the phone is `172.16.42.1`, the laptop `172.16.42.2` — SSH,
every tool, and the whole flash/test automation ride this link. The
serial console (`ttyGS0`) is the rescue shell.

**Parts:** `CONFIG_USB_DWC3=y`, `CONFIG_USB_DWC3_QCOM=y`, the QMP USB
+ femto PHYs `=y`; the host side is the NCM interface (predictable name
`enp18s0u4` here) — a replug re-enumerates it with NO IPv4, so the
host end must be re-set (`tools/r8q-test-image` does this
automatically; by hand: `nmcli connection modify <con> ipv4.method
manual ipv4.addresses 172.16.42.2/24 && nmcli connection up <con>`).

**Gotchas:** the physical link is flaky on this setup — replug at both
ends when the gadget vanishes; the automated tools retry around drops.

## 5. Wi-Fi — pcie0 (the control for the modem path)

**What:** the QCA6390 on the first PCIe root complex.

**Does:** pcie0 trains (`Gen.2 x1 link up`) once `phy_qcom_qmp_pcie`
loads — **it is a module** (`CONFIG_PHY_QCOM_QMP_PCIE=m`): without the
modules folder for the running kernel, BOTH RCs sit on the deferred
list forever. `ath11k_pci` then brings the radio.

**Parts:** pcie0 + its PHY enabled in the base DT; the DesignWare
iMSI-RX path (pcie0 has no `msi-parent`).

## 6. Modem — pcie2, the doorbell, MHI

**What:** the external SDX55M cellular modem on the second root
complex, the flashless variant sharing PCI ID 17cb:0306.

**Does:** the endpoint's MSI is a DMA write to a **doorbell register**
at `0x17a10040` — the GIC-600's GICA `SETSPI` alias page (write-only
by architecture). The block raises the banked SPI (832–863 for pcie2).
MHI carries the control protocol; SBL → AMSS → mission mode; QRTR
spans the services.

**Parts:** `qcom,pci-msi` doorbell node (base DT `status="disabled"`,
the modem overlay enables it with pcie2 — **the reg is zero-size
address-as-data: no driver may ever map or read it**, NOTES §32–33);
`drivers/irqchip/irq-qcom-pci-msi.c` (value-only: `iommu_dma_prepare_msi`
+ `msi_msg_set_addr`, the GIC parent hwirq as message data);
`mhi_pci_generic` + the modem overlay (Gen.1 x2 cap, the CX/perf
state votes, the mdm pinmuxes); MHI event-ring polling for the
no-MSI fallback; `tools/r8q-sahara` for the SBL loader side.

**State (2026-11-20):** **mission mode held stable, M0 active, MSI
vectors delivering** — past the 29-run +32.4 s death for the first
time (NOTES §35). Next rungs per MODEM-PLAN: QRTR services, rmtfs,
registration.

**Gotchas:** a CPU read of the doorbell page powers the phone off on
some boots (the §33 fatal path); the image still boots intermittently
(the unresolved state co-factor) — the ablated no-modem image stays
the daily baseline.

## 7. GPU — Adreno 650 (renderD128 / Vulkan / the LLM)

**What:** the a650 at `3d00000.gpu` with its GMU at `3d6a000`.

**Does:** DRM/KMS via the `msm` driver (**a module**: it needs the
matching modules folder, and it must load at boot — a hand `modprobe`
fails, NOTES §346); Turnip (freedreno) provides Vulkan 1.3.354;
`renderD128` is the render node the AI stack targets.

**Parts:** `CONFIG_DRM_MSM=m` (+ the whole DSI/DPU set built-in);
`/usr/share/vulkan/icd.d/freedreno_icd.json`; the LLM: the installed
`omamobile-llama` is **CPU-only by policy** (the wedge); a Vulkan
build lives at `~/…/llama.cpp-98d67c…/build-vk/` (built 2026-11-20);
`tools/r8q-ai-bench` runs the repro and classifies the result.

**State:** renderD128 + Turnip up on the current kernel. The compute
wedge (`a6xx_recover: cx gdsc didn't collapse`) is NOT fixed — the
single-line QUIC fix was falsified by the repro (NOTES §34); the full
7-patch "Improve GPU Recovery" series is the next step. The LLM
therefore still runs **CPU-only: 34.5 pp / 13.5 tg t/s** on
LFM3-1.7B Q4_K_M. No GPU tok/s exists yet.

## 8. Display, audio, cameras, power, sensors

- **Display:** `simpledrm` (the bootloader framebuffer) until the panel
  module loads (`panel_samsung_s6e3fc3`); 120 Hz via the DSI/DPU stack.
- **Audio:** CS35L41 smart amps over I2C/SPI + the LPASS macros;
  the `r8q-liveness` unit checks them at boot.
- **Cameras:** `qcom_camss` + the hi847 sensor (module-loaded).
- **Power:** `max77705` charger + `max17042` battery FG (**modules**;
  the FG is on the deferred list with a known missing-IRQ issue
  (`0-0036 simple-mfd-i2c: can't get irq`) — battery percentage reads
  once fixed). Samsung PMIC quirks (WTSR/SMPL) do not exist on this
  Qualcomm-PMIC board (NOTES §33).
- **Sensors/Bluetooth:** bluetooth + btbcm over hci_uart; the SLPI
  sensors remain a bring-up item.

## 9. The tool map

| Tool | Does |
|---|---|
| `tools/r8q-kernel` | build with `DEVICE=r8q` + verify the config/built-ins |
| `tools/r8q-boot pack --dual-boot [--overlay NAME]` | pack the recovery image (with the built-in assertions) |
| `tools/r8q-modules [--reboot]` | build/push/install the kernel modules; refuses version mismatches |
| `tools/r8q-test-image IMAGE` | flash over SSH → reboot → wait for the gadget → fix the host NCM → verify (kernel/UFS/gadget) → log + verdict |
| `tools/r8q-ai-bench` | the Adreno wedge repro with a PASS/WEDGE/CPU-ONLY verdict |
| `tools/r8q-push FILE REMOTE` | chunked, crash-safe file push (the USB writeback wedge guard) |
| `tools/r8q-merge-main --pass-sha` | the stable-merge gate (BRANCHING.md) |
| `tools/r8q-flash-recovery [--stable]` | restore the pinned stable bytes |
| `tools/r8q-ai` | the on-device LLM supervisor control |
| `tools/phone-ssh` / `phone-smoke` | the SSH driver / the read-only health report |

## 10. The known-bad list (do not re-learn these)

1. A kernel built without `DEVICE=r8q` boots to "0 disks" — always
   build through `tools/r8q-kernel`.
2. A CPU read of `0x17a10040` (the GICA doorbell page) can power the
   phone off during kernel start — the address is data, never memory.
3. The `msm` GPU module cannot be hot-loaded — reboot after a modules
   push.
4. The modem image boots intermittently until the §33 state co-factor
   is understood — keep the ablated image for daily use.
5. The flaky USB cable: replug both ends before blaming the phone.
6. The GPU compute wedge is unfixed — the LLM stays CPU-only until
   the full QUIC series lands.
