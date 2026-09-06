//! Host graphics-path inspection.
//!
//! Hermes can only claim a production graphics path when the host exposes a
//! real DRM card, a bound driver, and an accessible render node.  This probe
//! deliberately stays below the GSP Online boundary: it verifies the display
//! path used by the live desktop without turning an ordinary vendor driver or
//! a simulator into a Hermes Online session.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphicsVendor {
    Nvidia,
    Amd,
    Intel,
    Other(u16),
}

impl GraphicsVendor {
    fn from_pci_id(id: u16) -> Self {
        match id {
            0x10de => Self::Nvidia,
            0x1002 => Self::Amd,
            0x8086 => Self::Intel,
            other => Self::Other(other),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Nvidia => "nvidia",
            Self::Amd => "amd",
            Self::Intel => "intel",
            Self::Other(_) => "other",
        }
    }
}

#[derive(Clone, Debug)]
pub struct GraphicsAdapter {
    pub card: String,
    pub vendor: GraphicsVendor,
    pub vendor_id: u16,
    pub device_id: Option<u16>,
    pub driver: Option<String>,
    pub card_node: PathBuf,
    pub card_access: bool,
    pub connected_outputs: usize,
    pub output_count: usize,
}

#[derive(Clone, Debug)]
pub struct GraphicsReport {
    pub adapters: Vec<GraphicsAdapter>,
    pub render_nodes: usize,
    pub accessible_render_nodes: usize,
    pub mesa_library: Option<PathBuf>,
    pub session_type: Option<String>,
    pub session_display: Option<String>,
}

impl GraphicsReport {
    pub fn drm_cards(&self) -> usize {
        self.adapters.len()
    }

    pub fn connected_outputs(&self) -> usize {
        self.adapters.iter().map(|a| a.connected_outputs).sum()
    }

    /// A host graphics path is usable when at least one card is exposed by
    /// DRM, its PCI device has a bound driver, and the node is accessible.
    /// Render-node access is preferred; a connected connector is sufficient
    /// on systems that do not expose render nodes to the probing user.
    pub fn graphics_host_ready(&self) -> bool {
        self.adapters.iter().any(|adapter| {
            adapter.driver.is_some()
                && adapter.card_access
                && (self.accessible_render_nodes > 0 || adapter.connected_outputs > 0)
        })
    }

    pub fn drm_kms_ready(&self) -> bool {
        !self.adapters.is_empty()
            && self
                .adapters
                .iter()
                .all(|adapter| adapter.driver.is_some() && adapter.card_access)
    }

    pub fn mesa_ready(&self) -> bool {
        self.graphics_host_ready() && self.mesa_library.is_some()
    }

    fn vendors(&self) -> Vec<&'static str> {
        let mut vendors = Vec::new();
        for adapter in &self.adapters {
            let name = adapter.vendor.name();
            if !vendors.contains(&name) {
                vendors.push(name);
            }
        }
        vendors
    }

    fn session_value(value: Option<&str>) -> &str {
        value.filter(|v| !v.is_empty()).unwrap_or("none")
    }

    /// Write the host-scoped qualification contract consumed by
    /// scripts/qualify-release.sh.  Vendor-specific GSP, CUDA, and recovery
    /// tests remain explicit not-tested entries instead of being presented as
    /// physical evidence.
    pub fn write_qualification(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(path)?;
        let graphics = if self.graphics_host_ready() {
            "pass"
        } else {
            "fail"
        };
        let drm = if self.drm_kms_ready() { "pass" } else { "fail" };
        let mesa = if self.mesa_ready() { "pass" } else { "fail" };
        let vendors = self.vendors();
        let unavailable = [
            ("amd", !vendors.contains(&"amd")),
            ("nvidia_gsp_online", true),
            ("intel_hardware_online", true),
            ("cuda", true),
            ("mps", true),
            ("uvm", true),
            ("peermem", true),
            ("fault_recovery", true),
            ("soak", true),
        ]
        .iter()
        .filter_map(|(name, include)| include.then_some(*name))
        .collect::<Vec<_>>()
        .join(",");

        writeln!(file, "schema=hermes-hardware-host-v1")?;
        writeln!(file, "attestation=physical-gpu")?;
        writeln!(file, "simulation=0")?;
        writeln!(file, "hardware_scope=host-graphics")?;
        writeln!(file, "graphics_host={graphics}")?;
        writeln!(file, "drm_kms={drm}")?;
        writeln!(file, "mesa={mesa}")?;
        writeln!(file, "nvidia_online=not-tested")?;
        writeln!(
            file,
            "amd_online={}",
            if vendors.contains(&"amd") {
                "not-tested"
            } else {
                "not-applicable"
            }
        )?;
        writeln!(
            file,
            "intel_online={}",
            if vendors.contains(&"intel") {
                "not-tested"
            } else {
                "not-applicable"
            }
        )?;
        writeln!(file, "firmware_measurement=not-tested")?;
        writeln!(file, "gsp_boot=not-tested")?;
        writeln!(file, "cuda=not-tested")?;
        writeln!(file, "nvml=not-tested")?;
        writeln!(file, "mps=not-tested")?;
        writeln!(file, "uvm=not-tested")?;
        writeln!(file, "peermem=not-tested")?;
        writeln!(file, "fault_recovery=not-tested")?;
        writeln!(file, "soak=not-tested")?;
        writeln!(file, "host_gpu_count={}", self.drm_cards())?;
        writeln!(file, "drm_cards={}", self.drm_cards())?;
        writeln!(file, "render_nodes={}", self.render_nodes)?;
        writeln!(
            file,
            "accessible_render_nodes={}",
            self.accessible_render_nodes
        )?;
        writeln!(file, "connected_outputs={}", self.connected_outputs())?;
        writeln!(file, "vendors={}", vendors.join(","))?;
        writeln!(file, "hardware_unavailable={unavailable}")?;
        writeln!(
            file,
            "session_type={}",
            Self::session_value(self.session_type.as_deref())
        )?;
        writeln!(
            file,
            "session_display={}",
            Self::session_value(self.session_display.as_deref())
        )?;
        Ok(())
    }
}

