//! Adobe Photoshop (PSD) format parser
//!
//! PSD file structure:
//! - Header (26 bytes): signature, version, reserved, channels, height, width, depth, color mode
//! - Color Mode Data section
//! - Image Resources section (contains EXIF, IPTC, XMP, etc.)
//! - Layer and Mask Information section
//! - Image Data section

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use crate::io::buffered_reader::BufferedReader;
use crate::parsers::tiff::ifd_parser::{parse_ifd, ByteOrder};
use crate::tag_db::lookup_tag_name;

const PSD_SIGNATURE: &[u8] = b"8BPS";

/// Image resource IDs
const IPTC_NAA_RECORD: u16 = 0x0404; // IPTC-NAA record
const EXIF_DATA_1: u16 = 0x0422; // EXIF data 1
const EXIF_DATA_3: u16 = 0x0423; // EXIF data 3
const XMP_DATA: u16 = 0x0424; // XMP metadata
const ICC_PROFILE: u16 = 0x040F; // ICC profile
const RESOLUTION_INFO: u16 = 0x03ED; // Resolution info
const PRINT_FLAGS: u16 = 0x03F1; // Print flags
const COPYRIGHT_FLAG: u16 = 0x040A; // Copyright flag

/// Parser for Adobe Photoshop (PSD) document files
///
/// Extracts metadata from PSD files including dimensions, color mode, channels,
/// bit depth, and embedded EXIF/IPTC/XMP data.
pub struct PSDParser;

