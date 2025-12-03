//! GIF image format parser
//!
//! Implements basic metadata extraction from GIF (Graphics Interchange Format) files.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// GIF signature: "GIF87a" or "GIF89a"
const GIF87A_SIGNATURE: &[u8] = b"GIF87a";
const GIF89A_SIGNATURE: &[u8] = b"GIF89a";

/// GIF parser for extracting metadata from GIF images
pub struct GIFParser;

impl GIFParser {
    /// Verifies GIF signature
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 6 {
            return Ok(false);
        }
        let header = reader.read(0, 6)?;
        Ok(header == GIF87A_SIGNATURE || header == GIF89A_SIGNATURE)
    }

    /// Reads GIF version (87a or 89a)
    pub fn read_version(reader: &dyn FileReader) -> Result<&'static str> {
        if reader.size() < 6 {
            return Ok("Unknown");
        }
        let header = reader.read(0, 6)?;
        if header == GIF87A_SIGNATURE {
            Ok("87a")
        } else if header == GIF89A_SIGNATURE {
            Ok("89a")
        } else {
            Ok("Unknown")
        }
    }

    /// Reads image dimensions from GIF header (offset 6, 4 bytes: width, height in little-endian)
    pub fn read_dimensions(reader: &dyn FileReader) -> Result<(u16, u16)> {
        if reader.size() < 10 {
            return Ok((0, 0));
        }
        let dims = reader.read(6, 4)?;
        let width = u16::from_le_bytes([dims[0], dims[1]]);
        let height = u16::from_le_bytes([dims[2], dims[3]]);
        Ok((width, height))
    }

    /// Reads the logical screen descriptor (7 bytes at offset 6)
    fn read_logical_screen_descriptor(reader: &dyn FileReader) -> Result<LogicalScreenDescriptor> {
        if reader.size() < 13 {
            return Ok(LogicalScreenDescriptor::default());
        }
        let lsd = reader.read(6, 7)?;

        let width = u16::from_le_bytes([lsd[0], lsd[1]]);
        let height = u16::from_le_bytes([lsd[2], lsd[3]]);

        let packed = lsd[4];
        let global_color_table_flag = (packed & 0b10000000) != 0;
        let color_resolution = ((packed & 0b01110000) >> 4) + 1; // bits per primary color
        let _sort_flag = (packed & 0b00001000) != 0;
        let global_color_table_size = if global_color_table_flag {
            2u32.pow(((packed & 0b00000111) + 1) as u32)
        } else {
            0
        };

        let background_color_index = lsd[5];
        let pixel_aspect_ratio = lsd[6];

        Ok(LogicalScreenDescriptor {
            width,
            height,
            global_color_table_flag,
            color_resolution,
            global_color_table_size,
            background_color_index,
            pixel_aspect_ratio,
        })
    }

    /// Scans the file for extensions and image blocks
    fn scan_blocks(reader: &dyn FileReader) -> Result<BlockScanResult> {
        let lsd = Self::read_logical_screen_descriptor(reader)?;

        // Calculate start position after header and global color table
        let mut pos = 13u64; // 6 byte header + 7 byte logical screen descriptor
        if lsd.global_color_table_flag {
            pos += (lsd.global_color_table_size as u64) * 3;
        }

        let mut frame_count = 0;
        let mut is_animated = false;
        let mut comment = String::new();

        while pos < reader.size() {
            let byte = match reader.read(pos, 1) {
                Ok(b) => b[0],
                Err(_) => break,
            };

            match byte {
                0x21 => { // Extension introducer
                    if pos + 1 >= reader.size() {
                        break;
                    }
                    let label = reader.read(pos + 1, 1)?[0];
                    pos += 2;

                    match label {
                        0xFF => { // Application extension
                            if let Ok(block_size) = reader.read(pos, 1) {
                                let size = block_size[0] as u64;
                                pos += 1;

                                if size >= 11 && pos + 11 <= reader.size() {
                                    if let Ok(app_data) = reader.read(pos, 11) {
                                        // Check for NETSCAPE2.0 animation extension
                                        if &app_data[0..8] == b"NETSCAPE" {
                                            is_animated = true;
                                        }
                                    }
                                }
                                pos += size;
                            }
                            pos = Self::skip_sub_blocks(reader, pos)?;
                        }
                        0xFE => { // Comment extension
                            pos = Self::read_comment_blocks(reader, pos, &mut comment)?;
                        }
                        0xF9 => { // Graphic control extension
                            pos = Self::skip_sub_blocks(reader, pos)?;
                        }
                        _ => {
                            pos = Self::skip_sub_blocks(reader, pos)?;
                        }
                    }
                }
                0x2C => { // Image descriptor
                    frame_count += 1;
                    pos += 1;

                    if pos + 9 <= reader.size() {
                        let img_desc = reader.read(pos, 9)?;
                        pos += 9;

                        let packed = img_desc[8];
                        let has_local_color_table = (packed & 0b10000000) != 0;

                        if has_local_color_table {
                            let local_color_table_size = 2u32.pow(((packed & 0b00000111) + 1) as u32);
                            pos += (local_color_table_size as u64) * 3;
                        }

                        // Skip LZW minimum code size
                        if pos < reader.size() {
                            pos += 1;
                        }

                        // Skip image data sub-blocks
                        pos = Self::skip_sub_blocks(reader, pos)?;
                    }
                }
                0x3B => { // Trailer - end of GIF
                    break;
                }
                _ => {
                    pos += 1;
                }
            }
        }

        Ok(BlockScanResult {
            frame_count,
            is_animated,
            comment: if comment.is_empty() { None } else { Some(comment) },
        })
    }

    /// Skips sub-blocks (sequence of sized blocks terminated by 0x00)
    fn skip_sub_blocks(reader: &dyn FileReader, mut pos: u64) -> Result<u64> {
        while pos < reader.size() {
            let block_size = reader.read(pos, 1)?[0];
            pos += 1;
            if block_size == 0 {
                break;
            }
            pos += block_size as u64;
        }
        Ok(pos)
    }

    /// Reads comment blocks and appends to comment string
    fn read_comment_blocks(reader: &dyn FileReader, mut pos: u64, comment: &mut String) -> Result<u64> {
        while pos < reader.size() {
            let block_size = reader.read(pos, 1)?[0];
            pos += 1;
            if block_size == 0 {
                break;
            }
            if pos + (block_size as u64) <= reader.size() {
                if let Ok(data) = reader.read(pos, block_size as usize) {
                    if let Ok(text) = String::from_utf8(data.to_vec()) {
                        if !comment.is_empty() {
                            comment.push(' ');
                        }
                        comment.push_str(&text);
                    }
                }
            }
            pos += block_size as u64;
        }
        Ok(pos)
    }
}

