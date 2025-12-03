//! WebP image format parser
//!
//! WebP uses RIFF container with chunks:
//! - VP8/VP8L/VP8X: Image data
//! - EXIF: TIFF/EXIF metadata
//! - ICCP: ICC color profile
//! - XMP: XMP metadata

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use crate::parsers::tiff::ifd_parser::{parse_ifd, ByteOrder};
use crate::tag_db::lookup_tag_name;
use std::io;

/// WebP signature: "RIFF" + size + "WEBP"
const RIFF_SIGNATURE: &[u8] = b"RIFF";
const WEBP_SIGNATURE: &[u8] = b"WEBP";

/// WebP parser
pub struct WebPParser;

impl WebPParser {
    /// Verifies WebP signature
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 12 {
            return Ok(false);
        }
        let header = reader.read(0, 12)?;
        Ok(&header[0..4] == RIFF_SIGNATURE && &header[8..12] == WEBP_SIGNATURE)
    }
}

impl FormatParser for WebPParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid WebP signature"));
        }

        let mut metadata = MetadataMap::new();

        metadata.insert("FileType".to_string(), TagValue::String("WebP".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        // Parse RIFF chunks to find EXIF, XMP, and VP8X data
        parse_webp_chunks(reader, &mut metadata)?;

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::WebP)
    }
}

/// Parses metadata from WebP files.
///
/// This is a convenience wrapper around WebPParser that provides a functional API.
pub fn parse_webp_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = WebPParser;
    parser.parse(reader).map_err(|e| e.to_string())
}

/// Parse RIFF chunks in WebP file
fn parse_webp_chunks(reader: &dyn FileReader, metadata: &mut MetadataMap) -> Result<()> {
    let file_size = reader.size();

    // Skip RIFF header (12 bytes: "RIFF" + size + "WEBP")
    let mut offset = 12u64;

    while offset + 8 <= file_size {
        // Read chunk header: FourCC (4 bytes) + size (4 bytes, little-endian)
        let chunk_header = reader.read(offset, 8)?;
        let chunk_type = &chunk_header[0..4];
        let chunk_size = u32::from_le_bytes([
            chunk_header[4],
            chunk_header[5],
            chunk_header[6],
            chunk_header[7],
        ]) as u64;

        // Move past header
        let chunk_data_offset = offset + 8;

        match chunk_type {
            b"VP8X" => {
                // Extended WebP header with flags
                if chunk_size >= 10 {
                    let vp8x_data = reader.read(chunk_data_offset, 10)?;
                    let flags = vp8x_data[0];

                    // Extract image dimensions (24-bit values)
                    let width = u32::from_le_bytes([vp8x_data[4], vp8x_data[5], vp8x_data[6], 0]) + 1;
                    let height = u32::from_le_bytes([vp8x_data[7], vp8x_data[8], vp8x_data[9], 0]) + 1;

                    metadata.insert(
                        "WebP:ImageWidth".to_string(),
                        TagValue::Integer(width as i64),
                    );
                    metadata.insert(
                        "WebP:ImageHeight".to_string(),
                        TagValue::Integer(height as i64),
                    );

                    // Decode flags
                    if flags & 0x10 != 0 {
                        metadata.insert(
                            "WebP:HasICCP".to_string(),
                            TagValue::String("Yes".to_string()),
                        );
                    }
                    if flags & 0x20 != 0 {
                        metadata.insert(
                            "WebP:HasAlpha".to_string(),
                            TagValue::String("Yes".to_string()),
                        );
                    }
                    if flags & 0x08 != 0 {
                        metadata.insert(
                            "WebP:HasEXIF".to_string(),
                            TagValue::String("Yes".to_string()),
                        );
                    }
                    if flags & 0x04 != 0 {
                        metadata.insert(
                            "WebP:HasXMP".to_string(),
                            TagValue::String("Yes".to_string()),
                        );
                    }
                    if flags & 0x02 != 0 {
                        metadata.insert(
                            "WebP:IsAnimation".to_string(),
                            TagValue::String("Yes".to_string()),
                        );
                    }
                }
            }
            b"VP8 " => {
                // Lossy VP8 bitstream - extract dimensions from frame header
                if chunk_size >= 10 {
                    let vp8_data = reader.read(chunk_data_offset, 10)?;
                    // VP8 frame header starts with 3-byte frame tag
                    // Check if this is a keyframe
                    if vp8_data[0] & 0x01 == 0 {
                        // Keyframe - dimensions at bytes 6-9
                        let width = u16::from_le_bytes([vp8_data[6], vp8_data[7]]) & 0x3FFF;
                        let height = u16::from_le_bytes([vp8_data[8], vp8_data[9]]) & 0x3FFF;

                        if !metadata.contains_key("WebP:ImageWidth") {
                            metadata.insert(
                                "WebP:ImageWidth".to_string(),
                                TagValue::Integer(width as i64),
                            );
                            metadata.insert(
                                "WebP:ImageHeight".to_string(),
                                TagValue::Integer(height as i64),
                            );
                        }
                    }
                }
            }
            b"VP8L" => {
                // Lossless VP8L bitstream
                if chunk_size >= 5 {
                    let vp8l_data = reader.read(chunk_data_offset, 5)?;
                    // Check signature byte (0x2F)
                    if vp8l_data[0] == 0x2F {
                        // Dimensions are packed in bytes 1-4
                        let bits = u32::from_le_bytes([vp8l_data[1], vp8l_data[2], vp8l_data[3], vp8l_data[4]]);
                        let width = (bits & 0x3FFF) + 1;
                        let height = ((bits >> 14) & 0x3FFF) + 1;

                        if !metadata.contains_key("WebP:ImageWidth") {
                            metadata.insert(
                                "WebP:ImageWidth".to_string(),
                                TagValue::Integer(width as i64),
                            );
                            metadata.insert(
                                "WebP:ImageHeight".to_string(),
                                TagValue::Integer(height as i64),
                            );
                        }
                    }
                }
            }
            b"EXIF" => {
                // EXIF metadata - contains TIFF/EXIF data
                if chunk_size > 0 && chunk_data_offset + chunk_size <= file_size {
                    let exif_data = reader.read(chunk_data_offset, chunk_size as usize)?;
                    if let Err(_) = parse_webp_exif(exif_data, metadata) {
                        // Silently ignore EXIF parsing errors
                    }
                }
            }
            b"XMP " => {
                // XMP metadata - XML format
                if chunk_size > 0 && chunk_data_offset + chunk_size <= file_size {
                    let xmp_data = reader.read(chunk_data_offset, chunk_size as usize)?;
                    if let Ok(xmp_str) = std::str::from_utf8(xmp_data) {
                        // Store raw XMP (could be parsed further)
                        metadata.insert(
                            "XMP:RawXMP".to_string(),
                            TagValue::String(xmp_str.to_string()),
                        );
                    }
                }
            }
            b"ICCP" => {
                // ICC color profile
                metadata.insert(
                    "WebP:ICCProfileSize".to_string(),
                    TagValue::Integer(chunk_size as i64),
                );
            }
            _ => {
                // Skip unknown chunks
            }
        }

        // Move to next chunk (chunks are padded to even byte boundary)
        offset = chunk_data_offset + chunk_size;
        if chunk_size % 2 != 0 {
            offset += 1; // Padding byte
        }
    }

    Ok(())
}

