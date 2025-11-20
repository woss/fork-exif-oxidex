//! Metadata operations (Read/Write/Copy/Transform)
//!
//! This module defines core operations for metadata manipulation.
//! It orchestrates format detection, parser selection, and metadata extraction
//! following the hexagonal architecture pattern.

use super::{FileFormat, FileReader, MetadataMap, TagValue};
use crate::core::format_dispatch::dispatch_format_parser;
use crate::core::tag_conversion::{
    parse_string_to_tag_value, raw_bytes_to_tag_value, read_u16, read_u32,
};
use crate::core::tiff_helpers::{parse_exif_subifd, parse_gps_subifd, parse_ifd_chain};
use crate::core::validation::validate_tag_value_with_name;
use crate::error::{ExifToolError, Result};
use crate::io::MMapReader;
use crate::parsers::format_detector::detect_format;
use crate::parsers::jpeg::segment_parser::{parse_segments, Segment};
use crate::parsers::jpeg::xmp_parser::extract_xmp_from_segments;
use crate::parsers::tiff::ifd_parser::{parse_ifd, ByteOrder};
use crate::tag_db::lookup_tag_name;
use crate::tag_db::tag_registry::get_tag_descriptor;
use crate::writers::atomic_writer::write_atomic;
use crate::writers::jpeg_writer::write_exif_to_jpeg;
use std::path::Path;

// ============================================================================
// SECTION 1: PUBLIC API FUNCTIONS
// ============================================================================

/// Reads metadata from a file at the specified path.
///
/// This function orchestrates the complete metadata extraction workflow:
/// 1. Opens file with MMapReader (zero-copy memory-mapped access)
/// 2. Detects file format via magic bytes
/// 3. Selects and invokes appropriate format parser
/// 4. Parses raw metadata to MetadataMap
/// 5. Enriches metadata with tag descriptors from registry
///
/// # Arguments
///
/// * `path` - Path to the file to read metadata from
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully extracted metadata
/// * `Err(ExifToolError)` - I/O error, unsupported format, or parse error
///
/// # Examples
///
/// ```no_run
/// use oxidex::core::operations::read_metadata;
/// use std::path::Path;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let metadata = read_metadata(Path::new("photo.jpg"))?;
///
/// // Access typed metadata
/// if let Some(make) = metadata.get_string("EXIF:Make") {
///     println!("Camera: {}", make);
/// }
/// if let Some(iso) = metadata.get_integer("EXIF:ISO") {
///     println!("ISO: {}", iso);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - File cannot be opened or read (IoError)
/// - File format is unsupported (UnsupportedFormat)
/// - File contains invalid or truncated metadata (ParseError)
pub fn read_metadata(path: &Path) -> Result<MetadataMap> {
    // Step 1: Extract file system metadata (File:FileName, File:FileSize, etc.)
    // This is done first and independently of the file format
    let mut metadata = match crate::core::file_metadata::extract_file_metadata(path) {
        Ok(file_meta) => file_meta,
        Err(e) => {
            // If we can't get file metadata, log a warning but continue
            eprintln!("Warning: Failed to extract file metadata: {}", e);
            MetadataMap::new()
        }
    };

    // Step 2: Open file with MMapReader for zero-copy access
    let reader = MMapReader::new(path)?;

    // Step 3: Detect format via magic bytes
    let mut format = detect_format(&reader)?;

    // Step 3b: Check for camera raw formats using filename + magic bytes
    // Many raw formats are TIFF-based and need filename context for proper detection
    // (e.g., DNG, NEF, ARW all have TIFF magic bytes but different file extensions)
    if format == FileFormat::TIFF {
        // Get filename for raw format detection
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Read first 32 bytes for raw format detection
        if let Ok(magic_bytes) = reader.read(0, 32) {
            // Check if this is a camera raw format
            if let Some(raw_format) = crate::parsers::raw::detect_raw_format(magic_bytes, filename)
            {
                // Override TIFF detection with specific raw format
                format = FileFormat::CameraRaw(raw_format);
            }
        }
    }

    // Step 4: Route to appropriate parser based on detected format and extract format-specific metadata
    let format_metadata = dispatch_format_parser(&reader, format)?;

    // Step 5: Merge format-specific metadata into file metadata
    // Format-specific metadata takes precedence over file metadata in case of conflicts
    // Use into_iter() to consume format_metadata and avoid cloning keys and values
    for (key, value) in format_metadata {
        metadata.insert(key, value);
    }

    Ok(metadata)
}

