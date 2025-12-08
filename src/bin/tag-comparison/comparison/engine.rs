//! Tag comparison engine - Match and compare tags between OxiDex and ExifTool

use crate::models::{FormatComparison, TagInfo, ValueDifference};
use std::collections::{HashMap, HashSet};

/// Comparison engine for analyzing tag differences
pub struct ComparisonEngine;

/// Check if a value looks like an enum (alphabetic with optional numbers/separators)
/// Examples: "Mode3", "COLOR", "Normal", "Non-Frame/Portrait", "AF-S"
fn is_enum_like_value(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }

    // Must start with a letter
    let first_char = value.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() {
        return false;
    }

    // Check if it's primarily alphabetic with allowed characters
    // Allowed: letters, digits, spaces, hyphens, slashes, underscores, parentheses
    let alpha_count = value.chars().filter(|c| c.is_ascii_alphabetic()).count();
    let total_valid = value.chars().filter(|c| {
        c.is_ascii_alphanumeric() || *c == ' ' || *c == '-' || *c == '/' || *c == '_' || *c == '(' || *c == ')'
    }).count();

    // Must be all valid characters and at least 50% alphabetic
    total_valid == value.len() && alpha_count * 2 >= value.len()
}

/// Normalize a value for comparison to handle formatting differences
fn normalize_value_for_comparison(tag_key: &str, value: &str) -> String {
    let normalized = value.trim();

    // Handle JSON array formatting: ["a","b","c"] vs "a b c"
    // ExifTool sometimes outputs arrays as JSON arrays
    if normalized.starts_with('[') && normalized.ends_with(']') {
        // Try to parse as JSON-like array and convert to space-separated
        let inner = &normalized[1..normalized.len() - 1];
        // Split by comma, remove quotes, join with space
        let items: Vec<&str> = inner
            .split(',')
            .map(|s| s.trim().trim_matches('"'))
            .collect();
        return items.join(" ");
    }

    // Handle date format normalization: ISO 8601 vs EXIF-style
    // "2025-10-30T11:57:59+00:00" vs "2025:10:30 11:57:59"
    if tag_key.contains("Date") || tag_key.contains("Time") {
        // Try to normalize both formats to a common format
        // ISO 8601: YYYY-MM-DDTHH:MM:SS+TZ
        // EXIF: YYYY:MM:DD HH:MM:SS
        let date_normalized = normalized
            .replace('T', " ") // Replace T separator with space
            .replace('-', ":") // Replace dashes with colons in date
            .split('+')
            .next()
            .unwrap_or(normalized) // Remove timezone
            .split('-')
            .next()
            .unwrap_or(normalized) // Remove negative timezone
            .trim()
            .to_string();
        if date_normalized.len() >= 10 {
            return date_normalized;
        }
    }

    // Case normalization for certain tags
    // MakerNotes tags often have enum values with inconsistent case
    // e.g., "Mode3" vs "MODE3", "Color" vs "COLOR", "Normal" vs "NORMAL"
    let normalized = match tag_key {
        _ if tag_key.starts_with("MakerNotes:") => {
            // For MakerNotes, normalize case for enum-like values
            // Check if it's primarily alphabetic/simple enum
            let trimmed = normalized.trim();
            if is_enum_like_value(trimmed) {
                trimmed.to_lowercase()
            } else {
                normalized.to_string()
            }
        }
        _ if tag_key.contains("MeteringMode") => normalized.to_lowercase(),
        _ if tag_key.contains("FlashBits") || tag_key.contains("FlashActivity") => {
            normalized.to_lowercase()
        }
        _ => normalized.to_string(),
    };

    // Handle EV/exposure compensation formatting: "0" vs "+0.0"
    if tag_key.contains("Compensation")
        || tag_key.contains("EV")
        || tag_key.contains("Bracketing")
        || tag_key.contains("BracketValue")
        || tag_key.contains("AEB")
    {
        // Try to parse as a number and normalize
        let num_str = normalized.trim_start_matches('+');
        if let Ok(val) = num_str.parse::<f64>() {
            if val.abs() < 0.001 {
                return "0".to_string();
            }
            // Return numeric value without sign for small values
            return format!("{:.1}", val);
        }
        // Handle "Off" which is equivalent to 0
        if normalized.eq_ignore_ascii_case("off") {
            return "0".to_string();
        }
    }

    // Handle aperture formatting: "5" vs "f/5.0"
    if tag_key.contains("Aperture") || tag_key.contains("FNumber") {
        if let Some(stripped) = normalized.strip_prefix("f/") {
            // Try to parse and compare numerically
            if let Ok(val) = stripped.parse::<f64>() {
                // Return rounded to 1 decimal
                return format!("{:.1}", val);
            }
        }
        // If no prefix, also try to normalize
        if let Ok(val) = normalized.parse::<f64>() {
            return format!("{:.1}", val);
        }
    }

    // Handle focal length formatting: "21.3125 mm" vs "21.3 mm" or "682 mm"
    if tag_key.contains("FocalLength") && !tag_key.contains("Units") {
        if let Some(mm_pos) = normalized.find(" mm") {
            let num_str = &normalized[..mm_pos];
            if let Ok(val) = num_str.parse::<f64>() {
                // Round to 1 decimal place for comparison
                return format!("{:.1} mm", val);
            }
        }
        // Handle raw numbers
        if let Ok(val) = normalized.parse::<f64>() {
            return format!("{:.1}", val);
        }
    }

    // Handle FocalPlaneResolution precision: "19041.32231" vs "19041.32231405"
    if tag_key.contains("FocalPlane") && tag_key.contains("Resolution") {
        if let Ok(val) = normalized.parse::<f64>() {
            // Round to 5 decimal places
            return format!("{:.5}", val);
        }
    }

    // Handle ICC_Profile percentage values: "0.999%" vs "0.99945%"
    if tag_key.starts_with("ICC_Profile:") && normalized.ends_with('%') {
        let num_str = normalized.trim_end_matches('%');
        if let Ok(val) = num_str.parse::<f64>() {
            return format!("{:.3}%", val);
        }
    }

    // Handle "(none)" vs "None" or "none"
    if normalized.eq_ignore_ascii_case("(none)") || normalized.eq_ignore_ascii_case("none") {
        return "none".to_string();
    }

    // Handle version number formatting: "2.11" vs "0211"
    if tag_key.contains("Version") {
        // Try both directions: "0211" -> "2.11" or normalize to raw
        if normalized.len() == 4 && normalized.chars().all(|c| c.is_ascii_digit()) {
            // Could be raw version like "0211" -> "2.11"
            let major = &normalized[0..2];
            let minor = &normalized[2..4];
            let major_num: u32 = major.parse().unwrap_or(0);
            let minor_num: u32 = minor.parse().unwrap_or(0);
            return format!("{}.{:02}", major_num, minor_num);
        }
        // Handle dotted format -> normalize
        if let Some((major, minor)) = normalized.split_once('.') {
            if let (Ok(maj), Ok(min)) = (major.parse::<u32>(), minor.parse::<u32>()) {
                return format!("{}.{:02}", maj, min);
            }
        }
    }

    // Handle "n/a" vs "0" or specific values
    if normalized.eq_ignore_ascii_case("n/a") {
        return "n/a".to_string();
    }

    // Handle "Off" vs "0" for boolean-like MakerNotes tags
    if tag_key.starts_with("MakerNotes:") {
        if normalized == "0" || normalized.eq_ignore_ascii_case("off") {
            return "off".to_string();
        }
    }

    // Handle temperature formatting: "10000" vs "10000 K"
    if tag_key.contains("Temperature") {
        let num_str = normalized.trim_end_matches(" K").trim_end_matches("K");
        if let Ok(val) = num_str.parse::<i64>() {
            return val.to_string();
        }
    }

    // Handle angle formatting: "360" vs "360 deg"
    if tag_key.contains("Angle") {
        let num_str = normalized.trim_end_matches(" deg").trim_end_matches("deg");
        if let Ok(val) = num_str.parse::<i64>() {
            return val.to_string();
        }
    }

    // Handle parenthetical case normalization: "0 (Normal)" vs "0 (normal)"
    if normalized.contains('(') && normalized.contains(')') {
        return normalized.to_lowercase();
    }

    // Handle numeric comparison with slight differences
    if tag_key.contains("FocalType")
        || tag_key.contains("Contrast")
        || tag_key.contains("Saturation")
    {
        // These often have value lookup differences - normalize to raw number if possible
        if let Ok(_val) = normalized.parse::<i32>() {
            return normalized.to_string();
        }
        // "Normal" often means 0 for Contrast/Saturation
        if normalized.eq_ignore_ascii_case("normal") {
            return "0".to_string();
        }
        // FocalType: "Zoom" = 2, "Fixed" = 1
        if normalized.eq_ignore_ascii_case("zoom") {
            return "2".to_string();
        }
        if normalized.eq_ignore_ascii_case("fixed") {
            return "1".to_string();
        }
    }

    // Handle ControlMode: "Camera Local Control" = 1
    if tag_key.contains("ControlMode") {
        if normalized.eq_ignore_ascii_case("camera local control") {
            return "1".to_string();
        }
        // Extract number from "Unknown (X)" format
        if let Some(start) = normalized.find('(') {
            if let Some(end) = normalized.find(')') {
                if let Ok(val) = normalized[start + 1..end].parse::<i32>() {
                    return val.to_string();
                }
            }
        }
    }

    // Handle FlashActivity: "0" vs "Did not fire" - both mean no flash
    if tag_key.contains("FlashActivity") {
        if normalized == "0"
            || normalized.eq_ignore_ascii_case("did not fire")
            || normalized.eq_ignore_ascii_case("no flash")
        {
            return "0".to_string();
        }
        if normalized == "1"
            || normalized.eq_ignore_ascii_case("fired")
            || normalized.contains("flash fired")
        {
            return "1".to_string();
        }
    }

    // Handle FlashExposureComp more aggressively
    if tag_key.contains("Flash")
        && tag_key.contains("Comp")
        && (normalized == "+0.0" || normalized == "-0.0" || normalized == "0.0")
    {
        return "0".to_string();
    }

    // Handle AutoExposureBracketing: "Off" vs "+0.0"
    if (tag_key.contains("AutoExposureBracketing") || tag_key.contains("AEB"))
        && (normalized.eq_ignore_ascii_case("off") || normalized == "+0.0" || normalized == "0")
    {
        return "off".to_string();
    }

    // Handle FocusDistance: "0 m" vs "inf" for zero/infinite values
    if tag_key.contains("FocusDistance")
        && (normalized == "0 m" || normalized == "0" || normalized.eq_ignore_ascii_case("inf"))
    {
        return "0".to_string();
    }

    // Handle floating-point precision differences in arrays (e.g., PrimaryChromaticities)
    // Truncate long decimal sequences for comparison
    if tag_key.contains("Chromaticities") || tag_key.contains("WhitePoint") {
        // Truncate values to 6 decimal places for comparison
        let parts: Vec<String> = normalized
            .split_whitespace()
            .map(|s| {
                if let Ok(val) = s.parse::<f64>() {
                    format!("{:.6}", val)
                } else {
                    s.to_string()
                }
            })
            .collect();
        if !parts.is_empty() {
            return parts.join(" ");
        }
    }

    // Handle XP* tags: empty string vs "0"
    // ExifTool shows empty for unset XP tags, OxiDex might show "0"
    if tag_key.contains(":XP") && (normalized.is_empty() || normalized == "0") {
        return "".to_string();
    }

    // Handle XMP rational values: "104/100" vs "1.04"
    if tag_key.starts_with("XMP") && normalized.contains('/') && !normalized.contains(' ') {
        if let Some((num, denom)) = normalized.split_once('/') {
            if let (Ok(n), Ok(d)) = (num.parse::<f64>(), denom.parse::<f64>()) {
                if d != 0.0 {
                    let val = n / d;
                    // Round to reasonable precision
                    return format!("{:.6}", val).trim_end_matches('0').trim_end_matches('.').to_string();
                }
            }
        }
    }

    // Handle XMP ColorClass: "0 (None)" vs "0"
    if tag_key.contains("ColorClass") {
        if let Some(paren_idx) = normalized.find(" (") {
            return normalized[..paren_idx].to_string();
        }
    }

    // Handle boolean values: "Yes" vs "true", "No" vs "false"
    if tag_key.starts_with("XMP") {
        if normalized.eq_ignore_ascii_case("yes") || normalized.eq_ignore_ascii_case("true") {
            return "true".to_string();
        }
        if normalized.eq_ignore_ascii_case("no") || normalized.eq_ignore_ascii_case("false") {
            return "false".to_string();
        }
        // Handle XMP numeric formatting: "0.0" vs "0", "+0.70" vs "0.7"
        let num_str = normalized.trim_start_matches('+');
        if let Ok(val) = num_str.parse::<f64>() {
            // Normalize to simple number without trailing zeros or plus sign
            let formatted = format!("{}", val);
            return formatted;
        }
    }

    // Handle leading zeros in serial numbers
    if tag_key.contains("SerialNumber") {
        // Try to parse as number and compare - removes leading zeros
        let stripped = normalized.trim_start_matches('0');
        if !stripped.is_empty() && stripped.chars().all(|c| c.is_ascii_digit()) {
            return stripped.to_string();
        }
    }

    // Handle empty string cases for ICC_Profile
    if tag_key.starts_with("ICC_Profile:") && normalized.is_empty() {
        return "".to_string();
    }

    // Normalize binary data descriptions
    // ExifTool: "(Binary data 99 bytes, use -b option to ..."
    // OxiDex: "(Binary data, 99 bytes)"
    if normalized.starts_with("(Binary data") {
        // Extract just the byte count for comparison
        if let Some(bytes_match) = extract_binary_bytes(&normalized) {
            return format!("binary:{}", bytes_match);
        }
    }

    normalized
}

