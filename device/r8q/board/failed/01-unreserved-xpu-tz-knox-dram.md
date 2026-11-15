<!-- SPDX-License-Identifier: MIT -->

# Failed 01: Unreserved Samsung XPU / TZ / Knox TIMA DRAM

- **Symptom:** Gate D (`Hyprland` + `quickshell` under 3D/I/O load) froze at `t+12..14s`; SSH banner exchange stalled before a ~2 min watchdog reset.
- **Back:** [FAILED.md](../FAILED.md) · [Board Overview](../README.md)

## Root Cause (`console-ramoops-0`)
- Upstream `sm8250.dtsi` generic `reserved-memory` differed from Samsung `r8q` TZ XPU/hypervisor carveouts (and `cdsp_secure_heap` at `0x8bf00000` overlapped `gpu_mem` ZAP `0x9031a000`).
- Nine unreserved Samsung `no-map` regions (`5,880` 4 KiB pages = `23,520 KiB`: `xbl_aop_mem` upper part, `reserved_xbl_uefi_log`, `removed_mem` gaps, `cdsp_secure_heap` upper part `0x90500000..0x90a00000`, `dfps_data_mem`, `sec_debug_mem`, `ss_plog_mem`, `kaslr_mem`, `tima_mem` Knox hypervisor) remained mapped in `System RAM`.
- `mmc0` ADMA (`ADMA Ptr: 0x90800210` inside `cdsp_secure_heap`) / buddy allocators hit TrustZone XPU protection, wedging CPU 0 -> `rcu_preempt` stalls on CPUs `0,2,3,6,7` + `rpmh_write_batch -110` on pending GIC IRQ 37.

## Permanent Rule
- Keep all 10 Samsung `no-map` nodes in `sm8250-samsung-common.dtsi` (`1324fe066`).
- Keep `offline_unreserved_xpu_pages()` in `packages/omamobile-device-r8q/r8q-liveness-monitor` as an idempotent boot-time guard when running an older DTB.