/// Parse EXIF data from WebP EXIF chunk
fn parse_webp_exif(exif_data: &[u8], metadata: &mut MetadataMap) -> Result<()> {
    if exif_data.len() < 8 {
        return Err(ExifToolError::parse_error("EXIF data too short"));
    }

    // WebP EXIF chunk can start with "Exif\0\0" header (like JPEG) or directly with TIFF header
    let tiff_data = if exif_data.len() >= 6 && &exif_data[0..4] == b"Exif" {
        // Skip "Exif\0\0" header
        &exif_data[6..]
    } else {
        exif_data
    };

    if tiff_data.len() < 8 {
        return Err(ExifToolError::parse_error("TIFF data too short"));
    }

    // Detect byte order
    let byte_order = match &tiff_data[0..2] {
        b"II" => ByteOrder::LittleEndian,
        b"MM" => ByteOrder::BigEndian,
        _ => return Err(ExifToolError::parse_error("Invalid TIFF byte order")),
    };

    // Verify TIFF magic
    let magic = match byte_order {
        ByteOrder::LittleEndian => u16::from_le_bytes([tiff_data[2], tiff_data[3]]),
        ByteOrder::BigEndian => u16::from_be_bytes([tiff_data[2], tiff_data[3]]),
    };
    if magic != 0x002A {
        return Err(ExifToolError::parse_error("Invalid TIFF magic number"));
    }

    // Get IFD0 offset
    let ifd_offset = match byte_order {
        ByteOrder::LittleEndian => {
            u32::from_le_bytes([tiff_data[4], tiff_data[5], tiff_data[6], tiff_data[7]])
        }
        ByteOrder::BigEndian => {
            u32::from_be_bytes([tiff_data[4], tiff_data[5], tiff_data[6], tiff_data[7]])
        }
    };

    // Create in-memory reader
    let exif_reader = WebPExifReader::new(tiff_data.to_vec());

    // Track sub-IFD offsets
    let mut exif_ifd_offset = None;
    let mut gps_ifd_offset = None;

    // Parse IFD0
    if let Ok(ifd0_tags) = parse_ifd(&exif_reader, ifd_offset as u64, byte_order) {
        for (tag_id, field_type, value_count, raw_bytes) in &ifd0_tags {
            // Check for ExifIFD pointer
            if *tag_id == 0x8769 && raw_bytes.len() >= 4 {
                let offset = match byte_order {
                    ByteOrder::LittleEndian => {
                        u32::from_le_bytes([raw_bytes[0], raw_bytes[1], raw_bytes[2], raw_bytes[3]])
                    }
                    ByteOrder::BigEndian => {
                        u32::from_be_bytes([raw_bytes[0], raw_bytes[1], raw_bytes[2], raw_bytes[3]])
                    }
                };
                exif_ifd_offset = Some(offset as u64);
                continue;
            }

            // Check for GPS pointer
            if *tag_id == 0x8825 && raw_bytes.len() >= 4 {
                let offset = match byte_order {
                    ByteOrder::LittleEndian => {
                        u32::from_le_bytes([raw_bytes[0], raw_bytes[1], raw_bytes[2], raw_bytes[3]])
                    }
                    ByteOrder::BigEndian => {
                        u32::from_be_bytes([raw_bytes[0], raw_bytes[1], raw_bytes[2], raw_bytes[3]])
                    }
                };
                gps_ifd_offset = Some(offset as u64);
                continue;
            }

            let tag_name = lookup_tag_name(*tag_id, "IFD0");
            let tag_value = raw_bytes_to_tag_value(raw_bytes, *field_type, *value_count, byte_order);
            metadata.insert(tag_name, tag_value);
        }
    }

    // Parse ExifIFD
    if let Some(offset) = exif_ifd_offset {
        if let Ok(exif_tags) = parse_ifd(&exif_reader, offset, byte_order) {
            for (tag_id, field_type, value_count, raw_bytes) in exif_tags {
                let tag_name = lookup_tag_name(tag_id, "ExifIFD");
                let tag_value = raw_bytes_to_tag_value(&raw_bytes, field_type, value_count, byte_order);
                metadata.insert(tag_name, tag_value);
            }
        }
    }

    // Parse GPS IFD
    if let Some(offset) = gps_ifd_offset {
        if let Ok(gps_tags) = parse_ifd(&exif_reader, offset, byte_order) {
            for (tag_id, field_type, value_count, raw_bytes) in gps_tags {
                let tag_name = lookup_tag_name(tag_id, "GPS");
                let tag_value = raw_bytes_to_tag_value(&raw_bytes, field_type, value_count, byte_order);
                metadata.insert(tag_name, tag_value);
            }
        }
    }

    Ok(())
}

