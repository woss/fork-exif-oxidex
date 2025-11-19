//! List all metadata tag groups with counts
//!
//! This module provides functionality to list all available tag groups
//! (EXIF, XMP, IPTC, GPS, etc.) with tag counts and format support information.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// Parameters for the list_tag_groups tool
#[derive(Debug, Deserialize)]
struct TagGroupsParams {
    /// Optional filter by file format (currently not fully implemented)
    format: Option<String>,
}

/// Information about a tag group
#[derive(Debug, Clone)]
struct TagGroupInfo {
    name: String,
    tag_count: usize,
    description: String,
    supported_formats: Vec<String>,
}

/// Main handler for the list_tag_groups tool
///
/// Lists all metadata tag groups (format families) with tag counts and
/// information about which file formats support each group.
///
/// # Arguments
///
/// * `arguments` - JSON arguments (currently only format filter, not fully implemented)
///
/// # Returns
///
/// A formatted string listing all tag groups with their metadata
pub async fn handle(arguments: Value) -> Result<String> {
    let _params: TagGroupsParams =
        serde_json::from_value(arguments).context("Invalid arguments for list_tag_groups")?;

    // Collect information about all tag groups
    let groups = collect_tag_groups()?;

    if groups.is_empty() {
        return Ok("No tag groups found.".to_string());
    }

    // Format and return the output
    Ok(format_tag_groups(groups))
}

/// Collect information about all tag groups
///
/// Scans through all domain-specific tag databases and groups tags by their
/// format family, counting tags in each group.
///
/// # Returns
///
/// A vector of TagGroupInfo structs containing information about each group
fn collect_tag_groups() -> Result<Vec<TagGroupInfo>> {
    let mut group_counts: HashMap<String, usize> = HashMap::new();

    // Helper function to extract format family from table name
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
        } else if table_name.starts_with("FLAC::") {
            "FLAC"
        } else if table_name.starts_with("Ogg::") {
            "Ogg"
        } else if table_name.starts_with("RIFF::") {
            "RIFF"
        } else if table_name.contains("Canon") {
            "Canon"
        } else if table_name.contains("Nikon") {
            "Nikon"
        } else if table_name.contains("Sony") {
            "Sony"
        } else if table_name.contains("Olympus") {
            "Olympus"
        } else if table_name.contains("Panasonic") {
            "Panasonic"
        } else if table_name.contains("Fujifilm") {
            "Fujifilm"
        } else if table_name.contains("Pentax") {
            "Pentax"
        } else if table_name.contains("Leica") {
            "Leica"
        } else {
            "Other"
        }
    }

    // Helper function to count tags in a database
    let mut count_tags = |db: &oxidex::tag_db::TagDatabase| {
        for table in &db.tables {
            let family = get_format_family(&table.name);
            *group_counts.entry(family.to_string()).or_insert(0) += table.tags.len();
        }
    };

    // Count tags in all domain-specific databases
    count_tags(&oxidex::tag_db::core::CORE_TAGS);
    count_tags(&oxidex::tag_db::camera::CAMERA_TAGS);
    count_tags(&oxidex::tag_db::media::MEDIA_TAGS);
    count_tags(&oxidex::tag_db::image::IMAGE_TAGS);
    count_tags(&oxidex::tag_db::document::DOCUMENT_TAGS);
    count_tags(&oxidex::tag_db::specialty::SPECIALTY_TAGS);

    // Convert counts to TagGroupInfo structs with descriptions
    let mut groups: Vec<TagGroupInfo> = group_counts
        .into_iter()
        .map(|(name, count)| {
            let (description, supported_formats) = get_group_metadata(&name);
            TagGroupInfo {
                name,
                tag_count: count,
                description,
                supported_formats,
            }
        })
        .collect();

    // Sort by tag count (descending) for better readability
    groups.sort_by(|a, b| b.tag_count.cmp(&a.tag_count));

    Ok(groups)
}