/// Writes modified metadata to a file at the specified path.
///
/// This function orchestrates the complete metadata write workflow:
/// 1. Validates all tag values against their type definitions
/// 2. Opens the original file with MMapReader
/// 3. Detects file format via magic bytes
/// 4. Serializes metadata using appropriate format writer
/// 5. Writes result atomically using atomic_writer
///
/// # Arguments
///
/// * `path` - Path to the file to write metadata to
/// * `metadata` - MetadataMap containing tags to write
///
/// # Returns
///
/// * `Ok(())` - Successfully validated and wrote metadata
/// * `Err(ExifToolError)` - Validation failure, I/O error, or unsupported format
///
/// # Examples
///
/// ```no_run
/// use oxidex::core::operations::{read_metadata, write_metadata};
/// use oxidex::core::tag_value::TagValue;
/// use std::path::Path;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let path = Path::new("photo.jpg");
///
/// // Read existing metadata
/// let mut metadata = read_metadata(path)?;
///
/// // Modify a tag
/// metadata.insert("EXIF:Artist", TagValue::new_string("John Doe"));
///
/// // Write back to file
/// write_metadata(path, &metadata)?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - Any tag value fails validation (InvalidTagValue)
/// - File cannot be opened or read (IoError)
/// - File format is unsupported (UnsupportedFormat)
/// - Serialization fails (ParseError)
/// - Atomic write fails (IoError)
///
/// # Validation
///
/// All tags are validated before any file operations. Validation checks:
/// - Type matching (String, Integer, Float, Rational, etc.)
/// - Value constraints (e.g., Rational denominator != 0)
///
/// Tags not in the registry are skipped during validation (allows custom tags).
pub fn write_metadata(path: &Path, metadata: &MetadataMap) -> Result<()> {
    // PHASE 1: VALIDATION (fail fast before any file operations)
    // Iterate through all tags and validate each one against its descriptor
    for (tag_name, tag_value) in metadata.iter() {
        // Look up tag descriptor in registry
        if let Some(descriptor) = get_tag_descriptor(tag_name) {
            // Validate that the tag value matches the expected type
            // Pass the original tag_name (e.g., "IFD0:Make") for error messages
            validate_tag_value_with_name(tag_name, descriptor, tag_value)?;
        }
        // If tag is not in registry, skip validation (allows custom/rare tags)
    }

    // PHASE 2: READ ORIGINAL FILE
    // Open file with MMapReader for zero-copy access
    let reader = MMapReader::new(path)?;

    // PHASE 3: DETECT FORMAT
    let format = detect_format(&reader)?;

    // PHASE 4: SERIALIZE WITH APPROPRIATE WRITER
    let serialized_bytes = match format {
        FileFormat::JPEG => {
            // Use JPEG writer to serialize metadata
            write_exif_to_jpeg(&reader, metadata)?
        }
        FileFormat::TIFF => {
            // TIFF writer not yet implemented (will be in I3.T7)
            return Err(ExifToolError::unsupported_format(
                "TIFF write operations are not yet supported in this iteration",
            ));
        }
        _ => {
            return Err(ExifToolError::unsupported_format(format!(
                "Write operations for format {:?} are not supported",
                format
            )));
        }
    };

    // PHASE 5: ATOMIC WRITE
    // Write serialized bytes to file using atomic temp-file-and-rename pattern
    write_atomic(path, &serialized_bytes)?;

    Ok(())
}

