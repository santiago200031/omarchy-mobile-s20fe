// SPDX-License-Identifier: MIT
//! Serve Sahara image transfers over an MHI WWAN SAHARA port (r8q / SDX55M).
//!
//! Rust port of tools/r8q-sahara (Python). Built static (musl) for the
//! rescue initramfs, which has no Python interpreter.
//!
//! This is a *read-only* loader: every image source is opened O_RDONLY,
//! and the only writes performed are to the Sahara WWAN character device
//! (host -> modem). No UFS partition, NV file, or firmware file is ever
//! modified.
//!
//! Protocol constants and packet layouts follow the upstream Linux Sahara
//! implementation (drivers/accel/qaic/sahara.c, GPL-2.0-only, Qualcomm).
//! The image-ID map below was observed in this device's own vendor loader
//! (lib64/libmdmimgload.so, `sahara_map_arr`) and is used only as a
//! lookup table.

use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

// --- Sahara v2 protocol ---
const HELLO_CMD: u32 = 0x1;
const HELLO_RESP_CMD: u32 = 0x2;
const READ_DATA_CMD: u32 = 0x3;
const END_OF_IMAGE_CMD: u32 = 0x4;
const DONE_CMD: u32 = 0x5;
const DONE_RESP_CMD: u32 = 0x6;
const RESET_CMD: u32 = 0x7;
const RESET_RESP_CMD: u32 = 0x8;

const HELLO_LENGTH: u32 = 0x30; // 48 bytes on the wire (24 used + 24 reserved)
const DONE_LENGTH: u32 = 0x8; // header only

const SAHARA_VERSION: u32 = 2;
const SAHARA_SUCCESS: u32 = 0;

// mhi_wwan_ctrl caps a single write at MHI_WWAN_MAX_MTU (0x8000).
const CHUNK: usize = 0x8000;

// Image IDs observed in the r8q vendor Sahara map: (name, spec) where the
// spec is `file:<name>` (under the firmware dir) or `part:<PARTNAME>`
// (a UFS partition found by PARTNAME, read-only).
const IMAGE_MAP: &[(u32, &str, &str)] = &[
    (6, "apps.mbn", "file:apps.mbn"),
    (8, "qdsp6sw.mbn", "file:qdsp6sw.mbn"),
    (9, "mdmddr", "part:mdmddr"),
    (16, "mdm1m9kefs1", "part:mdm1m9kefs1"),
    (17, "mdm1m9kefs2", "part:mdm1m9kefs2"),
    (20, "mdm1m9kefs3", "part:mdm1m9kefs3"),
    (21, "sbl1.mbn", "file:sbl1.mbn"),
    (23, "aop.mbn", "file:aop.mbn"),
    (25, "tz.mbn", "file:tz.mbn"),
    (28, "dsp2.mbn", "file:dsp2.mbn"),
    (29, "acdb.mbn", "file:acdb.mbn"),
    (30, "sdi.mbn", "file:sdi.mbn"),
    (31, "mba.mbn", "file:mba.mbn"),
    (33, "hyp.mbn", "file:hyp.mbn"),
    (34, "mdmddr.mbn", "part:mdmddr"),
    (36, "multi_image_qti.mbn", "file:multi_image_qti.mbn"),
    (37, "multi_image.mbn", "file:multi_image.mbn"),
    (38, "xbl_cfg.elf", "file:xbl_cfg.elf"),
    (40, "apdp.mbn", "file:apdp.mbn"),
    (41, "devcfg.mbn", "file:devcfg.mbn"),
    (42, "sec.elf", "file:sec.elf"),
];

// IDs present in the vendor lib's map but belonging to its other platforms
// (mdm9x55 / sdx50m), not to r8q's sdx55m image set. Not staged; if the
// modem ever requests one, serving stops rather than guessing a file.
const CROSS_PLATFORM_IDS: &[u32] = &[28, 30, 31, 37];

fn log(msg: &str) {
    println!("[r8q-sahara] {msg}");
}

fn cmd_name(cmd: u32) -> String {
    match cmd {
        HELLO_CMD => "HELLO".into(),
        HELLO_RESP_CMD => "HELLO_RESP".into(),
        READ_DATA_CMD => "READ_DATA".into(),
        END_OF_IMAGE_CMD => "END_OF_IMAGE".into(),
        DONE_CMD => "DONE".into(),
        DONE_RESP_CMD => "DONE_RESP".into(),
        RESET_CMD => "RESET".into(),
        RESET_RESP_CMD => "RESET_RESP".into(),
        c => format!("{c}"),
    }
}

fn firmware_dir() -> PathBuf {
    std::env::var_os("R8Q_SAHARA_FW_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/lib/firmware/qcom/sdx55m"))
}