/// Get descriptive metadata for a tag group
///
/// Returns a human-readable description and list of supported file formats
/// for each tag group.
///
/// # Arguments
///
/// * `group_name` - Name of the tag group
///
/// # Returns
///
/// A tuple of (description, supported_formats)
fn get_group_metadata(group_name: &str) -> (String, Vec<String>) {
    match group_name {
        "EXIF" => (
            "Exchangeable Image File Format - Standard camera and image metadata".to_string(),
            vec![
                "JPEG".to_string(),
                "TIFF".to_string(),
                "RAW".to_string(),
                "DNG".to_string(),
                "CR2".to_string(),
                "NEF".to_string(),
                "ARW".to_string(),
            ],
        ),
        "GPS" => (
            "GPS metadata - Geographic location information".to_string(),
            vec![
                "JPEG".to_string(),
                "TIFF".to_string(),
                "RAW formats".to_string(),
            ],
        ),
        "XMP" => (
            "Adobe Extensible Metadata Platform - Structured metadata standard".to_string(),
            vec![
                "JPEG".to_string(),
                "PNG".to_string(),
                "PDF".to_string(),
                "TIFF".to_string(),
                "PSD".to_string(),
                "AI".to_string(),
            ],
        ),
        "IPTC" => (
            "International Press Telecommunications Council - News and media metadata".to_string(),
            vec!["JPEG".to_string(), "TIFF".to_string()],
        ),
        "ICC_Profile" => (
            "ICC Color Profile - Color management metadata".to_string(),
            vec![
                "JPEG".to_string(),
                "PNG".to_string(),
                "TIFF".to_string(),
                "PDF".to_string(),
            ],
        ),
        "Photoshop" => (
            "Adobe Photoshop metadata - Photoshop-specific tags".to_string(),
            vec![
                "PSD".to_string(),
                "JPEG".to_string(),
                "TIFF".to_string(),
            ],
        ),
        "JFIF" => (
            "JPEG File Interchange Format - JPEG container metadata".to_string(),
            vec!["JPEG".to_string()],
        ),
        "PNG" => (
            "Portable Network Graphics metadata".to_string(),
            vec!["PNG".to_string()],
        ),
        "PDF" => (
            "Portable Document Format metadata".to_string(),
            vec!["PDF".to_string()],
        ),
        "QuickTime" => (
            "QuickTime/MOV multimedia metadata".to_string(),
            vec![
                "MOV".to_string(),
                "MP4".to_string(),
                "M4V".to_string(),
                "M4A".to_string(),
            ],
        ),
        "FLAC" => (
            "Free Lossless Audio Codec metadata".to_string(),
            vec!["FLAC".to_string()],
        ),
        "Ogg" => (
            "Ogg Vorbis audio metadata".to_string(),
            vec!["OGG".to_string(), "OGA".to_string()],
        ),
        "RIFF" => (
            "Resource Interchange File Format - WAV and AVI metadata".to_string(),
            vec!["WAV".to_string(), "AVI".to_string()],
        ),
        "Canon" => (
            "Canon camera maker notes - Canon-specific camera settings".to_string(),
            vec!["Canon RAW".to_string(), "CR2".to_string(), "CR3".to_string()],
        ),
        "Nikon" => (
            "Nikon camera maker notes - Nikon-specific camera settings".to_string(),
            vec!["Nikon RAW".to_string(), "NEF".to_string(), "NRW".to_string()],
        ),
        "Sony" => (
            "Sony camera maker notes - Sony-specific camera settings".to_string(),
            vec!["Sony RAW".to_string(), "ARW".to_string(), "SR2".to_string()],
        ),
        "Olympus" => (
            "Olympus camera maker notes - Olympus-specific camera settings".to_string(),
            vec!["Olympus RAW".to_string(), "ORF".to_string()],
        ),
        "Panasonic" => (
            "Panasonic camera maker notes - Panasonic-specific camera settings".to_string(),
            vec!["Panasonic RAW".to_string(), "RW2".to_string()],
        ),
        "Fujifilm" => (
            "Fujifilm camera maker notes - Fujifilm-specific camera settings".to_string(),
            vec!["Fujifilm RAW".to_string(), "RAF".to_string()],
        ),
        "Pentax" => (
            "Pentax camera maker notes - Pentax-specific camera settings".to_string(),
            vec!["Pentax RAW".to_string(), "PEF".to_string(), "DNG".to_string()],
        ),
        "Leica" => (
            "Leica camera maker notes - Leica-specific camera settings".to_string(),
            vec!["Leica RAW".to_string(), "DNG".to_string()],
        ),
        _ => (
            "Other metadata tags".to_string(),
            vec!["Various".to_string()],
        ),
    }
}

