//! Text-based format detection
//!
//! Handles detection of text-based 3D and interchange formats including
//! DXF, OBJ, GLTF, STL, and EPS.

use crate::core::FileFormat;

/// Detect text-based 3D and interchange formats
///
/// Several formats use text-based representations with distinctive patterns:
/// - DXF: AutoCAD exchange format
/// - OBJ: Wavefront 3D object
/// - GLTF: GL Transmission Format (JSON)
/// - STL: Stereolithography (ASCII variant)
/// - EPS: Encapsulated PostScript
///
/// # Arguments
///
/// * `data` - Magic bytes buffer (at least 100 bytes recommended)
///
/// # Returns
///
/// `Some(FileFormat)` if text format detected, `None` otherwise
pub fn detect_text_formats(data: &[u8]) -> Option<FileFormat> {
    // EPS detection first (can be shorter than 100 bytes)
    // ASCII EPS: %!PS-Adobe
    if data.starts_with(b"%!PS-Adobe") {
        return Some(FileFormat::EPS);
    }

    // Binary EPS (DOS EPS): 0xC5D0D3C6 magic
    if data.len() >= 4
        && data[0] == 0xC5
        && data[1] == 0xD0
        && data[2] == 0xD3
        && data[3] == 0xC6
    {
        return Some(FileFormat::EPS);
    }

    if data.len() < 100 {
        return None;
    }

    let text = std::str::from_utf8(&data[0..100]).ok()?;

    // DXF: starts with "0\n" and contains "SECTION"
    if text.starts_with("0\n") && text.contains("SECTION") {
        return Some(FileFormat::DXF);
    }

    // OBJ: contains vertex definitions
    if text.contains("v ") || text.contains("vn ") || text.contains("vt ") {
        return Some(FileFormat::OBJ);
    }

    // GLTF: JSON with "asset" field
    if text.contains("\"asset\"") && text.contains("{") {
        return Some(FileFormat::GLTF);
    }

    // STL ASCII: starts with "solid"
    if text.starts_with("solid") {
        return Some(FileFormat::STL);
    }

    None
}