/// Resolve a UFS partition by PARTNAME in /sys/class/block/*/uevent.
fn find_partition(name: &str) -> Option<PathBuf> {
    let base = Path::new("/sys/class/block");
    let mut entries: Vec<_> = std::fs::read_dir(base).ok()?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    let want = format!("PARTNAME={name}");
    for entry in entries {
        let uevent = entry.path().join("uevent");
        let content = std::fs::read(&uevent).ok()?;
        if content.split(|&b| b == b'\n').any(|l| l == want.as_bytes()) {
            return Some(PathBuf::from("/dev").join(entry.file_name()));
        }
    }
    None
}

struct Source {
    path: PathBuf,
    file: File,
    size_bytes: u64,
}

fn open_source(spec: &str) -> Option<Source> {
    let (kind, value) = spec.split_once(':')?;
    let path = match kind {
        "file" => firmware_dir().join(value),
        "part" => find_partition(value)?,
        _ => return None,
    };
    let file = std::fs::OpenOptions::new().read(true).open(&path).ok()?;
    let mut size = file.metadata().ok()?.len();
    if size == 0 {
        // st_size is 0 for block devices; seek to the end for the size.
        use std::io::Seek;
        let mut f = file.try_clone().ok()?;
        size = f.seek(std::io::SeekFrom::End(0)).ok()?;
    }
    Some(Source { path, file, size_bytes: size })
}

impl Source {
    fn read_exact_at(&self, offset: u64, length: usize) -> std::io::Result<Vec<u8>> {
        let mut buf = vec![0u8; length];
        let mut got = 0;
        while got < length {
            let n = self.file.read_at(&mut buf[got..], offset + got as u64)?;
            if n == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    format!("short read from {}: {got}/{length} bytes", self.path.display()),
                ));
            }
            got += n;
        }
        Ok(buf)
    }
}

fn read_exact(file: &File, mut buf: &mut [u8]) -> std::io::Result<()> {
    while !buf.is_empty() {
        let mut chunk = [0u8; 0x4000];
        let want = chunk.len().min(buf.len());
        let n = read_raw(file, &mut chunk[..want])?;
        if n == 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "EOF on port"));
        }
        buf[..n].copy_from_slice(&chunk[..n]);
        buf = &mut buf[n..];
    }
    Ok(())
}

// libc-free read/write via raw fds so partial transfers are visible.
fn read_raw(file: &File, buf: &mut [u8]) -> std::io::Result<usize> {
    let ret = unsafe {
        sys_read(file.as_raw_fd(), buf.as_mut_ptr() as *mut _, buf.len())
    };
    if ret < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(ret as usize)
    }
}

fn write_all_chunked(file: &File, mut data: &[u8]) -> std::io::Result<()> {
    while !data.is_empty() {
        let step = data.len().min(CHUNK);
        let ret = unsafe {
            sys_write(file.as_raw_fd(), data.as_ptr() as *const _, step)
        };
        if ret <= 0 {
            return Err(if ret == 0 {
                std::io::Error::new(std::io::ErrorKind::WriteZero, "write returned 0")
            } else {
                std::io::Error::last_os_error()
            });
        }
        data = &data[ret as usize..];
    }
    Ok(())
}

// Minimal libc bindings to avoid pulling in the `libc` crate: the binary
// stays dependency-free and statically linked.
unsafe fn sys_read(fd: i32, buf: *mut u8, count: usize) -> isize {
    unsafe {
        extern "C" {
            #[link_name = "read"]
            fn sys_read(fd: i32, buf: *mut u8, count: usize) -> isize;
        }
        sys_read(fd, buf, count)
    }
}

unsafe fn sys_write(fd: i32, buf: *const u8, count: usize) -> isize {
    unsafe {
        extern "C" {
            #[link_name = "write"]
            fn sys_write(fd: i32, buf: *const u8, count: usize) -> isize;
        }
        sys_write(fd, buf, count)
    }
}

fn u32le(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
}

fn send_hello_resp(port: &File, mode: u32) -> std::io::Result<()> {
    // cmd=HELLO_RESP, length=0x30, version=2, compat=2, status=0, mode
    // plus the 24 reserved bytes the upstream implementation also sends.
    let mut pkt = Vec::with_capacity(HELLO_LENGTH as usize);
    for v in [HELLO_RESP_CMD, HELLO_LENGTH, SAHARA_VERSION, SAHARA_VERSION, SAHARA_SUCCESS, mode] {
        pkt.extend_from_slice(&v.to_le_bytes());
    }
    pkt.resize(HELLO_LENGTH as usize, 0);
    write_all_chunked(port, &pkt)
}

