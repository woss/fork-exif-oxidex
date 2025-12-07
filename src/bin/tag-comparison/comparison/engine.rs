//! Tag comparison engine - Match and compare tags between OxiDex and ExifTool

use crate::models::{TagInfo, FormatComparison};
use std::collections::HashSet;

/// Comparison engine for analyzing tag differences
pub struct ComparisonEngine;

impl ComparisonEngine {
    /// Compare OxiDex and ExifTool tags for a format
    ///
    /// # Arguments
    /// * `oxidex_tags` - Tags extracted from OxiDex
    /// * `exiftool_tags` - Tags extracted from ExifTool
    /// * `format` - Format name (e.g., "JPEG")
    /// * `previous` - Previous comparison for regression detection (optional)
    ///
    /// # Returns
    /// FormatComparison with matched/missing/extra analysis
    pub fn compare(
        oxidex_tags: Vec<TagInfo>,
        exiftool_tags: Vec<TagInfo>,
        format: &str,
        _previous: Option<&FormatComparison>,
    ) -> FormatComparison {
        let mut comparison = FormatComparison::new(format.to_string(), 0);
        comparison.total_exiftool_tags = exiftool_tags.len();
        let mut oxidex_matched = HashSet::new();
        let mut exiftool_matched = HashSet::new();

        // Try to match each OxiDex tag with ExifTool tag
        for (ox_idx, ox_tag) in oxidex_tags.iter().enumerate() {
            for (et_idx, et_tag) in exiftool_tags.iter().enumerate() {
                if Self::tags_match(&ox_tag, &et_tag) {
                    comparison.matched_tags.push(ox_tag.key());
                    oxidex_matched.insert(ox_idx);
                    exiftool_matched.insert(et_idx);
                    break;
                }
            }
        }

        // Unmatched OxiDex tags are "extra"
        for (idx, tag) in oxidex_tags.iter().enumerate() {
            if !oxidex_matched.contains(&idx) {
                comparison.extra_in_oxidex.push(tag.clone());
            }
        }

        // Unmatched ExifTool tags are "missing"
        for (idx, tag) in exiftool_tags.iter().enumerate() {
            if !exiftool_matched.contains(&idx) {
                comparison.missing_in_oxidex.push(tag.clone());
            }
        }

        // Calculate coverage
        comparison.calculate_coverage();

        comparison
    }

    /// Check if two tags match (represent the same metadata)
    ///
    /// Two tags match if:
    /// - Names are identical (case-sensitive)
    /// - Families match (EXIF, XMP, etc.)
    fn tags_match(oxidex: &TagInfo, exiftool: &TagInfo) -> bool {
        oxidex.name == exiftool.name && oxidex.family == exiftool.family
    }

    /// Normalize tag name for matching
    /// Handles variations like "EXIF:Make" vs "IFD0:Make"
    #[allow(dead_code)]
    fn normalize_name(name: &str) -> String {
        // TODO: Implementation
        // Handle variations in tag naming
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tags_match_identical() {
        let tag1 = TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string());
        let tag2 = TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string());
        assert!(ComparisonEngine::tags_match(&tag1, &tag2));
    }

    #[test]
    fn test_tags_dont_match_different_names() {
        let tag1 = TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string());
        let tag2 = TagInfo::new("Model".to_string(), "EXIF".to_string(), "5D".to_string());
        assert!(!ComparisonEngine::tags_match(&tag1, &tag2));
    }

    #[test]
    fn test_tags_dont_match_different_families() {
        let tag1 = TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string());
        let tag2 = TagInfo::new("Make".to_string(), "XMP".to_string(), "Canon".to_string());
        assert!(!ComparisonEngine::tags_match(&tag1, &tag2));
    }

    #[test]
    fn test_tags_case_sensitive() {
        let tag1 = TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string());
        let tag2 = TagInfo::new("make".to_string(), "EXIF".to_string(), "Canon".to_string());
        assert!(!ComparisonEngine::tags_match(&tag1, &tag2));
    }

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

        let result = ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", None);
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

        let result = ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", None);
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
            TagInfo::new("CustomTag".to_string(), "EXIF".to_string(), "Custom".to_string()),
        ];
        let exiftool_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
            TagInfo::new("Model".to_string(), "EXIF".to_string(), "5D".to_string()),
        ];

        let result = ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", None);
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

        let result = ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", None);
        assert_eq!(result.matched_tags.len(), 0);
        assert_eq!(result.missing_in_oxidex.len(), 2);
        assert_eq!(result.extra_in_oxidex.len(), 0);
        assert_eq!(result.coverage_percentage, 0.0);
    }

    #[test]
    fn test_compare_empty_exiftool() {
        let oxidex_tags = vec![
            TagInfo::new("Make".to_string(), "EXIF".to_string(), "Canon".to_string()),
        ];
        let exiftool_tags = vec![];

        let result = ComparisonEngine::compare(oxidex_tags, exiftool_tags, "JPEG", None);
        assert_eq!(result.matched_tags.len(), 0);
        assert_eq!(result.missing_in_oxidex.len(), 0);
        assert_eq!(result.extra_in_oxidex.len(), 1);
        assert_eq!(result.coverage_percentage, 0.0);
    }
}