impl PSDParser {
    /// Verifies the PSD file signature ("8BPS")
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 4 {
            return Ok(false);
        }
        let header = reader.read(0, 4)?;
        Ok(header == PSD_SIGNATURE)
    }

    /// Reads the PSD file version number (1 for PSD, 2 for PSB)
    pub fn read_version(reader: &dyn FileReader) -> Result<u16> {
        if reader.size() < 6 {
            return Ok(0);
        }
        let version_bytes = reader.read(4, 2)?;
        Ok(u16::from_be_bytes([version_bytes[0], version_bytes[1]]))
    }

    /// Parse the PSD header (26 bytes)
    fn parse_header(reader: &dyn FileReader, metadata: &mut MetadataMap) -> Result<()> {
        if reader.size() < 26 {
            return Ok(());
        }

        let header = reader.read(0, 26)?;

        // Version (offset 4, 2 bytes)
        let version = u16::from_be_bytes([header[4], header[5]]);
        let format_name = if version == 1 { "PSD" } else { "PSB" };
        metadata.insert(
            "FileType".to_string(),
            TagValue::String(format_name.to_string()),
        );
        metadata.insert("PSDVersion".to_string(), TagValue::Integer(version as i64));

        // Channels (offset 12, 2 bytes)
        let channels = u16::from_be_bytes([header[12], header[13]]);
        metadata.insert("NumChannels".to_string(), TagValue::Integer(channels as i64));

        // Height (offset 14, 4 bytes)
        let height = u32::from_be_bytes([header[14], header[15], header[16], header[17]]);
        metadata.insert("ImageHeight".to_string(), TagValue::Integer(height as i64));

        // Width (offset 18, 4 bytes)
        let width = u32::from_be_bytes([header[18], header[19], header[20], header[21]]);
        metadata.insert("ImageWidth".to_string(), TagValue::Integer(width as i64));

        // Bit Depth (offset 22, 2 bytes)
        let depth = u16::from_be_bytes([header[22], header[23]]);
        metadata.insert("BitDepth".to_string(), TagValue::Integer(depth as i64));

        // Color Mode (offset 24, 2 bytes)
        let color_mode = u16::from_be_bytes([header[24], header[25]]);
        let color_mode_name = match color_mode {
            0 => "Bitmap",
            1 => "Grayscale",
            2 => "Indexed",
            3 => "RGB",
            4 => "CMYK",
            7 => "Multichannel",
            8 => "Duotone",
            9 => "Lab",
            _ => "Unknown",
        };
        metadata.insert(
            "ColorMode".to_string(),
            TagValue::String(color_mode_name.to_string()),
        );

        Ok(())
    }

    /// Parse Image Resources section
    fn parse_image_resources(reader: &dyn FileReader, metadata: &mut MetadataMap) -> Result<()> {
        if reader.size() < 34 {
            return Ok(());
        }

        // Color mode data length at offset 26
        let cmd_len_bytes = reader.read(26, 4)?;
        let color_mode_data_length =
            u32::from_be_bytes([cmd_len_bytes[0], cmd_len_bytes[1], cmd_len_bytes[2], cmd_len_bytes[3]]);

        // Image resources section starts after color mode data
        let resources_offset = 30 + color_mode_data_length as usize;

        if reader.size() < (resources_offset + 4) as u64 {
            return Ok(());
        }

        // Image resources length
        let irl_bytes = reader.read(resources_offset as u64, 4)?;
        let resources_length =
            u32::from_be_bytes([irl_bytes[0], irl_bytes[1], irl_bytes[2], irl_bytes[3]]) as usize;

        if resources_length == 0 || reader.size() < (resources_offset + 4 + resources_length) as u64 {
            return Ok(());
        }

        // Read entire resources section
        let resources_data = reader.read((resources_offset + 4) as u64, resources_length)?;

        // Parse individual resources
        let mut pos = 0;
        while pos + 12 <= resources_data.len() {
            // Resource signature "8BIM"
            if &resources_data[pos..pos + 4] != b"8BIM" {
                break;
            }
            pos += 4;

            // Resource ID (2 bytes)
            let resource_id =
                u16::from_be_bytes([resources_data[pos], resources_data[pos + 1]]);
            pos += 2;

            // Pascal string name (padded to even)
            let name_len = resources_data[pos] as usize;
            let padded_name_len = if (name_len + 1) % 2 == 0 {
                name_len + 1
            } else {
                name_len + 2
            };
            pos += padded_name_len;

            if pos + 4 > resources_data.len() {
                break;
            }

            // Resource data size (4 bytes)
            let data_size = u32::from_be_bytes([
                resources_data[pos],
                resources_data[pos + 1],
                resources_data[pos + 2],
                resources_data[pos + 3],
            ]) as usize;
            pos += 4;

            if pos + data_size > resources_data.len() {
                break;
            }

            let resource_data = &resources_data[pos..pos + data_size];

            // Process specific resources
            match resource_id {
                RESOLUTION_INFO => {
                    Self::parse_resolution_info(resource_data, metadata);
                }
                EXIF_DATA_1 | EXIF_DATA_3 => {
                    Self::parse_exif_data(resource_data, metadata);
                }
                COPYRIGHT_FLAG => {
                    if !resource_data.is_empty() && resource_data[0] != 0 {
                        metadata.insert(
                            "Copyrighted".to_string(),
                            TagValue::String("Yes".to_string()),
                        );
                    }
                }
                XMP_DATA => {
                    if let Ok(xmp_str) = std::str::from_utf8(resource_data) {
                        Self::parse_xmp_data(xmp_str, metadata);
                    }
                }
                ICC_PROFILE => {
                    metadata.insert(
                        "HasICCProfile".to_string(),
                        TagValue::String("Yes".to_string()),
                    );
                }
                _ => {}
            }

            // Pad to even boundary
            let padded_size = if data_size % 2 == 0 {
                data_size
            } else {
                data_size + 1
            };
            pos += padded_size;
        }

        Ok(())
    }

    /// Parse resolution info resource
    fn parse_resolution_info(data: &[u8], metadata: &mut MetadataMap) {
        if data.len() < 16 {
            return;
        }

        // Horizontal resolution (fixed point 16.16)
        let h_res_fixed = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let h_res = h_res_fixed as f64 / 65536.0;

        // Resolution unit (offset 4, 2 bytes): 1=pixels/inch, 2=pixels/cm
        let res_unit = u16::from_be_bytes([data[4], data[5]]);
        let unit_name = if res_unit == 1 { "inch" } else { "cm" };

        // Vertical resolution (offset 8, fixed point 16.16)
        let v_res_fixed = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let v_res = v_res_fixed as f64 / 65536.0;

        metadata.insert(
            "XResolution".to_string(),
            TagValue::String(format!("{:.2}", h_res)),
        );
        metadata.insert(
            "YResolution".to_string(),
            TagValue::String(format!("{:.2}", v_res)),
        );
        metadata.insert(
            "ResolutionUnit".to_string(),
            TagValue::String(unit_name.to_string()),
        );
    }

    /// Parse embedded EXIF data
    fn parse_exif_data(data: &[u8], metadata: &mut MetadataMap) {
        if data.len() < 8 {
            return;
        }

        // Detect byte order
        let byte_order = match &data[0..2] {
            b"II" => ByteOrder::LittleEndian,
            b"MM" => ByteOrder::BigEndian,
            _ => return,
        };

        // Verify TIFF magic
        let magic = match byte_order {
            ByteOrder::LittleEndian => u16::from_le_bytes([data[2], data[3]]),
            ByteOrder::BigEndian => u16::from_be_bytes([data[2], data[3]]),
        };
        if magic != 0x002A {
            return;
        }

        // Get IFD0 offset
        let ifd0_offset = match byte_order {
            ByteOrder::LittleEndian => u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            ByteOrder::BigEndian => u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
        };

        // Create a BufferedReader from the TIFF data
        let reader = BufferedReader::from_bytes(data);

        // Parse IFD0
        if let Ok(entries) = parse_ifd(&reader, ifd0_offset as u64, byte_order) {
            for (tag_id, field_type, value_count, raw_bytes) in &entries {
                let tag_name = lookup_tag_name(*tag_id, "IFD0");
                let value = raw_bytes_to_tag_value(
                    raw_bytes.as_ref(),
                    *field_type,
                    *value_count,
                    *tag_id,
                    byte_order,
                );
                metadata.insert(tag_name, value);

                // Check for ExifIFD pointer
                if *tag_id == 0x8769 && raw_bytes.len() >= 4 {
                    let exif_offset = match byte_order {
                        ByteOrder::LittleEndian => u32::from_le_bytes([
                            raw_bytes[0],
                            raw_bytes[1],
                            raw_bytes[2],
                            raw_bytes[3],
                        ]),
                        ByteOrder::BigEndian => u32::from_be_bytes([
                            raw_bytes[0],
                            raw_bytes[1],
                            raw_bytes[2],
                            raw_bytes[3],
                        ]),
                    };
                    if let Ok(exif_entries) = parse_ifd(&reader, exif_offset as u64, byte_order) {
                        for (exif_tag_id, exif_field_type, exif_value_count, exif_raw_bytes) in
                            &exif_entries
                        {
                            let exif_tag_name = lookup_tag_name(*exif_tag_id, "ExifIFD");
                            let value = raw_bytes_to_tag_value(
                                exif_raw_bytes.as_ref(),
                                *exif_field_type,
                                *exif_value_count,
                                *exif_tag_id,
                                byte_order,
                            );
                            metadata.insert(exif_tag_name, value);
                        }
                    }
                }
            }
        }
    }

    /// Extract basic metadata from XMP
    fn parse_xmp_data(xmp: &str, metadata: &mut MetadataMap) {
        let patterns = [
            ("dc:creator", "XMP:Creator"),
            ("dc:title", "XMP:Title"),
            ("dc:description", "XMP:Description"),
            ("xmp:CreateDate", "XMP:CreateDate"),
            ("xmp:ModifyDate", "XMP:ModifyDate"),
            ("photoshop:ColorMode", "XMP:ColorMode"),
        ];

        for (tag, key) in patterns {
            if let Some(value) = Self::extract_xmp_value(xmp, tag) {
                metadata.insert(key.to_string(), TagValue::String(value));
            }
        }
    }

    /// Extract a value from XMP by tag name
    fn extract_xmp_value(xmp: &str, tag: &str) -> Option<String> {
        let open_tag = format!("<{}>", tag);
        let close_tag = format!("</{}>", tag);

        if let Some(start) = xmp.find(&open_tag) {
            let value_start = start + open_tag.len();
            if let Some(end) = xmp[value_start..].find(&close_tag) {
                let value = &xmp[value_start..value_start + end];
                return Some(value.trim().to_string());
            }
        }

        let attr_pattern = format!("{}=\"", tag);
        if let Some(start) = xmp.find(&attr_pattern) {
            let value_start = start + attr_pattern.len();
            if let Some(end) = xmp[value_start..].find('"') {
                let value = &xmp[value_start..value_start + end];
                return Some(value.to_string());
            }
        }

        None
    }
}