fn send_done(port: &File) -> std::io::Result<()> {
    let mut pkt = Vec::with_capacity(DONE_LENGTH as usize);
    for v in [DONE_CMD, DONE_LENGTH] {
        pkt.extend_from_slice(&v.to_le_bytes());
    }
    write_all_chunked(port, &pkt)
}

struct Args {
    device: String,
    wait_secs: u64,
    max_images: u64,
    dry_run: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        device: "/dev/wwan0sahara0".into(),
        wait_secs: 60,
        max_images: 0,
        dry_run: false,
    };
    let mut it = std::env::args_os().skip(1).peekable();
    while let Some(arg) = it.next() {
        let s = arg.as_bytes();
        if s == b"-d" || s == b"--device" {
            let v = it.next().ok_or("--device needs a value")?;
            a.device = v.to_string_lossy().into_owned();
        } else if s.starts_with(b"--device=") {
            a.device = String::from_utf8_lossy(&s[9..]).into_owned();
        } else if s == b"--wait" {
            let v = it.next().ok_or("--wait needs a value")?;
            a.wait_secs = v.to_string_lossy().parse().map_err(|_| "--wait needs a number")?;
        } else if s.starts_with(b"--wait=") {
            a.wait_secs = String::from_utf8_lossy(&s[7..]).parse().map_err(|_| "--wait needs a number")?;
        } else if s == b"--max-images" {
            let v = it.next().ok_or("--max-images needs a value")?;
            a.max_images = v.to_string_lossy().parse().map_err(|_| "--max-images needs a number")?;
        } else if s.starts_with(b"--max-images=") {
            a.max_images = String::from_utf8_lossy(&s[13..]).parse().map_err(|_| "--max-images needs a number")?;
        } else if s == b"--dry-run" {
            a.dry_run = true;
        } else if s == b"--help" || s == b"-h" {
            return Err("help".into());
        } else {
            return Err(format!("unexpected argument: {}", String::from_utf8_lossy(s)));
        }
    }
    Ok(a)
}

fn dry_run() -> ExitCode {
    let mut missing: Vec<String> = Vec::new();
    for (id, name, spec) in IMAGE_MAP {
        match open_source(spec) {
            None if !CROSS_PLATFORM_IDS.contains(id) => {
                missing.push(format!("{id} {name}"));
            }
            None => {}
            Some(src) => {
                log(&format!("id {:<3} {:<22} {:<40} {} bytes", id, name, src.path.display(), src.size_bytes));
            }
        }
    }
    if !missing.is_empty() {
        log(&format!("MISSING required sources: {}", missing.join(", ")));
    } else {
        log("all r8q image sources present and readable");
    }
    log(&format!(
        "cross-platform IDs (not staged, only an issue if requested): {}",
        CROSS_PLATFORM_IDS
            .iter()
            .map(|i| {
                let name = IMAGE_MAP.iter().find(|(id, _, _)| id == i).map(|(_, n, _)| *n).unwrap_or("?");
                format!("{i} {name}")
            })
            .collect::<Vec<_>>()
            .join(", ")
    ));
    if missing.is_empty() { ExitCode::SUCCESS } else { ExitCode::from(2) }
}

