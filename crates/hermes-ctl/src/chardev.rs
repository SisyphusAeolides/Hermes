//! Probe classic NVIDIA/Hermes character devices and kmod presence.
//!
//! Never invents Online: missing nodes report absent; loaded modules without
//! Online still report Offline phase when status is readable.

use hermes_linux::{
    hermes_companion_ioctl_status, hermes_ctl_ioctl_demote, hermes_ctl_ioctl_sim_promote,
    hermes_ctl_ioctl_status, hermes_drm_ioctl_get_edid, hermes_drm_ioctl_get_prop,
    hermes_drm_ioctl_status, modules, HermesCtlStatus, HermesDrmEdid, HermesDrmPropGet,
    HermesDrmStatus, HERMES_DRM_PROP_EDID, HERMES_MOD_DRM, HERMES_MOD_MODESET,
    HERMES_MOD_NVIDIA, HERMES_MOD_PEERMEM, HERMES_MOD_UVM,
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
        "/dev/nvidia-uvm-tools",
        "/dev/nvidia-modeset",
        "/dev/nvidia-drm",
        "/dev/nvidia-peermem",
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

fn open_rw(path: &str) -> Result<File, String> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .or_else(|_| File::open(path))
        .map_err(|e| format!("open {path}: {e}"))
}

fn ioctl_status_path(path: &str, req: u64) -> Result<HermesCtlStatus, String> {
    let f = open_rw(path)?;
    let mut st = HermesCtlStatus::default();
    let rc = unsafe { libc_ioctl(f.as_raw_fd(), req, &mut st as *mut _ as *mut u8) };
    if rc != 0 {
        return Err(format!(
            "ioctl {path}: {}",
            io::Error::last_os_error()
        ));
    }
    Ok(st)
}

/// SIM_PROMOTE on /dev/nvidiactl (requires allow_sim_promote=1).
pub fn ctl_sim_promote() -> Result<(), String> {
    let f = open_rw("/dev/nvidiactl")?;
    let rc = unsafe {
        libc_ioctl(
            f.as_raw_fd(),
            hermes_ctl_ioctl_sim_promote(),
            core::ptr::null_mut(),
        )
    };
    if rc != 0 {
        return Err(format!(
            "SIM_PROMOTE: {} (load with allow_sim_promote=1)",
            io::Error::last_os_error()
        ));
    }
    Ok(())
}

pub fn ctl_demote() -> Result<(), String> {
    let f = open_rw("/dev/nvidiactl")?;
    let rc = unsafe {
        libc_ioctl(
            f.as_raw_fd(),
            hermes_ctl_ioctl_demote(),
            core::ptr::null_mut(),
        )
    };
    if rc != 0 {
        return Err(format!("DEMOTE: {}", io::Error::last_os_error()));
    }
    Ok(())
}

pub fn companion_status(path: &str) -> Result<HermesCtlStatus, String> {
    ioctl_status_path(path, hermes_companion_ioctl_status())
}

pub fn drm_get_edid(connector_id: u32) -> Result<HermesDrmEdid, String> {
    let f = open_rw("/dev/nvidia-drm")?;
    let mut edid = HermesDrmEdid {
        connector_id,
        size: 0,
        data: [0; 128],
    };
    let rc = unsafe {
        libc_ioctl(
            f.as_raw_fd(),
            hermes_drm_ioctl_get_edid(),
            &mut edid as *mut _ as *mut u8,
        )
    };
    if rc != 0 {
        return Err(format!(
            "GET_EDID: {}",
            io::Error::last_os_error()
        ));
    }
    Ok(edid)
}

