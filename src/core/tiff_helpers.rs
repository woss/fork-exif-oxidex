//! TIFF metadata parsing helpers
//!
//! This module contains helper functions for parsing TIFF IFD structures,
//! processing tags, and handling sub-IFDs (EXIF, GPS) and MakerNotes.

use super::{FileReader, MetadataMap, TagValue};
use crate::core::operations_helpers::read_u32;
use crate::core::tag_conversion::raw_bytes_to_tag_value;
use crate::parsers::tiff::ifd_parser::{parse_ifd, ByteOrder};
use crate::parsers::tiff::makernotes::canon;
use crate::tag_db::lookup_tag_name;
use std::collections::HashMap;

/// Parses a chain of IFDs in a TIFF file.
///
/// TIFF files can contain multiple IFDs linked together. This function
/// traverses the chain and processes each IFD.
///
/// # Arguments
///
/// * `reader` - File reader providing access to the TIFF file
/// * `first_offset` - Offset to the first IFD
/// * `byte_order` - Byte order for interpreting multi-byte values
/// * `metadata` - MetadataMap to populate
pub fn parse_ifd_chain(
    reader: &dyn FileReader,
    first_offset: u64,
    byte_order: ByteOrder,
    metadata: &mut MetadataMap,
) -> crate::error::Result<()> {
    let mut ifd_offset = first_offset;
    let mut ifd_index = 0;

    while ifd_offset != 0 {
        // Determine IFD name based on index
        let ifd_name = get_ifd_name(ifd_index);

        // Parse this IFD
        let tags = parse_ifd(reader, ifd_offset, byte_order)?;

        // Process IFD tags and get sub-IFD information
        let (exif_offset, gps_offset, makernote_data) =
            process_tiff_ifd_tags(&tags, ifd_name, byte_order, metadata);

        // Parse EXIF Sub-IFD if present
        if let Some(offset) = exif_offset {
            parse_exif_subifd(reader, offset, byte_order, metadata);
        }

        // Parse GPS Sub-IFD if present
        if let Some(offset) = gps_offset {
            parse_gps_subifd(reader, offset, byte_order, metadata);
        }

        // Parse Canon MakerNote if present
        if let Some(makernote_bytes) = makernote_data {
            parse_makernote_if_canon(makernote_bytes, byte_order, metadata);
        }

        // Read next IFD offset
        let entry_count = tags.len();
        let next_offset_location = ifd_offset + 2 + (entry_count as u64 * 12);

        if next_offset_location + 4 > reader.size() {
            // Can't read next offset, end of chain
            break;
        }

        let next_offset_bytes = reader.read(next_offset_location, 4)?;
        ifd_offset = read_u32(next_offset_bytes, byte_order) as u64;
        ifd_index += 1;

        // Safety check: prevent infinite loops
        if ifd_index > 10 {
            eprintln!("Warning: More than 10 IFDs found, stopping to prevent infinite loop");
            break;
        }
    }

    Ok(())
}

/// Gets the canonical IFD name for a given index.
///
/// # Arguments
///
/// * `index` - Zero-based IFD index
///
/// # Returns
///
/// The IFD name (e.g., "IFD0", "IFD1", "IFD2", "IFD3")
fn get_ifd_name(index: usize) -> &'static str {
    match index {
        0 => "IFD0",
        1 => "IFD1",
        2 => "IFD2",
        3 => "IFD3",
        n => {
            // For IFD4 and beyond, just use IFDX format
            eprintln!("Warning: Found IFD{} which is unusual", n);
            "IFD0" // Fallback to IFD0 for unusual cases
        }
    }
}

