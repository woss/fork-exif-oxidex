//! List all available metadata tags tool
//!
//! This module provides functionality to browse the complete tag database
//! with filtering capabilities by group, format, writable status, and search term.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Parameters for the list_tags tool
#[derive(Debug, Deserialize)]
struct ListTagsParams {
    /// Filter by tag group (e.g., "EXIF", "XMP", "IPTC", "GPS")
    group: Option<String>,
    /// Filter by file format (currently not implemented - would need format-to-tag mapping)
    format: Option<String>,
    /// Only show writable tags
    writable: Option<bool>,
    /// Search tags by name or description (case-insensitive partial match)
    search: Option<String>,
}

/// Main handler for the list_tags tool
///
/// Lists all available metadata tags from the OxiDex tag database, with optional filtering.
/// Tags are organized by their format family (EXIF, XMP, IPTC, GPS, etc.).
///
/// # Arguments
///
/// * `arguments` - JSON arguments containing optional filters
///
/// # Returns
///
/// A formatted string listing all matching tags with their descriptions
pub async fn handle(arguments: Value) -> Result<String> {
    let params: ListTagsParams =
        serde_json::from_value(arguments).context("Invalid arguments for list_tags")?;

    // Collect all tags from all domain-specific tag databases
    let tags = collect_all_tags(&params)?;

    if tags.is_empty() {
        return Ok("No tags found matching the specified criteria.".to_string());
    }

    // Format the output grouped by format family
    Ok(format_tags_by_group(tags))
}

/// Represents a tag entry with all its metadata
#[derive(Debug, Clone)]
struct TagEntry {
    name: String,
    description: String,
    writable: bool,
    type_name: String,
    format_family: String,
}

/// Collect all tags from the YAML-based tag databases
///
/// This function scans through all domain-specific tag databases (core, camera,
/// media, image, document, specialty) and collects tags that match the filter criteria.
///
/// # Arguments
///
/// * `params` - Filter parameters for tag selection
///
/// # Returns
///
/// A vector of TagEntry structs containing all matching tags
fn collect_all_tags(params: &ListTagsParams) -> Result<Vec<TagEntry>> {
    let mut tags = Vec::new();
    let mut seen_names = HashSet::new(); // Deduplicate tags with same name

    // Helper function to extract format family prefix from table name
    fn get_format_family(table_name: &str) -> &str {
        if table_name.starts_with("Exif::") {
            "EXIF"
        } else if table_name.starts_with("GPS::") {
            "GPS"
        } else if table_name.starts_with("XMP::") {
            "XMP"
        } else if table_name.starts_with("IPTC::") {
            "IPTC"
        } else if table_name.starts_with("ICC_Profile::") {
            "ICC_Profile"
        } else if table_name.starts_with("Photoshop::") {
            "Photoshop"
        } else if table_name.starts_with("JFIF::") {
            "JFIF"
        } else if table_name.starts_with("PNG::") {
            "PNG"
        } else if table_name.starts_with("PDF::") {
            "PDF"
        } else if table_name.starts_with("QuickTime::") {
            "QuickTime"
        } else if table_name.contains("Canon")
            || table_name.contains("Nikon")
            || table_name.contains("Sony")
        {
            "MakerNotes"
        } else {
            "Other"
        }
    }

    // Helper function to process a tag table
    let mut process_table = |table: &oxidex::tag_db::TagTable| {
        let format_family = get_format_family(&table.name);

        // Skip if group filter doesn't match
        if let Some(ref group_filter) = params.group {
            if !format_family.eq_ignore_ascii_case(group_filter) {
                return;
            }
        }

        for tag in &table.tags {
            // Construct full tag name (e.g., "EXIF:Make")
            let full_name = format!("{}:{}", format_family, tag.name);

            // Skip duplicates
            if seen_names.contains(&full_name) {
                continue;
            }

            // Apply writable filter
            if let Some(writable_filter) = params.writable {
                if tag.writable != writable_filter {
                    continue;
                }
            }

            // Apply search filter (case-insensitive partial match on name or description)
            if let Some(ref search_term) = params.search {
                let search_lower = search_term.to_lowercase();
                let name_matches = full_name.to_lowercase().contains(&search_lower);
                let desc_matches = tag
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&search_lower))
                    .unwrap_or(false);

                if !name_matches && !desc_matches {
                    continue;
                }
            }

            // Add the tag
            tags.push(TagEntry {
                name: full_name.clone(),
                description: tag
                    .description
                    .clone()
                    .unwrap_or_else(|| "No description available".to_string()),
                writable: tag.writable,
                type_name: tag
                    .type_name
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                format_family: format_family.to_string(),
            });

            seen_names.insert(full_name);
        }
    };

    // Process all domain-specific tag databases
    for table in &oxidex::tag_db::core::CORE_TAGS.tables {
        process_table(table);
    }

    for table in &oxidex::tag_db::camera::CAMERA_TAGS.tables {
        process_table(table);
    }

    for table in &oxidex::tag_db::media::MEDIA_TAGS.tables {
        process_table(table);
    }

    for table in &oxidex::tag_db::image::IMAGE_TAGS.tables {
        process_table(table);
    }

    for table in &oxidex::tag_db::document::DOCUMENT_TAGS.tables {
        process_table(table);
    }

    for table in &oxidex::tag_db::specialty::SPECIALTY_TAGS.tables {
        process_table(table);
    }

    Ok(tags)
}

