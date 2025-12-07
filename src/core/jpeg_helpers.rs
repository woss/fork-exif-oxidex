//! JPEG metadata parsing helpers
//!
//! This module contains helper functions for parsing JPEG segment structures
//! and extracting metadata from different segment types (JFIF, EXIF, XMP, IPTC, ICC).

use super::{FileReader, MetadataMap, TagValue};
use crate::core::operations_helpers::read_u32;
use crate::core::tag_conversion::{parse_string_to_tag_value, raw_bytes_to_tag_value};
use crate::core::tiff_helpers::{parse_exif_subifd, parse_gps_subifd};
use crate::io::EndianReader;
use crate::parsers::jpeg::segment_parser::Segment;
use crate::parsers::jpeg::xmp_parser::extract_xmp_from_segments;
use crate::parsers::tiff::ifd_parser::{parse_ifd, ByteOrder};
use crate::parsers::tiff::tiff_subreader::TiffSubReader;
use crate::tag_db::lookup_tag_name;

/// Processes JFIF APP0 segments and extracts version and resolution metadata.
///
/// JFIF segments contain basic image information including version, resolution unit,
/// and X/Y resolution values.
///
/// # Arguments
///
/// * `segments` - Parsed JPEG segments
/// * `metadata` - MetadataMap to populate with JFIF tags
pub fn process_jfif_segments(segments: &[Segment], metadata: &mut MetadataMap) {
    for segment in segments.iter().filter(|s| s.marker == 0xFFE0) {
        // Also try extended APP0 parser for JFXX segments
        let _ = crate::parsers::jpeg::app_parsers::parse_app0_extended(segment.data, metadata);

        // Check if this is a JFIF segment (starts with "JFIF\0")
        if segment.data.len() >= 14 && &segment.data[0..5] == b"JFIF\0" {
            // JFIF structure after identifier:
            // Bytes 5-6: Version (major.minor)
            // Byte 7: Units (0=none, 1=inches, 2=cm)
            // Bytes 8-9: X density (big-endian u16)
            // Bytes 10-11: Y density (big-endian u16)
            let version_major = segment.data[5];
            let version_minor = segment.data[6];
            let units = segment.data[7];

            // JFIF uses big-endian byte order for density values
            let reader = EndianReader::big_endian(segment.data);
            let x_density = reader.u16_at(8).unwrap_or(0);
            let y_density = reader.u16_at(10).unwrap_or(0);

            // Add JFIF tags to metadata
            metadata.insert(
                "JFIF:JFIFVersion".to_string(),
                TagValue::Float(version_major as f64 + version_minor as f64 / 100.0),
            );

            let unit_string = match units {
                0 => "None",
                1 => "inches",
                2 => "cm",
                _ => "Unknown",
            };
            metadata.insert(
                "JFIF:ResolutionUnit".to_string(),
                TagValue::String(unit_string.to_string()),
            );

            metadata.insert(
                "JFIF:XResolution".to_string(),
                TagValue::Integer(x_density as i64),
            );

            metadata.insert(
                "JFIF:YResolution".to_string(),
                TagValue::Integer(y_density as i64),
            );
        }
    }
}

