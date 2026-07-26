//! Probe classic NVIDIA/Hermes character devices and kmod presence.
//!
//! Never invents Online: missing nodes report absent; loaded modules without
//! Online still report Offline phase when status is readable.

use hermes_linux::{
    hermes_ctl_ioctl_status, hermes_drm_ioctl_status, modules, HermesCtlStatus, HermesDrmStatus,
    HERMES_MOD_DRM, HERMES_MOD_MODESET, HERMES_MOD_NVIDIA, HERMES_MOD_PEERMEM, HERMES_MOD_UVM,
};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::io::AsRawFd;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct ModulePresence {
    pub name: &'static str,
    pub loaded: bool,
}

#[derive(Clone, Debug)]
pub struct NodePresence {
    pub path: &'static str,
    pub exists: bool,
}

#[derive(Clone, Debug)]
pub struct ChardevProbe {
    pub modules: Vec<ModulePresence>,
    pub nodes: Vec<NodePresence>,
    pub ctl_status: Option<HermesCtlStatus>,
    pub ctl_read: Option<String>,
    pub ctl_error: Option<String>,
    pub drm_status: Option<HermesDrmStatus>,
    pub drm_error: Option<String>,
}

pub fn module_loaded(name: &str) -> bool {
    // Kernel module names use '_' where the .ko filename may use '-'.
    let underscored = name.replace('-', "_");
    Path::new(&format!("/sys/module/{name}")).exists()
        || Path::new(&format!("/sys/module/{underscored}")).exists()
        || fs::read_to_string("/proc/modules")
            .map(|t| {
                t.lines().any(|l| {
                    let m = l.split_whitespace().next();
                    m == Some(name) || m == Some(underscored.as_str())
                })
            })
            .unwrap_or(false)
}

pub fn probe() -> ChardevProbe {
    let module_names = [
        modules::NVIDIA,
        modules::NVIDIA_MODESET,
        modules::NVIDIA_UVM,
        modules::NVIDIA_DRM,
        modules::NVIDIA_PEERMEM,
    ];
    let modules: Vec<_> = module_names
        .iter()
        .map(|n| ModulePresence {
            name: n,
            loaded: module_loaded(n),
        })
        .collect();

    let node_paths = [
        "/dev/nvidiactl",
        "/dev/nvidia0",
        "/dev/nvidia-uvm",
        "/dev/nvidia-modeset",
        "/dev/nvidia-drm",
    ];
    let nodes: Vec<_> = node_paths
        .iter()
        .map(|p| NodePresence {
            path: p,
            exists: Path::new(p).exists(),
        })
        .collect();

    let mut ctl_status = None;
    let mut ctl_read = None;
    let mut ctl_error = None;
    if Path::new("/dev/nvidiactl").exists() {
        match open_and_status_ctl() {
            Ok((st, line)) => {
                ctl_status = Some(st);
                ctl_read = line;
            }
            Err(e) => ctl_error = Some(e),
        }
    }

    let mut drm_status = None;
    let mut drm_error = None;
    if Path::new("/dev/nvidia-drm").exists() {
        match open_and_status_drm() {
            Ok(st) => drm_status = Some(st),
            Err(e) => drm_error = Some(e),
        }
    }

    ChardevProbe {
        modules,
        nodes,
        ctl_status,
        ctl_read,
        ctl_error,
        drm_status,
        drm_error,
    }
}

fn open_and_status_ctl() -> Result<(HermesCtlStatus, Option<String>), String> {
    let mut f = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/nvidiactl")
        .or_else(|_| File::open("/dev/nvidiactl"))
        .map_err(|e| format!("open: {e}"))?;

    let mut st = HermesCtlStatus::default();
    let rc = unsafe {
        libc_ioctl(
            f.as_raw_fd(),
            hermes_ctl_ioctl_status(),
            &mut st as *mut _ as *mut u8,
        )
    };
    if rc != 0 {
        // Fall back to read() status line if ioctl unavailable.
        let mut buf = String::new();
        f.read_to_string(&mut buf).map_err(|e| format!("read: {e}"))?;
        let line = buf.lines().next().map(|s| s.to_string());
        let parsed = parse_ctl_read_line(line.as_deref().unwrap_or(""));
        return Ok((parsed, line));
    }
    let mut buf = String::new();
    let _ = f.read_to_string(&mut buf);
    let line = if buf.is_empty() {
        None
    } else {
        Some(buf.lines().next().unwrap_or("").to_string())
    };
    Ok((st, line))
}

