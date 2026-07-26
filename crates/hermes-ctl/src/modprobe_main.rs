//! `nvidia-modprobe` drop-in — load/create helpers for Hermes open-stack names.
//!
//! Classic proprietary `nvidia-modprobe` is a setuid helper that loads modules
//! and creates `/dev/nvidia*` nodes. This binary speaks the same operator
//! surface and **never invents success**: module presence and node creation
//! are reported from sysfs / filesystem state, not assumed Online.

use hermes_linux::{devices, modules, MODULE_SURFACES};
use std::env;
use std::fs;
use std::path::Path;
use std::process::{self, Command};

#[derive(Default)]
struct Opts {
    verbose: bool,
    uvm: bool,
    modeset: bool,
    drm: bool,
    peermem: bool,
    /// Load the core `nvidia` (Hermes) module.
    core: bool,
    /// Attempt mknod-style device node creation (requires privileges).
    create_devices: bool,
    /// Minor index for /dev/nvidiaN (default 0 when create).
    minor: Option<u32>,
    /// Print catalog / status only.
    status: bool,
    help: bool,
}

fn main() {
    let opts = parse_args(env::args().skip(1));
    if opts.help {
        print_help();
        return;
    }

    if opts.status || nothing_requested(&opts) {
        print_status(opts.verbose);
        if nothing_requested(&opts) && !opts.status {
            // Classic modprobe with no flags still ensures core + devices.
            let mut o = opts;
            o.core = true;
            o.create_devices = true;
            o.minor = Some(0);
            run_actions(&o);
        }
        return;
    }

    run_actions(&opts);
}

fn nothing_requested(o: &Opts) -> bool {
    !o.uvm
        && !o.modeset
        && !o.drm
        && !o.peermem
        && !o.core
        && !o.create_devices
        && o.minor.is_none()
        && !o.status
}

fn parse_args(args: impl Iterator<Item = String>) -> Opts {
    let mut o = Opts::default();
    let mut it = args.peekable();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => o.help = true,
            "-v" | "--verbose" => o.verbose = true,
            "-u" | "--uvm" => o.uvm = true,
            "-m" | "--modeset" => o.modeset = true,
            "--drm" => o.drm = true,
            "--peermem" => o.peermem = true,
            "-s" | "--create-devices" => o.create_devices = true,
            "--status" => o.status = true,
            "-c" => {
                o.create_devices = true;
                if let Some(n) = it.peek().and_then(|s| s.parse::<u32>().ok()) {
                    o.minor = Some(n);
                    it.next();
                } else {
                    o.minor = Some(0);
                }
            }
            s if s.starts_with("-c") && s.len() > 2 => {
                o.create_devices = true;
                o.minor = s[2..].parse().ok().or(Some(0));
            }
            "--" => break,
            other => {
                eprintln!("nvidia-modprobe: unknown option '{other}' (try --help)");
                process::exit(2);
            }
        }
    }
    o
}

fn print_help() {
    println!(
        "nvidia-modprobe — Hermes drop-in (open-stack module / device helper)\n\
         \n\
         Usage: nvidia-modprobe [OPTIONS]\n\
         \n\
           -u, --uvm             Ensure nvidia-uvm is loadable / present\n\
           -m, --modeset         Ensure nvidia-modeset is present\n\
               --drm             Ensure nvidia-drm is present\n\
               --peermem         Ensure nvidia-peermem is present\n\
           -s, --create-devices  Create /dev/nvidia* nodes when possible\n\
           -c [N]                Create devices for minor N (default 0)\n\
               --status          Report module + device node state\n\
           -v, --verbose         Extra detail\n\
           -h, --help            This help\n\
         \n\
         Never implies GSP Online. Staging binaries alone do not load firmware."
    );
}

fn print_status(verbose: bool) {
    println!("Hermes nvidia-modprobe status");
    println!("modules (advertised):");
    for m in MODULE_SURFACES {
        let loaded = module_loaded(m.name);
        let mark = if loaded { "loaded" } else { "absent" };
        println!("  {:<18} {mark}", m.name);
        if verbose {
            println!("    replaces={} — {}", m.replaces, m.description);
        }
    }
    println!("devices:");
    for path in [
        devices::NVIDIA_CTL,
        devices::NVIDIA_0,
        devices::NVIDIA_UVM,
        devices::NVIDIA_MODESET,
        "/dev/nvidia-drm",
    ] {
        let ok = Path::new(path).exists();
        println!("  {path:<24} {}", if ok { "present" } else { "missing" });
    }
    if verbose {
        println!("note: presence of nodes/modules ≠ Hermes Online manifold");
    }
}

fn run_actions(opts: &Opts) {
    let mut failed = false;
    let mut wanted: Vec<&str> = Vec::new();
    if opts.core || opts.create_devices {
        wanted.push(modules::NVIDIA);
    }
    if opts.uvm {
        wanted.push(modules::NVIDIA_UVM);
    }
    if opts.modeset {
        wanted.push(modules::NVIDIA_MODESET);
    }
    if opts.drm {
        wanted.push(modules::NVIDIA_DRM);
    }
    if opts.peermem {
        wanted.push(modules::NVIDIA_PEERMEM);
    }
    // Classic -u often implies core first.
    if opts.uvm && !wanted.contains(&modules::NVIDIA) {
        wanted.insert(0, modules::NVIDIA);
    }

    for name in &wanted {
        match ensure_module(name, opts.verbose) {
            Ok(true) => {
                if opts.verbose {
                    println!("module {name}: already loaded");
                }
            }
            Ok(false) => {
                println!("module {name}: load attempted");
            }
            Err(e) => {
                eprintln!("module {name}: {e}");
                failed = true;
            }
        }
    }

    if opts.create_devices {
        let minor = opts.minor.unwrap_or(0);
        for path in device_paths_for_create(minor) {
            match ensure_device_node(&path, opts.verbose) {
                Ok(msg) => {
                    if opts.verbose || msg != "exists" {
                        println!("device {path}: {msg}");
                    }
                }
                Err(e) => {
                    eprintln!("device {path}: {e}");
                    failed = true;
                }
            }
        }
    }

    if failed {
        eprintln!("nvidia-modprobe: one or more actions failed (fail-closed)");
        process::exit(1);
    }
    if opts.verbose {
        println!("nvidia-modprobe: done (no Online claim)");
    }
}