/// Processes EXIF APP1 segments and extracts TIFF-based EXIF metadata.
///
/// EXIF data is stored in APP1 segments with a TIFF structure containing
/// IFD0, EXIF sub-IFD, and GPS sub-IFD.
///
/// # Arguments
///
/// * `segments` - Parsed JPEG segments
/// * `reader` - File reader for accessing full file (needed for offset calculations)
/// * `metadata` - MetadataMap to populate with EXIF tags
pub fn process_exif_segments(
    segments: &[Segment],
    reader: &dyn FileReader,
    metadata: &mut MetadataMap,
) {
    // Find all APP1 segments (EXIF/XMP/FLIR)
    let app1_segments: Vec<_> = segments.iter().filter(|s| s.is_app1()).collect();

    // Process each APP1 segment
    for segment in app1_segments {
        // Check if this is a FLIR segment (starts with "FLIR\0")
        if segment.data.len() >= 5 && &segment.data[0..5] == b"FLIR\0" {
            let _ = crate::parsers::jpeg::flir_parser::parse_flir_segment(segment.data, metadata);
            continue;
        }

        // Check if this is an EXIF segment (starts with "Exif\0\0")
        if segment.data.len() >= 6 && &segment.data[0..6] == b"Exif\0\0" {
            // Extract EXIF data starting after the 6-byte header
            let tiff_data = &segment.data[6..];

            if tiff_data.len() < 8 {
                // EXIF data too small for valid TIFF header
                continue;
            }

            // Detect byte order from TIFF header (bytes 0-1)
            let byte_order = if &tiff_data[0..2] == b"II" {
                ByteOrder::LittleEndian
            } else if &tiff_data[0..2] == b"MM" {
                ByteOrder::BigEndian
            } else {
                // Invalid byte order marker
                continue;
            };

            // Read IFD offset from bytes 4-7 (relative to TIFF data start)
            // Create EndianReader with appropriate byte order for the TIFF data
            let tiff_header_reader = match byte_order {
                ByteOrder::LittleEndian => EndianReader::little_endian(tiff_data),
                ByteOrder::BigEndian => EndianReader::big_endian(tiff_data),
            };
            let ifd_offset = tiff_header_reader.u32_at(4).unwrap_or(0) as u64;

            // Create a sub-reader for TIFF data
            // We need to create a wrapper that adjusts offsets to be relative to TIFF start
            let tiff_offset = segment.offset + 10; // Segment offset + marker(2) + length(2) + "Exif\0\0"(6)
            let tiff_reader = TiffSubReader::new(reader, tiff_offset);

            // Parse IFD structure
            if let Ok(tags) = parse_ifd(&tiff_reader, ifd_offset, byte_order) {
                // Process IFD0 tags and get sub-IFD offsets
                let (exif_ifd_offset, gps_ifd_offset) =
                    process_ifd0_tags(&tags, byte_order, metadata);

                // Parse EXIF Sub-IFD if present
                if let Some(offset) = exif_ifd_offset {
                    parse_exif_subifd(&tiff_reader, offset, byte_order, metadata);
                }

                // Parse GPS Sub-IFD if present
                if let Some(offset) = gps_ifd_offset {
                    parse_gps_subifd(&tiff_reader, offset, byte_order, metadata);
                }
            }
        }
    }
}

/// Processes IFD0 tags from JPEG EXIF data.
///
/// Extracts tags from the main IFD (IFD0) and identifies pointers to
/// EXIF and GPS sub-IFDs for further processing.
///
/// # Arguments
///
/// * `tags` - Parsed IFD tags
/// * `byte_order` - Byte order for interpreting multi-byte values
/// * `metadata` - MetadataMap to populate
///
/// # Returns
///
/// A tuple of (exif_ifd_offset, gps_ifd_offset) for sub-IFD parsing
fn process_ifd0_tags(
    tags: &[(u16, u16, u32, std::borrow::Cow<[u8]>)],
    byte_order: ByteOrder,
    metadata: &mut MetadataMap,
) -> (Option<u64>, Option<u64>) {
    let mut exif_ifd_offset = None;
    let mut gps_ifd_offset = None;

    // Convert raw tag data to MetadataMap entries
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

        // Convert tag ID to tag name (IFD0 for main JPEG EXIF)
        let tag_name = lookup_tag_name(*tag_id, "IFD0");

        // Convert raw bytes to TagValue
        let tag_value =
            raw_bytes_to_tag_value(bytes, *field_type, *value_count, *tag_id, byte_order);

        metadata.insert(tag_name, tag_value);
    }

    (exif_ifd_offset, gps_ifd_offset)
}

/// Processes XMP APP1 segments and extracts XMP metadata.
///
/// XMP (Extensible Metadata Platform) is an XML-based metadata format
/// stored in APP1 segments with "http://ns.adobe.com/xap/1.0/" marker.
///
/// # Arguments
///
/// * `segments` - Parsed JPEG segments
/// * `metadata` - MetadataMap to populate with XMP tags
pub fn process_xmp_segments(segments: &[Segment], metadata: &mut MetadataMap) {
    match extract_xmp_from_segments(segments) {
        Ok(xmp_tags) => {
            // Add all XMP tags to metadata
            for (tag_name, value) in xmp_tags {
                // Try to parse as integer first, then as float, otherwise keep as string
                let tag_value = parse_string_to_tag_value(&value);
                metadata.insert(tag_name, tag_value);
            }
        }
        Err(e) => {
            // Log error but continue processing (don't fail entire read)
            eprintln!("Warning: Failed to parse XMP: {}", e);
        }
    }
}