/// Format tag groups for human-readable output
///
/// Creates a formatted listing of all tag groups with their metadata.
///
/// # Arguments
///
/// * `groups` - Vector of tag group information
///
/// # Returns
///
/// A formatted string listing all tag groups
fn format_tag_groups(groups: Vec<TagGroupInfo>) -> String {
    let mut output = String::new();

    let total_tags: usize = groups.iter().map(|g| g.tag_count).sum();
    output.push_str(&format!(
        "Metadata Tag Groups ({} groups, {} total tags):\n\n",
        groups.len(),
        total_tags
    ));

    for group in groups {
        output.push_str(&format!("━━━ {} ({} tags) ━━━\n", group.name, group.tag_count));
        output.push_str(&format!("  Description: {}\n", group.description));
        output.push_str(&format!(
            "  Supported Formats: {}\n",
            group.supported_formats.join(", ")
        ));
        output.push('\n');
    }

    output.push_str("Usage Examples:\n");
    output.push_str("  list_tags --group EXIF           # List all EXIF tags\n");
    output.push_str("  list_tags --group GPS            # List all GPS tags\n");
    output.push_str("  get_tag_info --tag EXIF:Make     # Get details about a specific tag\n");

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_tag_groups() {
        let groups = collect_tag_groups().expect("Should collect tag groups");
        assert!(!groups.is_empty(), "Should find at least one tag group");

        // Check for expected major groups
        let group_names: Vec<&str> = groups.iter().map(|g| g.name.as_str()).collect();
        assert!(
            group_names.contains(&"EXIF"),
            "Should include EXIF group"
        );
    }

    #[test]
    fn test_group_has_tags() {
        let groups = collect_tag_groups().expect("Should collect tag groups");

        for group in groups {
            assert!(
                group.tag_count > 0,
                "Group {} should have at least one tag",
                group.name
            );
            assert!(
                !group.description.is_empty(),
                "Group {} should have a description",
                group.name
            );
            assert!(
                !group.supported_formats.is_empty(),
                "Group {} should have supported formats",
                group.name
            );
        }
    }

    #[test]
    fn test_exif_group_metadata() {
        let (desc, formats) = get_group_metadata("EXIF");
        assert!(desc.contains("Exchangeable Image"));
        assert!(!formats.is_empty());
        assert!(formats.contains(&"JPEG".to_string()));
    }

    #[test]
    fn test_gps_group_metadata() {
        let (desc, formats) = get_group_metadata("GPS");
        assert!(desc.contains("GPS"));
        assert!(!formats.is_empty());
    }

    #[test]
    fn test_xmp_group_metadata() {
        let (desc, formats) = get_group_metadata("XMP");
        assert!(desc.contains("Extensible Metadata"));
        assert!(formats.contains(&"PNG".to_string()));
    }

    #[test]
    fn test_format_output() {
        let groups = vec![
            TagGroupInfo {
                name: "EXIF".to_string(),
                tag_count: 150,
                description: "Standard camera metadata".to_string(),
                supported_formats: vec!["JPEG".to_string(), "TIFF".to_string()],
            },
            TagGroupInfo {
                name: "GPS".to_string(),
                tag_count: 30,
                description: "Location metadata".to_string(),
                supported_formats: vec!["JPEG".to_string()],
            },
        ];

        let output = format_tag_groups(groups);

        // Check that output contains expected elements
        assert!(output.contains("180 total tags"));
        assert!(output.contains("EXIF (150 tags)"));
        assert!(output.contains("GPS (30 tags)"));
        assert!(output.contains("Standard camera metadata"));
        assert!(output.contains("Location metadata"));
        assert!(output.contains("JPEG"));
    }

    #[test]
    fn test_groups_sorted_by_count() {
        let groups = collect_tag_groups().expect("Should collect tag groups");

        // Verify groups are sorted by tag count (descending)
        for i in 1..groups.len() {
            assert!(
                groups[i - 1].tag_count >= groups[i].tag_count,
                "Groups should be sorted by tag count descending"
            );
        }
    }
}