fn module_loaded(name: &str) -> bool {
    Path::new(&format!("/sys/module/{name}")).exists()
        || fs::read_to_string("/proc/modules")
            .map(|t| t.lines().any(|l| l.split_whitespace().next() == Some(name)))
            .unwrap_or(false)
}

fn ensure_module(name: &str, verbose: bool) -> Result<bool, String> {
    if module_loaded(name) {
        return Ok(true);
    }
    // Prefer modprobe; fall back to reporting absence honestly.
    let out = Command::new("modprobe").arg(name).output();
    match out {
        Ok(o) if o.status.success() => {
            if module_loaded(name) {
                Ok(false)
            } else {
                Err("modprobe returned success but module still absent".into())
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            let msg = stderr.trim();
            if verbose && !msg.is_empty() {
                eprintln!("  modprobe {name}: {msg}");
            }
            // Also try insmod path under linux/kmod if built.
            try_insmod_local(name, verbose)?;
            if module_loaded(name) {
                Ok(false)
            } else {
                Err(format!(
                    "not loaded (modprobe exit {}); build/load linux/kmod/{name}.ko",
                    o.status.code().unwrap_or(-1)
                ))
            }
        }
        Err(e) => {
            try_insmod_local(name, verbose)?;
            if module_loaded(name) {
                Ok(false)
            } else {
                Err(format!("modprobe unavailable ({e}); module not present"))
            }
        }
    }
}

fn try_insmod_local(name: &str, verbose: bool) -> Result<(), String> {
    let candidates = [
        format!("linux/kmod/{name}.ko"),
        format!("/home/Sisyphus/Projects/Hermes/linux/kmod/{name}.ko"),
    ];
    for p in &candidates {
        if !Path::new(p).exists() {
            continue;
        }
        if verbose {
            println!("  trying insmod {p}");
        }
        let o = Command::new("insmod").arg(p).output();
        match o {
            Ok(r) if r.status.success() => return Ok(()),
            Ok(r) => {
                if verbose {
                    eprintln!(
                        "  insmod {p}: {}",
                        String::from_utf8_lossy(&r.stderr).trim()
                    );
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("  insmod {p}: {e}");
                }
            }
        }
    }
    Ok(())
}

fn device_paths_for_create(minor: u32) -> Vec<String> {
    let mut v = vec![
        devices::NVIDIA_CTL.to_string(),
        format!("/dev/nvidia{minor}"),
        devices::NVIDIA_UVM.to_string(),
        devices::NVIDIA_MODESET.to_string(),
        "/dev/nvidia-drm".to_string(),
    ];
    v.sort();
    v.dedup();
    v
}

fn ensure_device_node(path: &str, verbose: bool) -> Result<&'static str, String> {
    if Path::new(path).exists() {
        return Ok("exists");
    }
    // Without a registered char major from a loaded module, refuse silent fake nodes.
    let major = char_major_for_path(path);
    if major.is_none() {
        return Err(
            "no char major in /proc/devices (load nvidia* kmod first; refuse forged node)".into(),
        );
    }
    let major = major.unwrap();
    let minor = minor_for_path(path);
    // Prefer mknod via `mknod` utility.
    let mode = "0666";
    let o = Command::new("mknod")
        .args([path, "c", &major.to_string(), &minor.to_string()])
        .output();
    match o {
        Ok(r) if r.status.success() => {
            let _ = Command::new("chmod").args([mode, path]).status();
            Ok("created")
        }
        Ok(r) => {
            let err = String::from_utf8_lossy(&r.stderr);
            if verbose {
                eprintln!("  mknod {path}: {}", err.trim());
            }
            Err(format!(
                "mknod failed (need privileges?): {}",
                err.trim()
            ))
        }
        Err(e) => Err(format!("mknod unavailable: {e}")),
    }
}

fn char_major_for_path(path: &str) -> Option<u32> {
    let text = fs::read_to_string("/proc/devices").ok()?;
    let mut in_char = false;
    let mut nvidia_major = None;
    let mut nvidiauvm_major = None;
    for line in text.lines() {
        if line.starts_with("Character devices:") {
            in_char = true;
            continue;
        }
        if line.starts_with("Block devices:") {
            break;
        }
        if !in_char {
            continue;
        }
        let mut parts = line.split_whitespace();
        let maj = parts.next().and_then(|s| s.parse::<u32>().ok());
        let name = parts.next().unwrap_or("");
        if let Some(m) = maj {
            if name == "nvidia" || name == "nvidia-frontend" {
                nvidia_major = Some(m);
            }
            if name == "nvidia-uvm" {
                nvidiauvm_major = Some(m);
            }
            if name == "nvidia-modeset" && path.contains("modeset") {
                return Some(m);
            }
        }
    }
    if path.contains("uvm") {
        return nvidiauvm_major.or(nvidia_major);
    }
    nvidia_major
}

fn minor_for_path(path: &str) -> u32 {
    if path.ends_with("nvidiactl") || path.ends_with("nvidia-modeset") {
        return 255;
    }
    if path.contains("nvidia-uvm") || path.contains("nvidia-drm") {
        return 0;
    }
    // /dev/nvidiaN
    path.rsplit(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}
