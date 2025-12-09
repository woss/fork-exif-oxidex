//! OxiDex tag extractor - Extract tags by running OxiDex on test fixtures
//!
//! This module extracts metadata tags from test fixture files using the OxiDex
//! library. It handles conversion of internal TagValue types to string representations
//! that match ExifTool's output format.
//!
//! # ExifTool Compatibility
//!
//! Before comparison, all metadata is passed through `format_for_exiftool()` to ensure
//! values are formatted consistently with ExifTool's output. This handles GPS references,
//! binary decoders, enum values, unit suffixes, and numeric precision.

use super::ExtractionResult;
use crate::models::TagInfo;
use oxidex::core::TagValue;
use oxidex::core::exiftool_compat::format_for_exiftool;
use oxidex::core::tag_normalization::normalize_tag_family;
use oxidex::core::value_formatter::{
    format_date_exif_style, format_rational_as_decimal, format_with_unit, is_decimal_rational_tag,
    needs_unit_suffix,
};
use oxidex::parsers::tiff::tiff_enums::tiff_enum_to_string;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Extract tags from OxiDex by processing test fixtures
pub struct OxiDexExtractor {
    fixture_path: PathBuf,
    cache: HashMap<String, ExtractionResult>,
}

impl OxiDexExtractor {
    /// Create a new OxiDex extractor
    pub fn new(fixture_path: PathBuf) -> Self {
        Self {
            fixture_path,
            cache: HashMap::new(),
        }
    }

    /// Extract tags from all fixtures of a specific format
    pub async fn extract_format_tags(
        &mut self,
        format: &str,
    ) -> Result<ExtractionResult, Box<dyn std::error::Error>> {
        // Check cache first
        if let Some(cached) = self.cache.get(format) {
            return Ok(cached.clone());
        }

        // Find files by extension recursively throughout the samples directory
        let files: Vec<PathBuf> = self.find_files_by_extension(format)?;

        let files_processed = files.len();

        if files.is_empty() {
            return Ok(ExtractionResult {
                tags: Vec::new(),
                files_processed: 0,
            });
        }

        // Extract tags from each file
        let mut all_tags: HashMap<String, (TagInfo, usize)> = HashMap::new();

        for file_path in &files {
            match self.extract_tags_from_file(file_path) {
                Ok(file_tags) => {
                    for tag_info in file_tags {
                        all_tags
                            .entry(format!("{}:{}", tag_info.family, tag_info.name))
                            .and_modify(|(_info, count)| *count += 1)
                            .or_insert((tag_info.clone(), 1));
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to extract tags from {}: {}",
                        file_path.display(),
                        e
                    );
                }
            }
        }

        let mut tags: Vec<TagInfo> = all_tags
            .into_values()
            .map(|(tag_info, _count)| tag_info)
            .collect();

        tags.sort_by_key(|a| a.key());

        let result = ExtractionResult {
            tags: tags.clone(),
            files_processed,
        };

        self.cache.insert(format.to_string(), result.clone());

        Ok(result)
    }

    /// Extract tags from a single file using OxiDex
    ///
    /// This method reads raw metadata from the file and applies ExifTool-compatible
    /// formatting before flattening into TagInfo structures. The formatting ensures
    /// that GPS references, binary values, enums, and numeric precision match
    /// ExifTool's output format for accurate comparison.
    fn extract_tags_from_file(
        &self,
        file_path: &Path,
    ) -> Result<Vec<TagInfo>, Box<dyn std::error::Error>> {
        // Step 1: Read raw metadata from the file
        let raw_metadata = oxidex::core::operations::read_metadata(file_path)?;

        // Step 2: Apply ExifTool-compatible formatting to all values
        // This ensures GPS refs, binary decoders, enums, units, and precision
        // match ExifTool's output before we compare the results
        let formatted_metadata = format_for_exiftool(&raw_metadata);

        // Step 3: Determine format from file extension
        let format = file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_uppercase());