/// Processes tags from a TIFF IFD.
///
/// Extracts tags and identifies special pointers (EXIF sub-IFD, GPS sub-IFD,
/// MakerNote, ICC profile).
///
/// # Arguments
///
/// * `tags` - Parsed IFD tags
/// * `ifd_name` - Name of the IFD (e.g., "IFD0")
/// * `byte_order` - Byte order for interpreting multi-byte values
/// * `metadata` - MetadataMap to populate
///
/// # Returns
///
/// A tuple of (exif_offset, gps_offset, makernote_data)
fn process_tiff_ifd_tags<'a>(
    tags: &'a [(u16, u16, u32, std::borrow::Cow<[u8]>)],
    ifd_name: &str,
    byte_order: ByteOrder,
    metadata: &mut MetadataMap,
) -> (Option<u64>, Option<u64>, Option<&'a [u8]>) {
    let mut exif_ifd_offset = None;
    let mut gps_ifd_offset = None;
    let mut makernote_data: Option<&[u8]> = None;

    // Convert tags to metadata
    for (tag_id, field_type, value_count, raw_bytes) in tags {
        // Convert Cow<[u8]> to &[u8] for processing
        let bytes = raw_bytes.as_ref();

        // Check for EXIF Sub-IFD pointer (tag 0x8769)
        if *tag_id == 0x8769 && bytes.len() >= 4 {
            let offset = read_u32(bytes, byte_order);
            exif_ifd_offset = Some(offset as u64);
            continue; // Don't add the pointer tag to metadata
        }

        // Check for GPS Sub-IFD pointer (tag 0x8825)
        if *tag_id == 0x8825 && bytes.len() >= 4 {
            let offset = read_u32(bytes, byte_order);
            gps_ifd_offset = Some(offset as u64);
            continue; // Don't add the pointer tag to metadata
        }

        // Check for MakerNote tag (0x927C)
        // Store the data for later processing after we've added other tags
        if *tag_id == 0x927C {
            makernote_data = Some(bytes);
            // Note: We don't continue here - we still add the raw MakerNote tag
            // to metadata so tools can see it, but we'll also parse it below
        }

        // Check for ICC Profile tag (0x8773)
        if *tag_id == 0x8773 && bytes.len() >= 128 {
            // Parse ICC profile data
            match crate::parsers::icc::parse_icc_profile_data(bytes) {
                Ok(icc_tags) => {
                    // Add all ICC tags to metadata with "Profile:" prefix
                    for (tag_name, value) in icc_tags {
                        metadata.insert(format!("Profile:{}", tag_name), value);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse ICC profile in TIFF: {}", e);
                }
            }
            // Don't continue - still add the raw ICC_Profile tag
        }

        // Convert tag to metadata
        let tag_name = lookup_tag_name(*tag_id, ifd_name);
        let tag_value =
            raw_bytes_to_tag_value(bytes, *field_type, *value_count, *tag_id, byte_order);
        metadata.insert(tag_name, tag_value);
    }

    (exif_ifd_offset, gps_ifd_offset, makernote_data)
}

/// Parses an EXIF sub-IFD and extracts tags.
///
/// The EXIF sub-IFD contains detailed camera settings and shooting parameters.
/// It may also contain MakerNote data specific to camera manufacturers.
///
/// # Arguments
///
/// * `reader` - File reader providing access to the file
/// * `offset` - Offset to the EXIF sub-IFD
/// * `byte_order` - Byte order for interpreting multi-byte values
/// * `metadata` - MetadataMap to populate
pub fn parse_exif_subifd(
    reader: &dyn FileReader,
    offset: u64,
    byte_order: ByteOrder,
    metadata: &mut MetadataMap,
) {
    if let Ok(exif_tags) = parse_ifd(reader, offset, byte_order) {
        // Track MakerNote in EXIF IFD
        let mut exif_makernote_data: Option<&[u8]> = None;

        // First pass: convert tags and capture MakerNote
        for (tag_id, field_type, value_count, raw_bytes) in &exif_tags {
            // Convert Cow<[u8]> to &[u8] for processing
            let bytes = raw_bytes.as_ref();

            // Check for MakerNote in EXIF IFD
            if *tag_id == 0x927C {
                exif_makernote_data = Some(bytes);
            }

            let tag_name = lookup_tag_name(*tag_id, "ExifIFD");
            let tag_value =
                raw_bytes_to_tag_value(bytes, *field_type, *value_count, *tag_id, byte_order);
            metadata.insert(tag_name, tag_value);
        }

        // Second pass: Parse Canon MakerNote if found in EXIF IFD
        if let Some(makernote_bytes) = exif_makernote_data {
            parse_makernote_if_canon(makernote_bytes, byte_order, metadata);
        }
    }
}

/// Parses a GPS sub-IFD and extracts GPS tags.
///
/// The GPS sub-IFD contains GPS positioning information including
/// latitude, longitude, altitude, and timestamp.
///
/// # Arguments
///
/// * `reader` - File reader providing access to the file
/// * `offset` - Offset to the GPS sub-IFD
/// * `byte_order` - Byte order for interpreting multi-byte values
/// * `metadata` - MetadataMap to populate
pub fn parse_gps_subifd(
    reader: &dyn FileReader,
    offset: u64,
    byte_order: ByteOrder,
    metadata: &mut MetadataMap,
) {
    if let Ok(gps_tags) = parse_ifd(reader, offset, byte_order) {
        for (tag_id, field_type, value_count, raw_bytes) in gps_tags {
            let tag_name = lookup_tag_name(tag_id, "GPS");
            let tag_value = raw_bytes_to_tag_value(
                raw_bytes.as_ref(),
                field_type,
                value_count,
                tag_id,
                byte_order,
            );
            metadata.insert(tag_name, tag_value);
        }
    }
}

/// Parses Canon MakerNote data if present.
///
/// Canon cameras store proprietary metadata in MakerNote tags.
/// This function detects and parses Canon-specific formats.
///
/// # Arguments
///
/// * `makernote_data` - Raw MakerNote bytes
/// * `byte_order` - Byte order for interpreting multi-byte values
/// * `metadata` - MetadataMap to populate with Canon tags
fn parse_makernote_if_canon(
    makernote_data: &[u8],
    byte_order: ByteOrder,
    metadata: &mut MetadataMap,
) {
    // Check if this is a Canon MakerNote
    if canon::is_canon_makernote(makernote_data) {
        // Parse Canon MakerNote tags
        let mut canon_tags = HashMap::new();
        canon::parse_canon_makernotes(makernote_data, byte_order, &mut canon_tags);
        // Add Canon tags to metadata
        // Note: tag names already include "Canon:" prefix from canon_tag_to_name()
        for (tag_name, tag_value_str) in canon_tags {
            // Convert string value to TagValue
            let tag_value = TagValue::String(tag_value_str);
            metadata.insert(tag_name, tag_value);
        }
    }
    // If not Canon, silently ignore - other vendors' MakerNotes
    // can be added in future phases (Nikon, Sony, etc.)
}