/// Modifies a single tag in a file's metadata.
///
/// This is a convenience function that:
/// 1. Reads existing metadata from the file
/// 2. Modifies the specified tag with the new value
/// 3. Writes all metadata back to the file
///
/// This ensures all other tags are preserved unchanged.
///
/// # Arguments
///
/// * `path` - Path to the file to modify
/// * `tag_name` - Canonical tag name (e.g., "EXIF:Artist")
/// * `new_value` - New value for the tag
///
/// # Returns
///
/// * `Ok(())` - Successfully modified tag and wrote file
/// * `Err(ExifToolError)` - Read error, validation error, or write error
///
/// # Examples
///
/// ```no_run
/// use oxidex::core::operations::modify_tag;
/// use oxidex::core::tag_value::TagValue;
/// use std::path::Path;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let path = Path::new("photo.jpg");
///
/// // Modify a single tag
/// modify_tag(
///     path,
///     "EXIF:Artist",
///     TagValue::new_string("John Doe")
/// )?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - File cannot be read (IoError)
/// - New value fails validation (InvalidTagValue)
/// - File cannot be written (IoError)
pub fn modify_tag(path: &Path, tag_name: &str, new_value: TagValue) -> Result<()> {
    // Step 1: Read existing metadata (preserves all other tags)
    let mut metadata = read_metadata(path)?;

    // Step 2: Modify the single tag
    metadata.insert(tag_name, new_value);

    // Step 3: Write all metadata back to file
    write_metadata(path, &metadata)?;

    Ok(())
}

/// Copies metadata from a source file to a destination file.
///
/// This function orchestrates the metadata copy workflow:
/// 1. Reads metadata from the source file
/// 2. Optionally filters to specified tags
/// 3. Reads existing metadata from destination file
/// 4. Merges source tags into destination metadata (preserving unspecified tags)
/// 5. Writes merged metadata back to destination file
///
/// # Arguments
///
/// * `src` - Path to the source file to copy metadata from
/// * `dest` - Path to the destination file to copy metadata to
/// * `tags` - Optional slice of tag names to copy. If `None`, all tags are copied.
///
/// # Returns
///
/// * `Ok(())` - Successfully copied metadata
/// * `Err(ExifToolError)` - Read error, validation error, or write error
///
/// # Examples
///
/// ```no_run
/// use oxidex::core::operations::copy_metadata;
/// use std::path::Path;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Copy all metadata from source to destination
/// copy_metadata(
///     Path::new("source.jpg"),
///     Path::new("dest.jpg"),
///     None
/// )?;
///
/// // Copy only specific tags
/// copy_metadata(
///     Path::new("source.jpg"),
///     Path::new("dest.jpg"),
///     Some(&["EXIF:Artist".to_string(), "EXIF:Copyright".to_string()])
/// )?;
/// # Ok(())
/// # }
/// ```
///
/// # Behavior
///
/// - Source tags are merged into destination metadata
/// - Existing destination tags not in the source are preserved
/// - If a tag exists in both source and destination, the source value overwrites it
/// - If `tags` filter is specified, only those tags are copied from source
///
/// # Errors
///
/// Returns an error if:
/// - Source file cannot be read (IoError)
/// - Destination file cannot be read (IoError)
/// - Any tag value fails validation (InvalidTagValue)
/// - Destination file cannot be written (IoError)
pub fn copy_metadata(src: &Path, dest: &Path, tags: Option<&[String]>) -> Result<()> {
    // Step 1: Read metadata from source file
    let source_metadata = read_metadata(src)?;

    // Step 2: Read existing metadata from destination file
    let mut dest_metadata = read_metadata(dest)?;

    // Step 3: Filter and merge source tags into destination metadata
    // Use into_iter() to consume source_metadata and avoid cloning when possible
    for (tag_name, tag_value) in source_metadata {
        // Check if this tag should be copied (if filter is specified)
        let should_copy = tags.is_none_or(|filter| filter.contains(&tag_name));

        if should_copy {
            // Insert tag into destination (merges with existing, preserving others)
            // No clone needed since we own the data from into_iter()
            dest_metadata.insert(tag_name, tag_value);
        }
    }

    // Step 4: Write merged metadata back to destination file
    write_metadata(dest, &dest_metadata)?;

    Ok(())
}

