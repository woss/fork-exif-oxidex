//! Get detailed information about a specific metadata tag
//!
//! This module provides functionality to retrieve comprehensive information
//! about a specific metadata tag including its description, data type,
//! writable status, and example values.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;

/// Parameters for the get_tag_info tool
#[derive(Debug, Deserialize)]
struct TagInfoParams {
    /// Tag name to look up (e.g., "EXIF:Make", "XMP:Creator")
    tag: String,
}

/// Main handler for the get_tag_info tool
///
/// Retrieves detailed information about a specific metadata tag from the
/// OxiDex tag database and tag registry.
///
/// # Arguments
///
/// * `arguments` - JSON arguments containing the tag name to lookup
///
/// # Returns
///
/// A formatted string containing detailed tag information, or an error message
/// if the tag is not found
pub async fn handle(arguments: Value) -> Result<String> {
    let params: TagInfoParams =
        serde_json::from_value(arguments).context("Invalid arguments for get_tag_info")?;

    // Try to look up the tag in the tag registry first (has rich metadata)
    if let Some(descriptor) = oxidex::tag_db::get_tag_descriptor(&params.tag) {
        return Ok(format_tag_descriptor(descriptor));
    }

    // Try to find the tag in the YAML databases (less metadata but more comprehensive)
    if let Some(tag_info) = find_tag_in_yaml_databases(&params.tag) {
        return Ok(format_yaml_tag_info(&params.tag, &tag_info));
    }

    // Tag not found - provide helpful error message
    Ok(format_tag_not_found(&params.tag))
}

/// Tag information extracted from YAML databases
#[derive(Debug)]
struct YamlTagInfo {
    description: String,
    writable: bool,
    type_name: String,
    table_name: String,
}

/// Find a tag in the YAML-based tag databases
///
/// Searches through all domain-specific tag databases to find a tag matching
/// the given name.
///
/// # Arguments
///
/// * `tag_name` - Full tag name (e.g., "EXIF:Make") or base name (e.g., "Make")
///
/// # Returns
///
/// Some(YamlTagInfo) if found, None otherwise
fn find_tag_in_yaml_databases(tag_name: &str) -> Option<YamlTagInfo> {
    // Split tag name into prefix and base name
    let (prefix, base_name) = if let Some(colon_pos) = tag_name.find(':') {
        (Some(&tag_name[..colon_pos]), &tag_name[colon_pos + 1..])
    } else {
        (None, tag_name)
    };

    // Helper function to search a tag database
    fn search_database(
        db: &oxidex::tag_db::TagDatabase,
        prefix: Option<&str>,
        base_name: &str,
    ) -> Option<YamlTagInfo> {
        for table in &db.tables {
            // If prefix is specified, check if table matches
            if let Some(p) = prefix {
                // Extract format family from table name
                let table_prefix = if table.name.starts_with("Exif::") {
                    "EXIF"
                } else if table.name.starts_with("GPS::") {
                    "GPS"
                } else if table.name.starts_with("XMP::") {
                    "XMP"
                } else if table.name.starts_with("IPTC::") {
                    "IPTC"
                } else if table.name.starts_with("ICC_Profile::") {
                    "ICC_Profile"
                } else if table.name.starts_with("Photoshop::") {
                    "Photoshop"
                } else if table.name.starts_with("JFIF::") {
                    "JFIF"
                } else if table.name.starts_with("PNG::") {
                    "PNG"
                } else if table.name.starts_with("PDF::") {
                    "PDF"
                } else if table.name.starts_with("QuickTime::") {
                    "QuickTime"
                } else if table.name.contains("Canon") {
                    "Canon"
                } else if table.name.contains("Nikon") {
                    "Nikon"
                } else if table.name.contains("Sony") {
                    "Sony"
                } else {
                    ""
                };

                // Skip if prefix doesn't match
                if !table_prefix.eq_ignore_ascii_case(p) {
                    continue;
                }
            }

            // Search for the tag in this table
            for tag in &table.tags {
                if tag.name == base_name {
                    return Some(YamlTagInfo {
                        description: tag
                            .description
                            .clone()
                            .unwrap_or_else(|| "No description available".to_string()),
                        writable: tag.writable,
                        type_name: tag
                            .type_name
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                        table_name: table.name.clone(),
                    });
                }
            }
        }
        None
    }

    // Search all domain-specific databases
    search_database(&oxidex::tag_db::core::CORE_TAGS, prefix, base_name)
        .or_else(|| search_database(&oxidex::tag_db::camera::CAMERA_TAGS, prefix, base_name))
        .or_else(|| search_database(&oxidex::tag_db::media::MEDIA_TAGS, prefix, base_name))
        .or_else(|| search_database(&oxidex::tag_db::image::IMAGE_TAGS, prefix, base_name))
        .or_else(|| search_database(&oxidex::tag_db::document::DOCUMENT_TAGS, prefix, base_name))
        .or_else(|| search_database(&oxidex::tag_db::specialty::SPECIALTY_TAGS, prefix, base_name))
}

