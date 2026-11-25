#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Sahara MEMORY_DEBUG (mode=2) crash-dump walker for the r8q (SDX55M) modem.

Reconstruction (2026-11-20) of the lost run17_diag.py walker, from the live
mode=2 records of runs 17-23 (logs/r8q/run21-sahara.log, run21d-sahara.log,
run22-sahara.log: the 32-entry SBL crashdload table, is64=False, 37 files /
2875204 bytes served end-to-end), the upstream Linux Sahara protocol
reference (drivers/accel/qaic/sahara.c, GPL-2.0-only, Qualcomm) and
device/r8q/MODEM-PLAN.md sections 925-934 and 13. The lost tool's own
scratch generation (never committed, recovered on the bring-up laptop)
pinned the exact function signatures and the walker behavior reproduced
here, including the apps.mbn ULog/context extras.

INTERFACE CONTRACT with tools/r8q-sahara
========================================

tools/r8q-sahara loads this file at its first mode=2 (MEMORY_DEBUG) HELLO,
from os.environ["R8Q_MEMDUMP_MODULE"] (default /usr/local/bin/run17_diag.py),
and calls exactly one entry point when the modem sends the debug table:

    files_saved, bytes_saved = diag.dump_memory_table(port, is64, rest, dump_dir)

    port      open fd of /dev/wwan0sahara0 (plain os.read/os.write/select)
    is64      True for MEMORY_DEBUG64 (cmd 0x10), False for MEMORY_DEBUG
              (cmd 9); decides the table entry width and read command
    rest      the MEM_DEBUG packet payload AFTER the 8-byte Sahara header:
              32-bit "<II" (table address, table length), 64-bit "<QQ"
    dump_dir  every file is written under this directory (r8q-sahara's
              --dump-dir, default /tmp/sdx55-crash)
    return    (files_saved, bytes_saved) as ints. files_saved counts every
              file written (raw regions plus the table/manifest metadata);
              bytes_saved counts only target memory saved to files. The
              recorded SBL-phase session yields 37 files / 2875204 bytes
              (38 files once crash-reasons.txt is added).

sahara_mem_read(fd, is64, addr, length) -> bytes is exported for reuse and is
what dump_memory_table itself reads with. Exceptions never escape
dump_memory_table: r8q-sahara has no handler around the call, and a lost
session must not kill the loader. The HELLO mode=2 accept itself (HELLO_RESP
mode=2) stays in r8q-sahara; nothing else is needed from this module there.

Session close, and why this module never sends RESET
=====================================================

Per the task contract this walker closes with DONE and a bounded DONE_RESP
wait (<= DONE_WAIT_SEC), leaving NO modem reset: it never sends RESET and
never resets the target. r8q-sahara sends its own RESET after this walker
returns - the recorded session end on this device (run21d: dump complete ->
"sending RESET" -> RESET_RESP -> clean normal boot). If the target does not
answer DONE, the wait expires without failing and the loader's RESET lands
exactly as recorded. Set R8Q_MEMDUMP_DONE_CLOSE=0 to skip the DONE close
entirely (the bare recorded run21d shape: return silently, loader RESETs).

Protocol decisions, with sources
=================================

- Direction: in the dump flow the HOST sends MEMORY_READ (cmd 0xa, 32-bit,
  16-byte packet) / MEMORY_READ64 (cmd 0x11, 24-byte) with the target
  address and length, and the TARGET answers with the raw data, no header
  (qaic sahara.c:502-508 "the protocol flips", 524-534, 669-679).
- A read the target cannot serve is answered with END_OF_IMAGE (16 bytes,
  status) instead of data (qaic sahara.c:740-755); the walker detects that,
  logs the status, and keeps the rest of the dump (the lost original would
  have hung a whole-chunk read into its 15 s timeout and aborted).
- Read requests are never exactly END_OF_IMAGE_LENGTH bytes, so an error
  response is identifiable (qaic sahara.c:663-665, 711-713); a 16-byte
  region is split into 8+8.
- The debug table: nonzero length, a whole multiple of the entry size
  (qaic sahara.c:495-500). 64-bit entries are type/address/length u64 plus
  description[20]/filename[20] = 64 bytes (sahara.c:99-105); the 32-bit
  entries this SDX55M actually sends are the u32 form = 52 bytes
  (live record: 1664-byte table = 32 x 52).
- Region reads are capped at MAX_REGION_BYTES (4 MiB): the recorded table
  lists "DDR CS0" at 256 MiB, which the crashdload client does not need and
  the ~38 s MEMORY_DEBUG deadline (MODEM-PLAN 925-934: an unresponded offer
  PS_HOLD warm-resets the modem) cannot afford anyway.
- The DONE/DONE_RESP close follows this repository's task contract; qaic
  itself closes with host RESET (sahara.c:274-289, 703), which is exactly
  what r8q-sahara does after this walker returns.

