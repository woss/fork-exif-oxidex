//! Data models for tag comparison and reporting

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Information about a single metadata tag
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TagInfo {
    /// Tag name (e.g., "Make", "Model", "DateTime")
    pub name: String,
    /// Tag family (e.g., "EXIF", "XMP", "IPTC", "MakerNote")
    pub family: String,
    /// Optional tag ID in hex format (e.g., "0x010F")
    pub tag_id: Option<String>,
    /// Frequency: how many files contain this tag (0-100%)
    pub frequency: usize,
    /// Human-readable description of the tag
    pub description: String,
}

impl TagInfo {
    /// Create a new TagInfo
    pub fn new(
        name: String,
        family: String,
        frequency: usize,
    ) -> Self {
        Self {
            name,
            family,
            tag_id: None,
            frequency,
            description: String::new(),
        }
    }

    /// Set the tag ID
    pub fn with_tag_id(mut self, tag_id: String) -> Self {
        self.tag_id = Some(tag_id);
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }
}

/// Comparison results for a single file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatComparison {
    /// Format name (e.g., "JPEG", "PNG", "TIFF")
    pub format: String,
    /// List of matched tag names
    pub matched_tags: Vec<String>,
    /// Tags found in ExifTool but missing in OxiDex
    pub missing_in_oxidex: Vec<TagInfo>,
    /// Tags found in OxiDex but not in ExifTool
    pub extra_in_oxidex: Vec<TagInfo>,
    /// Coverage percentage (matched / total_exiftool)
    pub coverage_percentage: f64,
    /// Total number of tags in ExifTool for this format
    pub total_exiftool_tags: usize,
    /// Timestamp when this comparison was generated
    pub timestamp: String,
}

impl FormatComparison {
    /// Create a new FormatComparison result
    pub fn new(format: String, total_exiftool_tags: usize) -> Self {
        Self {
            format,
            matched_tags: Vec::new(),
            missing_in_oxidex: Vec::new(),
            extra_in_oxidex: Vec::new(),
            coverage_percentage: 0.0,
            total_exiftool_tags,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Calculate coverage percentage
    pub fn calculate_coverage(&mut self) {
        if self.total_exiftool_tags == 0 {
            self.coverage_percentage = 0.0;
        } else {
            self.coverage_percentage =
                (self.matched_tags.len() as f64 / self.total_exiftool_tags as f64) * 100.0;
        }
    }

    /// Get summary statistics
    pub fn summary(&self) -> String {
        format!(
            "{}: {}/{} tags ({:.1}% coverage)",
            self.format,
            self.matched_tags.len(),
            self.total_exiftool_tags,
            self.coverage_percentage
        )
    }
}

/// Complete comparison report for all formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    /// When this report was generated
    pub generated_at: String,
    /// Comparisons indexed by format name
    pub by_format: HashMap<String, FormatComparison>,
    /// Overall coverage across all formats
    pub overall_coverage: f64,
    /// Summary text
    pub summary: String,
}

impl ComparisonReport {
    /// Create a new empty report
    pub fn new() -> Self {
        Self {
            generated_at: chrono::Utc::now().to_rfc3339(),
            by_format: HashMap::new(),
            overall_coverage: 0.0,
            summary: String::new(),
        }
    }

    /// Add a format comparison to the report
    pub fn add_format(&mut self, format: String, comparison: FormatComparison) {
        self.by_format.insert(format, comparison);
    }

    /// Calculate overall coverage across all formats
    pub fn calculate_overall_coverage(&mut self) {
        if self.by_format.is_empty() {
            self.overall_coverage = 0.0;
            self.summary = "No formats analyzed".to_string();
            return;
        }

        let total_matched: usize = self
            .by_format
            .values()
            .map(|c| c.matched_tags.len())
            .sum();

        let total_tags: usize = self
            .by_format
            .values()
            .map(|c| c.total_exiftool_tags)
            .sum();

        if total_tags == 0 {
            self.overall_coverage = 0.0;
        } else {
            self.overall_coverage = (total_matched as f64 / total_tags as f64) * 100.0;
        }

        let format_count = self.by_format.len();
        self.summary = format!(
            "Analyzed {} formats: {}/{} tags ({:.1}% overall coverage)",
            format_count, total_matched, total_tags, self.overall_coverage
        );
    }

    /// Get format names in sorted order
    pub fn format_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.by_format.keys().cloned().collect();
        names.sort();
        names
    }
}