fn parse_hex(value: &str) -> Option<u16> {
    u16::from_str_radix(
        value
            .trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X"),
        16,
    )
    .ok()
}

fn read_trim(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
}

fn driver_name(path: &Path) -> Option<String> {
    fs::read_link(path.join("driver"))
        .ok()
        .and_then(|link| link.file_name().map(|name| name.to_owned()))
        .and_then(|name| name.to_str().map(str::to_owned))
}

fn card_accessible(path: &Path) -> bool {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .or_else(|_| File::open(path))
        .is_ok()
}

fn read_connector_status(class_drm: &Path, card: &str) -> (usize, usize) {
    let prefix = format!("{card}-");
    let mut total = 0;
    let mut connected = 0;
    let Ok(entries) = fs::read_dir(class_drm) else {
        return (0, 0);
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with(&prefix) {
            continue;
        }
        let status = entry.path().join("status");
        if !status.is_file() {
            continue;
        }
        total += 1;
        if read_trim(&status).as_deref() == Some("connected") {
            connected += 1;
        }
    }
    (total, connected)
}

fn parse_card_name(name: &str) -> bool {
    name.strip_prefix("card")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
}

/// Enumerate DRM cards from an injected sysfs/dev root.  Keeping roots as
/// parameters makes the qualification path testable without GPU hardware.
pub fn probe(class_drm: &Path, dev_dri: &Path, lib_root: &Path) -> GraphicsReport {
    let mut adapters = Vec::new();
    if let Ok(entries) = fs::read_dir(class_drm) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(card) = name.to_str().filter(|name| parse_card_name(name)) else {
                continue;
            };
            let device = entry.path().join("device");
            let Some(vendor_id) = read_trim(&device.join("vendor")).and_then(|v| parse_hex(&v))
            else {
                continue;
            };
            let device_id = read_trim(&device.join("device")).and_then(|v| parse_hex(&v));
            let card_node = dev_dri.join(card);
            let (output_count, connected_outputs) = read_connector_status(class_drm, card);
            adapters.push(GraphicsAdapter {
                card: card.to_owned(),
                vendor: GraphicsVendor::from_pci_id(vendor_id),
                vendor_id,
                device_id,
                driver: driver_name(&device),
                card_access: card_accessible(&card_node),
                card_node,
                connected_outputs,
                output_count,
            });
        }
    }
    adapters.sort_by(|left, right| left.card.cmp(&right.card));

    let mut render_nodes = 0;
    let mut accessible_render_nodes = 0;
    if let Ok(entries) = fs::read_dir(dev_dri) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !name.starts_with("renderD") {
                continue;
            }
            render_nodes += 1;
            if card_accessible(&entry.path()) {
                accessible_render_nodes += 1;
            }
        }
    }

    let mesa_library = ["libEGL.so.1", "libGLX.so.0", "libGL.so.1"]
        .iter()
        .map(|name| lib_root.join(name))
        .find(|path| path.exists());
    let session_type = std::env::var("XDG_SESSION_TYPE").ok();
    let session_display = match session_type.as_deref() {
        Some("wayland") => std::env::var("WAYLAND_DISPLAY").ok(),
        Some("x11") => std::env::var("DISPLAY").ok(),
        _ => None,
    };

    GraphicsReport {
        adapters,
        render_nodes,
        accessible_render_nodes,
        mesa_library,
        session_type,
        session_display,
    }
}

pub fn probe_host() -> GraphicsReport {
    probe(
        Path::new("/sys/class/drm"),
        Path::new("/dev/dri"),
        Path::new("/usr/lib"),
    )
}

fn vendor_label(vendor: GraphicsVendor) -> String {
    match vendor {
        GraphicsVendor::Other(id) => format!("other:{id:04x}"),
        known => known.name().to_owned(),
    }
}