/// Extract byte count from binary data description
fn extract_binary_bytes(s: &str) -> Option<usize> {
    // Match patterns like "(Binary data 99 bytes" or "(Binary data, 99 bytes)"
    let s = s.trim_start_matches("(Binary data");
    let s = s.trim_start_matches(',').trim_start();

    // Find the number
    let num_end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if num_end > 0 {
        s[..num_end].parse().ok()
    } else {
        None
    }
}

impl ComparisonEngine {
    /// Compare OxiDex and ExifTool tags for a format
    ///
    /// # Arguments
    /// * `oxidex_tags` - Tags extracted from OxiDex
    /// * `exiftool_tags` - Tags extracted from ExifTool
    /// * `format` - Format name (e.g., "JPEG")
    /// * `files_tested` - Number of files processed during extraction
    /// * `previous` - Previous comparison for regression detection (optional)
    ///
    /// # Returns
    /// FormatComparison with matched/missing/extra/regression analysis
    pub fn compare(
        oxidex_tags: Vec<TagInfo>,
        exiftool_tags: Vec<TagInfo>,
        format: &str,
        files_tested: usize,
        previous: Option<&FormatComparison>,
    ) -> FormatComparison {
        let mut comparison = FormatComparison::new(format.to_string(), files_tested);
        comparison.total_exiftool_tags = exiftool_tags.len();

        // Build lookup map for efficient OxiDex tag lookup by key
        let oxidex_by_key: HashMap<String, &TagInfo> =
            oxidex_tags.iter().map(|t| (t.key(), t)).collect();

        // Track which ExifTool tags were matched
        let mut matched_exiftool_keys = HashSet::new();

        // Compare each ExifTool tag
        for et_tag in &exiftool_tags {
            let key = et_tag.key();

            if let Some(ox_tag) = oxidex_by_key.get(&key) {
                // Tag exists in both - check if values match
                matched_exiftool_keys.insert(key.clone());

                // Normalize values for comparison to handle formatting differences
                let norm_ox = normalize_value_for_comparison(&key, &ox_tag.value);
                let norm_et = normalize_value_for_comparison(&key, &et_tag.value);

                if norm_ox == norm_et {
                    // Values match after normalization
                    comparison.matched_tags.push(key);
                } else {
                    // Tag exists but values differ even after normalization
                    comparison.value_differences.push(ValueDifference {
                        tag_key: key,
                        exiftool_value: et_tag.value.clone(),
                        oxidex_value: ox_tag.value.clone(),
                        source_file: et_tag.source_file.clone().unwrap_or_default(),
                    });
                }
            } else {
                // Tag missing in OxiDex
                comparison.missing_in_oxidex.push(et_tag.clone());
            }
        }

        // Find extra tags in OxiDex (not in ExifTool)
        for ox_tag in &oxidex_tags {
            let key = ox_tag.key();
            if !matched_exiftool_keys.contains(&key) {
                comparison.extra_in_oxidex.push(ox_tag.clone());
            }
        }

        // Detect regressions: tags that were in previous.matched_tags but NOT in current matched_tags
        if let Some(prev) = previous {
            let current_matched: HashSet<_> = comparison.matched_tags.iter().collect();
            for prev_tag in &prev.matched_tags {
                if !current_matched.contains(prev_tag) {
                    comparison.regressions.push(prev_tag.clone());
                }
            }
        }

        // Calculate coverage
        comparison.calculate_coverage();

        comparison
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_all_matched() {
        let oxidex_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            TagInfo::new("Model".to_string(), "EXIF".to_string(), "5D".to_string()),
        ];
        let exiftool_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            TagInfo::new("Model".to_string(), "EXIF".to_string(), "5D".to_string()),
        ];

