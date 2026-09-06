//! Synthetic EDID blob builder for virtual connectors.
//!
//! Not a claim of real panel readout — produces a valid-checksum 128-byte
//! base EDID so modeset clients can parse preferred timing.

use crate::mode::DisplayMode;
use alloc::vec::Vec;

/// Build a minimal base EDID (128 bytes) for the given mode.
pub fn build_base_edid(mode: DisplayMode, name: &str) -> Vec<u8> {
    let mut e = alloc::vec![0u8; 128];
    // Header
    e[0] = 0x00;
    e[1] = 0xff;
    e[2] = 0xff;
    e[3] = 0xff;
    e[4] = 0xff;
    e[5] = 0xff;
    e[6] = 0xff;
    e[7] = 0x00;
    // Manufacturer "HRS" (Hermes) — compressed ASCII
    // (char - '@') packed: H=8, R=18, S=19
    let m = ((8u16) << 10) | ((18u16) << 5) | 19u16;
    e[8] = (m >> 8) as u8;
    e[9] = (m & 0xff) as u8;
    // Product code
    e[10] = 0x01;
    e[11] = 0x00;
    // Serial
    e[12] = 0x01;
    e[13] = 0x00;
    e[14] = 0x00;
    e[15] = 0x00;
    // Week/year (2024 = 2024-1990 = 34)
    e[16] = 1;
    e[17] = 34;
    // EDID version 1.4
    e[18] = 1;
    e[19] = 4;
    // Basic display parameters (digital)
    e[20] = 0x80;
    // Screen size cm (approx 60x34 for 27")
    e[21] = 60;
    e[22] = 34;
    // Gamma 2.2
    e[23] = 120;
    // Features
    e[24] = 0x0a;

    // Detailed timing descriptor 0 at offset 54
    write_dtd(&mut e[54..72], mode);
    // Monitor name descriptor at 72
    write_name_descriptor(&mut e[72..90], name);
    // Range limits dummy
    e[90] = 0x00;
    e[91] = 0x00;
    e[92] = 0x00;
    e[93] = 0xfd;
    e[94] = 0x00;
    e[95] = 30; // min vrate
    e[96] = 75; // max vrate
    e[97] = 30; // min hrate
    e[98] = 100; // max hrate
    e[99] = 16; // max pixel clock / 10MHz
                // Extension flag
    e[126] = 0;
    // Checksum
    let sum: u32 = e[..127].iter().map(|&b| b as u32).sum();
    e[127] = (256 - (sum % 256) as u32) as u8;
    e
}

fn write_dtd(d: &mut [u8], mode: DisplayMode) {
    // pixel clock / 10 kHz
    let clk = (mode.clock_khz / 10) as u16;
    d[0] = (clk & 0xff) as u8;
    d[1] = (clk >> 8) as u8;
    let ha = mode.hdisplay;
    let hb = mode.htotal.saturating_sub(mode.hdisplay);
    let va = mode.vdisplay;
    let vb = mode.vtotal.saturating_sub(mode.vdisplay);
    d[2] = (ha & 0xff) as u8;
    d[3] = (hb & 0xff) as u8;
    d[4] = (((ha >> 8) & 0xf) << 4 | ((hb >> 8) & 0xf)) as u8;
    d[5] = (va & 0xff) as u8;
    d[6] = (vb & 0xff) as u8;
    d[7] = (((va >> 8) & 0xf) << 4 | ((vb >> 8) & 0xf)) as u8;
    let hso = mode.hsync_start.saturating_sub(mode.hdisplay);
    let hspw = mode.hsync_end.saturating_sub(mode.hsync_start);
    let vso = mode.vsync_start.saturating_sub(mode.vdisplay);
    let vspw = mode.vsync_end.saturating_sub(mode.vsync_start);
    d[8] = (hso & 0xff) as u8;
    d[9] = (hspw & 0xff) as u8;
    d[10] = (((vso & 0xf) << 4) | (vspw & 0xf)) as u8;
    d[11] = ((((hso >> 8) & 0x3) << 6)
        | (((hspw >> 8) & 0x3) << 4)
        | (((vso >> 4) & 0x3) << 2)
        | ((vspw >> 4) & 0x3)) as u8;
    // mm size (600×340 mm → low 8 bits of each dimension)
    d[12] = (600u16 & 0xff) as u8;
    d[13] = (340u16 & 0xff) as u8;
    d[14] = 0;
    d[15] = 0;
    d[16] = 0;
    // Progressive, digital separate, +hsync +vsync
    d[17] = 0x1e;
}

fn write_name_descriptor(d: &mut [u8], name: &str) {
    d.fill(0);
    d[3] = 0xfc; // monitor name tag
    let bytes = name.as_bytes();
    let n = core::cmp::min(bytes.len(), 13);
    d[5..5 + n].copy_from_slice(&bytes[..n]);
    if n < 13 {
        d[5 + n] = 0x0a; // newline terminator
    }
}

/// Verify EDID checksum (sum of all 128 bytes ≡ 0 mod 256).
pub fn edid_checksum_ok(edid: &[u8]) -> bool {
    if edid.len() < 128 {
        return false;
    }
    edid[..128].iter().fold(0u8, |a, b| a.wrapping_add(*b)) == 0
}

/// Parse preferred hdisplay/vdisplay from first DTD if present.
pub fn edid_preferred_size(edid: &[u8]) -> Option<(u16, u16)> {
    if edid.len() < 72 || edid[0] != 0x00 || edid[1] != 0xff {
        return None;
    }
    let d = &edid[54..72];
    if d[0] == 0 && d[1] == 0 {
        return None;
    }
    let ha = d[2] as u16 | (((d[4] >> 4) as u16) << 8);
    let va = d[5] as u16 | (((d[7] >> 4) as u16) << 8);
    if ha == 0 || va == 0 {
        None
    } else {
        Some((ha, va))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fhd_edid_checksum_and_size() {
        let e = build_base_edid(DisplayMode::fhd_60(), "Hermes FHD");
        assert_eq!(e.len(), 128);
        assert!(edid_checksum_ok(&e));
        assert_eq!(edid_preferred_size(&e), Some((1920, 1080)));
    }
}