/// Processes IPTC APP13 segments and extracts IPTC metadata.
///
/// IPTC (International Press Telecommunications Council) metadata is
/// stored in APP13 segments and contains fields like keywords, caption, etc.
///
/// # Arguments
///
/// * `segments` - Parsed JPEG segments
/// * `metadata` - MetadataMap to populate with IPTC tags
pub fn process_iptc_segments(segments: &[Segment], metadata: &mut MetadataMap) {
    match crate::parsers::jpeg::iptc_parser::extract_iptc_from_segments(segments) {
        Ok(iptc_tags) => {
            // Add all IPTC tags to metadata
            for (tag_name, value) in iptc_tags {
                // Try to parse as integer first, then as float, otherwise keep as string
                let tag_value = parse_string_to_tag_value(&value);
                metadata.insert(tag_name, tag_value);
            }
        }
        Err(e) => {
            // Log error but continue processing
            eprintln!("Warning: Failed to extract IPTC metadata: {}", e);
        }
    }
}

/// Processes ICC profile APP2 segments and extracts color profile metadata.
///
/// ICC (International Color Consortium) profiles describe the color
/// characteristics of an image and are stored in APP2 segments.
///
/// # Arguments
///
/// * `segments` - Parsed JPEG segments
/// * `metadata` - MetadataMap to populate with ICC profile tags
pub fn process_icc_segments(segments: &[Segment], metadata: &mut MetadataMap) {
    for segment in segments.iter().filter(|s| s.marker == 0xFFE2) {
        // Check if this is an ICC profile segment (starts with "ICC_PROFILE\0")
        if segment.data.len() >= 14 && &segment.data[0..12] == b"ICC_PROFILE\0" {
            // ICC profile structure in JPEG APP2:
            // Bytes 0-11: "ICC_PROFILE\0" identifier
            // Byte 12: Chunk number (1-based)
            // Byte 13: Total chunks
            // Bytes 14+: ICC profile data

            // For now, only handle single-chunk ICC profiles (most common)
            let chunk_num = segment.data[12];
            let total_chunks = segment.data[13];

            if chunk_num == 1 && total_chunks == 1 {
                // Single chunk - parse ICC profile directly
                let icc_data = &segment.data[14..];
                match crate::parsers::icc::parse_icc_profile_data(icc_data) {
                    Ok(icc_tags) => {
                        // Add all ICC tags to metadata with "Profile:" prefix
                        for (tag_name, value) in icc_tags {
                            metadata.insert(format!("Profile:{}", tag_name), value);
                        }
                    }
                    Err(e) => {
                        // Log error but continue processing
                        eprintln!("Warning: Failed to parse ICC profile: {}", e);
                    }
                }
            } else {
                // Multi-chunk ICC profile - would need to reassemble chunks
                // This is less common, so we'll skip for now
                eprintln!(
                    "Warning: Multi-chunk ICC profile detected ({}/{}), not yet supported",
                    chunk_num, total_chunks
                );
            }
        }
    }
}

/// Processes SOF (Start of Frame) segments and extracts File-level dimension metadata.
///
/// SOF segments contain image dimensions, color information, and encoding details
/// extracted from the JPEG frame header.
///
/// # Arguments
///
/// * `segments` - Parsed JPEG segments
/// * `metadata` - MetadataMap to populate with File-level tags
pub fn process_sof_segments(segments: &[Segment], metadata: &mut MetadataMap) {
    // SOF markers range from 0xFFC0 to 0xFFCF (excluding 0xFFC4, 0xFFC8, 0xFFCC)
    const SOF_MARKERS: [u16; 13] = [
        0xFFC0, 0xFFC1, 0xFFC2, 0xFFC3, 0xFFC5, 0xFFC6, 0xFFC7, 0xFFC9, 0xFFCA, 0xFFCB, 0xFFCD,
        0xFFCE, 0xFFCF,
    ];

    for segment in segments.iter() {
        if SOF_MARKERS.contains(&segment.marker) {
            // Parse SOF segment using the app_parsers module
            let _ = crate::parsers::jpeg::app_parsers::parse_sof_segment(
                segment.marker,
                segment.data,
                metadata,
            );
            // Only process the first SOF segment found
            break;
        }
    }
}