The ULog linked-list walk and the targeted DDR slices (apps.mbn build
addresses below) are the recorded run17-era behavior that r8q-sahara's
MEM_DEBUG branch documents ("walks the ULog list, mhi/pcie contexts, smem
and tz_data"). In the SBL-phase stale dumps they read as zero and no-op
gracefully (run21d record); in a live AMSS-phase session they carry the
runtime logs and contexts.

SAFETY
======

Read-only toward the modem: this module only ever sends MEMORY_READ(64) and
DONE (plus HELLO_RESP in its own standalone server mode) - never RESET, no
image data, nothing that changes modem state. All files are written under
the caller's dump_dir only; no UFS partition is opened writable (the
standalone server opens image sources O_RDONLY like tools/r8q-sahara).
Raw region files are NOT redacted: never commit or share the dump dir
(repository rule: no serials/IMEIs in public copy). The ASCII string
extraction logged here and written to crash-reasons.txt IS redacted the
same way the offline decoder redacts (ChipSerial / Serial Number / PK_HASH
/ OEM Secure Boot / IMEI / MEID / ESN lines are replaced).
"""

import argparse
import errno
import json
import os
import re
import select
import struct
import sys
import threading
import time

# --- Sahara v2 protocol (see tools/r8q-sahara and drivers/accel/qaic/sahara.c) ---
HELLO_CMD = 0x1
HELLO_RESP_CMD = 0x2
READ_DATA_CMD = 0x3
END_OF_IMAGE_CMD = 0x4
DONE_CMD = 0x5
DONE_RESP_CMD = 0x6
RESET_CMD = 0x7
RESET_RESP_CMD = 0x8
MEM_DEBUG_CMD = 0x9          # memory-debug table, 32-bit addressing
MEM_READ_CMD = 0xa           # host -> target: read target memory, 32-bit
MEM_DEBUG64_CMD = 0x10       # memory-debug table, 64-bit addressing
MEM_READ64_CMD = 0x11        # host -> target: read target memory, 64-bit
READ_DATA64_CMD = 0x12       # 64-bit image read (image-transfer mode only)

HELLO_LENGTH = 0x30          # 48 bytes on the wire (24 used + 24 reserved)
READ_DATA_LENGTH = 0x14      # 20 bytes
END_OF_IMAGE_LENGTH = 0x10   # 16 bytes
DONE_LENGTH = 0x8            # header only
MEM_DEBUG_LENGTH = 0x10      # cmd + length + table address + table length
MEM_READ_LENGTH = 0x10       # cmd + length + memory address + memory length
MEM_DEBUG64_LENGTH = 0x18    # qaic sahara.h SAHARA_MEM_DEBUG64_LENGTH
MEM_READ64_LENGTH = 0x18     # qaic sahara.h SAHARA_MEM_READ64_LENGTH

SAHARA_VERSION = 2
SAHARA_SUCCESS = 0
MODE_MEMORY_DEBUG = 0x2

# mhi_wwan_ctrl caps a single write at MHI_WWAN_MAX_MTU (0x8000); reads of
# the target's raw data answer are looped by read_exact, so reads chunk at
# a larger size (the recorded run21d session served 2.87 MB this way).
CHUNK = 0x10000
SEND_CHUNK = 0x8000

# Never request exactly END_OF_IMAGE_LENGTH bytes, so a target error reply
# (a full 16-byte EOI packet) stays identifiable (qaic sahara.c:663-665).
EOI_LENGTH = END_OF_IMAGE_LENGTH

# Debug-table sanity: a table bigger than this is nonsense (>16k entries).
MAX_TABLE_BYTES = 1024 * 1024
# Region size cap: the recorded table lists "DDR CS0" at 0x10000000 (256 MiB);
# the crash regions that matter are all far below 4 MiB.
MAX_REGION_BYTES = 4 * 1024 * 1024

# Per-read timeout (recorded run17 value); the whole session must stay well
# inside the ~38 s MEMORY_DEBUG deadline of MODEM-PLAN 925-934.
READ_TIMEOUT = 15.0
# Bounded wait for the target's DONE_RESP before returning to the caller.
DONE_WAIT_SEC = 5.0

# apps.mbn (SDX55M r8q) build addresses, from the run17-era reverse
# engineering of the AMSS image. In the SBL-phase stale dumps these read as
# zero and the extras no-op gracefully (run21d record).
DDR_LO = 0x80000000
DDR_HI = 0x90000000
ULOG_CONTEXT_HEAD_VA = 0x80F7F320
PCIE_OSAL_LOG_CTX_VA = 0x8097685C
MHI_CTX_PTR_VA = 0x80973BD8
PCIE_CTX_PTR_VA = 0x80976AF0
TZ_DATA_VA = 0x8FF8E000
TZ_DATA_LEN = 0x58000
SMEM_HEAD_VA = 0x80000000
SMEM_HEAD_LEN = 0x200000

# Standalone-server image map (recorded run17 set): only the crashdload
# pre-dump images are served; any other image request means the SBL is
# proceeding to a normal boot, which tools/r8q-sahara owns.
FIRMWARE_DIR = "/lib/firmware/qcom/sdx55m"
PRE_DUMP_IMAGE_MAP = {
    9:  ("mdmddr", "part:mdmddr"),
    34: ("mdmddr.mbn", "part:mdmddr"),
    40: ("apdp.mbn", "file:apdp.mbn"),
}
# Known-but-not-pre-dump image ids, for the "the SBL is booting normally"
# exit (the recorded run17 distinction: exit 2 = unknown, exit 3 = normal
# boot the standalone listener must not serve).
KNOWN_IMAGE_NAMES = {
    6: "apps.mbn", 8: "qdsp6sw.mbn", 21: "sbl1.mbn", 23: "aop.mbn",
    25: "tz.mbn", 29: "acdb.mbn", 33: "hyp.mbn", 36: "multi_image_qti.mbn",
    38: "xbl_cfg.elf", 41: "devcfg.mbn", 42: "sec.elf",
}

# ASCII crash-reason scan: highlight lines that talk about a failure, and
# redact identity fields before anything is logged or written.
CRASH_KEYWORDS = (
    "ERR", "FATAL", "SSR", "ASSERT", "PANIC", "REASON", "CRASH",
    "RESET", "WDOG", "WATCHDOG", "NOC", "SDI", "PON", "ABORT",
    "TIMEOUT", "FAIL",
)
REDACT_KEYS = ("ChipSerial", "Serial Number", "PK_HASH", "OEM Secure Boot",
               "IMEI", "MEID", "ESN")
# Cap the per-region stdout listing; crash-reasons.txt keeps every hit.
MAX_LOGGED_STRINGS_PER_REGION = 12
STRING_MIN_LEN = 8

_STRING_RE = re.compile(rb"[ -~]{%d,}" % STRING_MIN_LEN)


def log(msg):
    print("[MEMDUMP %9.3f] %s" % (time.monotonic(), msg), flush=True)


class SaharaTargetError(Exception):
    """The target refused a memory read (EOI) or closed mid-session."""


def env_flag(name, default=True):
    val = os.environ.get(name)
    if val is None:
        return default
    return val.strip().lower() not in ("0", "false", "no", "off")


# ---------------------------------------------------------------------------
# Low-level channel I/O (plain fd, same approach as tools/r8q-sahara)
# ---------------------------------------------------------------------------

def read_exact(fd, n, timeout=READ_TIMEOUT):
    """Read exactly n bytes from the Sahara fd or raise TimeoutError/EOFError."""
    buf = bytearray()
    deadline = time.monotonic() + timeout
    while len(buf) < n:
        rem = deadline - time.monotonic()
        if rem <= 0:
            raise TimeoutError("timeout reading %d bytes (got %d)" % (n, len(buf)))
        r, _, _ = select.select([fd], [], [], rem)
        if not r:
            continue
        chunk = os.read(fd, n - len(buf))
        if not chunk:
            raise EOFError("EOF on sahara port")
        buf.extend(chunk)
    return bytes(buf)


def send_all(fd, data):
    view = memoryview(data)
    while view:
        n = os.write(fd, view[:SEND_CHUNK])
        if n <= 0:
            raise IOError("write returned %d" % n)
        view = view[n:]


def sahara_mem_read(fd, is64, addr, length):
    """Read `length` bytes of target memory over MEMORY_READ/READ64.

    Host -> target read request, target -> host raw data (qaic sahara.c:502-
    508). Chunked at 64 KiB; a 16-byte request is split in two so an EOI
    error reply stays identifiable (qaic sahara.c:663-665). Raises
    SaharaTargetError if the target answers a read with END_OF_IMAGE.
    """
    data = bytearray()
    pos = 0
    while pos < length:
        chunk_len = min(CHUNK, length - pos)
        if chunk_len == EOI_LENGTH:
            chunk_len = EOI_LENGTH // 2
        if is64:
            send_all(fd, struct.pack("<IIQQ", MEM_READ64_CMD,
                                     MEM_READ64_LENGTH, addr + pos, chunk_len))
        else:
            send_all(fd, struct.pack("<IIII", MEM_READ_CMD,
                                     MEM_READ_LENGTH, addr + pos, chunk_len))
        data.extend(_read_mem_response(fd, chunk_len))
        pos += chunk_len
    return bytes(data)


def _read_mem_response(fd, expect_len):
    """Read one raw-data response to a MEMORY_READ request.

    The response has no header; only a target error arrives framed, as a full
    END_OF_IMAGE packet (qaic sahara.c:740-755). Sniff the first 8 bytes for
    that frame; the odds of real data starting with exactly
    04 00 00 00 10 00 00 00 are 2^-64 per request, the same tradeoff the
    kernel driver makes by size alone.
    """
    if expect_len < 8:
        # Too short to carry (or be confused with) an EOI header; a target
        # that errors on such a read answers a 16-byte EOI, and the first
        # bytes of it would be taken as data - the same tradeoff qaic makes
        # for every size it cannot check.
        return read_exact(fd, expect_len)
    first = read_exact(fd, 8)
    cmd, length = struct.unpack("<2I", first)
    if cmd == END_OF_IMAGE_CMD and length == END_OF_IMAGE_LENGTH:
        rest = read_exact(fd, 8)
        image, status = struct.unpack("<2I", rest)
        raise SaharaTargetError(
            "target refused the read: END_OF_IMAGE image=%d status=%d"
            % (image, status))
    if cmd == RESET_CMD:
        raise SaharaTargetError("target sent RESET mid-session")
    if expect_len == 8:
        return first
    return first + read_exact(fd, expect_len - 8)


# ---------------------------------------------------------------------------
# ASCII crash-reason extraction (redacted)
# ---------------------------------------------------------------------------

def redact_string(s):
    """Redact identity fields in an extracted string (see module header)."""
    for key in REDACT_KEYS:
        if key in s:
            head = s.split(":", 1)[0].split("=", 1)[0]
            return "%s: <REDACTED>" % head
    return s


def extract_ascii_strings(data):
    """(offset, redacted string) for every printable run >= STRING_MIN_LEN."""
    out = []
    for m in _STRING_RE.finditer(data):
        text = m.group().strip()
        if text:
            out.append((m.start(), redact_string(text.decode("ascii", "replace"))))
    return out


def scan_region(label, data, hits):
    """Log the crash-reason hits of one dumped blob; accumulate all hits.

    A hit is a crash-keyword match OR a redacted identity field (the stub
    "<REDACTED>" proves the value itself never reaches any log or file).
    """
    strings = extract_ascii_strings(data)
    found = [(off, s) for off, s in strings
             if any(k in s.upper() for k in CRASH_KEYWORDS)
             or "<REDACTED>" in s]
    if not strings:
        return
    log("%s: %d strings >=%d chars, %d crash-keyword hits"
        % (label, len(strings), STRING_MIN_LEN, len(found)))
    for off, s in found[:MAX_LOGGED_STRINGS_PER_REGION]:
        log("  %s+0x%x: %s" % (label, off, s))
    if len(found) > MAX_LOGGED_STRINGS_PER_REGION:
        log("  %s: ... %d more hits in crash-reasons.txt"
            % (label, len(found) - MAX_LOGGED_STRINGS_PER_REGION))
    for off, s in found:
        hits.append((label, off, s))


def write_crash_reasons(outdir, hits):
    path = os.path.join(outdir, "crash-reasons.txt")
    with open(path, "w") as f:
        f.write("# Sahara MEMORY_DEBUG crash-reason hits (redacted)\n")
        for label, off, s in hits:
            f.write("%s+0x%x: %s\n" % (label, off, s))
    return path


# ---------------------------------------------------------------------------
# Debug-table parsing
# ---------------------------------------------------------------------------

def parse_debug_table(blob, is64):
    """Parse the memory-debug table blob into (type, addr, len, desc, file).

    64-bit entry: <QQQ20s20s, 64 bytes (qaic sahara.c:99-105). 32-bit entry:
    <III20s20s, 52 bytes (live record: 1664-byte table = 32 x 52).
    """
    esize, fmt = (64, "<QQQ20s20s") if is64 else (52, "<III20s20s")
    entries = []
    for off in range(0, len(blob) - esize + 1, esize):
        typ, addr, rlen, desc_b, fname_b = struct.unpack(fmt, blob[off:off + esize])
        desc = desc_b.split(b"\0", 1)[0].decode("ascii", "replace")
        fname = fname_b.split(b"\0", 1)[0].decode("ascii", "replace")
        if rlen == 0 and addr == 0:
            continue
        entries.append((typ, addr, rlen, desc, fname))
    return entries


# ---------------------------------------------------------------------------
# The walker r8q-sahara calls
# ---------------------------------------------------------------------------

def _is_efs_table(entries):
    """True when the MEMORY_DEBUG table carries only runtime m9kefs writebacks.

    apps.mbn's remotefs_sahara (MHI ch 10/11 EFS) serves only the m9kefs buffer
    and asserts (remotefs_sahara.c:850) on out-of-range crash-dump extra reads or
    a DONE packet; it expects RESET directly after reading the EFS region.
    """
    return bool(entries) and all(
        os.path.basename(fname or "").startswith("m9kefs")
        for _, _, _, _, fname in entries
    )


def dump_memory_table(fd, is64, payload, outdir):
    """Serve one Sahara MEMORY_DEBUG session: table, regions, extras, DONE.

    Called by tools/r8q-sahara as:
        files_saved, bytes_saved = diag.dump_memory_table(port, is64, rest, dump_dir)
    Never raises; returns the partial counts on failure. Reads every region
    of the table (<= MAX_REGION_BYTES), walks the apps.mbn ULog list, saves
    the mhi/pcie contexts, smem_head and tz_data, then closes with DONE and
    a bounded DONE_RESP wait - no RESET is ever sent from here.
    """
    files_saved = 0
    bytes_saved = 0
    hits = []
    state = {"dead": False, "fails": 0}
    os.makedirs(outdir, exist_ok=True)
    try:
        need = 16 if is64 else 8
        if len(payload) < need:
            log("malformed MEM_DEBUG payload: %d bytes (need %d)"
                % (len(payload), need))
            return 0, 0
        if is64:
            taddr, tlen = struct.unpack("<QQ", payload[:16])
        else:
            taddr, tlen = struct.unpack("<II", payload[:8])
        esize = 64 if is64 else 52
        if tlen == 0 or tlen % esize != 0:
            log("malformed debug table length %d (entry size %d)" % (tlen, esize))
            return 0, 0
        if tlen > MAX_TABLE_BYTES:
            log("debug table %d bytes exceeds the %d cap - not reading it"
                % (tlen, MAX_TABLE_BYTES))
            return 0, 0

        log("Debug table is64=%s addr=0x%x len=%d" % (is64, taddr, tlen))
        table = _guarded_read(fd, is64, taddr, tlen, state)
        if table is None:
            return files_saved, bytes_saved
        entries = parse_debug_table(table, is64)
        table_meta = []
        for typ, addr, rlen, desc, fname in entries:
            log("  entry type=0x%x addr=0x%x size=0x%x (%d) desc=%r file=%r"
                % (typ, addr, rlen, rlen, desc, fname))
            table_meta.append({"type": typ, "addr": addr, "size": rlen,
                               "desc": desc, "file": fname})
        with open(os.path.join(outdir, "table.json"), "w") as tf:
            json.dump(table_meta, tf, indent=2)
        files_saved += 1

        # Every region up to the size cap ("DDR CS0" at 256 MiB is skipped;
        # the crash reasons live in the small regions).
        used_names = set()
        for idx, (typ, addr, rlen, desc, fname) in enumerate(entries):
            if state["dead"]:
                log("session dead: skipping %r and the remaining regions" % fname)
                break
            if rlen > MAX_REGION_BYTES:
                log("skipping %r (%d bytes > %d cap)"
                    % (fname, rlen, MAX_REGION_BYTES))
                continue
            base = _region_filename(fname, idx, addr, used_names)
            outpath = os.path.join(outdir, base)
            log("reading %r (%d bytes)..." % (fname, rlen))
            try:
                data = sahara_mem_read(fd, is64, addr, rlen)
            except (SaharaTargetError, EOFError, OSError) as e:
                _note_read_failure(state, fname, e)
                continue
            state["fails"] = 0
            with open(outpath, "wb") as wf:
                wf.write(data)
            files_saved += 1
            bytes_saved += len(data)
            log("saved %s (%d bytes)" % (outpath, len(data)))
            scan_region(base, data, hits)

        efs_only = _is_efs_table(entries)
        # Targeted DDR slices: apps.mbn ULog list, contexts, smem, tz_data.
        if efs_only:
            log("EFS writeback table detected; skipping crash-dump extras")
        elif env_flag("R8Q_MEMDUMP_EXTRAS", default=True):
            files_saved, bytes_saved = _dump_extras(
                fd, is64, outdir, state, hits, files_saved, bytes_saved)
        else:
            log("extras disabled (R8Q_MEMDUMP_EXTRAS=0)")
    except (SaharaTargetError, EOFError, OSError) as e:
        log("WARN dump aborted: %s" % e)
    finally:
        try:
            if files_saved > 0:
                path = write_crash_reasons(outdir, hits)
                files_saved += 1
                log("crash-reason hits written to %s" % path)
        except OSError as e:
            log("WARN could not write crash-reasons.txt: %s" % e)
        if not locals().get("efs_only", False):
            _close_session(fd)
        else:
            log("EFS writeback table detected; skipping DONE (caller sends RESET)")
    return files_saved, bytes_saved


def _guarded_read(fd, is64, addr, length, state):
    """sahara_mem_read with the dead-session policy applied."""
    if state["dead"]:
        return None
    try:
        return sahara_mem_read(fd, is64, addr, length)
    except (SaharaTargetError, EOFError, OSError) as e:
        _note_read_failure(state, "0x%x (+%d bytes)" % (addr, length), e)
        return None


def _note_read_failure(state, name, exc):
    state["fails"] += 1
    log("WARN read of %s failed: %s" % (name, exc))
    if state["fails"] >= 2 and not state["dead"]:
        state["dead"] = True
        log("target stopped serving reads; not attempting further regions")


def _region_filename(fname, idx, addr, used_names):
    base = os.path.basename(fname or "").strip()
    base = re.sub(r"[^A-Za-z0-9._-]", "_", base)
    if not base:
        base = "region_%02d_%08x.bin" % (idx, addr)
    if base in used_names:
        alt = "%s.%02d" % (base, idx)
        log("WARNING duplicate region name %r; saving as %r" % (base, alt))
        base = alt
    used_names.add(base)
    return base


def _dump_extras(fd, is64, outdir, state, hits, files_saved, bytes_saved):
    """ULog walk, mhi/pcie contexts, smem_head, tz_data (recorded extras).

    All best-effort: a failed extra is logged and skipped, never fatal. The
    fixed addresses are apps.mbn build values; in SBL-phase stale dumps they
    read zero and no-op (run21d record).
    """
    extra_manifest = {}

    def save_blob(fname, blob):
        with open(os.path.join(outdir, fname), "wb") as wf:
            wf.write(blob)
        scan_region(fname, blob, hits)

    try:
        if not state["dead"]:
            pcie_log_ctx = sahara_mem_read(fd, is64, PCIE_OSAL_LOG_CTX_VA, 64)
            save_blob("pcie_osal_log_ctx.bin", pcie_log_ctx)
            files_saved += 1
            bytes_saved += len(pcie_log_ctx)
            pcie_ulog_ptr = struct.unpack_from("<I", pcie_log_ctx, 0x0C)[0]
            extra_manifest["pcie_osal_log_ctx_va"] = PCIE_OSAL_LOG_CTX_VA
            extra_manifest["pcie_ulog_ptr"] = pcie_ulog_ptr
            log("pcie_osal_log_ctx saved (pcie_ulog_ptr=0x%08x)" % pcie_ulog_ptr)
    except (SaharaTargetError, EOFError, OSError) as e:
        extra_manifest["pcie_osal_log_ctx_err"] = str(e)
        log("WARN pcie_osal_log_ctx failed: %s" % e)

    ulog_manifest = []
    seen_nodes = set()
    try:
        head_raw = _guarded_read(fd, is64, ULOG_CONTEXT_HEAD_VA, 4, state)
        if head_raw is None:
            extra_manifest["ulog_walk_err"] = "session dead"
        else:
            head = struct.unpack("<I", head_raw)[0]
            extra_manifest["ulog_context_head_va"] = ULOG_CONTEXT_HEAD_VA
            extra_manifest["ulog_head"] = head
            log("ulogContext.logHead = 0x%08x" % head)
            cur = head
            idx = 0
            while (not state["dead"] and idx < 64
                   and DDR_LO <= cur <= DDR_HI - 0x80 and cur not in seen_nodes):
                seen_nodes.add(cur)
                nxt, bsz = _dump_ulog_node(
                    fd, is64, cur, idx, outdir, state, hits, ulog_manifest)
                if ulog_manifest[-1]["file"]:
                    files_saved += 1
                    bytes_saved += bsz
                idx += 1
                if nxt == 0:
                    break
                cur = nxt
    except (SaharaTargetError, EOFError, OSError) as e:
        extra_manifest["ulog_walk_err"] = str(e)
        log("WARN ulog linked-list walk failed: %s" % e)

    try:
        if (DDR_LO <= extra_manifest.get("pcie_ulog_ptr", 0) <= DDR_HI - 0x80
                and extra_manifest.get("pcie_ulog_ptr", 0) not in seen_nodes
                and not state["dead"]):
            seen_nodes.add(extra_manifest["pcie_ulog_ptr"])
            _, bsz = _dump_ulog_node(fd, is64, extra_manifest["pcie_ulog_ptr"],
                                     len(ulog_manifest), outdir, state, hits,
                                     ulog_manifest)
            if ulog_manifest[-1]["file"]:
                files_saved += 1
                bytes_saved += bsz
    except (SaharaTargetError, EOFError, OSError) as e:
        extra_manifest["pcie_ulog_extra_err"] = str(e)
        log("WARN pcie_ulog_ptr extra dump failed: %s" % e)

    with open(os.path.join(outdir, "ulog_manifest.json"), "w") as uf:
        json.dump(ulog_manifest, uf, indent=2)
    files_saved += 1

    for label, ptr_va, size, fname in (
            ("mhi_ctx", MHI_CTX_PTR_VA, 0x2000, "mhi_ctx.bin"),
            ("pcie_ctx", PCIE_CTX_PTR_VA, 0x10000, "pcie_ctx.bin")):
        try:
            ptr_raw = _guarded_read(fd, is64, ptr_va, 4, state)
            if ptr_raw is None:
                extra_manifest["%s_skipped" % label] = "session dead"
                continue
            ptr = struct.unpack("<I", ptr_raw)[0]
            extra_manifest["%s_ptr" % label] = ptr
            if DDR_LO <= ptr <= DDR_HI - size and not state["dead"]:
                blob = sahara_mem_read(fd, is64, ptr, size)
                save_blob(fname, blob)
                files_saved += 1
                bytes_saved += len(blob)
                log("saved %s (%d bytes from 0x%08x)" % (fname, len(blob), ptr))
            else:
                log("%s_ptr=0x%08x out of bounds for size 0x%x" % (label, ptr, size))
        except (SaharaTargetError, EOFError, OSError) as e:
            extra_manifest["%s_err" % label] = str(e)
            log("WARN %s failed: %s" % (label, e))

    # smem_head first, then tz_data (last, in case the TZ region is
    # access-restricted - recorded ordering).
    for label, addr, size, fname in (
            ("smem_head", SMEM_HEAD_VA, SMEM_HEAD_LEN, "smem_head.bin"),
            ("tz_data", TZ_DATA_VA, TZ_DATA_LEN, "tz_data.bin")):
        try:
            if state["dead"]:
                extra_manifest["%s_skipped" % label] = "session dead"
                continue
            log("reading %s 0x%08x (0x%x bytes)..." % (label, addr, size))
            blob = sahara_mem_read(fd, is64, addr, size)
            save_blob(fname, blob)
            files_saved += 1
            bytes_saved += len(blob)
            extra_manifest["%s_bytes" % label] = len(blob)
            log("saved %s (%d bytes)" % (fname, len(blob)))
        except (SaharaTargetError, EOFError, OSError) as e:
            extra_manifest["%s_err" % label] = str(e)
            log("WARN %s failed: %s" % (label, e))

    with open(os.path.join(outdir, "extra_manifest.json"), "w") as ef:
        json.dump(extra_manifest, ef, indent=2)
    files_saved += 1
    return files_saved, bytes_saved


def _dump_ulog_node(fd, is64, cur, idx, outdir, state, hits, manifest):
    """Dump one ULog node; returns (next_va, bytes_saved). Recorded layout."""
    hdr = sahara_mem_read(fd, is64, cur, 0x80)
    nxt = struct.unpack_from("<I", hdr, 0x00)[0]
    ver = struct.unpack_from("<I", hdr, 0x04)[0]
    raw_name = hdr[0x08:0x20].split(b"\0", 1)[0].decode("ascii", "replace")
    status = struct.unpack_from("<I", hdr, 0x20)[0]
    buf_addr = struct.unpack_from("<I", hdr, 0x24)[0]
    buf_size = struct.unpack_from("<I", hdr, 0x28)[0]
    buf_mask = struct.unpack_from("<I", hdr, 0x2C)[0]
    writer_core_ptr = struct.unpack_from("<I", hdr, 0x4C)[0]

    wc = b""
    wc_words = []
    if DDR_LO <= writer_core_ptr <= DDR_HI - 0x40 and not state["dead"]:
        wc = sahara_mem_read(fd, is64, writer_core_ptr, 0x40)
        wc_words = list(struct.unpack_from("<4I", wc, 0))

    safe_name = "".join(c if c.isalnum() else "_" for c in
                        (raw_name or "unnamed")).strip("_") or "unnamed"
    bin_name = None
    bytes_saved = 0
    if DDR_LO <= buf_addr < DDR_HI and 0 < buf_size <= 0x100000 and not state["dead"]:
        read_len = min(buf_size, 0x40000, DDR_HI - buf_addr)
        rbuf = sahara_mem_read(fd, is64, buf_addr, read_len)
        bin_name = "ulog_%02d_%s.bin" % (idx, safe_name)
        outpath = os.path.join(outdir, bin_name)
        with open(outpath, "wb") as wf:
            wf.write(rbuf)
        bytes_saved = len(rbuf)
        scan_region(bin_name, rbuf, hits)

    manifest.append({
        "idx": idx,
        "node_va": cur,
        "next_va": nxt,
        "version": ver,
        "name": raw_name,
        "status": status,
        "buf_addr": buf_addr,
        "buf_size": buf_size,
        "buf_mask": buf_mask,
        "writer_core_ptr": writer_core_ptr,
        "wc_words": wc_words,
        "write_idx": wc_words[0] if len(wc_words) > 0 else None,
        "read_writer_1": wc_words[1] if len(wc_words) > 1 else None,
        "read_writer_2": wc_words[2] if len(wc_words) > 2 else None,
        "read_writer_3": wc_words[3] if len(wc_words) > 3 else None,
        "file": bin_name,
        "bytes_saved": bytes_saved,
    })
    log("ULog[%d] %r node=0x%08x buf=0x%08x size=0x%x wc=%s file=%s"
        % (idx, raw_name, cur, buf_addr, buf_size, wc_words, bin_name))
    return nxt, bytes_saved


def _close_session(fd):
    """Close the memory-debug session: DONE, then a bounded DONE_RESP wait.

    Never sends RESET (tools/r8q-sahara sends its own after this walker
    returns - the recorded session end). Non-fatal in every outcome: an
    unanswered DONE just costs DONE_WAIT_SEC before the caller's RESET.
    """
    if not env_flag("R8Q_MEMDUMP_DONE_CLOSE", default=True):
        log("DONE close disabled (R8Q_MEMDUMP_DONE_CLOSE=0); "
            "returning for the caller's session end")
        return
    try:
        send_all(fd, struct.pack("<2I", DONE_CMD, DONE_LENGTH))
        log("sent DONE to close the memory-debug session (no reset from here)")
        deadline = time.monotonic() + DONE_WAIT_SEC
        while True:
            rem = deadline - time.monotonic()
            if rem <= 0:
                log("no DONE_RESP within %.0fs; returning (the loader's "
                    "RESET ends the session as recorded)" % DONE_WAIT_SEC)
                return
            r, _, _ = select.select([fd], [], [], rem)
            if not r:
                continue
            cmd, length = struct.unpack("<2I", read_exact(fd, 8, timeout=rem))
            rest = read_exact(fd, length - 8, timeout=rem) if length > 8 else b""
            if cmd == DONE_RESP_CMD:
                log("DONE_RESP received - session closed without a reset")
                return
            if cmd == DONE_CMD:
                # The device closed the session itself: answer DONE_RESP
                # (tools/r8q-sahara's DONE handler documents this direction).
                send_all(fd, struct.pack("<2I", DONE_RESP_CMD, DONE_LENGTH))
                log("DONE from target; answered DONE_RESP - session closed")
                return
            if cmd == END_OF_IMAGE_CMD:
                image, status = struct.unpack("<2I", rest)
                log("target closed with END_OF_IMAGE image=%d status=%d"
                    % (image, status))
                return
            if cmd in (RESET_CMD, RESET_RESP_CMD):
                log("target closed with RESET/RESET_RESP; not answering")
                return
            log("ignoring cmd %d while closing" % cmd)
            continue
    except (SaharaTargetError, EOFError, OSError) as e:
        log("WARN while closing: %s (nothing further sent)" % e)


# ---------------------------------------------------------------------------
# Standalone mode=2 session server (run17_diag.py memdump, recorded CLI)
# ---------------------------------------------------------------------------

def find_partition(name):
    """Resolve a UFS partition by PARTNAME in /sys/class/block/*/uevent."""
    base = "/sys/class/block"
    for entry in sorted(os.listdir(base)):
        try:
            with open(os.path.join(base, entry, "uevent"), "r") as f:
                for line in f:
                    if line.strip() == "PARTNAME=" + name:
                        return "/dev/" + entry
        except OSError:
            continue
    return None


def open_sahara_source(spec):
    """Open an image-map spec read-only (O_RDONLY; never a write)."""
    try:
        kind, val = spec.split(":", 1)
    except ValueError:
        return None
    if kind == "file":
        path = os.path.join(FIRMWARE_DIR, val)
    elif kind == "part":
        path = find_partition(val)
        if not path:
            return None
    else:
        return None
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC)
    size = os.fstat(fd).st_size
    if size == 0:
        size = os.lseek(fd, 0, os.SEEK_END)
    return path, fd, size


def find_modem_mhi_states():
    """The modem's MHI controller states file.

    The QCA6390 WLAN chip is a second, healthy MHI controller on this phone
    (mhi1) that prints the same mission-mode lines; find the modem by a
    channel name only it has (SAHARA). Never hardcode mhi0.
    """
    base = "/sys/kernel/debug/mhi"
    try:
        controllers = sorted(os.listdir(base))
    except OSError:
        return None
    for ctl in controllers:
        try:
            with open(os.path.join(base, ctl, "channels"), "r") as f:
                if "SAHARA" in f.read():
                    return os.path.join(base, ctl, "states")
        except OSError:
            continue
    return None


def wait_for_mission_mode(timeout):
    """Wait until the modem's MHI controller reports MISSION MODE.

    With the SBL-only channel mask, the SBL -> AMSS transition destroys and
    re-creates the SAHARA port: the standalone listener must not grab the
    port before mission mode, or it serves (and rejects) the SBL image
    session instead of the in-AMSS memory-debug offer.
    """
    deadline = time.monotonic() + timeout
    path = find_modem_mhi_states() or "/sys/kernel/debug/mhi/mhi0/states"
    last = None
    log("watching %s for MISSION MODE (up to %.0fs)" % (path, timeout))
    while time.monotonic() < deadline:
        try:
            with open(path, "r") as f:
                state = " ".join(f.read().split())
        except OSError as e:
            state = "no mhi states: %s" % e
        if state != last:
            log("mhi: %s" % state)
            last = state
        if "MISSION MODE" in state:
            return True
        time.sleep(0.25)
    return False


def open_sahara_ready(dev, deadline):
    """Open the Sahara port and wait for its first HELLO, retrying on EIO."""
    retry_errnos = (errno.EIO, errno.ENOENT, errno.ENODEV, errno.EBUSY,
                    errno.EAGAIN)
    while True:
        rem = deadline - time.monotonic()
        if rem <= 0:
            raise TimeoutError("timed out waiting for Sahara HELLO on %s" % dev)
        fd = None
        try:
            fd = os.open(dev, os.O_RDWR | os.O_CLOEXEC)
            r, _, _ = select.select([fd], [], [], min(3.0, rem))
            if r:
                hdr = read_exact(fd, 8, timeout=5.0)
                return fd, hdr
        except (TimeoutError, EOFError):
            pass
        except OSError as e:
            if e.errno not in retry_errnos:
                if fd is not None:
                    os.close(fd)
                raise
        if fd is not None:
            try:
                os.close(fd)
            except OSError:
                pass
        time.sleep(0.2)


def run_memdump(dev="/dev/wwan0sahara0", outdir="/tmp/sdx55-crash",
                wait_sec=120.0):
    """Standalone memory-debug session server.

    Serves the crashdload pre-dump images (mdmddr/apdp) on mode=0 HELLOs,
    accepts the mode=2 MEMORY_DEBUG offer (HELLO_RESP mode=2), walks the
    table through dump_memory_table, and stops. Like the walker itself this
    server never sends RESET; a session it completes leaves the SBL
    crashdload client waiting for one (run21 record: "the SBL sits
    forever"), so the merged tools/r8q-sahara lifecycle is preferred - it
    sends the recorded RESET itself after this walker returns.
    """
    os.makedirs(outdir, exist_ok=True)
    log("waiting for %s (up to %.0fs)" % (dev, wait_sec))
    deadline = time.monotonic() + wait_sec
    fd, hdr = open_sahara_ready(dev, deadline)
    sources = {}
    try:
        while True:
            cmd, length = struct.unpack("<II", hdr)
            payload = read_exact(fd, length - 8) if length > 8 else b""
            if cmd == HELLO_CMD:
                ver, compat, maxlen, mode = struct.unpack("<IIII", payload[:16])
                log("HELLO ver=%d compat=%d maxlen=0x%x mode=%d"
                    % (ver, compat, maxlen, mode))
                if mode not in (0, 1, MODE_MEMORY_DEBUG):
                    log("WARNING unexpected HELLO mode %d; echoing it anyway"
                        % mode)
                resp = struct.pack("<12I", HELLO_RESP_CMD, HELLO_LENGTH,
                                   SAHARA_VERSION, SAHARA_VERSION,
                                   SAHARA_SUCCESS, mode, 0, 0, 0, 0, 0, 0)
                send_all(fd, resp)
                log("sent HELLO_RESP (mode=%d%s)"
                    % (mode, "; accepted memory-debug"
                       if mode == MODE_MEMORY_DEBUG else ""))
            elif cmd in (READ_DATA_CMD, READ_DATA64_CMD):
                if cmd == READ_DATA_CMD:
                    img_id, off, dlen = struct.unpack("<III", payload[:12])
                else:
                    img_id, off, dlen = struct.unpack("<QQQ", payload[:24])
                if img_id not in PRE_DUMP_IMAGE_MAP:
                    if img_id in KNOWN_IMAGE_NAMES:
                        log("SBL proceeded to normal-boot image_id=%d (%s); "
                            "the standalone listener does not serve that - "
                            "tools/r8q-sahara owns the boot flow"
                            % (img_id, KNOWN_IMAGE_NAMES[img_id]))
                        return 3
                    log("unknown image_id=%d" % img_id)
                    return 2
                if img_id not in sources:
                    name, spec = PRE_DUMP_IMAGE_MAP[img_id]
                    opened = open_sahara_source(spec)
                    if opened is None:
                        log("missing source for %d (%s)" % (img_id, name))
                        return 2
                    sources[img_id] = opened
                path, sfd, _ = sources[img_id]
                log("READ_DATA id=%d off=0x%x len=0x%x (%s)"
                    % (img_id, off, dlen, path))
                data = os.pread(sfd, dlen, off)
                send_all(fd, data)
            elif cmd == END_OF_IMAGE_CMD:
                img_id, status = struct.unpack("<II", payload[:8])
                log("END_OF_IMAGE id=%d status=%d" % (img_id, status))
                if img_id in sources:
                    os.close(sources.pop(img_id)[1])
                send_all(fd, struct.pack("<2I", DONE_CMD, DONE_LENGTH))
                log("sent DONE")
            elif cmd == DONE_RESP_CMD:
                log("DONE_RESP received; waiting for the mode=2 offer...")
            elif cmd in (MEM_DEBUG_CMD, MEM_DEBUG64_CMD):
                log("MEMORY_DEBUG%s table received"
                    % ("64" if cmd == MEM_DEBUG64_CMD else ""))
                files_saved, bytes_saved = dump_memory_table(
                    fd, cmd == MEM_DEBUG64_CMD, payload, outdir)
                log("DONE: %d files, %d bytes written to %s"
                    % (files_saved, bytes_saved, outdir))
                return 0
            elif cmd in (RESET_CMD, RESET_RESP_CMD):
                log("RESET cmd=0x%x" % cmd)
                return 1
            else:
                log("unexpected cmd=0x%x len=%d" % (cmd, length))
                return 1
            hdr = read_exact(fd, 8, timeout=20.0)
    finally:
        for _, sfd, _ in sources.values():
            os.close(sfd)
        os.close(fd)


# ---------------------------------------------------------------------------
# Self-test: replay of the recorded run21 mode=2 session
# ---------------------------------------------------------------------------

# The 32-entry SBL crashdload table exactly as the SDX55M sent it
# (logs/r8q/run21-sahara.log, 2026-11-19: is64=False, table at 0x8fdbb4e0,
# 1664 bytes = 32 x 52). Region payloads are synthesized; the Summary and
# sbl1_pre_log payloads embed the crash-reason strings those regions carry
# in a real dump (recorded plan text: TZBSP_ERR_FATAL_NOC_ERROR,
# SECURE_WDOG_EXPIRE; the SFR assert of section 13.2) plus one synthetic
# identity field to prove the redaction.
RECORDED_TABLE_ADDR = 0x8fdbb4e0
RECORDED_TABLE_ENTRIES = [
    (0x1, 0x14871320, 0x1000, "MDM_ERR_FATAL", "Summary.txt"),
    (0x1, 0x1486ea7c, 0x1000, "MDM_ERR_FATAL", "sbl1_pre_log.txt"),
    (0x1, 0x8fdfe040, 0x1fc0, "MDM_ERR_FATAL", "sbl1_post_log.txt"),
    (0x1, 0x14680000, 0x10000, "OCIMEM", "OCIMEM.BIN"),
    (0x1, 0xb000000, 0x18000, "AOP Code RAM region", "CODERAM.BIN"),
    (0x1, 0xb0e0000, 0x8000, "AOP Data RAM region", "DATARAM.BIN"),
    (0x1, 0xc300000, 0x400, "AOP MSG RAM region", "MSGRAM.BIN"),
    (0x1, 0x1ea0000, 0x9f00, "IPA IU region", "IPA_IU.BIN"),
    (0x1, 0x1e50000, 0x4b20, "IPA SRAM region", "IPA_SRAM.BIN"),
    (0x1, 0x1ec2000, 0x100, "IPA MBOX region", "IPA_MBOX.BIN"),
    (0x1, 0x1e60000, 0xb9b0, "IPA HRAM region", "IPA_HRAM.BIN"),
    (0x1, 0x1e81000, 0x300, "IPA SEQ region", "IPA_SEQ.BIN"),
    (0x1, 0x9060000, 0x10000, "SHRM Memory region", "SHRM_MEM.BIN"),
    (0x1, 0xc300000, 0x400, "AOP_MSG_DRV0", "MSGRAM0.BIN"),
    (0x1, 0xc310000, 0x400, "AOP_MSG_DRV1", "MSGRAM1.BIN"),
    (0x1, 0xc320000, 0x400, "AOP_MSG_DRV2", "MSGRAM2.BIN"),
    (0x1, 0xc330000, 0x400, "AOP_MSG_DRV3", "MSGRAM3.BIN"),
    (0x1, 0xc340000, 0x400, "AOP_MSG_DRV4", "MSGRAM4.BIN"),
    (0x1, 0xc350000, 0x400, "AOP_MSG_DRV5", "MSGRAM5.BIN"),
    (0x1, 0xc360000, 0x400, "AOP_MSG_DRV6", "MSGRAM6.BIN"),
    (0x1, 0xc370000, 0x400, "AOP_MSG_DRV7", "MSGRAM7.BIN"),
    (0x1, 0x1e06000, 0x1500, "IPA GSI region", "IPA_GSI.BIN"),
    (0x1, 0x10ae000, 0x2000, "DCC SRAM", "DCC_SRAM.BIN"),
    (0x1, 0x10a2000, 0x1000, "DCC CFG", "DCC_CFG.BIN"),
    (0x1, 0x8fdbb008, 0x8, "Pmic PON stat", "PMIC_PON.BIN"),
    (0x1, 0x8fdbb010, 0xcc, "Pmic PON History", "PMON_HIS.BIN"),
    (0x1, 0x8fdbb0e0, 0x400, "Pmic PBUS log dump", "PBUS_LOG.BIN"),
    (0x1, 0x8fdbb000, 0x4, "Reset Status Region", "RST_STAT.BIN"),
    (0x1, 0x1485b1f8, 0x4, "Aoss rst fsm stat", "FSM_STS.BIN"),
    (0x1, 0x1485b1fc, 0x4, "Aoss rst fsm ctrl", "FSM_CTRL.BIN"),
    (0x1, 0x80000000, 0x10000000, " DDR CS0 Memory", "DDRCS0.BIN"),
    (0x2, 0x8fd06898, 0x994, "CMM Script", "load.cmm"),
]

_SUMMARY_PAYLOAD = b"".join([
    b"Sahara crashdump Summary\n",
    b"MPSS crash reason:\n",
    b"APPS crash reason:\n",
    b"TZ reset reason : (6) : TZBSP_ERR_FATAL_NOC_ERROR\n",
    b"AOSS_CC_RESET_STATUS from SDI: (0x803) : SECURE_WDOG_EXPIRE\n",
    b"ChipSerial: 0x00FAKEBADA1D01\n",
    b"PS_HOLD warm PON\n",
]).ljust(0x1000, b"\0")

_SBL1_PRE_PAYLOAD = b"".join([
    b"SBL1 pre-log\n",
    b"ssr:AP:remotefs_sahara.c:850:Assertion "
    b"remotefs_sahara_info.sahara_status == "
    b"SAHARA_END_OF_IMG_TRANSFER fail:\n",
    b"Sahara session timeout FATAL\n",
]).ljust(0x1000, b"\0")


def _build_recorded_memory():
    """The recorded 32-entry table blob plus every payload the walker reads."""
    entries = []
    for typ, addr, rlen, desc, fname in RECORDED_TABLE_ENTRIES:
        entries.append(struct.pack("<III20s20s", typ, addr, rlen,
                                   desc.encode("ascii"), fname.encode("ascii")))
    table = b"".join(entries)
    assert len(table) == 1664

    mem = {RECORDED_TABLE_ADDR: table}
    for i, (typ, addr, rlen, desc, fname) in enumerate(RECORDED_TABLE_ENTRIES):
        if fname == "Summary.txt":
            blob = _SUMMARY_PAYLOAD[:rlen]
        elif fname == "sbl1_pre_log.txt":
            blob = _SBL1_PRE_PAYLOAD[:rlen]
        elif fname == "load.cmm":
            blob = (b"cmm script; DAP loaded reset reason region\n").ljust(rlen, b"\0")
        else:
            blob = (b"REGION %s %08x " % (fname.encode("ascii"), addr)
                    ).ljust(min(rlen, 64), b".") + bytes(min(rlen, 8))
            blob = blob.ljust(rlen, b"\0")
        if fname != "DDRCS0.BIN":  # skipped by the size cap, never read
            mem[addr] = blob
    # Extras, as the stale SBL-phase RAM served them (run21d: all zero).
    mem[PCIE_OSAL_LOG_CTX_VA] = bytes(64)
    mem[ULOG_CONTEXT_HEAD_VA] = bytes(4)
    mem[MHI_CTX_PTR_VA] = bytes(4)
    mem[PCIE_CTX_PTR_VA] = bytes(4)
    mem[SMEM_HEAD_VA] = bytes(SMEM_HEAD_LEN)
    mem[TZ_DATA_VA] = bytes(TZ_DATA_LEN)
    return mem


def _mem_lookup(mem, addr, rlen):
    for base, blob in mem.items():
        if base <= addr and addr + rlen <= base + len(blob):
            return blob[addr - base:addr - base + rlen]
    return bytes(rlen)


def _selftest_device(devsock, mem, calls):
    """Scripted SDX55M: serves MEMORY_READ, answers DONE, flags RESET."""
    try:
        while True:
            hdr = b""
            while len(hdr) < 8:
                chunk = devsock.recv(8 - len(hdr))
                if not chunk:
                    return
                hdr += chunk
            cmd, length = struct.unpack("<2I", hdr)
            body = b""
            while len(body) < length - 8:
                chunk = devsock.recv(length - 8 - len(body))
                if not chunk:
                    return
                body += chunk
            if cmd == MEM_READ_CMD:
                addr, rlen = struct.unpack("<II", body)
                calls.append(("mem_read", addr, rlen))
                devsock.sendall(_mem_lookup(mem, addr, rlen))
            elif cmd == MEM_READ64_CMD:
                addr, rlen = struct.unpack("<QQ", body)
                calls.append(("mem_read64", addr, rlen))
                devsock.sendall(_mem_lookup(mem, addr, rlen))
            elif cmd == DONE_CMD:
                calls.append(("done",))
                devsock.sendall(struct.pack("<2I", DONE_RESP_CMD, DONE_LENGTH))
            elif cmd == RESET_CMD:
                calls.append(("reset",))
                devsock.sendall(struct.pack("<2I", RESET_RESP_CMD, DONE_LENGTH))
            else:
                calls.append(("unexpected", cmd))
    except OSError:
        return


def _sockpair():
    import socket
    host, dev = socket.socketpair()
    return host.detach(), dev


def _check(results, ok, label):
    results.append((ok, label))
    log("selftest %s: %s" % ("ok  " if ok else "FAIL", label))


def selftest(outdir):
    """Replay the recorded mode=2 session against a scripted target.

    Proves: the MEM_READ direction and packet forms, the 32-entry table
    parse, the region file set and accounting (the recorded run21 session:
    37 files / 2875204 bytes, plus crash-reasons.txt), the crash-reason
    extraction with redaction, the DONE/DONE_RESP close, and that no RESET
    is ever sent (read-only contract). Stage 2 exercises the EOI-sized
    split and multi-chunk reads.
    """
    import inspect
    import tempfile

    results = []

    # Stage 0: the interface tools/r8q-sahara calls.
    sig = inspect.signature(dump_memory_table)
    _check(results, len(sig.parameters) == 4,
           "dump_memory_table(fd, is64, payload, outdir) signature (4 params)")
    sig = inspect.signature(sahara_mem_read)
    _check(results, len(sig.parameters) == 4,
           "sahara_mem_read(fd, is64, addr, length) signature (4 params)")

    # Stage 1: the recorded 32-entry session.
    if outdir is None:
        outdir = tempfile.mkdtemp(prefix="run17-selftest-")
    os.makedirs(outdir, exist_ok=True)
    mem = _build_recorded_memory()
    calls = []
    host_fd, devsock = _sockpair()
    t = threading.Thread(target=_selftest_device, args=(devsock, mem, calls))
    t.daemon = True
    t.start()

    payload = struct.pack("<II", RECORDED_TABLE_ADDR, 1664)
    files, byts = dump_memory_table(host_fd, False, payload, outdir)
    os.close(host_fd)
    devsock.close()
    t.join(timeout=10)

    _check(results, files == 38 and byts == 2875204,
           "recorded session accounting: %d files (31 regions + 6 metadata + "
           "crash-reasons.txt), %d bytes" % (files, byts))
    with open(os.path.join(outdir, "table.json")) as f:
        table = json.load(f)
    _check(results, len(table) == 32 and table[0]["file"] == "Summary.txt"
           and table[-1]["file"] == "load.cmm",
           "table.json holds the 32 recorded entries")
    _check(results, not os.path.exists(os.path.join(outdir, "DDRCS0.BIN")),
           "DDR CS0 (256 MiB) skipped by the region cap")
    _check(results, os.path.exists(os.path.join(outdir, "sbl1_post_log.txt"))
           and os.path.getsize(os.path.join(outdir, "sbl1_post_log.txt")) == 0x1fc0,
           "regions saved at recorded sizes")
    reasons = ""
    with open(os.path.join(outdir, "crash-reasons.txt")) as f:
        reasons = f.read()
    _check(results, "TZBSP_ERR_FATAL_NOC_ERROR" in reasons
           and "SECURE_WDOG_EXPIRE" in reasons,
           "crash-reasons.txt carries the recorded TZ reset reason")
    _check(results, "remotefs_sahara.c:850" in reasons,
           "crash-reasons.txt carries the SFR assert string")
    _check(results, "0x00FAKEBADA1D01" not in reasons
           and "ChipSerial: <REDACTED>" in reasons,
           "identity fields redacted in the extraction")
    dones = [c for c in calls if c[0] == "done"]
    resets = [c for c in calls if c[0] == "reset"]
    _check(results, len(dones) == 1 and not resets,
           "exactly one DONE sent, no RESET ever sent (read-only contract)")
    unexpected = [c for c in calls if c[0] == "unexpected"]
    _check(results, not unexpected, "no unexpected packets from the walker")

    # Stage 2: chunking shapes (EOI-sized split, 64 KiB chunks). The table
    # lives at a non-overlapping address: _mem_lookup serves the first blob
    # that covers the request, and the 100 KiB region spans 0x3000..0x1c000.
    stage2 = os.path.join(outdir, "stage2")
    TABLE2_ADDR = 0x50000
    mem2 = {}
    entries2 = []
    for typ, addr, rlen, desc, fname in (
            (0x1, 0x1000, 16, "split region", "SPLIT16.BIN"),
            (0x1, 0x2000, 8, "tiny region", "TINY8.BIN"),
            (0x1, 0x3000, 0x19000, "big region", "BIG.BIN")):
        entries2.append(struct.pack("<III20s20s", typ, addr, rlen,
                                     desc.encode("ascii"), fname.encode("ascii")))
        mem2[addr] = bytes(rlen)
    table2 = b"".join(entries2)
    mem2[TABLE2_ADDR] = table2
    calls2 = []
    host_fd, devsock = _sockpair()
    t = threading.Thread(target=_selftest_device, args=(devsock, mem2, calls2))
    t.daemon = True
    t.start()
    dump_memory_table(host_fd, False, struct.pack("<II", TABLE2_ADDR, len(table2)),
                      stage2)
    os.close(host_fd)
    devsock.close()
    t.join(timeout=10)
    reads = [(c[1], c[2]) for c in calls2 if c[0] == "mem_read"]
    _check(results, (0x1000, 8) in reads and (0x1008, 8) in reads
           and (0x1000, 16) not in reads,
           "16-byte region split into 8+8 (EOI-sized request avoided)")
    _check(results, (0x3000, CHUNK) in reads and (0x3000 + CHUNK, 0x9000) in reads,
           "100 KiB region read in two chunks")

    # Stage 3: runtime EFS writeback (m9kefs* table) skips extras and DONE.
    stage3 = os.path.join(outdir, "stage3")
    EFS_TABLE_ADDR = 0x60000
    efs_blob = b"EFS!" + bytes(4092)
    mem3 = {0x80200000: efs_blob}
    table3 = struct.pack("<III20s20s", 0x0, 0x80200000, len(efs_blob),
                         b"", b"m9kefs2")
    mem3[EFS_TABLE_ADDR] = table3
    calls3 = []
    host_fd, devsock = _sockpair()
    t = threading.Thread(target=_selftest_device, args=(devsock, mem3, calls3))
    t.daemon = True
    t.start()
    efs_files, efs_bytes = dump_memory_table(
        host_fd, False, struct.pack("<II", EFS_TABLE_ADDR, len(table3)), stage3)
    os.close(host_fd)
    devsock.close()
    t.join(timeout=10)
    reads3 = [c[1] for c in calls3 if c[0] == "mem_read"]
    _check(results, efs_files == 3 and efs_bytes == len(efs_blob),
           "EFS table saves only table.json + m9kefs2 + crash-reasons.txt")
    _check(results, PCIE_OSAL_LOG_CTX_VA not in reads3
           and SMEM_HEAD_VA not in reads3,
           "EFS table skips crash-dump extras")
    _check(results, not any(c[0] in ("done", "reset") for c in calls3),
           "EFS table skips DONE so caller sends RESET")

    failed = [label for ok, label in results if not ok]
    log("selftest: %d/%d checks passed%s"
        % (len(results) - len(failed), len(results),
           "" if not failed else "; FAILED: " + "; ".join(failed)))
    log("selftest dump dir: %s" % outdir)
    return 0 if not failed else 1


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser(
        description=__doc__,
        epilog=(
            "tools/r8q-sahara loads this file as its memory-debug walker from "
            "R8Q_MEMDUMP_MODULE (default /usr/local/bin/run17_diag.py) at the "
            "first mode=2 HELLO; deploying it to that path lets a RUNNING "
            "loader pick it up on its next MEMORY_DEBUG offer with no restart. "
            "This module never sends RESET: r8q-sahara ends the session with "
            "its own RESET after dump_memory_table returns."))
    ap.add_argument("mode", nargs="?", choices=["selftest", "memdump"],
                    help="selftest: replay the recorded run21 mode=2 session "
                         "against a scripted target; memdump: standalone "
                         "Sahara memory-debug listener")
    ap.add_argument("--device", default="/dev/wwan0sahara0",
                    help="Sahara WWAN port (default: %(default)s)")
    ap.add_argument("--dump-dir", default=None, metavar="DIR",
                    help="where the dump is written (memdump default: "
                         "/tmp/sdx55-crash; selftest uses a fresh temp dir "
                         "unless given one - it never writes the real dump "
                         "dir by default)")
    ap.add_argument("--wait-sec", type=float, default=120.0, metavar="SECONDS",
                    help="seconds to wait for the Sahara session "
                         "(default: %(default)s)")
    ap.add_argument("--after-mission", type=float, default=0.0,
                    metavar="SECONDS",
                    help="wait up to SECONDS for the modem's MHI controller "
                         "to reach MISSION MODE before opening the port "
                         "(0 = open immediately; the pre-Stage-5b "
                         "in-AMSS listener)")
    args = ap.parse_args()

    if args.mode == "selftest":
        return selftest(args.dump_dir)
    if args.mode == "memdump":
        outdir = args.dump_dir or "/tmp/sdx55-crash"
        if args.after_mission:
            if not wait_for_mission_mode(args.after_mission):
                log("timed out waiting for MISSION MODE")
                return 4
        try:
            return run_memdump(dev=args.device, outdir=outdir,
                                wait_sec=args.wait_sec)
        except TimeoutError as e:
            log("no Sahara session: %s" % e)
            return 4
    ap.print_usage(sys.stderr)
    print("run17_diag.py: nothing to do - pass 'selftest' or 'memdump'.\n"
          "As tools/r8q-sahara's dump module it needs no arguments: deploy it "
          "to /usr/local/bin/run17_diag.py (or point R8Q_MEMDUMP_MODULE at "
          "it) and the loader calls dump_memory_table() itself.",
          file=sys.stderr)
    return 2


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (IOError, OSError) as e:
        log("I/O error: %s" % e)
        sys.exit(2)
    except KeyboardInterrupt:
        log("interrupted")
        sys.exit(130)