/// Format tags grouped by their format family
///
/// This function organizes tags by format family and presents them in a
/// human-readable format with tag counts and descriptions.
///
/// # Arguments
///
/// * `tags` - Vector of tag entries to format
///
/// # Returns
///
/// A formatted string with tags organized by format family
fn format_tags_by_group(mut tags: Vec<TagEntry>) -> String {
    // Sort tags by format family, then by name
    tags.sort_by(|a, b| {
        a.format_family
            .cmp(&b.format_family)
            .then(a.name.cmp(&b.name))
    });

    // Group tags by format family
    let mut grouped: HashMap<String, Vec<&TagEntry>> = HashMap::new();
    for tag in &tags {
        grouped
            .entry(tag.format_family.clone())
            .or_insert_with(Vec::new)
            .push(tag);
    }

    // Format output
    let mut output = format!(
        "Found {} tags across {} format families:\n\n",
        tags.len(),
        grouped.len()
    );

    // Sort format families for consistent output
    let mut families: Vec<String> = grouped.keys().cloned().collect();
    families.sort();

    for family in families {
        if let Some(family_tags) = grouped.get(&family) {
            output.push_str(&format!(
                "━━━ {} ({} tags) ━━━\n",
                family,
                family_tags.len()
            ));

            for tag in family_tags {
                let writable_marker = if tag.writable { "✓" } else { "✗" };
                output.push_str(&format!(
                    "  {} {} [{}]\n      {}\n",
                    writable_marker, tag.name, tag.type_name, tag.description
                ));
            }
            output.push('\n');
        }
    }

    output.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_all_tags() {
        // Test listing all tags without filters
        let params = ListTagsParams {
            group: None,
            format: None,
            writable: None,
            search: None,
        };

        let tags = collect_all_tags(&params).expect("Should collect tags");
        assert!(!tags.is_empty(), "Should find tags in database");
    }

    #[test]
    fn test_filter_by_group() {
        // Test filtering by EXIF group
        let params = ListTagsParams {
            group: Some("EXIF".to_string()),
            format: None,
            writable: None,
            search: None,
        };

        let tags = collect_all_tags(&params).expect("Should collect tags");
        assert!(!tags.is_empty(), "Should find EXIF tags");

        // All tags should be EXIF
        for tag in tags {
            assert_eq!(tag.format_family, "EXIF", "All tags should be EXIF family");
        }
    }

    #[test]
    fn test_filter_writable_only() {
        // Test filtering only writable tags
        let params = ListTagsParams {
            group: None,
            format: None,
            writable: Some(true),
            search: None,
        };

        let tags = collect_all_tags(&params).expect("Should collect tags");
        assert!(!tags.is_empty(), "Should find writable tags");

        // All tags should be writable
        for tag in tags {
            assert!(tag.writable, "All tags should be writable");
        }
    }

    #[test]
    fn test_search_by_name() {
        // Test searching for tags containing "Make"
        let params = ListTagsParams {
            group: None,
            format: None,
            writable: None,
            search: Some("Make".to_string()),
        };

        let tags = collect_all_tags(&params).expect("Should collect tags");
        assert!(!tags.is_empty(), "Should find tags matching 'Make'");

        // All tags should contain "Make" in name or description
        for tag in &tags {
            let has_in_name = tag.name.to_lowercase().contains("make");
            let has_in_desc = tag.description.to_lowercase().contains("make");
            assert!(
                has_in_name || has_in_desc,
                "Tag {} should contain 'make' in name or description",
                tag.name
            );
        }
    }

    #[test]
    fn test_combined_filters() {
        // Test combining multiple filters
        let params = ListTagsParams {
            group: Some("EXIF".to_string()),
            format: None,
            writable: Some(true),
            search: Some("Date".to_string()),
        };

        let tags = collect_all_tags(&params).expect("Should collect tags");

        // Tags should match all criteria
        for tag in tags {
            assert_eq!(tag.format_family, "EXIF", "Should be EXIF family");
            assert!(tag.writable, "Should be writable");
            let has_date = tag.name.to_lowercase().contains("date")
                || tag.description.to_lowercase().contains("date");
            assert!(has_date, "Should contain 'date'");
        }
    }

    #[test]
    fn test_format_output() {
        let tags = vec![
            TagEntry {
                name: "EXIF:Make".to_string(),
                description: "Camera manufacturer".to_string(),
                writable: true,
                type_name: "string".to_string(),
                format_family: "EXIF".to_string(),
            },
            TagEntry {
                name: "GPS:Latitude".to_string(),
                description: "GPS latitude coordinate".to_string(),
                writable: true,
                type_name: "rational64u".to_string(),
                format_family: "GPS".to_string(),
            },
        ];

        let output = format_tags_by_group(tags);

        // Check that output contains expected elements
        assert!(output.contains("Found 2 tags"));
        assert!(output.contains("EXIF"));
        assert!(output.contains("GPS"));
        assert!(output.contains("EXIF:Make"));
        assert!(output.contains("GPS:Latitude"));
        assert!(output.contains("Camera manufacturer"));
        assert!(output.contains("GPS latitude coordinate"));
    }
}
