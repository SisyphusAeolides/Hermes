//! Live host silicon probe — fail-closed, never invents Online.
//!
//! Scans sysfs PCI for NVIDIA Turing+, admits staged GSP-RM from linux-firmware,
//! reports BAR/IOMMU/driver binding. MMIO map requires elevated privileges and
//! unbound driver; without them this path stays Offline by design.

use std::fs;
use std::path::{Path, PathBuf};

use hermes_core::{
    admit_display_device, is_nvidia_turing_or_newer, nvidia_architecture, pci_identity,
    NVIDIA_VENDOR_ID,
};
use hermes_gsp::{
    chip_gsp_relative, facts_from_sysfs, firmware_family_for_device, host_may_claim_online,
    host_online_blockers, openrm_gsp_relative, parse_gsp_rm_elf, sha256_bytes, FirmwareFamily,
    HostDeviceFacts, NvidiaChipDir, NvidiaGspFirmwareAuthority, fwversion_bytes,
};

#[derive(Clone, Debug)]
pub struct PciGpu {
    pub bdf: String,
    pub vendor: u16,
    pub device: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub driver: Option<String>,
    pub iommu_group: Option<u32>,
    pub bar0: Option<(u64, u64)>,
    #[allow(dead_code)]
    pub sysfs: PathBuf,
}

#[derive(Clone, Debug)]
pub struct FirmwareProbe {
    pub path: String,
    pub length: u64,
    pub sha256: [u8; 32],
    pub admitted: bool,
    pub elf_ok: bool,
    pub version_field: Option<String>,
    pub reject: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SiliconReport {
    pub gpus: Vec<PciGpu>,
    pub firmware: Vec<FirmwareProbe>,
    pub online_claimed: bool,
    pub blockers: Vec<String>,
    /// Per-GPU host gate facts used by shared preflight.
    pub host_facts: Vec<(String, HostDeviceFacts, bool)>,
}

/// Result of attempting to open PCI BAR0 via sysfs `resource0`.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BarMapAttempt {
    pub bdf: String,
    pub path: String,
    pub ok: bool,
    #[allow(dead_code)]
    pub bytes_readable: usize,
    pub error: Option<String>,
}

fn parse_hex_u16(s: &str) -> Option<u16> {
    let s = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    u16::from_str_radix(s, 16).ok()
}

fn parse_hex_u64(s: &str) -> Option<u64> {
    let s = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    u64::from_str_radix(s, 16).ok()
}

