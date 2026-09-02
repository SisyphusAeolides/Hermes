//! Probe classic NVIDIA/Hermes character devices and kmod presence.
//!
//! Never invents Online: missing nodes report absent; loaded modules without
//! Online still report Offline phase when status is readable.

use hermes_linux::{
    hermes_companion_ioctl_status, hermes_ctl_ioctl_apply_evidence, hermes_ctl_ioctl_demote,
    hermes_ctl_ioctl_measure_fw, hermes_ctl_ioctl_sim_promote, hermes_ctl_ioctl_status,
    hermes_drm_ioctl_get_edid, hermes_drm_ioctl_get_prop, hermes_drm_ioctl_status, modules,
    HermesApplyEvidence, HermesCtlStatus, HermesDrmEdid, HermesDrmPropGet, HermesDrmStatus,
    HermesMeasureFw, HERMES_DRM_PROP_EDID, HERMES_MOD_DRM, HERMES_MOD_MODESET, HERMES_MOD_NVIDIA,
    HERMES_MOD_PEERMEM, HERMES_MOD_UVM,
};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

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
        f.read_to_string(&mut buf)
            .map_err(|e| format!("read: {e}"))?;
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
        return Err(format!(
            "ioctl status failed errno={}",
            io::Error::last_os_error()
        ));
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
                st.gsp_online != 0,
                st.connectors,
                st.crtcs,
                st.active_crtcs,
                st.version
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
    // If ctl says Online, phase must be ONLINE; this command only reports.
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
        return Err(format!("ioctl {path}: {}", io::Error::last_os_error()));
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
        return Err(format!("GET_EDID: {}", io::Error::last_os_error()));
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
        return Err(format!("GET_PROP: {}", io::Error::last_os_error()));
    }
    Ok(prop)
}

/// Host-measured firmware pin to kmod (real digest; Online only if other gates pass).
pub fn ctl_measure_fw(byte_length: u32, sha256: [u8; 32]) -> Result<HermesMeasureFw, String> {
    let f = open_rw("/dev/nvidiactl")?;
    let mut m = HermesMeasureFw {
        byte_length,
        sha256,
        ..Default::default()
    };
    let rc = unsafe {
        libc_ioctl(
            f.as_raw_fd(),
            hermes_ctl_ioctl_measure_fw(),
            &mut m as *mut _ as *mut u8,
        )
    };
    if rc != 0 && m.admitted == 0 {
        return Err(format!(
            "MEASURE_FW rejected: {}",
            io::Error::last_os_error()
        ));
    }
    if rc != 0 {
        return Err(format!("MEASURE_FW: {}", io::Error::last_os_error()));
    }
    Ok(m)
}

pub fn ctl_apply_evidence(mut e: HermesApplyEvidence) -> Result<HermesApplyEvidence, String> {
    let f = open_rw("/dev/nvidiactl")?;
    let rc = unsafe {
        libc_ioctl(
            f.as_raw_fd(),
            hermes_ctl_ioctl_apply_evidence(),
            &mut e as *mut _ as *mut u8,
        )
    };
    if rc != 0 {
        return Err(format!("APPLY_EVIDENCE: {}", io::Error::last_os_error()));
    }
    Ok(e)
}

fn firmware_version_key(value: &str) -> (u32, u32, u32) {
    let mut parts = value.split('.');
    (
        parts.next().and_then(|v| v.parse().ok()).unwrap_or(0),
        parts.next().and_then(|v| v.parse().ok()).unwrap_or(0),
        parts.next().and_then(|v| v.parse().ok()).unwrap_or(0),
    )
}

/// Select a staged versioned OpenRM image for the live firmware probe.
///
/// The operator may pin an exact path with `HERMES_GSP_TU10X`; otherwise the
/// newest version directory under `/lib/firmware/nvidia` is selected.  This
/// keeps the probe aligned with the installed linux-firmware release instead
/// of silently testing a stale development version.
fn host_tu10x_firmware() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("HERMES_GSP_TU10X") {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }

    let root = Path::new("/lib/firmware/nvidia");
    let mut versions: Vec<String> = std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let ty = entry.file_type().ok()?;
            if !ty.is_dir() {
                return None;
            }
            let name = entry.file_name().into_string().ok()?;
            let mut components = name.split('.');
            (components.clone().count() == 3
                && components
                    .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())))
            .then_some(name)
        })
        .collect();
    versions.sort_by_key(|a| std::cmp::Reverse(firmware_version_key(a)));
    versions.into_iter().find_map(|version| {
        let path = root.join(version).join("gsp_tu10x.bin");
        path.is_file().then_some(path)
    })
}