        // Step 4: Flatten the formatted metadata into TagInfo structures
        let tags = self.flatten_metadata(&formatted_metadata, format.as_deref());
        Ok(tags)
    }

    /// Format a tag value to match ExifTool's output format
    fn format_value(&self, key: &str, name: &str, value: &TagValue) -> String {
        match value {
            TagValue::String(s) => {
                // Try to format dates in EXIF style
                if (key.contains("Date") || key.contains("Time"))
                    && (s.contains('T') || s.contains('-'))
                {
                    return format_date_exif_style(s, false);
                }
                s.clone()
            }
            TagValue::Integer(i) => {
                // Try enum decoding for known tags
                if let Some(decoded) = self.decode_enum(name, *i as u32) {
                    return decoded;
                }
                i.to_string()
            }
            TagValue::Float(f) => {
                // Format floats with reasonable precision
                let formatted = format!("{:.5}", f);
                formatted
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            }
            TagValue::Rational {
                numerator,
                denominator,
            } => {
                if *denominator == 0 {
                    return "inf".to_string();
                }

                // Special handling for FocalLength - round to 1 decimal
                if name == "FocalLength" {
                    let value = *numerator as f64 / *denominator as f64;
                    return format!("{:.1} mm", value);
                }

                // Handle APEX (Additive System of Photographic Exposure) tags
                // These require conversion from APEX units to human-readable values
                // ApertureValue/MaxApertureValue: F-number = 2^(APEX/2)
                if name == "ApertureValue" || name == "MaxApertureValue" {
                    let apex = *numerator as f64 / *denominator as f64;
                    let f_number = (2.0_f64).powf(apex / 2.0);
                    return format!("{:.1}", f_number);
                }

                // ShutterSpeedValue: Exposure time = 2^(-APEX)
                // Format as fraction (e.g., "1/501") for times < 1 second
                if name == "ShutterSpeedValue" {
                    let apex = *numerator as f64 / *denominator as f64;
                    let exposure_time = (2.0_f64).powf(-apex);
                    // Format as fraction for sub-second exposures
                    if exposure_time < 1.0 {
                        let denominator = (1.0 / exposure_time).round() as i64;
                        return format!("1/{}", denominator);
                    } else {
                        return format!("{:.1}", exposure_time);
                    }
                }

                // For tags that should be decimal values
                if is_decimal_rational_tag(key) || is_decimal_rational_tag(name) {
                    let decimal =
                        format_rational_as_decimal(*numerator as i64, *denominator as i64);
                    // Add unit if needed
                    if needs_unit_suffix(key) || needs_unit_suffix(name) {
                        return format_with_unit(name, &decimal);
                    }
                    return decimal;
                }

                // Default: compute decimal value
                // Use 9 decimal places for ExifTool compatibility
                let value = *numerator as f64 / *denominator as f64;
                let formatted = format!("{:.9}", value);
                let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');

                // Add unit suffix if needed
                if needs_unit_suffix(key) || needs_unit_suffix(name) {
                    format_with_unit(name, trimmed)
                } else {
                    trimmed.to_string()
                }
            }
            TagValue::Binary(bytes) => {
                // FileSource - single byte value indicating the source device
                // Values: 1=Film Scanner, 2=Reflection Print Scanner, 3=Digital Camera
                if name == "FileSource" && bytes.len() == 1 {
                    return match bytes[0] {
                        1 => "Film Scanner".to_string(),
                        2 => "Reflection Print Scanner".to_string(),
                        3 => "Digital Camera".to_string(),
                        _ => format!("Unknown ({})", bytes[0]),
                    };
                }

                // FlashpixVersion - 4 ASCII bytes representing version (e.g., "0100")
                if name == "FlashpixVersion"
                    && bytes.len() == 4
                    && let Ok(s) = std::str::from_utf8(bytes)
                {
                    return s.to_string();
                }

                // ExifVersion - 4 ASCII bytes representing version (e.g., "0232")
                if name == "ExifVersion"
                    && bytes.len() == 4
                    && let Ok(s) = std::str::from_utf8(bytes)
                {
                    return s.to_string();
                }

                // UserComment - starts with 8-byte encoding identifier followed by data
                // Encoding prefixes: "ASCII\0\0\0", "UNICODE\0", "JIS\0\0\0\0\0", etc.
                if name == "UserComment" && bytes.len() > 8 {
                    let encoding = &bytes[0..8];
                    let data = &bytes[8..];

                    // Check for ASCII encoding
                    if encoding.starts_with(b"ASCII\0\0\0") {
                        return String::from_utf8_lossy(data)
                            .trim_end_matches('\0')
                            .trim()
                            .to_string();
                    }

                    // Check for Unicode encoding (UTF-16)
                    if encoding.starts_with(b"UNICODE\0") {
                        // Decode as UTF-16 little-endian
                        let u16_data: Vec<u16> = data
                            .chunks_exact(2)
                            .map(|c| u16::from_le_bytes([c[0], c[1]]))
                            .collect();
                        return String::from_utf16_lossy(&u16_data)
                            .trim_end_matches('\0')
                            .trim()
                            .to_string();
                    }

                    // Empty or null-padded data - return empty string
                    if data.iter().all(|&b| b == 0) {
                        return String::new();
                    }
                }

                // Default fallback for unrecognized binary data
                // Format to match ExifTool: "(Binary data N bytes, use -b option to extract)"
                format!(
                    "(Binary data {} bytes, use -b option to extract)",
                    bytes.len()
                )
            }
            TagValue::DateTime(dt) => {
                // Format in EXIF style: YYYY:MM:DD HH:MM:SS
                dt.format("%Y:%m:%d %H:%M:%S").to_string()
            }
            TagValue::Struct(_) => "[Structured data]".to_string(),
            TagValue::Array(arr) => {
                // Format array elements
                let parts: Vec<String> = arr
                    .iter()
                    .map(|v| self.format_value(key, name, v))
                    .collect();
                parts.join(" ")
            }
        }
    }

    /// Decode enum values for known EXIF tags
    fn decode_enum(&self, tag_name: &str, value: u32) -> Option<String> {
        // Map tag names to TIFF tag IDs for enum lookup
        let tag_id = match tag_name {
            "ColorSpace" => 0xA001,
            "MeteringMode" => 0x9207,
            "ExposureMode" => 0xA402,
            "WhiteBalance" => 0xA403,
            "SceneCaptureType" => 0xA406,
            "Contrast" => 0xA408,
            "Saturation" => 0xA409,
            "Sharpness" => 0xA40A,
            "SubjectDistanceRange" => 0xA40C,
            "SensingMethod" => 0xA217,
            "CustomRendered" => 0xA401,
            "FocalPlaneResolutionUnit" | "ResolutionUnit" => 0x0128,
            "Orientation" => 0x0112,
            "YCbCrPositioning" => 0x0213,
            "Compression" => 0x0103,
            "ExposureProgram" => 0x8822,
            "LightSource" => 0x9208,
            "Flash" => 0x9209,
            "GainControl" => 0xA407,
            "ExtraSamples" => 0x0152,
            "FillOrder" => 0x010A,
            "PlanarConfiguration" => 0x011C,
            "Predictor" => 0x013D,
            "SubfileType" => 0x00FE,
            "SceneType" => 0xA301,
            "SensitivityType" => 0x8830,
            "CompositeImage" => 0xA460,
            "MakerNoteSafety" => 0xC635,
            "PhotometricInterpretation" => 0x0106,
            _ => return None,
        };

        // Special handling for Flash tag (bitmask)
        if tag_name == "Flash" {
            return Some(oxidex::core::exif_enums::decode_flash(value));
        }

        // Use TIFF enum decoder
        tiff_enum_to_string(tag_id, value as i64)
    }

    /// Add computed Composite tags
    fn add_composite_tags(&self, tag_map: &mut HashMap<String, String>) {
        // ImageSize
        if let (Some(w), Some(h)) = (
            tag_map
                .get("EXIF:ImageWidth")
                .or(tag_map.get("File:ImageWidth")),
            tag_map
                .get("EXIF:ImageHeight")
                .or(tag_map.get("File:ImageHeight")),
        ) {
            tag_map.insert("Composite:ImageSize".to_string(), format!("{}x{}", w, h));
        }

        // Megapixels
        if let (Some(w), Some(h)) = (
            tag_map
                .get("EXIF:ImageWidth")
                .or(tag_map.get("File:ImageWidth")),
            tag_map
                .get("EXIF:ImageHeight")
                .or(tag_map.get("File:ImageHeight")),
        ) && let (Ok(width), Ok(height)) = (w.parse::<f64>(), h.parse::<f64>())
        {
            let mp = (width * height) / 1_000_000.0;
            tag_map.insert("Composite:Megapixels".to_string(), format!("{:.3}", mp));
        }

        // Aperture - copy from FNumber
        if let Some(f) = tag_map.get("EXIF:FNumber") {
            tag_map.insert("Composite:Aperture".to_string(), f.clone());
        }

        // ShutterSpeed - copy from ExposureTime
        if let Some(e) = tag_map.get("EXIF:ExposureTime") {
            tag_map.insert("Composite:ShutterSpeed".to_string(), e.clone());
        }

        // ISO
        if let Some(iso) = tag_map.get("EXIF:ISO") {
            tag_map.insert("Composite:ISO".to_string(), iso.clone());
        }
    }

    /// Apply comparison-specific normalization for ExifTool compatibility reports
    /// This normalizes families for the comparison tool documentation output
    /// Check if a tag family should be skipped (pseudo-tags, not actual metadata)
    fn should_skip_family(family: &str) -> bool {
        matches!(family, "File" | "System" | "UNKNOWN")
    }

    fn normalize_for_comparison(tag_key: &str, format: Option<&str>) -> String {
        // Handle PNG special cases first
        // PNG:tEXt:Author → PNG:Author
        // PNG:tEXt:date:create → PNG:Datecreate
        // PNG-pHYs:PixelUnits → PNG:PixelUnits
        if let Some(rest) = tag_key.strip_prefix("PNG:tEXt:") {
            // Handle date:create → Datecreate format
            // ExifTool uses lowercase after "Date" (Datecreate, not DateCreate)
            if let Some(date_part) = rest.strip_prefix("date:") {
                // date:create → Datecreate, date:modify → Datemodify, date:timestamp → Datetimestamp
                return format!("PNG:Date{}", date_part);
            }
            return format!("PNG:{}", rest);
        }
        if let Some(rest) = tag_key.strip_prefix("PNG-pHYs:") {
            return format!("PNG:{}", rest);
        }
        if let Some(rest) = tag_key.strip_prefix("PNG:iTXt:") {
            return format!("PNG:{}", rest);
        }

        if let Some((family, name)) = tag_key.split_once(':') {
            let normalized_family = match family {
                // ExifIFD, IFD0, and GPS tags are output as EXIF in comparison reports
                // Perl ExifTool outputs GPS tags as EXIF:GPSxxx
                "ExifIFD" | "IFD0" | "GPS" => "EXIF",
                // Manufacturer maker notes are output as MakerNotes in comparison reports
                "Canon" | "Nikon" | "Sony" | "Fujifilm" | "Panasonic" | "Olympus" | "Pentax"
                | "Samsung" => "MakerNotes",
                // MP4/QuickTime: ItemList and UserData → QuickTime for comparison
                "ItemList" | "UserData" => "QuickTime",
                // Keep other families unchanged
                _ => family,
            };
            format!("{}:{}", normalized_family, name)
        } else if let Some(fmt) = format {
            // No family prefix - use format as family (e.g., GIF:GIFVersion)
            format!("{}:{}", fmt.to_uppercase(), tag_key)
        } else {
            tag_key.to_string()
        }
    }

    /// Flatten MetadataMap into TagInfo vector
    fn flatten_metadata(
        &self,
        metadata: &oxidex::core::MetadataMap,
        format: Option<&str>,
    ) -> Vec<TagInfo> {
        let mut tag_map: HashMap<String, String> = HashMap::new();

        for (key, value) in metadata.iter() {
            // Check if original family should be skipped (pseudo-tags)
            if let Some((original_family, _)) = key.split_once(':')
                && Self::should_skip_family(original_family)
            {
                continue;
            }

            // Normalize the tag family (core library normalization + comparison-specific)
            let normalized_key = Self::normalize_for_comparison(&normalize_tag_family(key), format);

            let (family, name) = if let Some(colon_pos) = normalized_key.find(':') {
                let (fam, nam) = normalized_key.split_at(colon_pos);
                (fam.to_string(), nam[1..].to_string())
            } else {
                ("UNKNOWN".to_string(), normalized_key.clone())
            };

            // Skip if normalized family should be skipped
            if Self::should_skip_family(&family) {
                continue;
            }
            let _family = family; // Keep for later use

            // Special handling for Canon FileNumber (check original key since family is normalized)
            if name == "FileNumber" && key.starts_with("Canon:") {
                let formatted = match value {
                    TagValue::Integer(val) => {
                        let directory = (*val >> 16) & 0xFFFF;
                        let file = *val & 0xFFFF;
                        format!("{}-{}", directory, file)
                    }
                    TagValue::String(s) => {
                        if let Ok(val) = s.parse::<i64>() {
                            let directory = (val >> 16) & 0xFFFF;
                            let file = val & 0xFFFF;
                            format!("{}-{}", directory, file)
                        } else {
                            s.clone()
                        }
                    }
                    _ => continue,
                };
                tag_map.insert(normalized_key, formatted);
                continue;
            }

            // Format the value
            let value_str = self.format_value(&normalized_key, &name, value);
            tag_map.insert(normalized_key, value_str);
        }

        // Add composite tags
        self.add_composite_tags(&mut tag_map);

        // Convert to Vec<TagInfo>
        let mut tags: Vec<TagInfo> = tag_map
            .into_iter()
            .map(|(key, value)| {
                if let Some(colon_pos) = key.find(':') {
                    let (family, name) = key.split_at(colon_pos);
                    TagInfo::new(name[1..].to_string(), family.to_string(), value)
                } else {
                    TagInfo::new(key.clone(), "UNKNOWN".to_string(), value)
                }
            })
            .collect();

        tags.sort_by_key(|a| a.key());
        tags
    }

    /// Find files by extension recursively throughout the samples directory
    fn find_files_by_extension(
        &self,
        format: &str,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let extensions = Self::format_to_extensions(format);
        if extensions.is_empty() {
            return Ok(Vec::new());
        }

        let files: Vec<PathBuf> = WalkDir::new(&self.fixture_path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| {
                if !e.path().is_file() {
                    return false;
                }
                // Skip hidden files and directories
                if e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("."))
                {
                    return false;
                }
                if let Some(ext) = e.path().extension().and_then(|e| e.to_str()) {
                    extensions.contains(&ext.to_lowercase().as_str())
                } else {
                    false
                }
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        Ok(files)
    }

    /// Map format name to file extensions
    fn format_to_extensions(format: &str) -> Vec<&'static str> {
        match format.to_uppercase().as_str() {
            "JPEG" => vec!["jpg", "jpeg"],
            "PNG" => vec!["png"],
            "TIFF" => vec!["tif", "tiff"],
            "GIF" => vec!["gif"],
            "WEBP" => vec!["webp"],
            "HEIC" => vec!["heic", "heif"],
            "MP4" => vec!["mp4", "m4v", "mov"],
            "AVI" => vec!["avi"],
            "MKV" => vec!["mkv"],
            "MP3" => vec!["mp3"],
            "WAV" => vec!["wav"],
            "PDF" => vec!["pdf"],
            "PSD" => vec!["psd"],
            "CR2" => vec!["cr2", "cr3"],
            "NEF" => vec!["nef"],
            "ARW" => vec!["arw"],
            "DNG" => vec!["dng"],
            "RAF" => vec!["raf"],
            "ORF" => vec!["orf"],
            "RW2" => vec!["rw2"],
            "XMP" => vec!["xmp"],
            "FLAC" => vec!["flac"],
            "OGG" => vec!["ogg", "oga", "ogv"],
            "BMP" => vec!["bmp"],
            "ICO" => vec!["ico"],
            "SVG" => vec!["svg"],
            "EPS" => vec!["eps", "ps"],
            "EXR" => vec!["exr"],
            "JXL" => vec!["jxl"],
            "AVIF" => vec!["avif"],
            "3GP" => vec!["3gp", "3g2"],
            "FLV" => vec!["flv"],
            "WMV" => vec!["wmv", "asf"],
            "MXF" => vec!["mxf"],
            "WEBM" => vec!["webm"],
            "ICC" => vec!["icc", "icm"],
            "PEF" => vec!["pef"],
            "SRW" => vec!["srw"],
            "X3F" => vec!["x3f"],
            "DCR" => vec!["dcr"],
            "RWL" => vec!["rwl"],
            "3FR" => vec!["3fr"],
            "FFF" => vec!["fff"],
            "MEF" => vec!["mef"],
            "MOS" => vec!["mos"],
            "MRW" => vec!["mrw"],
            "NRW" => vec!["nrw"],
            "SR2" => vec!["sr2", "srf"],
            "KDC" => vec!["kdc"],
            "ERF" => vec!["erf"],
            "RAW" => vec![
                "raw", "3fr", "ari", "bay", "crw", "dcr", "dcs", "dng", "erf", "fff", "k25", "kdc",
                "mef", "mos", "mrw", "nrw", "pef", "ptx", "r3d", "raf", "rw2", "rwl", "sr2", "srf",
                "srw", "x3f",
            ],
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oxidex_extractor_creation() {
        let extractor = OxiDexExtractor::new(PathBuf::from("tests/fixtures/jpeg"));
        assert_eq!(extractor.fixture_path, PathBuf::from("tests/fixtures/jpeg"));
    }

    #[test]
    fn test_flatten_metadata_empty() {
        let extractor = OxiDexExtractor::new(PathBuf::from("tests/fixtures"));
        let metadata = oxidex::core::MetadataMap::new();
        let tags = extractor.flatten_metadata(&metadata, None);
        assert_eq!(tags.len(), 0);
    }

    #[test]
    fn test_canon_file_number_formatting() {
        let extractor = OxiDexExtractor::new(PathBuf::from("tests/fixtures"));
        let mut metadata = oxidex::core::MetadataMap::new();
        metadata.insert("Canon:FileNumber".to_string(), TagValue::Integer(7669483));
        let tags = extractor.flatten_metadata(&metadata, None);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].value, "117-1771");
    }
}