/// In-memory FileReader for WebP EXIF data
struct WebPExifReader {
    data: Vec<u8>,
}

impl WebPExifReader {
    fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl FileReader for WebPExifReader {
    fn read(&self, offset: u64, length: usize) -> io::Result<&[u8]> {
        let start = offset as usize;
        let end = start + length;
        if end > self.data.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "read beyond end of EXIF data",
            ));
        }
        Ok(&self.data[start..end])
    }

    fn size(&self) -> u64 {
        self.data.len() as u64
    }
}

/// Convert raw EXIF bytes to TagValue
fn raw_bytes_to_tag_value(
    bytes: &[u8],
    field_type: u16,
    value_count: u32,
    byte_order: ByteOrder,
) -> TagValue {
    use crate::parsers::common::exif_types::ExifType;

    if let Some(exif_type) = ExifType::from_u16(field_type) {
        match exif_type {
            ExifType::Byte if !bytes.is_empty() => {
                if value_count == 1 {
                    return TagValue::Integer(bytes[0] as i64);
                }
                return TagValue::Binary(bytes.to_vec());
            }
            ExifType::Ascii => {
                let text = String::from_utf8_lossy(bytes);
                return TagValue::String(text.trim_end_matches('\0').to_string());
            }
            ExifType::Short if bytes.len() >= 2 => {
                let value = match byte_order {
                    ByteOrder::LittleEndian => u16::from_le_bytes([bytes[0], bytes[1]]),
                    ByteOrder::BigEndian => u16::from_be_bytes([bytes[0], bytes[1]]),
                };
                return TagValue::Integer(value as i64);
            }
            ExifType::Long if bytes.len() >= 4 => {
                let value = match byte_order {
                    ByteOrder::LittleEndian => {
                        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                    }
                    ByteOrder::BigEndian => {
                        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                    }
                };
                return TagValue::Integer(value as i64);
            }
            ExifType::Rational if bytes.len() >= 8 => {
                let (num, den) = match byte_order {
                    ByteOrder::LittleEndian => (
                        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                        u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
                    ),
                    ByteOrder::BigEndian => (
                        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                        u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
                    ),
                };
                if den != 0 {
                    return TagValue::Float(num as f64 / den as f64);
                }
            }
            ExifType::Undefined => {
                if bytes.iter().all(|&b| b.is_ascii_graphic() || b.is_ascii_whitespace() || b == 0) {
                    let text = String::from_utf8_lossy(bytes);
                    let trimmed = text.trim_end_matches('\0');
                    if !trimmed.is_empty() {
                        return TagValue::String(trimmed.to_string());
                    }
                }
                return TagValue::Binary(bytes.to_vec());
            }
            ExifType::SRational if bytes.len() >= 8 => {
                let (num, den) = match byte_order {
                    ByteOrder::LittleEndian => (
                        i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                        i32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
                    ),
                    ByteOrder::BigEndian => (
                        i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                        i32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
                    ),
                };
                if den != 0 {
                    return TagValue::Float(num as f64 / den as f64);
                }
            }
            _ => {}
        }
    }
    TagValue::Binary(bytes.to_vec())
}