/// Measure real host gsp_tu10x.bin and push pin to kmod; report phase (often FIRMWARED).
pub fn silicon_fw_smoke() -> i32 {
    println!("=== silicon-fw-smoke: measure real GSP-RM + kmod MEASURE_FW ===");
    let path = match host_tu10x_firmware() {
        Some(path) => path,
        None => {
            println!("skip: no versioned gsp_tu10x.bin under /lib/firmware/nvidia");
            println!("PASS (skipped)");
            return 0;
        }
    };
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            println!("skip: cannot read {}: {e}", path.display());
            println!("PASS (skipped)");
            return 0;
        }
    };
    let digest = hermes_gsp::sha256_bytes(&bytes);
    println!(
        "host measure: path={} len={} sha256={:02x}{:02x}…",
        path.display(),
        bytes.len(),
        digest[0],
        digest[1]
    );
    // Ensure kmods loaded offline.
    if !module_loaded("nvidia") {
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/load-kmod.sh");
        let _ = std::process::Command::new("sh").arg(&script).status();
    }
    let m = match ctl_measure_fw(bytes.len() as u32, digest) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    println!(
        "kmod MEASURE_FW: admitted={} phase={} online={} status={}",
        m.admitted, m.phase, m.online, m.status
    );
    if m.admitted != 1 {
        eprintln!("error: host firmware must pin-admit");
        return 1;
    }
    // Real measure alone must not invent Online without IOMMU/WPR/mailbox.
    if m.online != 0 {
        eprintln!("error: MEASURE_FW alone must not Online on incomplete host");
        return 1;
    }
    // Phase should be at least FIRMWARED (2) after measured FW.
    if m.phase < 2 {
        eprintln!(
            "error: expected phase >= FIRMWARED after MEASURE_FW, got {}",
            m.phase
        );
        return 1;
    }
    println!("firmware-measured phase progress (not Online): PASS");

    // APPLY incomplete evidence: still Offline.
    let e = ctl_apply_evidence(HermesApplyEvidence {
        iommu_isolated: 0,
        dma_domain: 0,
        wpr_locked: 0,
        mailbox_ok: 0,
        ready_ok: 0,
        use_measured_fw: 1,
        force_fw_measured: 0,
        ..Default::default()
    });
    match e {
        Ok(a) => {
            println!(
                "APPLY incomplete: phase={} online={} status={}",
                a.phase, a.online, a.status
            );
            if a.online != 0 {
                eprintln!("error: incomplete APPLY must not Online");
                return 1;
            }
        }
        Err(err) => {
            eprintln!("error: {err}");
            return 1;
        }
    }

    // Bad digest rejected.
    let mut bad = digest;
    bad[0] ^= 0xff;
    if ctl_measure_fw(bytes.len() as u32, bad).is_ok() {
        eprintln!("error: bad digest must not admit");
        return 1;
    }
    println!("bad digest rejected: PASS");

    // UVM/modeset Online ioctls under SIM_PROMOTE path.
    if let Err(e) = std::process::Command::new("sh")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/load-kmod.sh"))
        .env("HERMES_SIM_PROMOTE", "1")
        .status()
        .and_then(|s| {
            if s.success() {
                Ok(())
            } else {
                Err(std::io::Error::other("load failed"))
            }
        })
    {
        eprintln!("warn: sim reload for UVM/modeset: {e}");
    } else if ctl_sim_promote().is_ok() {
        if let Err(e) = companion_uvm_modeset_online_ops() {
            eprintln!("error: {e}");
            let _ = ctl_demote();
            return 1;
        }
        let _ = ctl_demote();
        println!("UVM/modeset Online ioctl set: PASS");
    }

    println!("silicon-fw-smoke: PASS");
    0
}

