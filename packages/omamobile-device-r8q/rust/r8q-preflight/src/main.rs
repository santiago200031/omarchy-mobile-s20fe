// SPDX-License-Identifier: MIT
// r8q-preflight: verify SM8250 firmware versions/blobs, UFS read-only guard,
// modprobe blacklist state, and device-tree overlay status on Galaxy S20 FE (r8q).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const MIN_SQE_VERSION: u32 = 0x095;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckResult {
    name: &'static str,
    ok: bool,
    detail: String,
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

pub fn parse_sqe_version(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < 8 {
        return None;
    }
    let word = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    Some(word & 0xfff)
}

fn resolve_firmware(root: &Path, rel: &str) -> Option<PathBuf> {
    for base in [
        root.join("lib/firmware/updates"),
        root.join("usr/lib/firmware/updates"),
        root.join("lib/firmware"),
        root.join("usr/lib/firmware"),
    ] {
        let p = base.join(rel);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn check_sqe(root: &Path) -> CheckResult {
    let Some(path) = resolve_firmware(root, "qcom/a650_sqe.fw") else {
        return CheckResult {
            name: "a650_sqe.fw",
            ok: false,
            detail: "missing qcom/a650_sqe.fw under /lib/firmware or /usr/lib/firmware".into(),
        };
    };
    let Ok(bytes) = fs::read(&path) else {
        return CheckResult {
            name: "a650_sqe.fw",
            ok: false,
            detail: format!("unreadable: {}", path.display()),
        };
    };
    match parse_sqe_version(&bytes) {
        Some(ver) if ver >= MIN_SQE_VERSION => CheckResult {
            name: "a650_sqe.fw",
            ok: true,
            detail: format!("{} (fw 0x{ver:03x} >= 0x{MIN_SQE_VERSION:03x})", path.display()),
        },
        Some(ver) => CheckResult {
            name: "a650_sqe.fw",
            ok: false,
            detail: format!(
                "{} has fw 0x{ver:03x} < 0x{MIN_SQE_VERSION:03x} (stock/LineageOS microcode breaks mainline a650)",
                path.display()
            ),
        },
        None => CheckResult {
            name: "a650_sqe.fw",
            ok: false,
            detail: format!("{} is truncated (< 8 bytes)", path.display()),
        },
    }
}

fn check_blob(root: &Path, name: &'static str, rel: &str) -> CheckResult {
    match resolve_firmware(root, rel) {
        Some(path) => CheckResult {
            name,
            ok: true,
            detail: path.display().to_string(),
        },
        None => CheckResult {
            name,
            ok: false,
            detail: format!("missing {rel}"),
        },
    }
}

fn dt_status(sys: &Path, node_rel: &str) -> String {
    let p = sys.join("firmware/devicetree/base").join(node_rel).join("status");
    match fs::read(&p) {
        Ok(raw) => String::from_utf8_lossy(&raw)
            .trim_end_matches('\0')
            .trim()
            .to_string(),
        Err(_) => "absent".into(),
    }
}

fn check_ufs_ro_guard(sys: &Path) -> CheckResult {
    let block_dir = sys.join("block");
    let Ok(entries) = fs::read_dir(&block_dir) else {
        return CheckResult {
            name: "ufs_guard",
            ok: false,
            detail: format!("cannot read {}", block_dir.display()),
        };
    };
    let mut writable = Vec::new();
    let mut checked = 0usize;
    for entry in entries.flatten() {
        let fname = entry.file_name();
        let s = fname.to_string_lossy();
        if !s.starts_with("sd") {
            continue;
        }
        let path = entry.path();
        let Ok(subs) = fs::read_dir(&path) else {
            continue;
        };
        for sub in subs.flatten() {
            let subname = sub.file_name();
            let sn = subname.to_string_lossy();
            if !sn.starts_with(s.as_ref()) {
                continue;
            }
            let ro_path = sub.path().join("ro");
            if let Ok(ro_val) = fs::read_to_string(&ro_path) {
                checked += 1;
                if ro_val.trim() != "1" {
                    writable.push(sn.into_owned());
                }
            }
        }
    }
    if checked == 0 {
        return CheckResult {
            name: "ufs_guard",
            ok: false,
            detail: "no UFS partitions visible under sysfs".into(),
        };
    }
    if writable.is_empty() {
        CheckResult {
            name: "ufs_guard",
            ok: true,
            detail: format!("{checked} UFS partitions read-only"),
        }
    } else {
        writable.sort();
        CheckResult {
            name: "ufs_guard",
            ok: false,
            detail: format!("writable UFS partitions: {}", writable.join(", ")),
        }
    }
}

fn is_msm_blacklisted(root: &Path) -> bool {
    for dir in [root.join("etc/modprobe.d"), root.join("usr/lib/modprobe.d")] {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("conf") {
                continue;
            }
            if let Ok(text) = fs::read_to_string(&path) {
                for line in text.lines() {
                    let trimmed = line.split('#').next().unwrap_or("").trim();
                    let mut parts = trimmed.split_whitespace();
                    if parts.next() == Some("blacklist") && parts.next() == Some("msm") {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn print_usage() {
    eprintln!(
        "Usage: r8q-preflight [--root DIR] [--sys DIR] [--require-gpu] [--require-audio] [--json]"
    );
}

fn main() -> ExitCode {
    let mut root = PathBuf::from("/");
    let mut sys = PathBuf::from("/sys");
    let mut require_gpu = false;
    let mut require_audio = false;
    let mut json_mode = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => {
                let Some(v) = args.next() else {
                    print_usage();
                    return ExitCode::from(2);
                };
                root = PathBuf::from(v);
            }
            "--sys" => {
                let Some(v) = args.next() else {
                    print_usage();
                    return ExitCode::from(2);
                };
                sys = PathBuf::from(v);
            }
            "--require-gpu" => require_gpu = true,
            "--require-audio" => require_audio = true,
            "--json" => json_mode = true,
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("Unknown argument: {other}");
                print_usage();
                return ExitCode::from(2);
            }
        }
    }

    let fw_checks = vec![
        check_sqe(&root),
        check_blob(&root, "a650_gmu.bin", "qcom/a650_gmu.bin"),
        check_blob(&root, "a650_zap.mbn", "qcom/sm8250/samsung/r8q/a650_zap.mbn"),
        check_blob(&root, "adsp.mbn", "qcom/sm8250/samsung/r8q/adsp.mbn"),
        check_blob(&root, "cdsp.mbn", "qcom/sm8250/samsung/r8q/cdsp.mbn"),
        check_blob(&root, "slpi.mbn", "qcom/sm8250/samsung/r8q/slpi.mbn"),
        check_blob(&root, "ath11k_amss", "ath11k/QCA6390/hw2.0/amss.bin"),
        check_blob(&root, "qca_bt", "qca/htbtfw20.tlv"),
    ];

    let ufs = check_ufs_ro_guard(&sys);
    let msm_blacklisted = is_msm_blacklisted(&root);

    let dt_nodes = [
        ("mdss", dt_status(&sys, "soc@0/display-subsystem@ae00000")),
        ("panel", dt_status(&sys, "soc@0/display-subsystem@ae00000/dsi@ae94000/panel@0")),
        ("adsp", dt_status(&sys, "soc@0/remoteproc@17300000")),
        ("cdsp", dt_status(&sys, "soc@0/remoteproc@8300000")),
        ("slpi", dt_status(&sys, "soc@0/remoteproc@5c00000")),
        ("gpi_dma1", dt_status(&sys, "soc@0/dma-controller@a00000")),
        ("i2c11", dt_status(&sys, "soc@0/geniqup@ac0000/i2c@a8c000")),
        ("sound", dt_status(&sys, "sound")),
    ];

    let gpu_ready = fw_checks[0].ok && fw_checks[1].ok && fw_checks[2].ok;
    let audio_ready = fw_checks[3].ok;

    if json_mode {
        print!("{{\"gpu_ready\":{gpu_ready},\"audio_ready\":{audio_ready},\"msm_blacklisted\":{msm_blacklisted},\"ufs_guard\":{{\"ok\":{},\"detail\":\"{}\"}},\"firmware\":[",
            ufs.ok, json_escape(&ufs.detail));
        for (idx, c) in fw_checks.iter().enumerate() {
            if idx > 0 {
                print!(",");
            }
            print!(
                "{{\"name\":\"{}\",\"ok\":{},\"detail\":\"{}\"}}",
                json_escape(c.name),
                c.ok,
                json_escape(&c.detail)
            );
        }
        print!("],\"dt_nodes\":{{");
        for (idx, (k, v)) in dt_nodes.iter().enumerate() {
            if idx > 0 {
                print!(",");
            }
            print!("\"{}\":\"{}\"", json_escape(k), json_escape(v));
        }
        println!("}}}}");
    } else {
        println!("r8q preflight check (root={}, sys={}):", root.display(), sys.display());
        println!(
            "  [{}] {}: {}",
            if ufs.ok { "OK" } else { "WARN" },
            ufs.name,
            ufs.detail
        );
        println!(
            "  [INFO] msm modprobe blacklist: {}",
            if msm_blacklisted { "active" } else { "removed" }
        );
        for c in &fw_checks {
            println!(
                "  [{}] {:<12} {}",
                if c.ok { "OK" } else { "MISS" },
                c.name,
                c.detail
            );
        }
        print!("  [DT]  ");
        for (idx, (k, v)) in dt_nodes.iter().enumerate() {
            if idx > 0 {
                print!(", ");
            }
            print!("{k}={v}");
        }
        println!();
    }

    if (require_gpu && !gpu_ready) || (require_audio && !audio_ready) {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = env::temp_dir().join(format!("r8q-preflight-{tag}-{nanos}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn parses_sqe_versions() {
        let old = [0, 0, 0, 0, 0x93, 0xd0, 0x6d, 0x01];
        let upstream = [0, 0, 0, 0, 0x12, 0xd1, 0x6d, 0x01];
        assert_eq!(parse_sqe_version(&old), Some(0x093));
        assert_eq!(parse_sqe_version(&upstream), Some(0x112));
        assert_eq!(parse_sqe_version(&[1, 2, 3]), None);
    }

    #[test]
    fn rejects_old_sqe_and_accepts_upstream() {
        let root = temp_dir("sqe");
        let qcom = root.join("lib/firmware/qcom");
        fs::create_dir_all(&qcom).unwrap();

        fs::write(qcom.join("a650_sqe.fw"), [0, 0, 0, 0, 0x93, 0xd0, 0x6d, 0x01]).unwrap();
        assert!(!check_sqe(&root).ok);

        fs::write(qcom.join("a650_sqe.fw"), [0, 0, 0, 0, 0x12, 0xd1, 0x6d, 0x01]).unwrap();
        assert!(check_sqe(&root).ok);

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn detects_ufs_ro_and_blacklist() {
        let root = temp_dir("ufs");
        let mp = root.join("usr/lib/modprobe.d");
        fs::create_dir_all(&mp).unwrap();
        fs::write(mp.join("50-r8q-gpu-blacklist.conf"), "blacklist msm\n").unwrap();
        assert!(is_msm_blacklisted(&root));

        let part = root.join("sys/block/sda/sda24");
        fs::create_dir_all(&part).unwrap();
        fs::write(part.join("ro"), "1\n").unwrap();
        assert!(check_ufs_ro_guard(&root.join("sys")).ok);

        fs::write(part.join("ro"), "0\n").unwrap();
        assert!(!check_ufs_ro_guard(&root.join("sys")).ok);

        fs::remove_dir_all(&root).unwrap();
    }
}