impl Default for ComparisonReport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_info_creation() {
        let tag = TagInfo::new("Make".to_string(), "EXIF".to_string(), 50);
        assert_eq!(tag.name, "Make");
        assert_eq!(tag.family, "EXIF");
        assert_eq!(tag.frequency, 50);
        assert_eq!(tag.tag_id, None);
    }

    #[test]
    fn test_tag_info_with_builder() {
        let tag = TagInfo::new("Make".to_string(), "EXIF".to_string(), 50)
            .with_tag_id("0x010F".to_string())
            .with_description("Camera manufacturer".to_string());

        assert_eq!(tag.name, "Make");
        assert_eq!(tag.tag_id, Some("0x010F".to_string()));
        assert_eq!(tag.description, "Camera manufacturer");
    }

    #[test]
    fn test_format_comparison_creation() {
        let comp = FormatComparison::new("JPEG".to_string(), 100);
        assert_eq!(comp.format, "JPEG");
        assert_eq!(comp.total_exiftool_tags, 100);
        assert_eq!(comp.matched_tags.len(), 0);
        assert_eq!(comp.coverage_percentage, 0.0);
    }

    #[test]
    fn test_format_comparison_coverage_calculation() {
        let mut comp = FormatComparison::new("JPEG".to_string(), 100);
        comp.matched_tags = vec![
            "Make".to_string(),
            "Model".to_string(),
            "DateTime".to_string(),
        ];
        comp.calculate_coverage();

        assert_eq!(comp.coverage_percentage, 3.0); // 3/100 = 3%
    }

    #[test]
    fn test_format_comparison_coverage_zero_tags() {
        let mut comp = FormatComparison::new("JPEG".to_string(), 0);
        comp.calculate_coverage();
        assert_eq!(comp.coverage_percentage, 0.0);
    }

    #[test]
    fn test_format_comparison_summary() {
        let mut comp = FormatComparison::new("JPEG".to_string(), 100);
        comp.matched_tags = vec!["Make".to_string(), "Model".to_string()];
        comp.calculate_coverage();

        let summary = comp.summary();
        assert!(summary.contains("JPEG"));
        assert!(summary.contains("2/100"));
        assert!(summary.contains("2.0% coverage"));
    }

    #[test]
    fn test_comparison_report_creation() {
        let report = ComparisonReport::new();
        assert_eq!(report.by_format.len(), 0);
        assert_eq!(report.overall_coverage, 0.0);
    }

    #[test]
    fn test_comparison_report_add_format() {
        let mut report = ComparisonReport::new();
        let comp = FormatComparison::new("JPEG".to_string(), 100);
        report.add_format("JPEG".to_string(), comp);

        assert_eq!(report.by_format.len(), 1);
        assert!(report.by_format.contains_key("JPEG"));
    }

    #[test]
    fn test_comparison_report_overall_coverage_single_format() {
        let mut report = ComparisonReport::new();
        let mut comp = FormatComparison::new("JPEG".to_string(), 100);
        comp.matched_tags = (0..50).map(|i| format!("Tag{}", i)).collect();
        comp.calculate_coverage();
        report.add_format("JPEG".to_string(), comp);
        report.calculate_overall_coverage();

        assert_eq!(report.overall_coverage, 50.0); // 50/100 = 50%
        assert!(report.summary.contains("50.0%"));
    }

    #[test]
    fn test_comparison_report_overall_coverage_multiple_formats() {
        let mut report = ComparisonReport::new();

        // Format 1: 50/100 = 50%
        let mut comp1 = FormatComparison::new("JPEG".to_string(), 100);
        comp1.matched_tags = (0..50).map(|i| format!("Tag{}", i)).collect();
        comp1.calculate_coverage();

        // Format 2: 30/50 = 60%
        let mut comp2 = FormatComparison::new("PNG".to_string(), 50);
        comp2.matched_tags = (0..30).map(|i| format!("Tag{}", i)).collect();
        comp2.calculate_coverage();

        report.add_format("JPEG".to_string(), comp1);
        report.add_format("PNG".to_string(), comp2);
        report.calculate_overall_coverage();

        // Total: 80/150 = 53.33%
        assert!((report.overall_coverage - 53.33).abs() < 0.1);
        assert!(report.summary.contains("2 formats"));
    }

    #[test]
    fn test_comparison_report_format_names() {
        let mut report = ComparisonReport::new();
        report.add_format("PNG".to_string(), FormatComparison::new("PNG".to_string(), 100));
        report.add_format("JPEG".to_string(), FormatComparison::new("JPEG".to_string(), 100));
        report.add_format("TIFF".to_string(), FormatComparison::new("TIFF".to_string(), 100));

        let names = report.format_names();
        assert_eq!(names, vec!["JPEG", "PNG", "TIFF"]); // Sorted order
    }

    #[test]
    fn test_comparison_report_empty_summary() {
        let mut report = ComparisonReport::new();
        report.calculate_overall_coverage();
        assert!(report.summary.contains("No formats"));
    }

    #[test]
    fn test_tag_info_serialization() {
        let tag = TagInfo::new("Make".to_string(), "EXIF".to_string(), 50);
        let json = serde_json::to_string(&tag).unwrap();
        let deserialized: TagInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(tag, deserialized);
    }

    #[test]
    fn test_format_comparison_serialization() {
        let mut comp = FormatComparison::new("JPEG".to_string(), 100);
        comp.matched_tags = vec!["Make".to_string(), "Model".to_string()];
        comp.calculate_coverage();

        let json = serde_json::to_string(&comp).unwrap();
        let deserialized: FormatComparison = serde_json::from_str(&json).unwrap();
        assert_eq!(comp.format, deserialized.format);
        assert_eq!(comp.matched_tags, deserialized.matched_tags);
    }
}