/// Logical Screen Descriptor data structure
#[derive(Debug, Default)]
struct LogicalScreenDescriptor {
    width: u16,
    height: u16,
    global_color_table_flag: bool,
    color_resolution: u8,
    global_color_table_size: u32,
    background_color_index: u8,
    pixel_aspect_ratio: u8,
}

/// Result of scanning GIF blocks
#[derive(Debug)]
struct BlockScanResult {
    frame_count: u32,
    is_animated: bool,
    comment: Option<String>,
}

impl FormatParser for GIFParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid GIF signature"));
        }

        let mut metadata = MetadataMap::new();

        // Basic file information
        metadata.insert("FileType".to_string(), TagValue::String("GIF".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        let version = Self::read_version(reader)?;
        metadata.insert(
            "GIFVersion".to_string(),
            TagValue::String(version.to_string()),
        );

        // Logical Screen Descriptor fields
        let lsd = Self::read_logical_screen_descriptor(reader)?;

        metadata.insert(
            "ImageWidth".to_string(),
            TagValue::String(lsd.width.to_string()),
        );
        metadata.insert(
            "ImageHeight".to_string(),
            TagValue::String(lsd.height.to_string()),
        );

        metadata.insert(
            "ColorResolution".to_string(),
            TagValue::Integer(lsd.color_resolution as i64),
        );

        metadata.insert(
            "HasGlobalColorTable".to_string(),
            TagValue::String(if lsd.global_color_table_flag { "yes" } else { "no" }.to_string()),
        );

        if lsd.global_color_table_flag {
            metadata.insert(
                "GlobalColorTableSize".to_string(),
                TagValue::Integer(lsd.global_color_table_size as i64),
            );
        }

        metadata.insert(
            "BackgroundColorIndex".to_string(),
            TagValue::Integer(lsd.background_color_index as i64),
        );

        if lsd.pixel_aspect_ratio != 0 {
            metadata.insert(
                "PixelAspectRatio".to_string(),
                TagValue::Integer(lsd.pixel_aspect_ratio as i64),
            );
        }

        // Scan for extensions and image blocks
        let scan_result = Self::scan_blocks(reader)?;

        metadata.insert(
            "FrameCount".to_string(),
            TagValue::Integer(scan_result.frame_count as i64),
        );

        if scan_result.is_animated {
            metadata.insert(
                "Animation".to_string(),
                TagValue::String("yes".to_string()),
            );
        }

        if let Some(comment) = scan_result.comment {
            metadata.insert(
                "Comment".to_string(),
                TagValue::String(comment),
            );
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::GIF)
    }
}

/// Parses metadata from GIF files.
pub fn parse_gif_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = GIFParser;
    parser.parse(reader).map_err(|e| e.to_string())
}