impl FormatParser for PSDParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid PSD signature"));
        }

        let mut metadata = MetadataMap::new();
        metadata.insert("FileSize".to_string(), TagValue::Integer(reader.size() as i64));

        // Parse header
        Self::parse_header(reader, &mut metadata)?;

        // Parse image resources (EXIF, XMP, etc.)
        Self::parse_image_resources(reader, &mut metadata)?;

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::PSD)
    }
}

/// Parses metadata from PSD files.
pub fn parse_psd_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = PSDParser;
    parser.parse(reader).map_err(|e| e.to_string())
}

/// Converts raw bytes to TagValue
fn raw_bytes_to_tag_value(
    bytes: &[u8],
    field_type: u16,
    _value_count: u32,
    tag_id: u16,
    byte_order: ByteOrder,
) -> TagValue {
    use crate::parsers::common::exif_types::ExifType;

    if let Some(exif_type) = ExifType::from_u16(field_type) {
        match exif_type {
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
                let num = match byte_order {
                    ByteOrder::LittleEndian => {
                        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                    }
                    ByteOrder::BigEndian => {
                        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                    }
                };
                let den = match byte_order {
                    ByteOrder::LittleEndian => {
                        u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]])
                    }
                    ByteOrder::BigEndian => {
                        u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]])
                    }
                };
                if den == 1 {
                    return TagValue::Integer(num as i64);
                }
                return TagValue::Rational {
                    numerator: num as i32,
                    denominator: den as i32,
                };
            }
            ExifType::Undefined => {
                if tag_id == 0x9000 && bytes.len() >= 4 {
                    let version = String::from_utf8_lossy(&bytes[0..4]);
                    return TagValue::String(version.to_string());
                }
                return TagValue::Binary(bytes.to_vec());
            }
            _ => {}
        }
    }

    if bytes.iter().all(|&b| b.is_ascii() || b == 0) {
        let text = String::from_utf8_lossy(bytes);
        TagValue::String(text.trim_end_matches('\0').to_string())
    } else {
        TagValue::Binary(bytes.to_vec())
    }
}
