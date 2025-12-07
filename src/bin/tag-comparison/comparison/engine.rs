//! Tag comparison engine - Match and compare tags between OxiDex and ExifTool

use crate::models::{FormatComparison, TagInfo, ValueDifference};
use std::collections::{HashMap, HashSet};

/// Comparison engine for analyzing tag differences
pub struct ComparisonEngine;

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

                if ox_tag.value == et_tag.value {
                    // Perfect match - same tag, same value
                    comparison.matched_tags.push(key);
                } else {
                    // Tag exists but values differ
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