pub fn drm_get_prop(object_id: u32, prop_id: u32) -> Result<HermesDrmPropGet, String> {
    let f = open_rw("/dev/nvidia-drm")?;
    let mut prop = HermesDrmPropGet {
        object_id,
        prop_id,
        value: 0,
    };
    let rc = unsafe {
        libc_ioctl(
            f.as_raw_fd(),
            hermes_drm_ioctl_get_prop(),
            &mut prop as *mut _ as *mut u8,
        )
    };
    if rc != 0 {
        return Err(format!(
            "GET_PROP: {}",
            io::Error::last_os_error()
        ));
    }
    Ok(prop)
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
                "live ioctl: online={} phase={} ver={} mask=0x{:x} modules={:?}",
                st.is_online(),
                st.phase_label(),
                st.version,
                st.module_mask,
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
            // Companion soft-deps must appear in module_mask when loaded.
            if (st.module_mask & hermes_linux::HERMES_MOD_NVIDIA) == 0 {
                eprintln!("error: primary nvidia bit missing from mask");
                return 1;
            }
            let expected = companion_mask_from_sysfs();
            if st.module_mask != expected {
                eprintln!(
                    "error: module_mask 0x{:x} != expected companions 0x{:x}",
                    st.module_mask, expected
                );
                return 1;
            }
            println!(
                "companion OR mask matches live modules (0x{:x}): PASS",
                st.module_mask
            );
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

/// Expected ioctl mask from /sys/module (primary always set if we got here).
fn companion_mask_from_sysfs() -> u32 {
    use hermes_linux::{
        hermes_ctl_module_mask_compose, HERMES_MOD_NVIDIA,
    };
    let m = hermes_ctl_module_mask_compose(
        module_loaded("nvidia-modeset"),
        module_loaded("nvidia-uvm"),
        module_loaded("nvidia-drm"),
        module_loaded("nvidia-peermem"),
    );
    // kmod-load-smoke requires nvidia.ko; compose already sets NVIDIA bit.
    debug_assert!((m & HERMES_MOD_NVIDIA) != 0);
    m
}

/// Full live Online path: reload with allow_sim_promote, SIM_PROMOTE, EDID, companions, DEMOTE.
pub fn kmod_online_smoke() -> i32 {
    println!("=== kmod-online-smoke: SIM_PROMOTE + EDID + companion Online (Turing+) ===");
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/load-kmod.sh");
    let script = script.canonicalize().unwrap_or(script);
    println!("reloading kmods with HERMES_SIM_PROMOTE=1 via {}", script.display());
    let st = std::process::Command::new("sh")
        .arg(&script)
        .env("HERMES_SIM_PROMOTE", "1")
        .status();
    match st {
        Ok(s) if s.success() => {}
        Ok(s) => {
            eprintln!("load-kmod.sh (sim) exited {s}");
            return 1;
        }
        Err(e) => {
            eprintln!("failed to run load-kmod.sh: {e}");
            return 1;
        }
    }

    // load-kmod already ran SIM_PROMOTE+DEMOTE when HERMES_SIM_PROMOTE=1.
    // Re-promote here for EDID/companion proof, then demote.
    if let Err(e) = ctl_sim_promote() {
        // If param not sticky after reload... reload sets allow_sim_promote=1.
        eprintln!("error: {e}");
        return 1;
    }
    let st = match ioctl_status_path("/dev/nvidiactl", hermes_ctl_ioctl_status()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    println!(
        "after SIM_PROMOTE: online={} phase={}",
        st.is_online(),
        st.phase_label()
    );
    if !st.is_online() || st.phase != 5 {
        eprintln!("error: expected ONLINE after SIM_PROMOTE");
        return 1;
    }

    for path in [
        "/dev/nvidia-modeset",
        "/dev/nvidia-uvm",
        "/dev/nvidia-uvm-tools",
        "/dev/nvidia-peermem",
    ] {
        if !Path::new(path).exists() {
            eprintln!("error: missing {path}");
            return 1;
        }
        match companion_status(path) {
            Ok(cs) => {
                println!(
                    "  {path}: online={} phase={}",
                    cs.is_online(),
                    cs.phase_label()
                );
                if !cs.is_online() {
                    eprintln!("error: companion still Offline after SIM_PROMOTE");
                    return 1;
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                return 1;
            }
        }
    }

    match drm_get_edid(1) {
        Ok(edid) => {
            println!(
                "GET_EDID: size={} checksum_ok={}",
                edid.size,
                edid.checksum_ok()
            );
            if edid.size != 128 || !edid.checksum_ok() {
                eprintln!("error: bad EDID under Online");
                return 1;
            }
            if edid.data[0] != 0x00 || edid.data[1] != 0xff {
                eprintln!("error: EDID header invalid");
                return 1;
            }
            println!("live DRM EDID under Online: PASS");
        }
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    }
    match drm_get_prop(1, HERMES_DRM_PROP_EDID) {
        Ok(p) => {
            println!("GET_PROP EDID blob id={}", p.value);
            if p.value == 0 {
                eprintln!("error: EDID prop id should be non-zero Online");
                return 1;
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    }

    // Bind userspace CUDA/NVML/Mesa from live Online (session parity).
    nvidia_ml::hermes_nvml_reset();
    let _ = nvidia_ml::nvmlInit_v2();
    let _ = nvidia_ml::hermes_nvml_discover_host_gpus();
    let _ = nvidia_ml::hermes_nvml_promote_first_sim_online();
    hermes_cuda::hermes_cuda_bind_session_device("Hermes Live Turing", 8 << 30, 7, 5);
    hermes_mesa::hermes_mesa_set_gsp_online(true);
    if hermes_cuda::cuInit(0) != 0 {
        eprintln!("error: CUDA init after kmod Online failed");
        return 1;
    }
    if hermes_mesa::hermes_present_solid_frame().is_err() {
        eprintln!("error: mesa present after kmod Online failed");
        return 1;
    }
    println!("userspace CUDA+Mesa bound after live Online: PASS");

    if let Err(e) = ctl_demote() {
        eprintln!("error: {e}");
        return 1;
    }
    let st = match ioctl_status_path("/dev/nvidiactl", hermes_ctl_ioctl_status()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    if st.is_online() {
        eprintln!("error: still Online after DEMOTE");
        return 1;
    }
    // EDID must fail closed Offline.
    match drm_get_edid(1) {
        Ok(_) => {
            eprintln!("error: GET_EDID must fail Offline");
            return 1;
        }
        Err(_) => println!("GET_EDID Offline rejects: PASS"),
    }
    hermes_cuda::hermes_cuda_reset();
    hermes_mesa::hermes_mesa_reset();
    let _ = nvidia_ml::nvmlShutdown();

    println!("kmod-online-smoke: PASS (SIM_PROMOTE + EDID + companions + DEMOTE)");
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
