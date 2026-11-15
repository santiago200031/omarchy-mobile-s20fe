<!-- SPDX-License-Identifier: MIT -->

# Failed 07: Upstream `stmfts5` 8-Byte Events Dropped `r8q` `fts5cu56a` 16-Byte Touch Packets

- **Symptom:** Touchscreen glass taps fired `stmfts_irq` (GPIO 15) and `Hyprland` held `/dev/input/event2` (`Touch Device: stmfts`), yet no cursor/tap movement occurred in the UI. `chip_id`/`fw_ver` read `0x0`, and `dmesg` logged alternating pairs `stmfts 5-0049: unknown FTS5 event 0x80` (`0x84`/`0x88`/`0xc0`) and `unknown FTS5 event 0x1` (`0x2`).
- **Back:** [FAILED.md](../FAILED.md) · [Board Overview](../README.md)

## Root Cause
- **Hardware / Downstream Protocol:** `touchscreen@49` on `i2c5` (`994000.i2c`) is Samsung's `fts5cu56a` controller (`tsp_stm/fts5cu56a_r8.bin`), wired in DT with `compatible = "st,stmfts5"`. Disassembling `fts_event_handler_type_b` (`0xcd8d00`) in the LineageOS `r8q` kernel proved Samsung FTS5 Type-B reads **16-byte packets** via `0x60` (`FTS_READ_ONE_EVENT`) + `0x61` (`FTS_READ_ALL_EVENT`, `data[7] & 0x1f` pending count), where:
  - `eid = event[0] & 0x03` (`0` = coordinate, `1` = status/ready `0x09 0x00 0x10`, `2` = vendor, `3` = error)
  - `tchsta = event[0] >> 6` (`1` = press, `2` = move, `3` = release), `slot_id = (event[0] >> 2) & 0x0f`
  - `x = (event[1] << 4) | (event[3] >> 4)`, `y = (event[2] << 4) | (event[3] & 0x0f)`
  - `maj = event[4]`, `min = event[5]`, `area = event[6] & 0x3f`
- **Upstream `stmfts5_parse_events()` Mismatch:** Assumed 8-byte events (`STMFTS_EVENT_SIZE = 8`) keyed on `event[0] & 0xf0` (`0x10`/`0x20`/`0x30`), splitting each 16-byte Samsung packet in half: byte `0` (`0x40` press silently ignored as `STATUS_UPDATE`; `0x80`/`0xc0` logged unknown) and byte `8` (`0x01`/`0x02` logged unknown). Additionally, `994000.i2c` had `power/control="auto"` (`suspended` between touches).

## Permanent Rule
- Use `stmfts5_read_events()` (`0x60`/`0x61` 16-byte block read with `0x86` fallback) and `stmfts5_parse_type_b_events()` (kernel commit `5866f774c` on `omamobile-r8q`, live backport module `stmfts-r8q.ko` for `gd469bd3bf2a9` managed by `r8q-liveness-monitor`).
- Keep `994000.i2c` and `5-0049` pinned to `power/control="on"` in `80-r8q-gpu.rules` and `r8q-liveness-monitor` (`0.1.17`).