fn open_and_status_drm() -> Result<HermesDrmStatus, String> {
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/nvidia-drm")
        .or_else(|_| File::open("/dev/nvidia-drm"))
        .map_err(|e| format!("open: {e}"))?;
    let mut st = HermesDrmStatus::default();
    let rc = unsafe {
        libc_ioctl(
            f.as_raw_fd(),
            hermes_drm_ioctl_status(),
            &mut st as *mut _ as *mut u8,
        )
    };
    if rc != 0 {
        return Err(format!("ioctl status failed errno={}", io::Error::last_os_error()));
    }
    Ok(st)
}

/// Minimal ioctl without libc crate dependency.
unsafe fn libc_ioctl(fd: i32, req: u64, arg: *mut u8) -> i32 {
    #[cfg(target_env = "gnu")]
    {
        extern "C" {
            fn ioctl(fd: i32, request: u64, ...) -> i32;
        }
        ioctl(fd, req, arg)
    }
    #[cfg(not(target_env = "gnu"))]
    {
        extern "C" {
            fn ioctl(fd: i32, request: core::ffi::c_ulong, ...) -> i32;
        }
        ioctl(fd, req as core::ffi::c_ulong, arg)
    }
}

/// Parse `hermes gsp_online=N phase=NAME ...` from chardev read().
pub fn parse_ctl_read_line(line: &str) -> HermesCtlStatus {
    let mut online = false;
    let mut phase = 0u32;
    let mut mask = HERMES_MOD_NVIDIA;
    for part in line.split_whitespace() {
        if let Some(v) = part.strip_prefix("gsp_online=") {
            online = v == "1" || v.eq_ignore_ascii_case("true");
        }
        if let Some(v) = part.strip_prefix("phase=") {
            phase = match v.to_ascii_uppercase().as_str() {
                "OFFLINE" => 0,
                "PROBED" => 1,
                "FIRMWARED" => 2,
                "QUEUED" => 3,
                "NEGOTIATED" => 4,
                "ONLINE" => 5,
                "RECOVERING" => 6,
                "QUARANTINED" => 7,
                _ => 0,
            };
        }
        if let Some(v) = part.strip_prefix("modules=") {
            mask = 0;
            for m in v.split(',') {
                match m {
                    "nvidia" => mask |= HERMES_MOD_NVIDIA,
                    "nvidia-modeset" | "modeset" => mask |= HERMES_MOD_MODESET,
                    "nvidia-uvm" | "uvm" => mask |= HERMES_MOD_UVM,
                    "nvidia-drm" | "drm" => mask |= HERMES_MOD_DRM,
                    "nvidia-peermem" | "peermem" => mask |= HERMES_MOD_PEERMEM,
                    _ => {}
                }
            }
            if mask == 0 {
                mask = HERMES_MOD_NVIDIA;
            }
        }
    }
    HermesCtlStatus::fill(online, phase, mask)
}

pub fn print_probe(p: &ChardevProbe) {
    println!("Hermes kmod / chardev probe (no Online claim from presence alone)");
    println!("modules:");
    for m in &p.modules {
        println!(
            "  {:<18} {}",
            m.name,
            if m.loaded { "loaded" } else { "absent" }
        );
    }
    println!("nodes:");
    for n in &p.nodes {
        println!(
            "  {:<22} {}",
            n.path,
            if n.exists { "present" } else { "missing" }
        );
    }
    match (&p.ctl_status, &p.ctl_error) {
        (Some(st), _) => {
            println!(
                "nvidiactl ioctl: online={} phase={} ver={} modules={:?}",
                st.is_online(),
                st.phase_label(),
                st.version,
                st.modules_listed()
            );
            if let Some(line) = &p.ctl_read {
                println!("nvidiactl read: {line}");
            }
        }
        (None, Some(e)) => println!("nvidiactl: {e}"),
        (None, None) => println!("nvidiactl: node missing (load nvidia.ko)"),
    }
    match (&p.drm_status, &p.drm_error) {
        (Some(st), _) => {
            println!(
                "nvidia-drm ioctl: online={} connectors={} crtcs={} active={} ver={}",
                st.gsp_online != 0, st.connectors, st.crtcs, st.active_crtcs, st.version
            );
        }
        (None, Some(e)) => println!("nvidia-drm: {e}"),
        (None, None) => println!("nvidia-drm: node missing (load nvidia-drm.ko)"),
    }
}

