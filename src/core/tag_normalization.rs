//! Tag family normalization to match ExifTool conventions
//!
//! This module provides functions to normalize tag family prefixes from OxiDex's
//! internal representation to ExifTool's conventions. This improves compatibility
//! with ExifTool's output format.
//!
//! # Mapping Rules
//!
//! - `ExifIFD:` -> `EXIF:`
//! - `IFD0:`, `IFD1:`, `GPS:` remain unchanged
//! - Manufacturer names (`Canon:`, `Nikon:`, `Sony:`, etc.) remain unchanged

use std::collections::HashMap;
use std::sync::LazyLock;

/// Family prefix mappings from OxiDex to ExifTool conventions
static FAMILY_MAPPINGS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    // EXIF IFD mapping - this is the main normalization
    m.insert("ExifIFD", "EXIF");
    // GPS IFD mapping (unchanged, but included for completeness)
    m.insert("GPS", "GPS");
    // InteropIFD mapping (unchanged)
    m.insert("InteropIFD", "InteropIFD");
    // IFD0 and IFD1 remain unchanged but are included for documentation
    m.insert("IFD0", "IFD0");
    m.insert("IFD1", "IFD1");
    // Maker note families (unchanged)
    m.insert("Canon", "Canon");
    m.insert("Nikon", "Nikon");
    m.insert("Sony", "Sony");
    m.insert("Fujifilm", "Fujifilm");
    m.insert("Panasonic", "Panasonic");
    m.insert("Olympus", "Olympus");
    m.insert("Pentax", "Pentax");
    m.insert("Samsung", "Samsung");
    m
});

/// Normalize a tag key to match ExifTool family conventions
///
/// # Arguments
/// * `tag_key` - Full tag key like "ExifIFD:Make"
///
/// # Returns
/// Normalized key like "EXIF:Make"
///
/// # Examples
///
/// ```
/// use oxidex::core::tag_normalization::normalize_tag_family;
///
/// assert_eq!(normalize_tag_family("ExifIFD:Make"), "EXIF:Make");
/// assert_eq!(normalize_tag_family("IFD0:Make"), "IFD0:Make");
/// assert_eq!(normalize_tag_family("Canon:LensModel"), "Canon:LensModel");
/// ```
pub fn normalize_tag_family(tag_key: &str) -> String {
    if let Some((family, name)) = tag_key.split_once(':') {
        if let Some(normalized) = FAMILY_MAPPINGS.get(family) {
            return format!("{}:{}", normalized, name);
        }
    }
    tag_key.to_string()
}

/// Normalize all tags in a MetadataMap
///
/// This function creates a new MetadataMap with all tag keys normalized
/// to match ExifTool conventions. The original map is not modified.
///
/// # Arguments
/// * `map` - The metadata map to normalize
///
/// # Returns
/// A new MetadataMap with normalized tag keys
///
/// # Examples
///
/// ```
/// use oxidex::core::{MetadataMap, TagValue};
/// use oxidex::core::tag_normalization::normalize_metadata_map;
///
/// let mut map = MetadataMap::new();
/// map.insert("ExifIFD:Make", TagValue::new_string("Canon"));
/// map.insert("IFD0:Model", TagValue::new_string("EOS R5"));
///
/// let normalized = normalize_metadata_map(&map);
/// assert_eq!(normalized.get_string("EXIF:Make"), Some("Canon"));
/// assert_eq!(normalized.get_string("IFD0:Model"), Some("EOS R5"));
/// ```
pub fn normalize_metadata_map(
    map: &crate::core::MetadataMap,
) -> crate::core::MetadataMap {
    let mut normalized = crate::core::MetadataMap::with_capacity(map.len());
    for (key, value) in map.iter() {
        let normalized_key = normalize_tag_family(key);
        normalized.insert(normalized_key, value.clone());
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{MetadataMap, TagValue};

    #[test]
    fn test_exififd_normalization() {
        assert_eq!(normalize_tag_family("ExifIFD:Make"), "EXIF:Make");
        assert_eq!(normalize_tag_family("ExifIFD:Model"), "EXIF:Model");
        assert_eq!(normalize_tag_family("ExifIFD:DateTimeOriginal"), "EXIF:DateTimeOriginal");
    }

    #[test]
    fn test_ifd0_unchanged() {
        assert_eq!(normalize_tag_family("IFD0:Make"), "IFD0:Make");
        assert_eq!(normalize_tag_family("IFD0:Model"), "IFD0:Model");
    }

    #[test]
    fn test_ifd1_unchanged() {
        assert_eq!(normalize_tag_family("IFD1:Compression"), "IFD1:Compression");
    }

    #[test]
    fn test_gps_unchanged() {
        assert_eq!(normalize_tag_family("GPS:GPSLatitude"), "GPS:GPSLatitude");
    }

    #[test]
    fn test_makernotes_unchanged() {
        assert_eq!(normalize_tag_family("Canon:LensModel"), "Canon:LensModel");
        assert_eq!(normalize_tag_family("Nikon:ShutterCount"), "Nikon:ShutterCount");
        assert_eq!(normalize_tag_family("Sony:SonyModelID"), "Sony:SonyModelID");
    }

    #[test]
    fn test_unknown_family_unchanged() {
        assert_eq!(normalize_tag_family("Custom:Tag"), "Custom:Tag");
        assert_eq!(normalize_tag_family("Unknown:Field"), "Unknown:Field");
    }

    #[test]
    fn test_no_colon_unchanged() {
        assert_eq!(normalize_tag_family("NoColonHere"), "NoColonHere");
        assert_eq!(normalize_tag_family("SimpleTag"), "SimpleTag");
    }

    #[test]
    fn test_normalize_metadata_map() {
        let mut map = MetadataMap::new();
        map.insert("ExifIFD:Make", TagValue::new_string("Canon"));
        map.insert("ExifIFD:Model", TagValue::new_string("EOS R5"));
        map.insert("IFD0:Software", TagValue::new_string("OxiDex"));
        map.insert("GPS:GPSLatitude", TagValue::new_string("37.7749"));
        map.insert("Canon:LensModel", TagValue::new_string("EF 24-70mm"));

        let normalized = normalize_metadata_map(&map);

        // ExifIFD should be normalized to EXIF
        assert_eq!(normalized.get_string("EXIF:Make"), Some("Canon"));
        assert_eq!(normalized.get_string("EXIF:Model"), Some("EOS R5"));

        // IFD0, GPS, and Canon should remain unchanged
        assert_eq!(normalized.get_string("IFD0:Software"), Some("OxiDex"));
        assert_eq!(normalized.get_string("GPS:GPSLatitude"), Some("37.7749"));
        assert_eq!(normalized.get_string("Canon:LensModel"), Some("EF 24-70mm"));

        // Verify we have the same number of tags
        assert_eq!(normalized.len(), map.len());
    }

    #[test]
    fn test_normalize_empty_map() {
        let map = MetadataMap::new();
        let normalized = normalize_metadata_map(&map);
        assert_eq!(normalized.len(), 0);
        assert!(normalized.is_empty());
    }

    #[test]
    fn test_normalize_preserves_values() {
        let mut map = MetadataMap::new();
        map.insert("ExifIFD:ISO", TagValue::new_integer(400));
        map.insert("ExifIFD:FNumber", TagValue::new_float(2.8));

        let normalized = normalize_metadata_map(&map);

        assert_eq!(normalized.get_integer("EXIF:ISO"), Some(400));
        assert_eq!(normalized.get_float("EXIF:FNumber"), Some(2.8));
    }
}