// ============================================================================
// SECTION 3: JPEG METADATA PARSING
// ============================================================================

/// Parses metadata from a JPEG file.
///
/// JPEG files contain metadata in APP segments with EXIF, JFIF, XMP, IPTC, and ICC data.
/// This function coordinates parsing of all segment types.
///
/// # Arguments
///
/// * `reader` - File reader providing access to the JPEG file
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully parsed metadata from all segments
/// * `Err(ExifToolError)` - Parse error or invalid JPEG structure
pub(crate) fn parse_jpeg_metadata(reader: &dyn FileReader) -> Result<MetadataMap> {
    // Parse JPEG segment structure
    let segments = parse_segments(reader)?;

    let mut metadata = MetadataMap::new();

    // Process different segment types
    process_jfif_segments(&segments, &mut metadata);
    process_exif_segments(&segments, reader, &mut metadata);
    process_xmp_segments(&segments, &mut metadata);
    process_iptc_segments(&segments, &mut metadata);
    process_icc_segments(&segments, &mut metadata);

    Ok(metadata)
}

/// Processes JFIF APP0 segments and extracts version and resolution metadata.
///
/// JFIF segments contain basic image information including version, resolution unit,
/// and X/Y resolution values.
///
/// # Arguments
///
/// * `segments` - Parsed JPEG segments
/// * `metadata` - MetadataMap to populate with JFIF tags
fn process_jfif_segments(segments: &[Segment], metadata: &mut MetadataMap) {
    for segment in segments.iter().filter(|s| s.marker == 0xFFE0) {
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
            let x_density = u16::from_be_bytes([segment.data[8], segment.data[9]]);
            let y_density = u16::from_be_bytes([segment.data[10], segment.data[11]]);

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
fn process_exif_segments(
    segments: &[Segment],
    reader: &dyn FileReader,
    metadata: &mut MetadataMap,
) {
    // Find all APP1 segments (EXIF/XMP)
    let app1_segments: Vec<_> = segments.iter().filter(|s| s.is_app1()).collect();

    // Process each APP1 segment
    for segment in app1_segments {
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
            let ifd_offset = match byte_order {
                ByteOrder::LittleEndian => {
                    u32::from_le_bytes([tiff_data[4], tiff_data[5], tiff_data[6], tiff_data[7]])
                }
                ByteOrder::BigEndian => {
                    u32::from_be_bytes([tiff_data[4], tiff_data[5], tiff_data[6], tiff_data[7]])
                }
            } as u64;

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
fn process_xmp_segments(segments: &[Segment], metadata: &mut MetadataMap) {
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
fn process_iptc_segments(segments: &[Segment], metadata: &mut MetadataMap) {
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
fn process_icc_segments(segments: &[Segment], metadata: &mut MetadataMap) {
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
                match crate::parsers::icc_parser::parse_icc_profile_data(icc_data) {
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

// ============================================================================
// SECTION 4: TIFF METADATA PARSING
// ============================================================================

/// Parses metadata from a TIFF file.
///
/// TIFF files begin with a TIFF header followed by IFD structures.
/// This function coordinates parsing of all IFDs and sub-IFDs.
///
/// # Arguments
///
/// * `reader` - File reader providing access to the TIFF file
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully parsed metadata from all IFDs
/// * `Err(ExifToolError)` - Parse error or invalid TIFF structure
pub(crate) fn parse_tiff_metadata(reader: &dyn FileReader) -> Result<MetadataMap> {
    // Read TIFF header (first 8 bytes)
    let header = reader.read(0, 8)?;

    // Detect byte order from bytes 0-1
    let byte_order = if &header[0..2] == b"II" {
        ByteOrder::LittleEndian
    } else if &header[0..2] == b"MM" {
        ByteOrder::BigEndian
    } else {
        return Err(ExifToolError::parse_error("Invalid TIFF byte order marker"));
    };

    // Verify magic number 42 (bytes 2-3)
    let magic = read_u16(&header[2..4], byte_order);

    if magic != 42 {
        return Err(ExifToolError::parse_error(format!(
            "Invalid TIFF magic number: expected 42, got {}",
            magic
        )));
    }

    // Read first IFD offset from bytes 4-7
    let first_ifd_offset = read_u32(&header[4..8], byte_order) as u64;

    // Parse all IFDs in the chain (IFD0, IFD1, IFD2, ...)
    let mut metadata = MetadataMap::new();
    parse_ifd_chain(reader, first_ifd_offset, byte_order, &mut metadata)?;

    Ok(metadata)
}

/// Parses metadata from a Casio CAM file.
///
/// Casio CAM files are proprietary JPEG containers with a 70-byte header.
/// This function skips the header and parses the embedded JPEG data.
///
/// # Arguments
///
/// * `reader` - File reader providing access to the Casio CAM file
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully parsed metadata from embedded JPEG
/// * `Err(ExifToolError)` - Parse error or invalid file structure
pub(crate) fn parse_casio_cam_metadata(reader: &dyn FileReader) -> Result<MetadataMap> {
    // Casio CAM format: 70-byte proprietary header + JPEG data
    const HEADER_SIZE: u64 = 70;

    if reader.size() <= HEADER_SIZE {
        return Err(ExifToolError::parse_error(
            "File too small to be a valid Casio CAM file",
        ));
    }

    // Read the JPEG data starting at offset 70
    let jpeg_size = (reader.size() - HEADER_SIZE) as usize;
    let jpeg_data = reader.read(HEADER_SIZE, jpeg_size)?;

    // Create an in-memory reader for the JPEG data
    struct CasioCamJpegReader {
        data: Vec<u8>,
    }

    impl FileReader for CasioCamJpegReader {
        fn read(&self, offset: u64, length: usize) -> std::io::Result<&[u8]> {
            let start = offset as usize;
            let end = start + length;

            if end > self.data.len() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "read beyond end of JPEG data",
                ));
            }

            Ok(&self.data[start..end])
        }

        fn size(&self) -> u64 {
            self.data.len() as u64
        }
    }

    let jpeg_reader = CasioCamJpegReader {
        data: jpeg_data.to_vec(),
    };

    // Parse the JPEG metadata
    let mut metadata = parse_jpeg_metadata(&jpeg_reader)?;

    // Add warning tag to match ExifTool's behavior
    metadata.insert(
        "File:Warning".to_string(),
        TagValue::String("Processing JPEG-like data after unknown 70-byte header".to_string()),
    );

    Ok(metadata)
}

/// A FileReader wrapper that adjusts offsets to be relative to a base offset.
///
/// This is used to create a "view" into the file where offset 0 corresponds
/// to a specific position in the original file. Needed for parsing TIFF data
/// embedded within JPEG segments.
struct TiffSubReader<'a> {
    reader: &'a dyn FileReader,
    base_offset: u64,
}

impl<'a> TiffSubReader<'a> {
    fn new(reader: &'a dyn FileReader, base_offset: u64) -> Self {
        Self {
            reader,
            base_offset,
        }
    }
}

impl<'a> FileReader for TiffSubReader<'a> {
    fn read(&self, offset: u64, length: usize) -> std::io::Result<&[u8]> {
        // Adjust offset to be relative to base
        self.reader.read(self.base_offset + offset, length)
    }

    fn size(&self) -> u64 {
        // Return size relative to base (remaining size from base to end)
        let total_size = self.reader.size();
        total_size.saturating_sub(self.base_offset)
    }
}

// ============================================================================
// SECTION 7: TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    /// Test implementation of FileReader for unit testing
    struct TestReader {
        data: Vec<u8>,
    }

    impl TestReader {
        fn new(data: Vec<u8>) -> Self {
            Self { data }
        }
    }

    impl FileReader for TestReader {
        fn read(&self, offset: u64, length: usize) -> io::Result<&[u8]> {
            let start = offset as usize;
            let end = start + length;

            if end > self.data.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "read beyond end of data",
                ));
            }

            Ok(&self.data[start..end])
        }

        fn size(&self) -> u64 {
            self.data.len() as u64
        }
    }

    #[test]
    fn test_lookup_tag_name_known_tags() {
        use crate::tag_db::lookup_tag_name;
        assert_eq!(lookup_tag_name(0x010F, "IFD0"), "IFD0:Make");
        assert_eq!(lookup_tag_name(0x0110, "IFD0"), "IFD0:Model");
        assert_eq!(lookup_tag_name(0x0112, "IFD0"), "IFD0:Orientation");
    }

    #[test]
    fn test_lookup_tag_name_unknown_tags() {
        use crate::tag_db::lookup_tag_name;
        // Use tag IDs from unused ranges in the database
        assert_eq!(lookup_tag_name(0xF999, "IFD0"), "IFD0:0xF999");
        assert_eq!(lookup_tag_name(0xF998, "GPS"), "GPS:0xF998");
    }

    #[test]
    fn test_raw_bytes_to_tag_value_string() {
        use crate::parsers::tiff::ifd_parser::ByteOrder;
        let bytes = b"Canon\0";
        // Use tag_id=0x010F (Make tag) instead of 0 to avoid GPS_VERSION_ID special handler
        let value = raw_bytes_to_tag_value(bytes, 2, 1, 0x010F, ByteOrder::LittleEndian); // Type 2 = ASCII
        assert_eq!(value.as_string(), Some("Canon"));
    }

    #[test]
    fn test_raw_bytes_to_tag_value_integer_u16() {
        use crate::parsers::tiff::ifd_parser::ByteOrder;
        let bytes = [0x05, 0x00]; // 5 in little-endian
                                  // Use tag_id=0x0112 (Orientation) instead of 0
        let value = raw_bytes_to_tag_value(&bytes, 3, 1, 0x0112, ByteOrder::LittleEndian); // Type 3 = SHORT
        assert_eq!(value.as_integer(), Some(5));
    }

    #[test]
    fn test_raw_bytes_to_tag_value_integer_u32() {
        use crate::parsers::tiff::ifd_parser::ByteOrder;
        let bytes = [0x64, 0x00, 0x00, 0x00]; // 100 in little-endian
                                              // Use tag_id=0x0100 (ImageWidth) instead of 0
        let value = raw_bytes_to_tag_value(&bytes, 4, 1, 0x0100, ByteOrder::LittleEndian); // Type 4 = LONG
        assert_eq!(value.as_integer(), Some(100));
    }

    #[test]
    fn test_raw_bytes_to_tag_value_binary() {
        use crate::parsers::tiff::ifd_parser::ByteOrder;
        let bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x10, 0x20]; // Non-ASCII bytes
                                                              // Use tag_id=0xFFFF which doesn't match any special handlers
        let value = raw_bytes_to_tag_value(&bytes, 7, 1, 0xFFFF, ByteOrder::LittleEndian); // Type 7 = UNDEFINED
        assert!(value.is_binary());
    }

    #[test]
    fn test_tiff_sub_reader_offset_adjustment() {
        let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let reader = TestReader::new(data);
        let sub_reader = TiffSubReader::new(&reader, 5);

        // Reading offset 0 from sub_reader should read offset 5 from base
        let result = sub_reader.read(0, 3).unwrap();
        assert_eq!(result, &[5, 6, 7]);

        // Reading offset 2 from sub_reader should read offset 7 from base
        let result = sub_reader.read(2, 2).unwrap();
        assert_eq!(result, &[7, 8]);
    }

    #[test]
    fn test_tiff_sub_reader_size() {
        let data = vec![0; 100];
        let reader = TestReader::new(data);
        let sub_reader = TiffSubReader::new(&reader, 20);

        // Size should be (100 - 20) = 80
        assert_eq!(sub_reader.size(), 80);
    }
}