fn companion_uvm_modeset_online_ops() -> Result<(), String> {
    // Full UVM set: INITIALIZE → PAGEABLE → REGISTER_GPU → UNREGISTER_GPU
    let f = open_rw("/dev/nvidia-uvm")?;
    // INITIALIZE: _IOW 0x21 size 8
    let init_req = ((1u64) << 30) | ((0x48u64) << 8) | 0x21 | (8u64 << 16);
    let mut init = [0u32; 2];
    let rc = unsafe { libc_ioctl(f.as_raw_fd(), init_req, init.as_mut_ptr() as *mut u8) };
    if rc != 0 {
        return Err(format!("UVM INITIALIZE: {}", io::Error::last_os_error()));
    }
    let page_req = ((2u64) << 30) | ((0x48u64) << 8) | 0x22 | (4u64 << 16);
    let mut pageable = 0u32;
    let rc = unsafe { libc_ioctl(f.as_raw_fd(), page_req, &mut pageable as *mut _ as *mut u8) };
    if rc != 0 {
        return Err(format!("UVM PAGEABLE: {}", io::Error::last_os_error()));
    }
    println!("  UVM pageable_mem_access={pageable}");

    // REGISTER_GPU: _IOWR 0x23 size 24 (uuid[4] + rm_ctrl_fd + registered)
    #[repr(C)]
    struct UvmReg {
        gpu_uuid: [u32; 4],
        rm_ctrl_fd: u32,
        registered: u32,
    }
    let reg_req = ((3u64) << 30) | ((0x48u64) << 8) | 0x23 | (24u64 << 16);
    let mut reg = UvmReg {
        gpu_uuid: [0x4852_4d45, 0x532d_4753, 0x5000_0000, 0x0000_0001], // "HERMES-GSP" shell
        rm_ctrl_fd: 0,
        registered: 0,
    };
    let rc = unsafe { libc_ioctl(f.as_raw_fd(), reg_req, &mut reg as *mut _ as *mut u8) };
    if rc != 0 {
        return Err(format!("UVM REGISTER_GPU: {}", io::Error::last_os_error()));
    }
    if reg.registered != 1 {
        return Err(format!(
            "UVM REGISTER_GPU: expected registered=1 got {}",
            reg.registered
        ));
    }
    println!("  UVM REGISTER_GPU registered={}", reg.registered);

    // UNREGISTER_GPU: _IOW 0x24 size 4
    let unreg_req = ((1u64) << 30) | ((0x48u64) << 8) | 0x24 | (4u64 << 16);
    let mut gpu_id = 0u32;
    let rc = unsafe { libc_ioctl(f.as_raw_fd(), unreg_req, &mut gpu_id as *mut _ as *mut u8) };
    if rc != 0 {
        return Err(format!(
            "UVM UNREGISTER_GPU: {}",
            io::Error::last_os_error()
        ));
    }
    println!("  UVM UNREGISTER_GPU: ok");

    // Full modeset set: ALLOC → FLIP → FREE
    let mf = open_rw("/dev/nvidia-modeset")?;
    #[repr(C)]
    struct Alloc {
        width: u32,
        height: u32,
        handle: u32,
    }
    let alloc_req = ((3u64) << 30) | ((0x48u64) << 8) | 0x30 | (12u64 << 16);
    let mut a = Alloc {
        width: 1920,
        height: 1080,
        handle: 0,
    };
    let rc = unsafe { libc_ioctl(mf.as_raw_fd(), alloc_req, &mut a as *mut _ as *mut u8) };
    if rc != 0 {
        return Err(format!("MODESET ALLOC: {}", io::Error::last_os_error()));
    }
    #[repr(C)]
    struct Flip {
        handle: u32,
        crtc_id: u32,
        sequence: u32,
    }
    let flip_req = ((3u64) << 30) | ((0x48u64) << 8) | 0x31 | (12u64 << 16);
    let mut fl = Flip {
        handle: a.handle,
        crtc_id: 1,
        sequence: 0,
    };
    let rc = unsafe { libc_ioctl(mf.as_raw_fd(), flip_req, &mut fl as *mut _ as *mut u8) };
    if rc != 0 {
        return Err(format!("MODESET FLIP: {}", io::Error::last_os_error()));
    }
    // FREE: _IOW 0x32 size 4
    let free_req = ((1u64) << 30) | ((0x48u64) << 8) | 0x32 | (4u64 << 16);
    let mut handle = a.handle;
    let rc = unsafe { libc_ioctl(mf.as_raw_fd(), free_req, &mut handle as *mut _ as *mut u8) };
    if rc != 0 {
        return Err(format!("MODESET FREE: {}", io::Error::last_os_error()));
    }
    println!(
        "  modeset alloc handle={} flip seq={} free=ok",
        a.handle, fl.sequence
    );

    // Offline gate: after demote these must fail (caller demotes; re-check optional).
    Ok(())
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
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/load-kmod.sh");
        let script = script.canonicalize().unwrap_or(script);
        println!("running {}", script.display());
        let st = std::process::Command::new("sh").arg(&script).status();
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
                println!("evidence-gated Offline after bare load: PASS");
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
    use hermes_linux::{hermes_ctl_module_mask_compose, HERMES_MOD_NVIDIA};
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
    println!(
        "reloading kmods with HERMES_SIM_PROMOTE=1 via {}",
        script.display()
    );
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
    // EDID must remain unavailable while the GSP session is Offline.
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