fn read_trim(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn parse_resource0(path: &Path) -> Option<(u64, u64)> {
    let text = fs::read_to_string(path).ok()?;
    let line = text.lines().next()?;
    let mut parts = line.split_whitespace();
    let start = parse_hex_u64(parts.next()?)?;
    let end = parse_hex_u64(parts.next()?)?;
    if start == 0 && end == 0 {
        return None;
    }
    Some((start, end.saturating_sub(start).saturating_add(1)))
}

fn iommu_group_id(dev: &Path) -> Option<u32> {
    let link = fs::read_link(dev.join("iommu_group")).ok()?;
    link.file_name()?
        .to_str()?
        .parse()
        .ok()
}

fn driver_name(dev: &Path) -> Option<String> {
    let link = fs::read_link(dev.join("driver")).ok()?;
    link.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
}

/// Enumerate NVIDIA display-class devices under `/sys/bus/pci/devices`.
pub fn scan_nvidia_gpus(sys_pci: &Path) -> Vec<PciGpu> {
    let mut out = Vec::new();
    let rd = match fs::read_dir(sys_pci) {
        Ok(r) => r,
        Err(_) => return out,
    };
    for ent in rd.flatten() {
        let path = ent.path();
        let vendor = match read_trim(&path.join("vendor")).and_then(|s| parse_hex_u16(&s)) {
            Some(v) => v,
            None => continue,
        };
        if vendor != NVIDIA_VENDOR_ID {
            continue;
        }
        let device = match read_trim(&path.join("device")).and_then(|s| parse_hex_u16(&s)) {
            Some(d) => d,
            None => continue,
        };
        let class_raw = read_trim(&path.join("class")).unwrap_or_default();
        // class is 0x0clssubprog as 6 hex digits often with 0x
        let class_u32 = parse_hex_u64(&class_raw).unwrap_or(0) as u32;
        let class_code = ((class_u32 >> 16) & 0xff) as u8;
        let subclass = ((class_u32 >> 8) & 0xff) as u8;
        if class_code != 0x03 {
            continue;
        }
        let bdf = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        out.push(PciGpu {
            bdf,
            vendor,
            device,
            class_code,
            subclass,
            driver: driver_name(&path),
            iommu_group: iommu_group_id(&path),
            bar0: parse_resource0(&path.join("resource")),
            sysfs: path,
        });
    }
    out.sort_by(|a, b| a.bdf.cmp(&b.bdf));
    out
}

pub fn probe_firmware_file(path: &Path, device_id: u16) -> FirmwareProbe {
    let path_s = path.display().to_string();
    match fs::read(path) {
        Ok(bytes) => {
            let digest = sha256_bytes(&bytes);
            let auth = NvidiaGspFirmwareAuthority::default_allow_list();
            let admit = auth.admit(device_id, &bytes);
            let elf = parse_gsp_rm_elf(&bytes);
            let version_field = elf.as_ref().ok().and_then(|e| {
                fwversion_bytes(&bytes, e)
                    .ok()
                    .map(|s| String::from_utf8_lossy(s).into_owned())
            });
            match admit {
                Ok(_) => FirmwareProbe {
                    path: path_s,
                    length: bytes.len() as u64,
                    sha256: digest,
                    admitted: true,
                    elf_ok: elf.is_ok(),
                    version_field,
                    reject: None,
                },
                Err(e) => FirmwareProbe {
                    path: path_s,
                    length: bytes.len() as u64,
                    sha256: digest,
                    admitted: false,
                    elf_ok: elf.is_ok(),
                    version_field,
                    reject: Some(format!("{e:?}")),
                },
            }
        }
        Err(e) => FirmwareProbe {
            path: path_s,
            length: 0,
            sha256: [0; 32],
            admitted: false,
            elf_ok: false,
            version_field: None,
            reject: Some(format!("read: {e}")),
        },
    }
}

/// Attempt a real open+read of sysfs `resource0` (BAR0). Never invents success.
pub fn attempt_bar0_map(gpu: &PciGpu) -> BarMapAttempt {
    let path = gpu.sysfs.join("resource0");
    let path_s = path.display().to_string();
    match fs::File::open(&path) {
        Ok(mut f) => {
            use std::io::Read;
            let mut buf = [0u8; 4];
            match f.read(&mut buf) {
                Ok(n) if n > 0 => BarMapAttempt {
                    bdf: gpu.bdf.clone(),
                    path: path_s,
                    ok: true,
                    bytes_readable: n,
                    error: None,
                },
                Ok(_) => BarMapAttempt {
                    bdf: gpu.bdf.clone(),
                    path: path_s,
                    ok: false,
                    bytes_readable: 0,
                    error: Some("read returned 0 bytes".into()),
                },
                Err(e) => BarMapAttempt {
                    bdf: gpu.bdf.clone(),
                    path: path_s,
                    ok: false,
                    bytes_readable: 0,
                    error: Some(format!("read: {e}")),
                },
            }
        }
        Err(e) => BarMapAttempt {
            bdf: gpu.bdf.clone(),
            path: path_s,
            ok: false,
            bytes_readable: 0,
            error: Some(format!("open: {e}")),
        },
    }
}

/// Build host facts from a scanned GPU + optional BAR map result.
pub fn facts_for_gpu(gpu: &PciGpu, bar_mapped: bool) -> HostDeviceFacts {
    facts_from_sysfs(
        gpu.iommu_group,
        gpu.driver.as_deref(),
        gpu.bar0.is_some(),
        bar_mapped,
    )
}

/// Full host silicon report. `online_claimed` is true only when host gate
/// allows Online *and* operator explicitly runs a full evidence path — this
/// probe never claims Online (no WPR/mailbox authority here).
pub fn probe_host(firmware_root: &Path) -> SiliconReport {
    let mut blockers = Vec::new();
    let gpus = scan_nvidia_gpus(Path::new("/sys/bus/pci/devices"));
    if gpus.is_empty() {
        blockers.push("no NVIDIA display-class PCI devices in sysfs".into());
    }

    let mut firmware = Vec::new();
    let mut any_turing_plus = false;
    let mut any_fw_admit = false;
    let mut host_facts = Vec::new();

    for gpu in &gpus {
        let turing = is_nvidia_turing_or_newer(gpu.device);
        if turing {
            any_turing_plus = true;
        } else {
            blockers.push(format!(
                "{} device {:04x} is pre-Turing — Hermes rejects",
                gpu.bdf, gpu.device
            ));
            continue;
        }

        // Real BAR open attempt (usually fails without root / while Nouveau owns it).
        let bar_try = attempt_bar0_map(gpu);
        let facts = facts_for_gpu(gpu, bar_try.ok);
        let may = host_may_claim_online(&facts);
        host_facts.push((gpu.bdf.clone(), facts, may));

        for b in host_online_blockers(&facts) {
            blockers.push(format!("{}: {}", gpu.bdf, b.as_str()));
        }
        if !bar_try.ok {
            blockers.push(format!(
                "{} BAR0 map attempt failed: {}",
                gpu.bdf,
                bar_try.error.as_deref().unwrap_or("unknown")
            ));
        }

        let id = pci_identity(gpu.vendor, gpu.device, gpu.class_code, gpu.subclass);
        if let Err(e) = admit_display_device(&id) {
            blockers.push(format!("{} admission: {e:?}", gpu.bdf));
        }

        let family = firmware_family_for_device(gpu.device).unwrap_or(FirmwareFamily::Tu10x);
        let mut candidates = vec![firmware_root.join(openrm_gsp_relative("610.43.02", family))];
        if family == FirmwareFamily::Tu10x {
            candidates.push(firmware_root.join(chip_gsp_relative(NvidiaChipDir::Tu117, "570.144")));
            candidates.push(firmware_root.join("nvidia/tu117/gsp/gsp-570.144.bin"));
        }
        for c in &candidates {
            if c.exists() {
                let probe = probe_firmware_file(c, gpu.device);
                if probe.admitted {
                    any_fw_admit = true;
                }
                firmware.push(probe);
            }
        }
    }

    if firmware.is_empty() {
        for (fam, dev) in [
            (FirmwareFamily::Tu10x, 0x1fb9u16),
            (FirmwareFamily::Ga10x, 0x2204u16),
        ] {
            let p = firmware_root.join(openrm_gsp_relative("610.43.02", fam));
            if p.exists() {
                let probe = probe_firmware_file(&p, dev);
                if probe.admitted {
                    any_fw_admit = true;
                }
                firmware.push(probe);
            }
        }
    }

    if !any_fw_admit {
        blockers.push("no GSP-RM image admitted from firmware root".into());
    }
    if !any_turing_plus && !gpus.is_empty() {
        blockers.push("no Turing+ GPU present".into());
    }

    // Probe never claims Online — no WPR/mailbox/ready authority here.
    let online_claimed = false;
    debug_assert!(!host_facts.iter().any(|(_, _, may)| *may) || !online_claimed);

    SiliconReport {
        gpus,
        firmware,
        online_claimed,
        blockers,
        host_facts,
    }
}

/// Host-bar smoke: for each GPU, attempt BAR0 and print shared gate verdict.
pub fn host_bar_smoke() {
    let gpus = scan_nvidia_gpus(Path::new("/sys/bus/pci/devices"));
    println!("Hermes host-bar smoke (real resource0 open; fail-closed)");
    if gpus.is_empty() {
        println!("no NVIDIA display GPUs");
        println!("PASS");
        return;
    }
    let mut any_online_gate = false;
    for gpu in &gpus {
        let attempt = attempt_bar0_map(gpu);
        let facts = facts_for_gpu(gpu, attempt.ok);
        let may = host_may_claim_online(&facts);
        any_online_gate |= may;
        println!(
            "  {} driver={} iommu={:?} bar_described={} bar_map_ok={} may_claim_online={}",
            gpu.bdf,
            gpu.driver.as_deref().unwrap_or("-"),
            gpu.iommu_group,
            facts.bar0_described,
            attempt.ok,
            may
        );
        if !attempt.ok {
            println!(
                "    bar error: {}",
                attempt.error.as_deref().unwrap_or("?")
            );
        }
        for b in host_online_blockers(&facts) {
            println!("    blocker: {}", b.as_str());
        }
    }
    // On this class of host we expect may_claim_online=false for all.
    if any_online_gate {
        println!("note: host gate clear — Online still requires full bring-up evidence");
    }
    println!("online_claimed: false");
    println!("PASS");
}

pub fn print_report(r: &SiliconReport) {
    println!("Hermes silicon probe (fail-closed; Online never invented)");
    println!("GPUs found: {}", r.gpus.len());
    for g in &r.gpus {
        let arch = nvidia_architecture(g.device)
            .map(|a| a.as_str())
            .unwrap_or("?");
        println!(
            "  {} {:04x}:{:04x} class={:02x}:{:02x} arch={} turing+={} driver={} iommu={:?} bar0={:?}",
            g.bdf,
            g.vendor,
            g.device,
            g.class_code,
            g.subclass,
            arch,
            is_nvidia_turing_or_newer(g.device),
            g.driver.as_deref().unwrap_or("-"),
            g.iommu_group,
            g.bar0.map(|(a, l)| format!("{a:#x}+{l:#x}"))
        );
    }
    for (bdf, facts, may) in &r.host_facts {
        println!(
            "  host_gate {bdf}: iommu={:?} foreign={} bar_desc={} bar_map={} may_claim_online={may}",
            facts.iommu_group, facts.foreign_driver_bound, facts.bar0_described, facts.bar0_mapped
        );
    }
    println!("Firmware probes: {}", r.firmware.len());
    for f in &r.firmware {
        println!(
            "  {} len={} admit={} elf={} ver={} reject={}",
            f.path,
            f.length,
            f.admitted,
            f.elf_ok,
            f.version_field.as_deref().unwrap_or("-"),
            f.reject.as_deref().unwrap_or("-")
        );
        println!(
            "    sha256={:02x}{:02x}{:02x}{:02x}…",
            f.sha256[0], f.sha256[1], f.sha256[2], f.sha256[3]
        );
    }
    println!("online_claimed: {}", r.online_claimed);
    if r.blockers.is_empty() {
        println!("blockers: (none listed — still Offline without privileged WPR/mailbox path)");
    } else {
        println!("blockers:");
        for b in &r.blockers {
            println!("  - {b}");
        }
    }
    println!("PASS (probe complete; Online not claimed)");
}