fn serve(args: &Args) -> ExitCode {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(args.wait_secs);
    while !Path::new(&args.device).exists() {
        if std::time::Instant::now() >= deadline {
            log(&format!("port {} did not appear within {}s", args.device, args.wait_secs));
            return ExitCode::from(2);
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    let port = match std::fs::OpenOptions::new().read(true).write(true).open(&args.device) {
        Ok(f) => f,
        Err(e) => {
            log(&format!("cannot open {}: {e}", args.device));
            return ExitCode::from(2);
        }
    };
    log(&format!("listening on {}", args.device));

    let mut served: u64 = 0;
        let mut _active: Option<Source> = None;
    loop {
        let mut hdr = [0u8; 8];
        if read_exact(&port, &mut hdr).is_err() {
            log("port closed by peer");
            return ExitCode::SUCCESS;
        }
        let cmd = u32le(&hdr, 0);
        let length = u32le(&hdr, 4) as usize;
        if length < 8 {
            log(&format!("unexpected cmd {} (length {length:#x}) - stopping", cmd_name(cmd)));
            return ExitCode::from(2);
        }
        let mut rest = vec![0u8; length - 8];
        if read_exact(&port, &mut rest).is_err() {
            log(&format!("truncated packet for cmd {}", cmd_name(cmd)));
            return ExitCode::from(2);
        }
        let mut pkt = Vec::with_capacity(length);
        pkt.extend_from_slice(&hdr);
        pkt.extend_from_slice(&rest);

        match cmd {
            HELLO_CMD => {
                if pkt.len() < 24 {
                    log("truncated HELLO - stopping");
                    return ExitCode::from(2);
                }
                let (version, compat, maxlen, mode) =
                    (u32le(&pkt, 8), u32le(&pkt, 12), u32le(&pkt, 16), u32le(&pkt, 20));
                log(&format!("HELLO version={version} compat={compat} maxlen={maxlen:#x} mode={mode}"));
                if length as u32 != HELLO_LENGTH {
                    log(&format!("WARNING: HELLO length {length:#x} (expected {HELLO_LENGTH:#x})"));
                }
                if send_hello_resp(&port, mode).is_err() {
                    log("I/O error sending HELLO_RESP");
                    return ExitCode::from(2);
                }
                log(&format!("sent HELLO_RESP (version={SAHARA_VERSION}, mode={mode})"));
            }
            READ_DATA_CMD => {
                if pkt.len() < 20 {
                    log("truncated READ_DATA - stopping");
                    return ExitCode::from(2);
                }
                let (image_id, offset, data_len) = (u32le(&pkt, 8), u32le(&pkt, 12) as u64, u32le(&pkt, 16) as usize);
                _active = None;
                let Some(entry) = IMAGE_MAP.iter().find(|(id, _, _)| *id == image_id) else {
                    log(&format!("UNKNOWN IMAGE ID {image_id} (no map entry) - stopping"));
                    return ExitCode::from(2);
                };
                let src = match open_source(entry.2) {
                    Some(s) => s,
                    None => {
                        log(&format!("IMAGE ID {image_id} ({}): source {} unavailable - stopping", entry.1, entry.2));
                        return ExitCode::from(2);
                    }
                };
                if offset + data_len as u64 > src.size_bytes {
                    log(&format!(
                        "IMAGE ID {image_id} ({}): request {offset:#x}+{data_len:#x} exceeds size {:#x} - stopping",
                        entry.1, src.size_bytes
                    ));
                    return ExitCode::from(2);
                }
                log(&format!(
                    "IMAGE ID {image_id} ({}): offset={offset:#x} length={data_len:#x} from {}",
                    entry.1,
                    src.path.display()
                ));
                let data = match src.read_exact_at(offset, data_len) {
                    Ok(d) => d,
                    Err(e) => {
                        log(&format!("IMAGE ID {image_id}: read failed: {e}"));
                        return ExitCode::from(2);
                    }
                };
                if write_all_chunked(&port, &data).is_err() {
                    log(&format!("IMAGE ID {image_id}: send failed"));
                    return ExitCode::from(2);
                }
                log(&format!("IMAGE ID {image_id}: sent {data_len} bytes"));
                _active = Some(src);
                served += 1;
                if args.max_images != 0 && served >= args.max_images {
                    log(&format!("reached --max-images={}; exiting", args.max_images));
                    return ExitCode::SUCCESS;
                }
            }
            END_OF_IMAGE_CMD => {
                if pkt.len() < 16 {
                    log("truncated END_OF_IMAGE - stopping");
                    return ExitCode::from(2);
                }
                let (image_id, status) = (u32le(&pkt, 8), u32le(&pkt, 12));
                log(&format!("END_OF_IMAGE image={image_id} status={status}"));
                if status != SAHARA_SUCCESS {
                    log("WARNING: modem reported non-zero EOI status");
                }
                _active = None;
                if send_done(&port).is_err() {
                    log("I/O error sending DONE");
                    return ExitCode::from(2);
                }
                log("sent DONE");
            }
            DONE_RESP_CMD => {
                /* Stock's loader holds one channel for the whole image
                 * walk (klein: "15 images in 15 mini-sessions, each
                 * with HELLO/DONE cycle").  Keep the fd open and wait
                 * for the next HELLO instead of exiting and forcing a
                 * channel reset on every reopen.
                 */
                log("DONE_RESP received - keeping the channel open");
                if args.max_images != 0 && served >= args.max_images {
                    log(&format!("reached --max-images={}; exiting", args.max_images));
                    return ExitCode::SUCCESS;
                }
            }
            RESET_CMD | RESET_RESP_CMD => {
                log(&format!("{} received - stopping", cmd_name(cmd)));
                return ExitCode::from(1);
            }
            _ => {
                log(&format!("unexpected cmd {} (length {:#x}) - stopping", cmd_name(cmd), length));
                return ExitCode::from(2);
            }
        }
    }
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) if e == "help" => {
            println!("r8q-sahara: serve Sahara image transfers over an MHI WWAN SAHARA port");
            println!("usage: r8q-sahara [-d DEVICE] [--wait SECONDS] [--max-images N] [--dry-run]");
            println!("env R8Q_SAHARA_FW_DIR overrides /lib/firmware/qcom/sdx55m");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            log(&format!("{e}"));
            return ExitCode::from(2);
        }
    };
    if args.dry_run {
        return dry_run();
    }
    serve(&args)
}