/// Smoke: always succeeds if probe runs; fails only if Online invented without evidence.
pub fn smoke() -> i32 {
    let p = probe();
    print_probe(&p);
    // Fail-closed: if ctl says Online, phase must be ONLINE and we still only report.
    if let Some(st) = &p.ctl_status {
        if st.is_online() && st.phase != 5 {
            eprintln!("error: gsp_online set but phase != ONLINE (inconsistent uAPI)");
            return 1;
        }
    }
    // Parse round-trip on canned line.
    let canned = "hermes gsp_online=0 phase=OFFLINE modules=nvidia status_ver=2";
    let parsed = parse_ctl_read_line(canned);
    assert!(!parsed.is_online());
    assert_eq!(parsed.phase_label(), "OFFLINE");
    println!("parse canned offline line: PASS");
    println!("chardev-smoke: PASS");
    0
}

/// Prefer real ioctl prove when modules loaded; else invoke `scripts/load-kmod.sh`.
pub fn kmod_load_smoke() -> i32 {
    println!("=== kmod-load-smoke: load nvidia*.ko + prove /dev/nvidiactl ioctl ===");
    let already = module_loaded("nvidia") && Path::new("/dev/nvidiactl").exists();
    if already {
        println!("nvidia.ko already loaded; proving ioctl");
        // Best-effort chmod if we can.
        let _ = std::process::Command::new("sudo")
            .args(["-n", "chmod", "666", "/dev/nvidiactl", "/dev/nvidia0"])
            .status();
    } else {
        let script = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scripts/load-kmod.sh");
        let script = script.canonicalize().unwrap_or(script);
        println!("running {}", script.display());
        let st = std::process::Command::new("sh")
            .arg(&script)
            .status();
        match st {
            Ok(s) if s.success() => {}
            Ok(s) => {
                eprintln!("load-kmod.sh exited {s}");
                return 1;
            }
            Err(e) => {
                eprintln!("failed to run load-kmod.sh: {e}");
                return 1;
            }
        }
    }

    let p = probe();
    print_probe(&p);
    if !module_loaded("nvidia") {
        eprintln!("error: nvidia module still not loaded");
        return 1;
    }
    if !Path::new("/dev/nvidiactl").exists() {
        eprintln!("error: /dev/nvidiactl missing after load");
        return 1;
    }
    match &p.ctl_status {
        Some(st) => {
            println!(
                "live ioctl: online={} phase={} ver={} mask={:?}",
                st.is_online(),
                st.phase_label(),
                st.version,
                st.modules_listed()
            );
            if st.version < 2 {
                eprintln!("error: bad status version");
                return 1;
            }
            if st.is_online() && st.phase != 5 {
                eprintln!("error: invented Online with wrong phase");
                return 1;
            }
            // Bare insmod without full evidence should be Offline.
            if st.is_online() {
                println!("note: Online reported (unexpected on bare load) — still phase-checked");
            } else {
                println!("fail-closed Offline after bare load: PASS");
            }
        }
        None => {
            eprintln!(
                "error: ioctl/read failed: {}",
                p.ctl_error.as_deref().unwrap_or("unknown")
            );
            return 1;
        }
    }
    println!("kmod-load-smoke: PASS");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_offline_and_online_lines() {
        let off = parse_ctl_read_line("hermes gsp_online=0 phase=OFFLINE modules=nvidia");
        assert!(!off.is_online());
        let on = parse_ctl_read_line(
            "hermes gsp_online=1 phase=ONLINE modules=nvidia,nvidia-drm status_ver=2",
        );
        assert!(on.is_online());
        assert_eq!(on.phase_label(), "ONLINE");
        assert!(on.module_mask & HERMES_MOD_DRM != 0);
    }
}