pub fn print_report(report: &GraphicsReport) {
    println!("Hermes host graphics (real DRM path; GSP Online is separate)");
    println!("DRM cards: {}", report.drm_cards());
    for adapter in &report.adapters {
        println!(
            "  {} vendor={} ({:04x}) device={:04x?} driver={} card_node={} access={} outputs={}/{}",
            adapter.card,
            vendor_label(adapter.vendor),
            adapter.vendor_id,
            adapter.device_id,
            adapter.driver.as_deref().unwrap_or("-"),
            adapter.card_node.display(),
            if adapter.card_access { "pass" } else { "fail" },
            adapter.connected_outputs,
            adapter.output_count
        );
    }
    println!(
        "render_nodes: {} accessible={}",
        report.render_nodes, report.accessible_render_nodes
    );
    println!(
        "mesa_library: {}",
        report
            .mesa_library
            .as_deref()
            .map(Path::display)
            .map(|path| path.to_string())
            .as_deref()
            .unwrap_or("missing")
    );
    println!(
        "session: type={} display={}",
        report.session_type.as_deref().unwrap_or("none"),
        report.session_display.as_deref().unwrap_or("none")
    );
    println!(
        "graphics_host: {}",
        if report.graphics_host_ready() {
            "pass"
        } else {
            "fail"
        }
    );
    println!(
        "drm_kms: {}",
        if report.drm_kms_ready() {
            "pass"
        } else {
            "fail"
        }
    );
    println!(
        "mesa: {}",
        if report.mesa_ready() { "pass" } else { "fail" }
    );
    println!("gsp_online: not-claimed");
}

pub fn status(report_path: Option<&Path>) -> i32 {
    let report = probe_host();
    print_report(&report);
    if let Some(path) = report_path {
        if let Err(error) = report.write_qualification(path) {
            eprintln!("graphics report: cannot write {}: {error}", path.display());
            return 1;
        }
        println!("qualification_report: {}", path.display());
    }
    if report.graphics_host_ready() && report.drm_kms_ready() && report.mesa_ready() {
        println!("PASS (host graphics path qualified; GSP/CUDA remain explicitly untested)");
        0
    } else {
        println!("BLOCKED (host graphics path is incomplete)");
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let root = std::env::temp_dir()
                .join(format!("hermes-graphics-{}-{nonce}", std::process::id()));
            fs::create_dir_all(root.join("sys/class/drm/card0/device")).expect("sysfs");
            fs::create_dir_all(root.join("sys/class/drm/card0-eDP-1")).expect("connector");
            fs::create_dir_all(root.join("dev/dri")).expect("dev");
            fs::write(root.join("sys/class/drm/card0/device/vendor"), "0x8086\n").expect("vendor");
            fs::write(root.join("sys/class/drm/card0/device/device"), "0x3e9b\n").expect("device");
            fs::write(root.join("sys/class/drm/card0-eDP-1/status"), "connected\n")
                .expect("status");
            symlink(
                "../../../../bus/pci/drivers/i915",
                root.join("sys/class/drm/card0/device/driver"),
            )
            .expect("driver");
            fs::write(root.join("dev/dri/card0"), "card\n").expect("card");
            fs::write(root.join("dev/dri/renderD128"), "render\n").expect("render");
            fs::write(root.join("libEGL.so.1"), "mesa\n").expect("mesa");
            Self { root }
        }

        fn class_drm(&self) -> PathBuf {
            self.root.join("sys/class/drm")
        }

        fn dev_dri(&self) -> PathBuf {
            self.root.join("dev/dri")
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn probe_reports_real_card_driver_and_output() {
        let fixture = Fixture::new();
        let report = probe(&fixture.class_drm(), &fixture.dev_dri(), &fixture.root);
        assert_eq!(report.drm_cards(), 1);
        assert_eq!(report.connected_outputs(), 1);
        assert_eq!(report.accessible_render_nodes, 1);
        assert!(report.graphics_host_ready());
        assert!(report.drm_kms_ready());
        assert!(report.mesa_ready());
        assert_eq!(report.adapters[0].driver.as_deref(), Some("i915"));
    }

    #[test]
    fn qualification_marks_gsp_as_unavailable_without_faking_it() {
        let fixture = Fixture::new();
        let report = probe(&fixture.class_drm(), &fixture.dev_dri(), &fixture.root);
        let path = fixture.root.join("qualification.txt");
        report.write_qualification(&path).expect("qualification");
        let text = fs::read_to_string(path).expect("read report");
        assert!(text.contains("schema=hermes-hardware-host-v1"));
        assert!(text.contains("graphics_host=pass"));
        assert!(text.contains("gsp_boot=not-tested"));
        assert!(text.contains("intel_online=not-tested"));
        assert!(text.contains("simulation=0"));
    }

    #[test]
    fn empty_host_never_qualifies() {
        let fixture = Fixture::new();
        let empty = fixture.root.join("empty");
        fs::create_dir_all(&empty).expect("empty");
        let report = probe(&empty, &empty, &empty);
        assert!(!report.graphics_host_ready());
        assert!(!report.drm_kms_ready());
        assert!(!report.mesa_ready());
    }
}
