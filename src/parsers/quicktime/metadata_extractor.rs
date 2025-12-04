//! QuickTime/MP4 metadata extraction.
//!
//! This module extracts metadata from QuickTime and MP4 files from various locations:
//! 1. Classic QuickTime user data atoms (©xxx in moov→udta)
//! 2. iTunes-style metadata (moov→udta→meta)
//! 3. MP4 metadata with keys/ilst (moov→meta→keys + moov→meta→ilst)
//! 4. XMP metadata in uuid atoms
//! 5. HEIF/HEIC EXIF data from meta→iinf/iloc referencing mdat

use super::atom_parser::Atom;
use crate::core::{FileReader, MetadataMap, TagValue};
use crate::parsers::tiff::ifd_parser::{parse_ifd, ByteOrder};
use crate::tag_db::lookup_tag_name;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io;

/// Helper for reading big-endian integers from byte slices
struct BigEndianReader<'a>(&'a [u8]);

impl<'a> BigEndianReader<'a> {
    fn u16_at(&self, offset: usize) -> Option<u16> {
        self.0
            .get(offset..offset + 2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]))
    }

    fn u32_at(&self, offset: usize) -> Option<u32> {
        self.0
            .get(offset..offset + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64_at(&self, offset: usize) -> Option<u64> {
        self.0
            .get(offset..offset + 8)
            .map(|b| u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }

    fn i32_at(&self, offset: usize) -> Option<i32> {
        self.0
            .get(offset..offset + 4)
            .map(|b| i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i16_at(&self, offset: usize) -> Option<i16> {
        self.0
            .get(offset..offset + 2)
            .map(|b| i16::from_be_bytes([b[0], b[1]]))
    }

    fn str_at(&self, offset: usize, len: usize) -> Option<&'a str> {
        self.0
            .get(offset..offset + len)
            .and_then(|b| std::str::from_utf8(b).ok())
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

/// Helper for reading integers with configurable byte order (for EXIF parsing)
struct EndianReader<'a> {
    data: &'a [u8],
    order: ByteOrder,
}

impl<'a> EndianReader<'a> {
    fn new(data: &'a [u8], order: ByteOrder) -> Self {
        Self { data, order }
    }

    fn u16_at(&self, offset: usize) -> Option<u16> {
        self.data.get(offset..offset + 2).map(|b| match self.order {
            ByteOrder::LittleEndian => u16::from_le_bytes([b[0], b[1]]),
            ByteOrder::BigEndian => u16::from_be_bytes([b[0], b[1]]),
        })
    }

    fn u32_at(&self, offset: usize) -> Option<u32> {
        self.data.get(offset..offset + 4).map(|b| match self.order {
            ByteOrder::LittleEndian => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            ByteOrder::BigEndian => u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
        })
    }

    fn u64_at(&self, offset: usize) -> Option<u64> {
        self.data.get(offset..offset + 8).map(|b| match self.order {
            ByteOrder::LittleEndian => {
                u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
            }
            ByteOrder::BigEndian => {
                u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
            }
        })
    }

    fn i16_at(&self, offset: usize) -> Option<i16> {
        self.data.get(offset..offset + 2).map(|b| match self.order {
            ByteOrder::LittleEndian => i16::from_le_bytes([b[0], b[1]]),
            ByteOrder::BigEndian => i16::from_be_bytes([b[0], b[1]]),
        })
    }

    fn i32_at(&self, offset: usize) -> Option<i32> {
        self.data.get(offset..offset + 4).map(|b| match self.order {
            ByteOrder::LittleEndian => i32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            ByteOrder::BigEndian => i32::from_be_bytes([b[0], b[1], b[2], b[3]]),
        })
    }

    fn f32_at(&self, offset: usize) -> Option<f32> {
        self.u32_at(offset).map(f32::from_bits)
    }

    fn f64_at(&self, offset: usize) -> Option<f64> {
        self.u64_at(offset).map(f64::from_bits)
    }

    fn rational_at(&self, offset: usize) -> Option<f64> {
        let num = self.u32_at(offset)?;
        let den = self.u32_at(offset + 4)?;
        if den != 0 {
            Some(num as f64 / den as f64)
        } else {
            None
        }
    }

    fn srational_at(&self, offset: usize) -> Option<f64> {
        let num = self.i32_at(offset)?;
        let den = self.i32_at(offset + 4)?;
        if den != 0 {
            Some(num as f64 / den as f64)
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

/// Extract all metadata from QuickTime/MP4 atoms
pub fn extract_metadata(root_atoms: &[Atom]) -> Result<MetadataMap, String> {
    let mut metadata = MetadataMap::with_capacity(50);

    // Extract file-level metadata from ftyp and mdat atoms
    extract_file_level_metadata(root_atoms, &mut metadata);

    // Find the moov atom (movie container) - optional for HEIF/HIF files
    let moov = root_atoms
        .iter()
        .find(|atom| atom.atom_type.matches("moov"));

    // If we have a moov atom, extract traditional QuickTime/MP4 metadata
    if let Some(moov) = moov {
        // Extract movie header metadata (mvhd)
        if let Some(mvhd) = moov.find_child("mvhd") {
            extract_movie_header(&mvhd, &mut metadata)?;
        }

        // Extract track headers (tkhd) from all trak atoms
        if let Ok(children) = moov.parse_children() {
            let trak_atoms: Vec<_> = children
                .iter()
                .filter(|a| a.atom_type.matches("trak"))
                .collect();

            for (index, trak) in trak_atoms.iter().enumerate() {
                if let Some(tkhd) = trak.find_child("tkhd") {
                    let _ = extract_track_header(&tkhd, &mut metadata, index);
                }
            }
        }

        // Extract from all possible locations
        if let Some(udta) = moov.find_child("udta") {
            // Extract handler metadata (hdlr) - may be in udta or udta→meta
            if let Some(meta) = udta.find_child("meta") {
                // Parse meta children (skip version/flags)
                let meta_data = if meta.data.len() >= 4 && meta.data[0..4] == [0, 0, 0, 0] {
                    &meta.data[4..]
                } else {
                    meta.data
                };

                if let Ok((_, atoms)) = super::atom_parser::parse_atoms(meta_data) {
                    if let Some(hdlr) = atoms.iter().find(|a| a.atom_type.matches("hdlr")) {
                        extract_handler_metadata(hdlr, &mut metadata)?;
                    }
                }
            }

            // Also check for hdlr directly in udta
            if let Some(hdlr) = udta.find_child("hdlr") {
                extract_handler_metadata(&hdlr, &mut metadata)?;
            }
            // Extract classic QuickTime user data (©xxx atoms)
            extract_user_data_atoms(&udta, &mut metadata)?;

            // Extract iTunes-style metadata (udta→meta)
            if let Some(meta) = udta.find_child("meta") {
                extract_itunes_metadata(&meta, &mut metadata)?;
            }
        }

        // Extract MP4 metadata (moov→meta with keys/ilst)
        if let Some(meta) = moov.find_child("meta") {
            extract_mp4_metadata(&meta, &mut metadata)?;
        }
    }

    // HEIF/HIF files have a root-level meta atom instead of moov
    // Extract metadata from root-level meta atom if present
    if let Some(meta) = root_atoms.iter().find(|a| a.atom_type.matches("meta")) {
        // Extract handler metadata from root-level meta
        let meta_data = if meta.data.len() >= 4 && meta.data[0..4] == [0, 0, 0, 0] {
            &meta.data[4..]
        } else {
            meta.data
        };

        if let Ok((_, atoms)) = super::atom_parser::parse_atoms(meta_data) {
            if let Some(hdlr) = atoms.iter().find(|a| a.atom_type.matches("hdlr")) {
                extract_handler_metadata(hdlr, &mut metadata)?;
            }
        }

        // Extract HEIF-specific metadata (iinf, iloc, etc.) including EXIF data
        extract_heif_metadata(meta, root_atoms, &mut metadata)?;
    }

    // If no metadata was extracted, return error
    if metadata.is_empty() {
        Err("No metadata found in QuickTime/MP4 file".to_string())
    } else {
        Ok(metadata)
    }
}

/// Extract file-level metadata from ftyp and mdat atoms
fn extract_file_level_metadata(root_atoms: &[Atom], metadata: &mut MetadataMap) {
    // Extract file type information from ftyp atom
    if let Some(ftyp) = root_atoms.iter().find(|a| a.atom_type.matches("ftyp")) {
        if ftyp.data.len() >= 8 {
            // Major brand (4 bytes)
            let brand_bytes = &ftyp.data[0..4];
            if let Ok(brand) = std::str::from_utf8(brand_bytes) {
                let brand_desc = match brand {
                    "isom" => "MP4 Base Media v1 [IS0 14496-12:2003]",
                    "iso2" => "MP4 Base Media v2",
                    "mp41" => "MP4 v1 [ISO 14496-1:ch13]",
                    "mp42" => "MP4 v2 [ISO 14496-14]",
                    "M4A " | "M4B " => "Apple iTunes AAC-LC (.M4A) Audio",
                    "M4V " => "Apple iTunes Video (.M4V) Video",
                    "qt  " => "Apple QuickTime (.MOV/QT)",
                    "mp4 " => "MP4 Base Media v1 [IS0 14496-12:2003]",
                    _ => brand,
                };
                metadata.insert(
                    "QuickTime:MajorBrand".to_string(),
                    TagValue::String(brand_desc.to_string()),
                );
            }

            // Minor version (4 bytes)
            if ftyp.data.len() >= 8 {
                let version_bytes = &ftyp.data[4..8];
                let minor_version = u32::from_be_bytes([
                    version_bytes[0],
                    version_bytes[1],
                    version_bytes[2],
                    version_bytes[3],
                ]);
                let version_str = format!(
                    "{}.{}.{}",
                    (minor_version >> 16) & 0xFF,
                    (minor_version >> 8) & 0xFF,
                    minor_version & 0xFF
                );
                metadata.insert(
                    "QuickTime:MinorVersion".to_string(),
                    TagValue::String(version_str),
                );
            }

            // Compatible brands (remaining bytes, each 4 bytes)
            if ftyp.data.len() > 8 {
                let mut compatible_brands = Vec::new();
                let mut offset = 8;
                while offset + 4 <= ftyp.data.len() {
                    if let Ok(brand) = std::str::from_utf8(&ftyp.data[offset..offset + 4]) {
                        compatible_brands.push(TagValue::String(brand.to_string()));
                    }
                    offset += 4;
                }
                if !compatible_brands.is_empty() {
                    metadata.insert(
                        "QuickTime:CompatibleBrands".to_string(),
                        TagValue::Array(compatible_brands),
                    );
                }
            }
        }
    }

    // Extract media data offset and size from mdat atom
    // We need to track position in the original file
    let mut offset = 0u64;
    for atom in root_atoms {
        if atom.atom_type.matches("mdat") {
            metadata.insert(
                "QuickTime:MediaDataSize".to_string(),
                TagValue::Integer(atom.data.len() as i64),
            );
            metadata.insert(
                "QuickTime:MediaDataOffset".to_string(),
                TagValue::Integer((offset + 8) as i64), // +8 for atom header
            );
            break;
        }
        // Calculate atom size (8-byte header + data length)
        offset += 8 + atom.data.len() as u64;
    }
}

/// Extract movie header metadata from mvhd atom
fn extract_movie_header(mvhd: &Atom, metadata: &mut MetadataMap) -> Result<(), String> {
    if mvhd.data.len() < 100 {
        return Ok(());
    }

    let r = BigEndianReader(mvhd.data);
    let version = mvhd.data[0];

    // Parse time fields based on version (v0: 32-bit, v1: 64-bit)
    let (creation_time, modification_time, timescale, duration, rate_offset) = if version == 1 {
        if r.len() < 32 {
            return Ok(());
        }
        (
            r.u64_at(4).unwrap_or(0),
            r.u64_at(12).unwrap_or(0),
            r.u32_at(20).unwrap_or(0),
            r.u64_at(24).unwrap_or(0),
            32usize,
        )
    } else {
        (
            r.u32_at(4).unwrap_or(0) as u64,
            r.u32_at(8).unwrap_or(0) as u64,
            r.u32_at(12).unwrap_or(0),
            r.u32_at(16).unwrap_or(0) as u64,
            20usize,
        )
    };

    metadata.insert(
        "QuickTime:MovieHeaderVersion".to_string(),
        TagValue::Integer(version as i64),
    );

    // Add both legacy CreateDate/ModifyDate and new MediaCreateDate/MediaModifyDate
    let create_date_str = mac_time_to_string(creation_time);
    let modify_date_str = mac_time_to_string(modification_time);

    metadata.insert(
        "QuickTime:CreateDate".to_string(),
        TagValue::String(create_date_str.clone()),
    );
    metadata.insert(
        "QuickTime:MediaCreateDate".to_string(),
        TagValue::String(create_date_str),
    );
    metadata.insert(
        "QuickTime:ModifyDate".to_string(),
        TagValue::String(modify_date_str.clone()),
    );
    metadata.insert(
        "QuickTime:MediaModifyDate".to_string(),
        TagValue::String(modify_date_str),
    );
    metadata.insert(
        "QuickTime:TimeScale".to_string(),
        TagValue::Integer(timescale as i64),
    );

    let duration_sec = if timescale > 0 {
        duration as f64 / timescale as f64
    } else {
        0.0
    };
    metadata.insert(
        "QuickTime:Duration".to_string(),
        TagValue::String(format!("{:.2} s", duration_sec)),
    );

    // Preferred rate (fixed-point 16.16)
    if let Some(rate) = r.i32_at(rate_offset) {
        metadata.insert(
            "QuickTime:PreferredRate".to_string(),
            TagValue::Integer((rate as f64 / 65536.0) as i64),
        );
    }

    // Preferred volume (fixed-point 8.8)
    if let Some(volume) = r.i16_at(rate_offset + 4) {
        metadata.insert(
            "QuickTime:PreferredVolume".to_string(),
            TagValue::String(format!("{:.2}%", (volume as f64 / 256.0) * 100.0)),
        );
    }

    // Matrix structure (9 x 4 bytes)
    let matrix_offset = if version == 1 { rate_offset + 16 } else { 36 };
    if r.len() >= matrix_offset + 36 {
        let matrix: Vec<i32> = (0..9)
            .filter_map(|i| r.i32_at(matrix_offset + i * 4))
            .collect();
        if matrix.len() == 9 {
            let matrix_str = format!(
                "{} {} {} {} {} {} {} {} {}",
                matrix[0] / 65536,
                matrix[1] / 65536,
                matrix[2] / 65536,
                matrix[3] / 65536,
                matrix[4] / 65536,
                matrix[5] / 65536,
                matrix[6] / 1073741824,
                matrix[7] / 1073741824,
                matrix[8] / 1073741824
            );
            metadata.insert(
                "QuickTime:MatrixStructure".to_string(),
                TagValue::String(matrix_str),
            );
        }
    }

    // Time fields (preview, poster, selection, current)
    let time_offset = if version == 1 { rate_offset + 52 } else { 72 };
    let ts = timescale.max(1);
    let time_fields = [
        ("PreviewTime", 0),
        ("PreviewDuration", 4),
        ("PosterTime", 8),
        ("SelectionTime", 12),
        ("SelectionDuration", 16),
        ("CurrentTime", 20),
    ];
    for (name, offset) in time_fields {
        if let Some(val) = r.u32_at(time_offset + offset) {
            metadata.insert(
                format!("QuickTime:{}", name),
                TagValue::String(format!("{} s", val / ts)),
            );
        }
    }

    // Next track ID
    let next_track_offset = if version == 1 { time_offset + 24 } else { 96 };
    if let Some(next_track_id) = r.u32_at(next_track_offset) {
        metadata.insert(
            "QuickTime:NextTrackID".to_string(),
            TagValue::Integer(next_track_id as i64),
        );
    }

    Ok(())
}

/// Extract track header metadata from tkhd atom
fn extract_track_header(
    tkhd: &Atom,
    metadata: &mut MetadataMap,
    track_index: usize,
) -> Result<(), String> {
    if tkhd.data.len() < 84 {
        return Ok(());
    }

    let r = BigEndianReader(tkhd.data);
    let version = tkhd.data[0];

    // Parse time fields based on version (v0: 32-bit, v1: 64-bit)
    let (creation_time, modification_time) = if version == 1 {
        if r.len() < 20 {
            return Ok(());
        }
        (r.u64_at(4).unwrap_or(0), r.u64_at(12).unwrap_or(0))
    } else {
        (
            r.u32_at(4).unwrap_or(0) as u64,
            r.u32_at(8).unwrap_or(0) as u64,
        )
    };

    // Add track-specific timestamp tags
    let create_date_str = mac_time_to_string(creation_time);
    let modify_date_str = mac_time_to_string(modification_time);

    // Use track index for tag names if we have multiple tracks
    let track_suffix = if track_index > 0 {
        format!("_{}", track_index + 1)
    } else {
        String::new()
    };

    metadata.insert(
        format!("QuickTime:TrackCreateDate{}", track_suffix),
        TagValue::String(create_date_str),
    );
    metadata.insert(
        format!("QuickTime:TrackModifyDate{}", track_suffix),
        TagValue::String(modify_date_str),
    );

    Ok(())
}

/// Extract handler metadata from hdlr atom
fn extract_handler_metadata(hdlr: &Atom, metadata: &mut MetadataMap) -> Result<(), String> {
    if hdlr.data.len() < 24 {
        return Ok(());
    }

    // Skip version/flags (4 bytes) and pre-defined (4 bytes)
    let handler_type = &hdlr.data[8..12];
    if let Ok(handler_str) = std::str::from_utf8(handler_type) {
        let handler_desc = match handler_str {
            "mdir" => "Metadata",
            "vide" => "Video Track",
            "soun" => "Audio Track",
            "hint" => "Hint Track",
            "meta" => "Timed Metadata",
            "text" => "Text Track",
            "tmcd" => "Time Code",
            _ => handler_str,
        };
        metadata.insert(
            "QuickTime:HandlerType".to_string(),
            TagValue::String(handler_desc.to_string()),
        );
    }

    // Handler vendor ID (4 bytes at offset 12, but it's actually 'reserved' fields)
    // The real vendor/manufacturer is at offset 16-20 in some implementations
    // or it's set to "appl" for Apple
    // Let's check multiple offsets
    if hdlr.data.len() >= 16 {
        // Try reserved fields first (offset 12-16) - often contains "appl" for Apple
        let vendor_bytes = &hdlr.data[12..16];
        if let Ok(vendor) = std::str::from_utf8(vendor_bytes) {
            let trimmed = vendor.trim_matches('\0').trim();
            if !trimmed.is_empty() && trimmed != "\0\0\0\0" {
                let vendor_name = match trimmed {
                    "appl" => "Apple",
                    _ => trimmed,
                };
                metadata.insert(
                    "QuickTime:HandlerVendorID".to_string(),
                    TagValue::String(vendor_name.to_string()),
                );
            }
        }
    }

    Ok(())
}

/// Convert Mac epoch time (seconds since 1904-01-01) to ISO 8601 date string
fn mac_time_to_string(mac_time: u64) -> String {
    // Mac epoch is 1904-01-01 00:00:00 UTC, Unix epoch is 1970-01-01 00:00:00 UTC
    // Difference is 66 years = 2082844800 seconds
    const MAC_EPOCH_OFFSET: i64 = 2082844800;

    if mac_time == 0 {
        return "0000:00:00 00:00:00".to_string();
    }

    let unix_time = mac_time as i64 - MAC_EPOCH_OFFSET;
    if unix_time <= 0 {
        return "1904:01:01 00:00:00".to_string();
    }

    // Convert Unix timestamp to date components
    // This is a simplified calculation for dates after 1970-01-01
    const SECONDS_PER_DAY: i64 = 86400;
    const SECONDS_PER_HOUR: i64 = 3600;
    const SECONDS_PER_MINUTE: i64 = 60;

    let days_since_epoch = unix_time / SECONDS_PER_DAY;
    let remaining_seconds = unix_time % SECONDS_PER_DAY;
    let hours = remaining_seconds / SECONDS_PER_HOUR;
    let minutes = (remaining_seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
    let seconds = remaining_seconds % SECONDS_PER_MINUTE;

    // Simple year/month/day calculation (approximate, good enough for metadata)
    // Using average of 365.25 days per year
    let mut year = 1970;
    let mut days = days_since_epoch;

    // Add years
    while days >= 365 {
        let year_days = if is_leap_year(year) { 366 } else { 365 };
        if days >= year_days {
            days -= year_days;
            year += 1;
        } else {
            break;
        }
    }

    // Calculate month and day
    let (month, day) = days_to_month_day(days as u32, is_leap_year(year));

    format!(
        "{:04}:{:02}:{:02} {:02}:{:02}:{:02}",
        year, month, day, hours, minutes, seconds
    )
}

/// Check if a year is a leap year
fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Convert day of year to month and day
fn days_to_month_day(mut days: u32, is_leap: bool) -> (u32, u32) {
    const MONTH_DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    const MONTH_DAYS_LEAP: [u32; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let month_days = if is_leap {
        &MONTH_DAYS_LEAP
    } else {
        &MONTH_DAYS
    };

    for (i, &month_len) in month_days.iter().enumerate() {
        if days < month_len {
            return ((i + 1) as u32, days + 1);
        }
        days -= month_len;
    }

    // Fallback for invalid input
    (12, 31)
}

/// Extract classic QuickTime user data atoms (©xxx)
fn extract_user_data_atoms(udta: &Atom, metadata: &mut MetadataMap) -> Result<(), String> {
    let children = udta.parse_children().unwrap_or_default();

    for atom in children {
        let atom_bytes = atom.atom_type.as_bytes();

        // QuickTime user data atoms start with © character (0xA9)
        if atom_bytes[0] == 0xA9 {
            if let Some(value) = extract_string_value(atom.data) {
                let suffix = match atom_bytes {
                    b"\xa9nam" => Some("Title"),
                    b"\xa9ART" => Some("Artist"),
                    b"\xa9alb" => Some("Album"),
                    b"\xa9day" => Some("Year"),
                    b"\xa9cmt" => Some("Comment"),
                    b"\xa9cpy" => Some("Copyright"),
                    b"\xa9gen" => Some("Genre"),
                    b"\xa9too" => Some("Encoder"),
                    b"\xa9des" => Some("Description"),
                    b"\xa9dir" => Some("Director"),
                    b"\xa9prd" => Some("Producer"),
                    b"\xa9prf" => Some("Performers"),
                    _ => None,
                };

                if let Some(suffix) = suffix {
                    metadata.insert(
                        format!("QuickTime:{}", suffix),
                        TagValue::new_string(value.clone()),
                    );
                    metadata.insert(format!("UserData:{}", suffix), TagValue::new_string(value));
                }
            }
        }
    }

    Ok(())
}

/// Extract iTunes-style metadata from meta atom
fn extract_itunes_metadata(meta: &Atom, metadata: &mut MetadataMap) -> Result<(), String> {
    // Meta atoms have a 4-byte version/flags header before the child atoms
    // We need to skip this header to parse the children correctly
    let data = if meta.data.len() >= 4 && meta.data[0..4] == [0, 0, 0, 0] {
        &meta.data[4..]
    } else {
        meta.data
    };

    // Parse children from the adjusted data
    let children = match super::atom_parser::parse_atoms(data) {
        Ok((_, atoms)) => atoms,
        Err(_) => return Ok(()), // Gracefully handle parsing errors
    };

    // Find ilst atom among the children
    let ilst = children.iter().find(|atom| atom.atom_type.matches("ilst"));

    // iTunes metadata is in the ilst (item list) atom
    if let Some(ilst) = ilst {
        let items = ilst.parse_children().unwrap_or_default();

        for item in items {
            let atom_bytes = item.atom_type.as_bytes();

            // Each item contains a data atom
            if let Some(data_atom) = item.find_child("data") {
                if let Some(value) = extract_itunes_data_value(data_atom.data) {
                    let mut add_year_tag = false;
                    let tag_name: Cow<'static, str> = match atom_bytes {
                        b"\xa9nam" => Cow::Borrowed("ItemList:Title"),
                        b"\xa9ART" => Cow::Borrowed("ItemList:Artist"),
                        b"\xa9alb" => Cow::Borrowed("ItemList:Album"),
                        b"\xa9day" => {
                            add_year_tag = true;
                            Cow::Borrowed("ItemList:ContentCreateDate")
                        }
                        b"\xa9cmt" => Cow::Borrowed("ItemList:Comment"),
                        b"\xa9gen" => Cow::Borrowed("ItemList:Genre"),
                        b"\xa9too" => Cow::Borrowed("ItemList:Encoder"),
                        b"aART" => Cow::Borrowed("ItemList:AlbumArtist"),
                        b"\xa9wrt" => Cow::Borrowed("ItemList:Composer"),
                        b"\xa9grp" => Cow::Borrowed("ItemList:Grouping"),
                        b"trkn" => Cow::Borrowed("ItemList:TrackNumber"),
                        b"disk" => Cow::Borrowed("ItemList:DiscNumber"),
                        b"cprt" | b"\xa9cpy" => Cow::Borrowed("ItemList:Copyright"),
                        _ => {
                            if let Ok(s) = std::str::from_utf8(atom_bytes) {
                                Cow::Owned(format!("ItemList:{}", s))
                            } else {
                                Cow::Owned(format!(
                                    "ItemList:{:02X}{:02X}{:02X}{:02X}",
                                    atom_bytes[0], atom_bytes[1], atom_bytes[2], atom_bytes[3]
                                ))
                            }
                        }
                    };

                    metadata.insert(tag_name.into_owned(), value.clone());

                    if add_year_tag {
                        if let TagValue::String(ref text) = value {
                            if text.len() >= 4 {
                                let year = text.chars().take(4).collect::<String>();
                                metadata.insert(
                                    "ItemList:Year".to_string(),
                                    TagValue::new_string(year),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Extract MP4 metadata using keys/ilst atoms
fn extract_mp4_metadata(meta: &Atom, metadata: &mut MetadataMap) -> Result<(), String> {
    // MP4 metadata uses a keys atom to define key names
    // and an ilst atom to store the values
    let keys_atom = meta.find_child("keys");
    let ilst_atom = meta.find_child("ilst");

    if let (Some(keys), Some(ilst)) = (keys_atom, ilst_atom) {
        // Parse the keys
        let key_map = parse_mp4_keys(keys.data)?;

        // Parse the ilst items
        let items = ilst.parse_children().unwrap_or_default();

        for item in items {
            // MP4 ilst uses numeric atom types that correspond to key indices
            // The atom type is a 4-byte integer (index into keys)
            let atom_type_bytes = item.atom_type.as_bytes();

            // Try to interpret as a big-endian integer
            let key_index = u32::from_be_bytes(*atom_type_bytes);

            if let Some(data_atom) = item.find_child("data") {
                if let Some(value) = extract_itunes_data_value(data_atom.data) {
                    // Look up the key name
                    if let Some(key_name) = key_map.get(&key_index) {
                        // Map Apple-specific keys to standard tag names
                        let tag_name = map_apple_key_to_tag(key_name);
                        metadata.insert(tag_name, value.clone());

                        // Special handling for GPS coordinates
                        if key_name == "com.apple.quicktime.location.ISO6709" {
                            if let TagValue::String(ref gps_str) = value {
                                if let Some((lat, lon, alt)) = parse_iso6709(gps_str) {
                                    metadata.insert(
                                        "QuickTime:GPSLatitude".to_string(),
                                        TagValue::Float(lat),
                                    );
                                    metadata.insert(
                                        "QuickTime:GPSLongitude".to_string(),
                                        TagValue::Float(lon),
                                    );
                                    if let Some(altitude) = alt {
                                        metadata.insert(
                                            "QuickTime:GPSAltitude".to_string(),
                                            TagValue::Float(altitude),
                                        );
                                    }
                                }
                            }
                        }
                    } else {
                        // Fallback to using the atom type as the tag name
                        let tag_name = format!("MP4:{}", item.atom_type.as_str());
                        metadata.insert(tag_name, value);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Map Apple-specific mdta keys to standard QuickTime tag names
fn map_apple_key_to_tag(key_name: &str) -> String {
    match key_name {
        "com.apple.quicktime.location.ISO6709" => "QuickTime:GPSCoordinates".to_string(),
        "com.apple.quicktime.location.accuracy.horizontal" => {
            "QuickTime:LocationAccuracyHorizontal".to_string()
        }
        "com.apple.quicktime.location.role" => "QuickTime:LocationRole".to_string(),
        "com.apple.quicktime.creationLocation.name" => "QuickTime:CreationLocationName".to_string(),
        "com.apple.quicktime.make" => "QuickTime:Make".to_string(),
        "com.apple.quicktime.model" => "QuickTime:Model".to_string(),
        "com.apple.quicktime.software" => "QuickTime:Software".to_string(),
        "com.apple.quicktime.creationdate" => "QuickTime:ContentCreateDate".to_string(),
        _ => format!("QuickTime:{}", key_name),
    }
}

/// Parse MP4 keys atom to build a map of key indices to key names
fn parse_mp4_keys(data: &[u8]) -> Result<HashMap<u32, String>, String> {
    let mut keys = HashMap::new();

    // Keys atom format:
    // 4 bytes: version + flags
    // 4 bytes: entry count
    // For each entry:
    //   4 bytes: key size
    //   4 bytes: key namespace (e.g., "mdta")
    //   N bytes: key value

    if data.len() < 8 {
        return Ok(keys);
    }

    let entry_count = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let mut offset = 8;
    let mut index = 1; // Keys are 1-indexed

    for _ in 0..entry_count {
        if offset + 8 > data.len() {
            break;
        }

        let key_size = u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;

        if key_size < 8 || offset + key_size > data.len() {
            break;
        }

        // Skip namespace (4 bytes)
        let key_data = &data[offset + 8..offset + key_size];
        if let Ok(key_name) = std::str::from_utf8(key_data) {
            keys.insert(index, key_name.to_string());
        }

        offset += key_size;
        index += 1;
    }

    Ok(keys)
}

/// Extract string value from QuickTime user data atom
fn extract_string_value(data: &[u8]) -> Option<String> {
    // QuickTime user data format:
    // 2 bytes: data size
    // 2 bytes: language code
    // N bytes: string data

    if data.len() < 4 {
        return None;
    }

    let size = u16::from_be_bytes([data[0], data[1]]) as usize;
    // Skip language code (2 bytes)
    let text_start = 4;

    if text_start >= data.len() {
        return None;
    }

    let text_data = &data[text_start..data.len().min(text_start + size)];
    String::from_utf8(text_data.to_vec()).ok()
}

/// Extract value from iTunes data atom
fn extract_itunes_data_value(data: &[u8]) -> Option<TagValue> {
    // iTunes data atom format:
    // 4 bytes: version + flags (type indicator)
    // 4 bytes: reserved (usually 0)
    // N bytes: actual data

    if data.len() < 8 {
        return None;
    }

    let type_indicator = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let value_data = &data[8..];

    match type_indicator {
        1 => {
            // UTF-8 text
            String::from_utf8(value_data.to_vec())
                .ok()
                .map(TagValue::String)
        }
        2 => {
            // UTF-16 text
            decode_utf16(value_data).map(TagValue::String)
        }
        21 => {
            // Signed integer (1, 2, 3, or 4 bytes)
            match value_data.len() {
                1 => Some(TagValue::Integer(value_data[0] as i64)),
                2 => Some(TagValue::Integer(
                    i16::from_be_bytes([value_data[0], value_data[1]]) as i64,
                )),
                4 => Some(TagValue::Integer(i32::from_be_bytes([
                    value_data[0],
                    value_data[1],
                    value_data[2],
                    value_data[3],
                ]) as i64)),
                _ => None,
            }
        }
        13 | 14 => {
            // JPEG or PNG image data
            Some(TagValue::Binary(value_data.to_vec()))
        }
        _ => {
            // Unknown type, try as string
            String::from_utf8(value_data.to_vec())
                .ok()
                .map(TagValue::String)
        }
    }
}

/// Decode UTF-16 big-endian text
fn decode_utf16(data: &[u8]) -> Option<String> {
    if !data.len().is_multiple_of(2) {
        return None;
    }

    let utf16_chars: Vec<u16> = data
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&utf16_chars).ok()
}

/// Parse ISO 6709 GPS coordinate string
/// Format: +DD.DDDD+DDD.DDDD+AAA.AAA/ or variations
/// Returns (latitude, longitude, altitude)
fn parse_iso6709(gps_string: &str) -> Option<(f64, f64, Option<f64>)> {
    let s = gps_string.trim();
    if s.is_empty() {
        return None;
    }

    // Remove trailing slash if present
    let s = s.trim_end_matches('/');

    // Parse latitude (starts with + or -)
    let (lat_str, rest) = if let Some(pos) = s[1..].find(&['+', '-'][..]) {
        s.split_at(pos + 1)
    } else {
        return None;
    };

    let latitude = lat_str.parse::<f64>().ok()?;

    // Parse longitude (next + or -)
    let (lon_str, alt_str) = if let Some(pos) = rest[1..].find(&['+', '-'][..]) {
        rest.split_at(pos + 1)
    } else {
        (rest, "")
    };

    let longitude = lon_str.parse::<f64>().ok()?;

    // Parse altitude if present
    let altitude = if !alt_str.is_empty() {
        alt_str.parse::<f64>().ok()
    } else {
        None
    };

    Some((latitude, longitude, altitude))
}

/// Extract HEIF-specific metadata from meta atom including EXIF data
fn extract_heif_metadata(
    meta: &Atom,
    root_atoms: &[Atom],
    metadata: &mut MetadataMap,
) -> Result<(), String> {
    let meta_data = skip_version_flags(meta.data);
    let children = match super::atom_parser::parse_atoms(meta_data) {
        Ok((_, atoms)) => atoms,
        Err(_) => return Ok(()),
    };

    // Find Exif item ID from iinf atom
    let exif_item_id = find_exif_item_id(&children, metadata);

    // Parse iloc to build item locations
    let item_locations = parse_iloc_locations(&children);

    // Extract image dimensions from ispe atoms
    extract_ispe_dimensions(&children, metadata);

    // Extract EXIF data from mdat if we found an Exif item
    if let Some(id) = exif_item_id {
        if let Some(&(offset, length)) = item_locations.get(&id) {
            extract_exif_from_mdat(root_atoms, offset, length, metadata);
        }
    }

    Ok(())
}

/// Skip version/flags header if present in atom data
fn skip_version_flags(data: &[u8]) -> &[u8] {
    if data.len() >= 4 && data[0..4] == [0, 0, 0, 0] {
        &data[4..]
    } else {
        data
    }
}

/// Find the Exif item ID from iinf (item information) atom
fn find_exif_item_id(children: &[Atom], metadata: &mut MetadataMap) -> Option<u16> {
    let iinf = children.iter().find(|a| a.atom_type.matches("iinf"))?;
    if iinf.data.len() < 6 {
        return None;
    }

    let r = BigEndianReader(iinf.data);
    let version = iinf.data[0];

    let (entry_count, entries_offset) = if version == 0 {
        (r.u16_at(4)? as u32, 6usize)
    } else if iinf.data.len() >= 8 {
        (r.u32_at(4)?, 8usize)
    } else {
        return None;
    };

    metadata.insert(
        "HEIF:ItemCount".to_string(),
        TagValue::Integer(entry_count as i64),
    );

    // Parse infe atoms to find Exif item
    let (_, infe_atoms) = super::atom_parser::parse_atoms(&iinf.data[entries_offset..]).ok()?;
    for infe in infe_atoms.iter().filter(|a| a.atom_type.matches("infe")) {
        if infe.data.len() >= 12 {
            let item_id = u16::from_be_bytes([infe.data[4], infe.data[5]]);
            if &infe.data[8..12] == b"Exif" {
                return Some(item_id);
            }
        }
    }
    None
}

/// Parse iloc (item location) atom to build item locations map
fn parse_iloc_locations(children: &[Atom]) -> HashMap<u16, (u64, u64)> {
    let mut locations = HashMap::new();

    let Some(iloc) = children.iter().find(|a| a.atom_type.matches("iloc")) else {
        return locations;
    };

    if iloc.data.len() < 8 {
        return locations;
    }

    let r = BigEndianReader(iloc.data);
    let version = iloc.data[0];
    let offset_size = ((iloc.data[4] >> 4) & 0x0F) as usize;
    let length_size = (iloc.data[4] & 0x0F) as usize;
    let base_offset_size = ((iloc.data[5] >> 4) & 0x0F) as usize;

    let (item_count, mut pos) = if version < 2 {
        (r.u16_at(6).unwrap_or(0) as u32, 8usize)
    } else if iloc.data.len() >= 10 {
        (r.u32_at(6).unwrap_or(0), 10usize)
    } else {
        return locations;
    };

    for _ in 0..item_count {
        if pos + 2 > iloc.data.len() {
            break;
        }

        // Read item_id based on version
        let item_id = if version < 2 {
            let id = r.u16_at(pos).unwrap_or(0);
            pos += 2;
            id
        } else {
            let id = r.u32_at(pos).unwrap_or(0) as u16;
            pos += 4;
            id
        };

        if version >= 1 {
            pos += 2; // construction_method
        }
        pos += 2; // data_reference_index

        let base_offset = read_variable_size(iloc.data, &mut pos, base_offset_size);

        if pos + 2 > iloc.data.len() {
            break;
        }
        let extent_count = r.u16_at(pos).unwrap_or(0);
        pos += 2;

        if extent_count >= 1 {
            let extent_offset = read_variable_size(iloc.data, &mut pos, offset_size);
            let extent_length = read_variable_size(iloc.data, &mut pos, length_size);
            locations.insert(item_id, (base_offset + extent_offset, extent_length));

            // Skip remaining extents
            for _ in 1..extent_count {
                pos += offset_size + length_size;
            }
        }
    }

    locations
}

/// Extract image dimensions from ispe (image spatial extents) atoms
fn extract_ispe_dimensions(children: &[Atom], metadata: &mut MetadataMap) {
    for atom in children {
        if atom.atom_type.matches("ispe") && atom.data.len() >= 12 {
            let r = BigEndianReader(atom.data);
            if let (Some(width), Some(height)) = (r.u32_at(4), r.u32_at(8)) {
                if !metadata.contains_key("HEIF:ImageWidth") {
                    metadata.insert(
                        "HEIF:ImageWidth".to_string(),
                        TagValue::Integer(width as i64),
                    );
                    metadata.insert(
                        "HEIF:ImageHeight".to_string(),
                        TagValue::Integer(height as i64),
                    );
                }
            }
        }
    }
}

/// Extract EXIF data from mdat atom using iloc offset/length
fn extract_exif_from_mdat(
    root_atoms: &[Atom],
    offset: u64,
    length: u64,
    metadata: &mut MetadataMap,
) {
    let Some(mdat) = root_atoms.iter().find(|a| a.atom_type.matches("mdat")) else {
        return;
    };

    let exif_length = length as usize;

    // Try to find EXIF data with different header size assumptions
    let tiff_data = [8u64, 16u64].iter().find_map(|&header_size| {
        let file_offset: u64 = root_atoms
            .iter()
            .take_while(|a| !a.atom_type.matches("mdat"))
            .map(|a| 8 + a.data.len() as u64)
            .sum();
        let mdat_start = file_offset + header_size;

        if offset >= mdat_start {
            let mdat_offset = (offset - mdat_start) as usize;
            if mdat_offset + exif_length <= mdat.data.len() {
                let exif_data = &mdat.data[mdat_offset..mdat_offset + exif_length];
                if exif_data.len() >= 10 && &exif_data[4..8] == b"Exif" {
                    return Some(&exif_data[10..]);
                }
            }
        }
        None
    });

    // Fallback: try direct offset
    let tiff_data = tiff_data.or_else(|| {
        let off = offset as usize;
        if off + exif_length <= mdat.data.len() {
            let exif_data = &mdat.data[off..off + exif_length];
            if exif_data.len() >= 10 && &exif_data[4..8] == b"Exif" {
                return Some(&exif_data[10..]);
            }
        }
        None
    });

    if let Some(data) = tiff_data {
        let _ = parse_heif_exif_data(data, metadata);
    }
}

/// Helper function to read variable-size integers from iloc data
fn read_variable_size(data: &[u8], pos: &mut usize, size: usize) -> u64 {
    if *pos + size > data.len() {
        return 0;
    }

    let value = match size {
        0 => 0u64,
        1 => data[*pos] as u64,
        2 => u16::from_be_bytes([data[*pos], data[*pos + 1]]) as u64,
        4 => {
            u32::from_be_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]) as u64
        }
        8 => u64::from_be_bytes([
            data[*pos],
            data[*pos + 1],
            data[*pos + 2],
            data[*pos + 3],
            data[*pos + 4],
            data[*pos + 5],
            data[*pos + 6],
            data[*pos + 7],
        ]),
        _ => 0,
    };
    *pos += size;
    value
}

/// Simple in-memory FileReader for EXIF data embedded in HEIF files
struct HeifExifDataReader {
    data: Vec<u8>,
}

impl HeifExifDataReader {
    fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl FileReader for HeifExifDataReader {
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

/// Parse TIFF/EXIF data from HEIF Exif item and insert into metadata
fn parse_heif_exif_data(tiff_data: &[u8], metadata: &mut MetadataMap) -> Result<(), String> {
    if tiff_data.len() < 8 {
        return Err("TIFF data too short".to_string());
    }

    // Detect byte order from TIFF header
    let byte_order = match &tiff_data[0..2] {
        b"II" => ByteOrder::LittleEndian,
        b"MM" => ByteOrder::BigEndian,
        _ => return Err("Invalid TIFF byte order marker".to_string()),
    };

    // Verify TIFF magic number (0x002A)
    let magic = match byte_order {
        ByteOrder::LittleEndian => u16::from_le_bytes([tiff_data[2], tiff_data[3]]),
        ByteOrder::BigEndian => u16::from_be_bytes([tiff_data[2], tiff_data[3]]),
    };

    if magic != 0x002A {
        return Err(format!("Invalid TIFF magic number: 0x{:04X}", magic));
    }

    // Read IFD0 offset
    let ifd_offset = match byte_order {
        ByteOrder::LittleEndian => {
            u32::from_le_bytes([tiff_data[4], tiff_data[5], tiff_data[6], tiff_data[7]])
        }
        ByteOrder::BigEndian => {
            u32::from_be_bytes([tiff_data[4], tiff_data[5], tiff_data[6], tiff_data[7]])
        }
    };

    // Create a reader for the TIFF data
    let exif_reader = HeifExifDataReader::new(tiff_data.to_vec());

    // Track sub-IFD offsets
    let mut exif_ifd_offset = None;
    let mut gps_ifd_offset = None;

    // Parse IFD0
    let ifd0_tags = parse_ifd(&exif_reader, ifd_offset as u64, byte_order)
        .map_err(|e| format!("Failed to parse IFD0: {}", e))?;

    for (tag_id, field_type, value_count, raw_bytes) in &ifd0_tags {
        // Check for ExifIFD pointer (tag 0x8769)
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

        // Check for GPS Sub-IFD pointer (tag 0x8825)
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

        // Convert tag to name and value
        let tag_name = lookup_tag_name(*tag_id, "IFD0");
        let tag_value = raw_bytes_to_tag_value(raw_bytes, *field_type, *value_count, byte_order);
        metadata.insert(tag_name, tag_value);
    }

    // Parse ExifIFD if present
    if let Some(offset) = exif_ifd_offset {
        if let Ok(exif_tags) = parse_ifd(&exif_reader, offset, byte_order) {
            for (tag_id, field_type, value_count, raw_bytes) in exif_tags {
                let tag_name = lookup_tag_name(tag_id, "ExifIFD");
                let tag_value =
                    raw_bytes_to_tag_value(&raw_bytes, field_type, value_count, byte_order);
                metadata.insert(tag_name, tag_value);
            }
        }
    }

    // Parse GPS IFD if present
    if let Some(offset) = gps_ifd_offset {
        if let Ok(gps_tags) = parse_ifd(&exif_reader, offset, byte_order) {
            for (tag_id, field_type, value_count, raw_bytes) in gps_tags {
                let tag_name = lookup_tag_name(tag_id, "GPS");
                let tag_value =
                    raw_bytes_to_tag_value(&raw_bytes, field_type, value_count, byte_order);
                metadata.insert(tag_name, tag_value);
            }
        }
    }

    Ok(())
}

/// Convert raw EXIF bytes to TagValue
fn raw_bytes_to_tag_value(
    bytes: &[u8],
    field_type: u16,
    value_count: u32,
    byte_order: ByteOrder,
) -> TagValue {
    use crate::parsers::common::exif_types::ExifType;

    let r = EndianReader::new(bytes, byte_order);

    let Some(exif_type) = ExifType::from_u16(field_type) else {
        return TagValue::Binary(bytes.to_vec());
    };

    match exif_type {
        ExifType::Byte if !bytes.is_empty() => {
            if value_count == 1 {
                TagValue::Integer(bytes[0] as i64)
            } else {
                TagValue::Binary(bytes.to_vec())
            }
        }
        ExifType::Ascii => {
            let text = String::from_utf8_lossy(bytes);
            TagValue::String(text.trim_end_matches('\0').to_string())
        }
        ExifType::Short if r.len() >= 2 => {
            if value_count == 1 {
                r.u16_at(0)
                    .map(|v| TagValue::Integer(v as i64))
                    .unwrap_or_else(|| TagValue::Binary(bytes.to_vec()))
            } else {
                let values: Vec<_> = (0..value_count as usize)
                    .filter_map(|i| r.u16_at(i * 2).map(|v| v.to_string()))
                    .collect();
                TagValue::String(values.join(" "))
            }
        }
        ExifType::Long if r.len() >= 4 => r
            .u32_at(0)
            .map(|v| TagValue::Integer(v as i64))
            .unwrap_or_else(|| TagValue::Binary(bytes.to_vec())),
        ExifType::Rational if r.len() >= 8 => {
            if value_count == 1 {
                r.rational_at(0)
                    .map(TagValue::Float)
                    .unwrap_or_else(|| TagValue::Binary(bytes.to_vec()))
            } else {
                let values: Vec<_> = (0..value_count as usize)
                    .filter_map(|i| r.rational_at(i * 8).map(|v| format!("{}", v)))
                    .collect();
                TagValue::String(values.join(" "))
            }
        }
        ExifType::SByte if !bytes.is_empty() => TagValue::Integer(bytes[0] as i8 as i64),
        ExifType::Undefined => {
            if bytes
                .iter()
                .all(|&b| b.is_ascii_graphic() || b.is_ascii_whitespace() || b == 0)
            {
                let text = String::from_utf8_lossy(bytes);
                let trimmed = text.trim_end_matches('\0');
                if !trimmed.is_empty() {
                    return TagValue::String(trimmed.to_string());
                }
            }
            TagValue::Binary(bytes.to_vec())
        }
        ExifType::SShort if r.len() >= 2 => r
            .i16_at(0)
            .map(|v| TagValue::Integer(v as i64))
            .unwrap_or_else(|| TagValue::Binary(bytes.to_vec())),
        ExifType::SLong if r.len() >= 4 => r
            .i32_at(0)
            .map(|v| TagValue::Integer(v as i64))
            .unwrap_or_else(|| TagValue::Binary(bytes.to_vec())),
        ExifType::SRational if r.len() >= 8 => r
            .srational_at(0)
            .map(TagValue::Float)
            .unwrap_or_else(|| TagValue::Binary(bytes.to_vec())),
        ExifType::Float if r.len() >= 4 => r
            .f32_at(0)
            .map(|v| TagValue::Float(v as f64))
            .unwrap_or_else(|| TagValue::Binary(bytes.to_vec())),
        ExifType::Double if r.len() >= 8 => r
            .f64_at(0)
            .map(TagValue::Float)
            .unwrap_or_else(|| TagValue::Binary(bytes.to_vec())),
        _ => TagValue::Binary(bytes.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_string_value() {
        // Create QuickTime user data: size=11, lang=0, text="Hello World"
        let data = [
            0x00, 0x0B, // size = 11
            0x00, 0x00, // language = 0
            b'H', b'e', b'l', b'l', b'o', b' ', b'W', b'o', b'r', b'l', b'd',
        ];

        let result = extract_string_value(&data);
        assert_eq!(result, Some("Hello World".to_string()));
    }

    #[test]
    fn test_extract_itunes_utf8_value() {
        // Create iTunes data atom: type=1 (UTF-8), reserved=0, text="Test"
        let data = [
            0x00, 0x00, 0x00, 0x01, // type = 1 (UTF-8)
            0x00, 0x00, 0x00, 0x00, // reserved
            b'T', b'e', b's', b't', // text = "Test"
        ];

        let result = extract_itunes_data_value(&data);
        match result {
            Some(TagValue::String(s)) => assert_eq!(s, "Test"),
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_extract_itunes_integer_value() {
        // Create iTunes data atom: type=21 (signed int), value=42
        let data = [
            0x00, 0x00, 0x00, 0x15, // type = 21 (signed int)
            0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x2A, // value = 42
        ];

        let result = extract_itunes_data_value(&data);
        match result {
            Some(TagValue::Integer(i)) => assert_eq!(i, 42),
            _ => panic!("Expected integer value"),
        }
    }

    #[test]
    fn test_decode_utf16() {
        // "Hi" in UTF-16 BE
        let data = [0x00, 0x48, 0x00, 0x69]; // H=0x0048, i=0x0069

        let result = decode_utf16(&data);
        assert_eq!(result, Some("Hi".to_string()));
    }

    #[test]
    fn test_mac_time_to_string() {
        // Test zero time
        assert_eq!(mac_time_to_string(0), "0000:00:00 00:00:00");

        // Test Mac epoch (1904-01-01)
        const MAC_EPOCH_OFFSET: u64 = 2082844800;
        assert_eq!(mac_time_to_string(MAC_EPOCH_OFFSET), "1904:01:01 00:00:00");

        // Test Unix epoch (1970-01-01)
        assert_eq!(mac_time_to_string(MAC_EPOCH_OFFSET), "1904:01:01 00:00:00");

        // Test a known timestamp: 2024-01-01 00:00:00 UTC
        // Unix timestamp for 2024-01-01: 1704067200
        let mac_time = 1704067200 + MAC_EPOCH_OFFSET;
        let result = mac_time_to_string(mac_time);
        assert!(result.starts_with("2024:01:01"));
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2000)); // Divisible by 400
        assert!(is_leap_year(2004)); // Divisible by 4, not by 100
        assert!(!is_leap_year(1900)); // Divisible by 100, not by 400
        assert!(!is_leap_year(2001)); // Not divisible by 4
        assert!(is_leap_year(2024));
    }

    #[test]
    fn test_days_to_month_day() {
        // Test regular year
        assert_eq!(days_to_month_day(0, false), (1, 1)); // Jan 1
        assert_eq!(days_to_month_day(31, false), (2, 1)); // Feb 1
        assert_eq!(days_to_month_day(59, false), (3, 1)); // Mar 1
        assert_eq!(days_to_month_day(364, false), (12, 31)); // Dec 31

        // Test leap year
        assert_eq!(days_to_month_day(59, true), (2, 29)); // Feb 29
        assert_eq!(days_to_month_day(60, true), (3, 1)); // Mar 1
    }

    #[test]
    fn test_parse_iso6709_basic() {
        // Test basic GPS coordinates
        let result = parse_iso6709("+37.7749-122.4194/");
        assert!(result.is_some());
        let (lat, lon, alt) = result.unwrap();
        assert!((lat - 37.7749).abs() < 0.0001);
        assert!((lon - (-122.4194)).abs() < 0.0001);
        assert!(alt.is_none());
    }

    #[test]
    fn test_parse_iso6709_with_altitude() {
        // Test GPS coordinates with altitude
        let result = parse_iso6709("+40.7128-074.0060+010.5/");
        assert!(result.is_some());
        let (lat, lon, alt) = result.unwrap();
        assert!((lat - 40.7128).abs() < 0.0001);
        assert!((lon - (-74.0060)).abs() < 0.0001);
        assert!(alt.is_some());
        assert!((alt.unwrap() - 10.5).abs() < 0.01);
    }

    #[test]
    fn test_parse_iso6709_no_slash() {
        // Test without trailing slash
        let result = parse_iso6709("+51.5074-000.1278");
        assert!(result.is_some());
        let (lat, lon, _) = result.unwrap();
        assert!((lat - 51.5074).abs() < 0.0001);
        assert!((lon - (-0.1278)).abs() < 0.0001);
    }

    #[test]
    fn test_parse_iso6709_invalid() {
        // Test invalid inputs
        assert!(parse_iso6709("").is_none());
        assert!(parse_iso6709("invalid").is_none());
        assert!(parse_iso6709("+37.7749").is_none()); // Missing longitude
    }

    #[test]
    fn test_map_apple_key_to_tag() {
        assert_eq!(
            map_apple_key_to_tag("com.apple.quicktime.location.ISO6709"),
            "QuickTime:GPSCoordinates"
        );
        assert_eq!(
            map_apple_key_to_tag("com.apple.quicktime.make"),
            "QuickTime:Make"
        );
        assert_eq!(
            map_apple_key_to_tag("com.apple.quicktime.model"),
            "QuickTime:Model"
        );
        assert_eq!(
            map_apple_key_to_tag("com.apple.quicktime.software"),
            "QuickTime:Software"
        );
        assert_eq!(map_apple_key_to_tag("unknown.key"), "QuickTime:unknown.key");
    }
}
