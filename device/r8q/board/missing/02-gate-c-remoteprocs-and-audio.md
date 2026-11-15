<!-- SPDX-License-Identifier: MIT -->

# Missing 02: Gate C — Remoteprocs (ADSP/CDSP/SLPI) & Stereo Audio

- **Priority:** P2 ([RUNBOOK-DISPLAY-GPU.md](../../RUNBOOK-DISPLAY-GPU.md), Gate C).
- **Back:** [MISSING.md](../MISSING.md) · [Board Overview](../README.md)

## Remaining Work (One Overlay Per Boot)
1. **`remoteprocs.dtso` (`M4.3`):** Enable `adsp`, `cdsp`, and `slpi` remoteprocs; bring up `qrtr-ns`, `pd-mapper`, and `tqftpserv`. Verify firmware loads without GPI DMA / IOMMU faults.
2. **`audio-amps.dtso` (`M5.3`):** Enable `gpi_dma1` and `i2c11@a8c000` for dual `cs35l41@40` / `cs35l41@41` speaker amplifiers; confirm both I2C devices bind without DC/thermal faults.
3. **`audio-sound.dtso` (`M5.3`):** Enable `TERTIARY_TDM_RX_0` (`sound/soc/qcom/sm8250.c` TDM ops in `8e889264e`) and test ALSA UCM2 (`Samsung-Galaxy-S20-FE`) via `aplay -l` + low-volume `-20 dBFS` `speaker-test`.
4. Silence benign `dsi0_phy_pll` rcg2 clock warning in `dispcc` once audio lands.