        let result = ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", 1, None);
        assert_eq!(result.matched_tags.len(), 2);
        assert_eq!(result.missing_in_oxidex.len(), 0);
        assert_eq!(result.extra_in_oxidex.len(), 0);
        assert_eq!(result.coverage_percentage, 100.0);
    }

    #[test]
    fn test_compare_partial_match() {
        let oxidex_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            // Model is missing
        ];
        let exiftool_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            TagInfo::new("Model".to_string(), "EXIF".to_string(), "5D".to_string()),
        ];

        let result = ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", 1, None);
        assert_eq!(result.matched_tags.len(), 1);
        assert_eq!(result.missing_in_oxidex.len(), 1);
        assert_eq!(result.extra_in_oxidex.len(), 0);
        assert_eq!(result.coverage_percentage, 50.0);
    }

    #[test]
    fn test_compare_with_extra_tags() {
        let oxidex_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            TagInfo::new("Model".to_string(), "EXIF".to_string(), "5D".to_string()),
            TagInfo::new(
                "CustomTag".to_string(),
                "EXIF".to_string(),
                "Custom".to_string(),
            ),
        ];
        let exiftool_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            TagInfo::new("Model".to_string(), "EXIF".to_string(), "5D".to_string()),
        ];

        let result = ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", 1, None);
        assert_eq!(result.matched_tags.len(), 2);
        assert_eq!(result.missing_in_oxidex.len(), 0);
        assert_eq!(result.extra_in_oxidex.len(), 1);
        assert_eq!(result.coverage_percentage, 100.0);
    }

    #[test]
    fn test_compare_empty_oxidex() {
        let oxidex_tags = vec![];
        let exiftool_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            TagInfo::new("Model".to_string(), "EXIF".to_string(), "5D".to_string()),
        ];

        let result = ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", 1, None);
        assert_eq!(result.matched_tags.len(), 0);
        assert_eq!(result.missing_in_oxidex.len(), 2);
        assert_eq!(result.extra_in_oxidex.len(), 0);
        assert_eq!(result.coverage_percentage, 0.0);
    }

    #[test]
    fn test_compare_empty_exiftool() {
        let oxidex_tags = vec![TagInfo::new(
            "Make".to_string(),
            "EXIF".to_string(),
            "Canon".to_string(),
        )];
        let exiftool_tags = vec![];

        let result = ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", 1, None);
        assert_eq!(result.matched_tags.len(), 0);
        assert_eq!(result.missing_in_oxidex.len(), 0);
        assert_eq!(result.extra_in_oxidex.len(), 1);
        assert_eq!(result.coverage_percentage, 0.0);
    }

    #[test]
    fn test_regression_detection() {
        let oxidex_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            // Model is now missing - this is a regression
        ];
        let exiftool_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            TagInfo::new("Model".to_string(), "EXIF".to_string(), "5D".to_string()),
        ];

        // Previous baseline had both tags matched
        let mut previous = FormatComparison::new("JPEG".to_string(), 2);
        previous.matched_tags = vec!["EXIF:Make".to_string(), "EXIF:Model".to_string()];

        let result =
            ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", 2, Some(&previous));

        // Should have 1 regression (Model is missing)
        assert_eq!(result.regressions.len(), 1);
        assert!(result.regressions.contains(&"EXIF:Model".to_string()));

        // Should have 1 matched tag (Make)
        assert_eq!(result.matched_tags.len(), 1);
        assert!(result.matched_tags.contains(&"EXIF:Make".to_string()));

        // Model should be in missing_in_oxidex
        assert_eq!(result.missing_in_oxidex.len(), 1);
        assert_eq!(result.missing_in_oxidex[0].name, "Model");
    }

    #[test]
    fn test_regression_detection_no_previous() {
        let oxidex_tags = vec![TagInfo::new(
            "Make".to_string(),
            "EXIF".to_string(),
            "Canon".to_string(),
        )];
        let exiftool_tags = vec![TagInfo::new(
            "Make".to_string(),
            "EXIF".to_string(),
            "Canon".to_string(),
        )];

        let result = ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", 1, None);

        // No regressions when there's no previous baseline
        assert_eq!(result.regressions.len(), 0);
    }

    #[test]
    fn test_regression_detection_no_regressions() {
        let oxidex_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            TagInfo::new("Model".to_string(), "EXIF".to_string(), "5D".to_string()),
        ];
        let exiftool_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            TagInfo::new("Model".to_string(), "EXIF".to_string(), "5D".to_string()),
        ];

        // Previous baseline had only one tag
        let mut previous = FormatComparison::new("JPEG".to_string(), 1);
        previous.matched_tags = vec!["EXIF:Make".to_string()];

        let result =
            ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", 1, Some(&previous));

        // No regressions - we still have Make, and we added Model
        assert_eq!(result.regressions.len(), 0);
        assert_eq!(result.matched_tags.len(), 2);
    }

    #[test]
    fn test_value_difference_detection() {
        let oxidex_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            TagInfo::new(
                "Model".to_string(),
                "EXIF".to_string(),
                "EOS 5D".to_string(),
            ), // Different value
        ];
        let exiftool_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            TagInfo::new(
                "Model".to_string(),
                "EXIF".to_string(),
                "5D Mark II".to_string(),
            ), // Different value
        ];

        let result = ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", 1, None);

        // Make should match perfectly
        assert_eq!(result.matched_tags.len(), 1);
        assert!(result.matched_tags.contains(&"EXIF:Make".to_string()));

        // Model should have value difference
        assert_eq!(result.value_differences.len(), 1);
        assert_eq!(result.value_differences[0].tag_key, "EXIF:Model");
        assert_eq!(result.value_differences[0].exiftool_value, "5D Mark II");
        assert_eq!(result.value_differences[0].oxidex_value, "EOS 5D");

        // Nothing should be missing or extra
        assert_eq!(result.missing_in_oxidex.len(), 0);
        assert_eq!(result.extra_in_oxidex.len(), 0);
    }

    #[test]
    fn test_complex_comparison_with_all_categories() {
        let oxidex_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()), // Match
            TagInfo::new(
                "Model".to_string(),
                "EXIF".to_string(),
                "EOS 5D".to_string(),
            ), // Value diff
            TagInfo::new(
                "CustomTag".to_string(),
                "EXIF".to_string(),
                "Custom".to_string(),
            ), // Extra
                                                                                       // DateTime is missing - will be a regression
        ];
        let exiftool_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            TagInfo::new(
                "Model".to_string(),
                "EXIF".to_string(),
                "5D Mark II".to_string(),
            ),
            TagInfo::new(
                "DateTime".to_string(),
                "EXIF".to_string(),
                "2025:12:07 10:00:00".to_string(),
            ),
            TagInfo::new("ISO".to_string(), "EXIF".to_string(), "400".to_string()), // Missing in oxidex
        ];

        // Previous had Make and DateTime
        let mut previous = FormatComparison::new("JPEG".to_string(), 1);
        previous.matched_tags = vec!["EXIF:Make".to_string(), "EXIF:DateTime".to_string()];

        let result =
            ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", 1, Some(&previous));

        // Matched: Make
        assert_eq!(result.matched_tags.len(), 1);
        assert!(result.matched_tags.contains(&"EXIF:Make".to_string()));

        // Value differences: Model
        assert_eq!(result.value_differences.len(), 1);
        assert_eq!(result.value_differences[0].tag_key, "EXIF:Model");

        // Missing in OxiDex: DateTime, ISO
        assert_eq!(result.missing_in_oxidex.len(), 2);
        let missing_names: Vec<_> = result.missing_in_oxidex.iter().map(|t| &t.name).collect();
        assert!(missing_names.contains(&&"DateTime".to_string()));
        assert!(missing_names.contains(&&"ISO".to_string()));

        // Extra in OxiDex: CustomTag
        assert_eq!(result.extra_in_oxidex.len(), 1);
        assert_eq!(result.extra_in_oxidex[0].name, "CustomTag");

        // Regressions: DateTime (was in previous, not in current matched)
        assert_eq!(result.regressions.len(), 1);
        assert!(result.regressions.contains(&"EXIF:DateTime".to_string()));

        // Coverage: 1 matched out of 4 total = 25%
        assert_eq!(result.coverage_percentage, 25.0);
    }
}
