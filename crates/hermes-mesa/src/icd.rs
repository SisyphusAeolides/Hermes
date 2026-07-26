//! Vulkan ICD discovery payload (nvidia_icd.json shape).

/// Classic NVIDIA ICD JSON filename operators install under `/etc/vulkan/icd.d/`.
pub const NVIDIA_ICD_JSON_NAME: &str = "nvidia_icd.json";
/// Hermes-owned ICD JSON name (coexists until full soname cutover).
pub const HERMES_ICD_JSON_NAME: &str = "hermes_icd.json";

/// Shared library basename exported by the hermes-mesa cdylib.
pub const ICD_LIBRARY_BASENAME: &str = "libhermes_mesa.so";
/// Drop-in soname operators may symlink for NVIDIA ICD loaders.
pub const NVIDIA_VULKAN_SONAME: &str = "libGLX_nvidia.so.0";

/// Produce a Vulkan-Loader ICD JSON document.
pub fn vulkan_icd_json(library_path: &str, api_version: &str) -> String {
    format!(
        r#"{{
    "file_format_version": "1.0.0",
    "ICD": {{
        "library_path": "{library_path}",
        "api_version": "{api_version}"
    }}
}}
"#
    )
}

pub fn default_icd_json() -> String {
    vulkan_icd_json(ICD_LIBRARY_BASENAME, "1.3.0")
}

/// Install paths documented for stage-dropin.sh (not written at runtime).
pub const ICD_SEARCH_PATHS: &[&str] = &[
    "/usr/share/vulkan/icd.d",
    "/etc/vulkan/icd.d",
    "/usr/local/share/vulkan/icd.d",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_contains_library_and_version() {
        let j = default_icd_json();
        assert!(j.contains(ICD_LIBRARY_BASENAME));
        assert!(j.contains("1.3.0"));
        assert!(j.contains("file_format_version"));
    }
}