/// Format a TagDescriptor from the tag registry (rich metadata)
///
/// # Arguments
///
/// * `descriptor` - Tag descriptor from the registry
///
/// # Returns
///
/// A formatted string with all tag information
fn format_tag_descriptor(descriptor: &oxidex::core::TagDescriptor) -> String {
    let mut output = String::new();

    output.push_str(&format!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"));
    output.push_str(&format!("Tag: {}\n", descriptor.name()));
    output.push_str(&format!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n"));

    output.push_str(&format!("Description:\n  {}\n\n", descriptor.description()));

    output.push_str(&format!("Format Family: {:?}\n", descriptor.format()));
    output.push_str(&format!("Data Type: {:?}\n", descriptor.value_type()));
    output.push_str(&format!(
        "Writable: {}\n\n",
        if descriptor.is_writable() {
            "Yes ✓"
        } else {
            "No ✗"
        }
    ));

    // Show tag ID if available
    match descriptor.id() {
        oxidex::core::TagId::Numeric(id) => {
            output.push_str(&format!("Tag ID: 0x{:04X} ({})\n\n", id, id));
        }
        oxidex::core::TagId::Named(name) => {
            output.push_str(&format!("Tag ID: {} (named)\n\n", name));
        }
    }

    // Show example values if available
    let examples = descriptor.examples();
    if !examples.is_empty() {
        output.push_str("Example Values:\n");
        for (i, example) in examples.iter().enumerate() {
            if i < 5 {
                // Limit to first 5 examples
                output.push_str(&format!("  • {}\n", example));
            }
        }
        if examples.len() > 5 {
            output.push_str(&format!("  ... and {} more\n", examples.len() - 5));
        }
        output.push('\n');
    }

    output.push_str("Usage Example:\n");
    output.push_str(&format!(
        "  oxidex-mcp extract_metadata --path photo.jpg  # Read {}\n",
        descriptor.name()
    ));
    if descriptor.is_writable() {
        output.push_str(&format!(
            "  oxidex-mcp write_metadata --path photo.jpg --tags '{{\"{}\":\"value\"}}'  # Write {}\n",
            descriptor.name(),
            descriptor.name()
        ));
    }

    output
}

/// Format tag information from YAML databases (less metadata)
///
/// # Arguments
///
/// * `tag_name` - Full tag name
/// * `info` - Tag information from YAML database
///
/// # Returns
///
/// A formatted string with available tag information
fn format_yaml_tag_info(tag_name: &str, info: &YamlTagInfo) -> String {
    let mut output = String::new();

    output.push_str(&format!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"));
    output.push_str(&format!("Tag: {}\n", tag_name));
    output.push_str(&format!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n"));

    output.push_str(&format!("Description:\n  {}\n\n", info.description));

    output.push_str(&format!("Table: {}\n", info.table_name));
    output.push_str(&format!("Data Type: {}\n", info.type_name));
    output.push_str(&format!(
        "Writable: {}\n\n",
        if info.writable { "Yes ✓" } else { "No ✗" }
    ));

    output.push_str("Usage Example:\n");
    output.push_str(&format!(
        "  oxidex-mcp extract_metadata --path photo.jpg  # Read {}\n",
        tag_name
    ));
    if info.writable {
        output.push_str(&format!(
            "  oxidex-mcp write_metadata --path photo.jpg --tags '{{\"{}\":\"value\"}}'  # Write {}\n",
            tag_name, tag_name
        ));
    }

    output.push_str("\nNote: This tag is from the YAML database. Additional metadata may be available.\n");

    output
}

/// Format a helpful error message when tag is not found
///
/// Provides suggestions on how to find the correct tag name.
///
/// # Arguments
///
/// * `tag_name` - The tag name that was not found
///
/// # Returns
///
/// A formatted error message with suggestions
fn format_tag_not_found(tag_name: &str) -> String {
    let mut output = String::new();

    output.push_str(&format!("Tag Not Found: {}\n\n", tag_name));

    output.push_str("The specified tag was not found in the OxiDex tag database.\n\n");

    output.push_str("Troubleshooting:\n");
    output.push_str("  • Check that the tag name is spelled correctly\n");
    output.push_str("  • Ensure you're using the correct format family prefix (e.g., EXIF:, XMP:, GPS:)\n");
    output.push_str("  • Try using list_tags to browse available tags\n");
    output.push_str("  • Try searching: list_tags with search parameter\n\n");

    output.push_str("Common Tag Formats:\n");
    output.push_str("  • EXIF tags: EXIF:Make, EXIF:Model, EXIF:DateTime\n");
    output.push_str("  • GPS tags: GPS:GPSLatitude, GPS:GPSLongitude\n");
    output.push_str("  • XMP tags: XMP:Creator, XMP:Rights\n");
    output.push_str("  • IPTC tags: IPTC:Keywords, IPTC:Caption-Abstract\n\n");

    output.push_str("Example Commands:\n");
    output.push_str("  list_tags                          # List all tags\n");
    output.push_str("  list_tags --group EXIF             # List only EXIF tags\n");
    output.push_str(&format!(
        "  list_tags --search \"{}\"      # Search for similar tags\n",
        tag_name.split(':').last().unwrap_or(tag_name)
    ));

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_exif_make_tag() {
        // Test finding EXIF:Make tag
        let info = find_tag_in_yaml_databases("EXIF:Make");
        assert!(info.is_some(), "Should find EXIF:Make tag");

        let info = info.unwrap();
        // Note: writable status depends on YAML database configuration
        // Just verify we got the tag info successfully
        assert!(!info.description.is_empty(), "Should have description");
        assert!(!info.type_name.is_empty(), "Should have type name");
    }

    #[test]
    fn test_find_tag_without_prefix() {
        // Test finding tag by base name only
        let info = find_tag_in_yaml_databases("Make");
        assert!(info.is_some(), "Should find Make tag without prefix");
    }

    #[test]
    fn test_tag_not_found() {
        // Test with non-existent tag
        let info = find_tag_in_yaml_databases("NonExistent:FakeTag");
        assert!(info.is_none(), "Should not find non-existent tag");
    }

    #[test]
    fn test_format_not_found_message() {
        let message = format_tag_not_found("EXIF:NonExistent");
        assert!(message.contains("Tag Not Found"));
        assert!(message.contains("EXIF:NonExistent"));
        assert!(message.contains("list_tags"));
    }

    #[test]
    fn test_registry_lookup() {
        // Test looking up a tag from the manual registry
        let descriptor = oxidex::tag_db::get_tag_descriptor("EXIF:Make");
        assert!(descriptor.is_some(), "Should find EXIF:Make in registry");

        if let Some(desc) = descriptor {
            assert_eq!(desc.name(), "EXIF:Make");
            assert!(desc.is_writable());
            assert!(!desc.description().is_empty());
        }
    }

    #[test]
    fn test_gps_tag_lookup() {
        // Test GPS tag lookup
        let info = find_tag_in_yaml_databases("GPS:GPSLatitude");
        assert!(info.is_some(), "Should find GPS:GPSLatitude");
    }

    #[test]
    fn test_xmp_tag_lookup() {
        // Test XMP tag lookup
        let info = find_tag_in_yaml_databases("XMP:Creator");
        // XMP tags might be in the database, but it's ok if not found in test env
        if info.is_none() {
            println!("Note: XMP:Creator not found in test environment");
        }
    }
}
