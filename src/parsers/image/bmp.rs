//! BMP (Bitmap) image format parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// BMP signature: "BM" (0x42 0x4D)
const BMP_SIGNATURE: &[u8] = b"BM";

/// BMP parser for extracting metadata from Windows bitmap images
pub struct BMPParser;

impl BMPParser {
    /// Verifies BMP signature
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 2 {
            return Ok(false);
        }
        let header = reader.read(0, 2)?;
        Ok(header == BMP_SIGNATURE)
    }

    /// Reads image dimensions from BMP header
    /// Width at offset 18 (4 bytes), Height at offset 22 (4 bytes), both little-endian
    pub fn read_dimensions(reader: &dyn FileReader) -> Result<(i32, i32)> {
        if reader.size() < 26 {
            return Ok((0, 0));
        }
        let width_bytes = reader.read(18, 4)?;
        let height_bytes = reader.read(22, 4)?;
        let width = i32::from_le_bytes([
            width_bytes[0],
            width_bytes[1],
            width_bytes[2],
            width_bytes[3],
        ]);
        let height = i32::from_le_bytes([
            height_bytes[0],
            height_bytes[1],
            height_bytes[2],
            height_bytes[3],
        ]);
        Ok((width, height))
    }

    /// Reads bit depth from BMP header (offset 28, 2 bytes)
    pub fn read_bit_depth(reader: &dyn FileReader) -> Result<u16> {
        if reader.size() < 30 {
            return Ok(0);
        }
        let bits = reader.read(28, 2)?;
        Ok(u16::from_le_bytes([bits[0], bits[1]]))
    }

    /// Reads compression method from BMP header (offset 30, 4 bytes)
    pub fn read_compression(reader: &dyn FileReader) -> Result<u32> {
        if reader.size() < 34 {
            return Ok(0);
        }
        let comp = reader.read(30, 4)?;
        Ok(u32::from_le_bytes([comp[0], comp[1], comp[2], comp[3]]))
    }

    /// Reads horizontal resolution from BMP header (offset 38, 4 bytes)
    /// Returns pixels per meter
    pub fn read_h_resolution(reader: &dyn FileReader) -> Result<i32> {
        if reader.size() < 42 {
            return Ok(0);
        }
        let res = reader.read(38, 4)?;
        Ok(i32::from_le_bytes([res[0], res[1], res[2], res[3]]))
    }

    /// Reads vertical resolution from BMP header (offset 42, 4 bytes)
    /// Returns pixels per meter
    pub fn read_v_resolution(reader: &dyn FileReader) -> Result<i32> {
        if reader.size() < 46 {
            return Ok(0);
        }
        let res = reader.read(42, 4)?;
        Ok(i32::from_le_bytes([res[0], res[1], res[2], res[3]]))
    }

    /// Reads number of colors in palette (offset 46, 4 bytes)
    pub fn read_num_colors(reader: &dyn FileReader) -> Result<u32> {
        if reader.size() < 50 {
            return Ok(0);
        }
        let colors = reader.read(46, 4)?;
        Ok(u32::from_le_bytes([
            colors[0], colors[1], colors[2], colors[3],
        ]))
    }

    /// Reads number of important colors (offset 50, 4 bytes)
    pub fn read_num_important_colors(reader: &dyn FileReader) -> Result<u32> {
        if reader.size() < 54 {
            return Ok(0);
        }
        let colors = reader.read(50, 4)?;
        Ok(u32::from_le_bytes([
            colors[0], colors[1], colors[2], colors[3],
        ]))
    }
}

impl FormatParser for BMPParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid BMP signature"));
        }

        let mut metadata = MetadataMap::new();

        metadata.insert("FileType".to_string(), TagValue::String("BMP".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        let (width, height) = Self::read_dimensions(reader)?;
        metadata.insert(
            "ImageWidth".to_string(),
            TagValue::String(width.abs().to_string()),
        );
        metadata.insert(
            "ImageHeight".to_string(),
            TagValue::String(height.abs().to_string()),
        );

        let bit_depth = Self::read_bit_depth(reader)?;
        metadata.insert(
            "BitDepth".to_string(),
            TagValue::String(bit_depth.to_string()),
        );

        // Compression method
        let compression = Self::read_compression(reader)?;
        let compression_str = match compression {
            0 => "None",
            1 => "RLE 8-bit",
            2 => "RLE 4-bit",
            3 => "Bitfields",
            4 => "JPEG",
            5 => "PNG",
            _ => "Unknown",
        };
        metadata.insert(
            "Compression".to_string(),
            TagValue::String(compression_str.to_string()),
        );

        // Resolution
        let h_res = Self::read_h_resolution(reader)?;
        let v_res = Self::read_v_resolution(reader)?;
        if h_res > 0 {
            metadata.insert(
                "XResolution".to_string(),
                TagValue::String(format!("{} pixels/meter", h_res)),
            );
        }
        if v_res > 0 {
            metadata.insert(
                "YResolution".to_string(),
                TagValue::String(format!("{} pixels/meter", v_res)),
            );
        }

        // Color palette information
        let num_colors = Self::read_num_colors(reader)?;
        if num_colors > 0 {
            metadata.insert(
                "NumColors".to_string(),
                TagValue::Integer(num_colors as i64),
            );
        }

        let important_colors = Self::read_num_important_colors(reader)?;
        if important_colors > 0 {
            metadata.insert(
                "NumImportantColors".to_string(),
                TagValue::Integer(important_colors as i64),
            );
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::BMP)
    }
}

/// Parses metadata from BMP files.
///
/// This is a convenience wrapper around BMPParser that provides a functional API.
pub fn parse_bmp_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = BMPParser;
    parser.parse(reader).map_err(|e| e.to_string())
}
