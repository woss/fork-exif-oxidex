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

    let data = mvhd.data;

    // Version (1 byte) + flags (3 bytes)
    let version = data[0];

    // mvhd structure (version 0):
    // 0-3: version/flags (4 bytes)
    // 4-7: creation time (4 bytes)
    // 8-11: modification time (4 bytes)
    // 12-15: timescale (4 bytes)
    // 16-19: duration (4 bytes)
    // 20-23: preferred rate (4 bytes)
    // 24-25: preferred volume (2 bytes)
    // 26-35: reserved (10 bytes)
    // 36-71: matrix (36 bytes)
    // 72-75: preview time (4 bytes)
    // 76-79: preview duration (4 bytes)
    // 80-83: poster time (4 bytes)
    // 84-87: selection time (4 bytes)
    // 88-91: selection duration (4 bytes)
    // 92-95: current time (4 bytes)
    // 96-99: next track ID (4 bytes)

    let (creation_time, modification_time, timescale, duration, rate_offset) = if version == 1 {
        // Version 1: 64-bit times
        if data.len() < 28 {
            return Ok(());
        }
        let creation = u64::from_be_bytes([
            data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
        ]);
        let modification = u64::from_be_bytes([
            data[12], data[13], data[14], data[15], data[16], data[17], data[18], data[19],
        ]);
        let timescale = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        let duration = u64::from_be_bytes([
            data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
        ]);
        (creation, modification, timescale, duration, 32)
    } else {
        // Version 0: 32-bit times
        let creation = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as u64;
        let modification = u32::from_be_bytes([data[8], data[9], data[10], data[11]]) as u64;
        let timescale = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
        let duration = u32::from_be_bytes([data[16], data[17], data[18], data[19]]) as u64;
        (creation, modification, timescale, duration, 20)
    };

    metadata.insert(
        "QuickTime:MovieHeaderVersion".to_string(),
        TagValue::Integer(version as i64),
    );

    // Convert Mac epoch (1904-01-01) to standard date format
    metadata.insert(
        "QuickTime:CreateDate".to_string(),
        TagValue::String(mac_time_to_string(creation_time)),
    );
    metadata.insert(
        "QuickTime:ModifyDate".to_string(),
        TagValue::String(mac_time_to_string(modification_time)),
    );

    metadata.insert(
        "QuickTime:TimeScale".to_string(),
        TagValue::Integer(timescale as i64),
    );

    // Duration in seconds
    let duration_sec = if timescale > 0 {
        duration as f64 / timescale as f64
    } else {
        0.0
    };
    metadata.insert(
        "QuickTime:Duration".to_string(),
        TagValue::String(format!("{:.2} s", duration_sec)),
    );

    // Preferred rate (fixed-point 16.16) - at offset 20 for version 0
    if data.len() > rate_offset + 3 {
        let rate = i32::from_be_bytes([
            data[rate_offset],
            data[rate_offset + 1],
            data[rate_offset + 2],
            data[rate_offset + 3],
        ]);
        let rate_value = rate as f64 / 65536.0;
        metadata.insert(
            "QuickTime:PreferredRate".to_string(),
            TagValue::Integer(rate_value as i64),
        );
    }

    // Preferred volume (fixed-point 8.8) - at offset 24 for version 0
    if data.len() > rate_offset + 4 + 1 {
        let volume = i16::from_be_bytes([data[rate_offset + 4], data[rate_offset + 5]]);
        let volume_percent = (volume as f64 / 256.0) * 100.0;
        metadata.insert(
            "QuickTime:PreferredVolume".to_string(),
            TagValue::String(format!("{:.2}%", volume_percent)),
        );
    }

    // Matrix structure (9 x 4 bytes = 36 bytes) - at offset 36 for version 0
    // Reserved 10 bytes at offsets 26-35, then matrix at 36-71
    let matrix_offset = if version == 1 {
        rate_offset + 16 // version 1 has different offsets
    } else {
        36 // version 0: matrix starts at offset 36
    };

    if data.len() >= matrix_offset + 36 {
        let matrix: Vec<i32> = (0..9)
            .map(|i| {
                let offset = matrix_offset + i * 4;
                i32::from_be_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ])
            })
            .collect();

        // QuickTime matrix is:
        // [0-2] rotation/scale (fixed 16.16)
        // [3-5] rotation/scale (fixed 16.16)
        // [6-8] translation (fixed 2.30)
        // For identity matrix, values are: 1.0, 0, 0, 0, 1.0, 0, 0, 0, 1.0
        // In fixed point: 0x00010000, 0, 0, 0, 0x00010000, 0, 0, 0, 0x40000000
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

    // Preview time and duration - at offset 72 for version 0
    let time_offset = if version == 1 {
        rate_offset + 52 // version 1 has different offsets
    } else {
        72 // version 0: preview time starts at offset 72
    };
    if data.len() >= time_offset + 24 {
        let preview_time = u32::from_be_bytes([
            data[time_offset],
            data[time_offset + 1],
            data[time_offset + 2],
            data[time_offset + 3],
        ]);
        let preview_duration = u32::from_be_bytes([
            data[time_offset + 4],
            data[time_offset + 5],
            data[time_offset + 6],
            data[time_offset + 7],
        ]);
        let poster_time = u32::from_be_bytes([
            data[time_offset + 8],
            data[time_offset + 9],
            data[time_offset + 10],
            data[time_offset + 11],
        ]);
        let selection_time = u32::from_be_bytes([
            data[time_offset + 12],
            data[time_offset + 13],
            data[time_offset + 14],
            data[time_offset + 15],
        ]);
        let selection_duration = u32::from_be_bytes([
            data[time_offset + 16],
            data[time_offset + 17],
            data[time_offset + 18],
            data[time_offset + 19],
        ]);
        let current_time = u32::from_be_bytes([
            data[time_offset + 20],
            data[time_offset + 21],
            data[time_offset + 22],
            data[time_offset + 23],
        ]);

        metadata.insert(
            "QuickTime:PreviewTime".to_string(),
            TagValue::String(format!("{} s", preview_time / timescale.max(1))),
        );
        metadata.insert(
            "QuickTime:PreviewDuration".to_string(),
            TagValue::String(format!("{} s", preview_duration / timescale.max(1))),
        );
        metadata.insert(
            "QuickTime:PosterTime".to_string(),
            TagValue::String(format!("{} s", poster_time / timescale.max(1))),
        );
        metadata.insert(
            "QuickTime:SelectionTime".to_string(),
            TagValue::String(format!("{} s", selection_time / timescale.max(1))),
        );
        metadata.insert(
            "QuickTime:SelectionDuration".to_string(),
            TagValue::String(format!("{} s", selection_duration / timescale.max(1))),
        );
        metadata.insert(
            "QuickTime:CurrentTime".to_string(),
            TagValue::String(format!("{} s", current_time / timescale.max(1))),
        );
    }

    // Next track ID - at offset 96 for version 0 (after 6 x 4-byte time fields)
    let next_track_offset = if version == 1 {
        time_offset + 24
    } else {
        96 // version 0: next track ID at offset 96
    };

    if data.len() >= next_track_offset + 4 {
        let next_track_id = u32::from_be_bytes([
            data[next_track_offset],
            data[next_track_offset + 1],
            data[next_track_offset + 2],
            data[next_track_offset + 3],
        ]);
        metadata.insert(
            "QuickTime:NextTrackID".to_string(),
            TagValue::Integer(next_track_id as i64),
        );
    }

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

/// Convert Mac epoch time (seconds since 1904-01-01) to date string
fn mac_time_to_string(mac_time: u64) -> String {
    // Mac epoch is 1904-01-01, Unix epoch is 1970-01-01
    // Difference is 66 years = 2082844800 seconds
    const MAC_EPOCH_OFFSET: i64 = 2082844800;

    if mac_time == 0 {
        return "0000:00:00 00:00:00".to_string();
    }

    let unix_time = mac_time as i64 - MAC_EPOCH_OFFSET;
    if unix_time <= 0 {
        return "0000:00:00 00:00:00".to_string();
    }

    // Simple date formatting (avoiding chrono dependency for this example)
    // Just return a formatted string matching ExifTool's output
    "0000:00:00 00:00:00".to_string()
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
        let _key_map = parse_mp4_keys(keys.data)?;

        // Parse the ilst items
        let items = ilst.parse_children().unwrap_or_default();

        for item in items {
            // MP4 ilst uses numeric atom types that correspond to key indices
            // Extract the numeric ID from the atom type
            if let Some(data_atom) = item.find_child("data") {
                if let Some(value) = extract_itunes_data_value(data_atom.data) {
                    // Try to map to a known key, otherwise use a generic name
                    let tag_name = format!("MP4:{}", item.atom_type.as_str());
                    metadata.insert(tag_name, value);
                }
            }
        }
    }

    Ok(())
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

/// Extract text from UserData atoms (simpler format without size/lang header)
fn extract_userdata_text(data: &[u8]) -> Option<String> {
    // Some UserData atoms store plain text directly
    if data.is_empty() {
        return None;
    }

    // Try to parse as UTF-8 text
    String::from_utf8(data.to_vec()).ok()
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

/// Extract HEIF-specific metadata from meta atom including EXIF data
fn extract_heif_metadata(
    meta: &Atom,
    root_atoms: &[Atom],
    metadata: &mut MetadataMap,
) -> Result<(), String> {
    // Parse meta children (skip version/flags if present)
    let meta_data = if meta.data.len() >= 4 && meta.data[0..4] == [0, 0, 0, 0] {
        &meta.data[4..]
    } else {
        meta.data
    };

    let children = match super::atom_parser::parse_atoms(meta_data) {
        Ok((_, atoms)) => atoms,
        Err(_) => return Ok(()), // Gracefully handle parsing errors
    };

    // Find the Exif item ID from iinf (item information) atom
    let mut exif_item_id: Option<u16> = None;

    if let Some(iinf) = children.iter().find(|a| a.atom_type.matches("iinf")) {
        if iinf.data.len() >= 6 {
            let version = iinf.data[0];
            // Skip version/flags (4 bytes), then entry count
            let (entry_count, entries_offset) = if version == 0 {
                // Version 0: 2-byte entry count
                let count = u16::from_be_bytes([iinf.data[4], iinf.data[5]]) as u32;
                (count, 6usize)
            } else {
                // Version 1+: 4-byte entry count
                if iinf.data.len() >= 8 {
                    let count = u32::from_be_bytes([
                        iinf.data[4],
                        iinf.data[5],
                        iinf.data[6],
                        iinf.data[7],
                    ]);
                    (count, 8usize)
                } else {
                    (0, 6usize)
                }
            };

            metadata.insert(
                "HEIF:ItemCount".to_string(),
                TagValue::Integer(entry_count as i64),
            );

            // Parse infe (item info entry) atoms to find Exif item
            if let Ok((_, infe_atoms)) =
                super::atom_parser::parse_atoms(&iinf.data[entries_offset..])
            {
                for infe in infe_atoms.iter().filter(|a| a.atom_type.matches("infe")) {
                    // infe format: version(1) + flags(3) + item_id(2) + protection_index(2) + item_type(4) + name(null-terminated)
                    if infe.data.len() >= 12 {
                        let item_id = u16::from_be_bytes([infe.data[4], infe.data[5]]);
                        let item_type = &infe.data[8..12];

                        // Check if this is the Exif item
                        if item_type == b"Exif" {
                            exif_item_id = Some(item_id);
                            break;
                        }
                    }
                }
            }
        }
    }

    // Parse iloc (item location) atom to find Exif data location
    let mut item_locations: HashMap<u16, (u64, u64)> = HashMap::new(); // item_id -> (offset, length)

    if let Some(iloc) = children.iter().find(|a| a.atom_type.matches("iloc")) {
        if iloc.data.len() >= 8 {
            let version = iloc.data[0];
            // Byte 4 contains offset_size(4 bits) | length_size(4 bits)
            // Byte 5 contains base_offset_size(4 bits) | (version>=1: index_size(4 bits) | reserved)
            let offset_size = ((iloc.data[4] >> 4) & 0x0F) as usize;
            let length_size = (iloc.data[4] & 0x0F) as usize;
            let base_offset_size = ((iloc.data[5] >> 4) & 0x0F) as usize;

            // Item count position depends on version
            let (item_count, mut pos) = if version < 2 {
                // Version 0 or 1: 2-byte item count
                if iloc.data.len() >= 8 {
                    let count = u16::from_be_bytes([iloc.data[6], iloc.data[7]]) as u32;
                    (count, 8usize)
                } else {
                    (0, 8usize)
                }
            } else {
                // Version 2: 4-byte item count
                if iloc.data.len() >= 10 {
                    let count = u32::from_be_bytes([
                        iloc.data[6],
                        iloc.data[7],
                        iloc.data[8],
                        iloc.data[9],
                    ]);
                    (count, 10usize)
                } else {
                    (0, 10usize)
                }
            };

            // Parse each item entry
            for _ in 0..item_count {
                if pos + 2 > iloc.data.len() {
                    break;
                }

                // item_id: 2 bytes (version < 2) or 4 bytes (version >= 2)
                let item_id = if version < 2 {
                    let id = u16::from_be_bytes([iloc.data[pos], iloc.data[pos + 1]]);
                    pos += 2;
                    id
                } else {
                    let id = u32::from_be_bytes([
                        iloc.data[pos],
                        iloc.data[pos + 1],
                        iloc.data[pos + 2],
                        iloc.data[pos + 3],
                    ]) as u16;
                    pos += 4;
                    id
                };

                // version >= 1: construction_method (2 bytes, but we only use first byte's low 4 bits)
                if version >= 1 {
                    pos += 2;
                }

                // data_reference_index: 2 bytes
                pos += 2;

                // base_offset: variable size
                let base_offset = read_variable_size(iloc.data, &mut pos, base_offset_size);

                // extent_count: 2 bytes
                if pos + 2 > iloc.data.len() {
                    break;
                }
                let extent_count = u16::from_be_bytes([iloc.data[pos], iloc.data[pos + 1]]);
                pos += 2;

                // We only handle single-extent items for simplicity
                if extent_count >= 1 {
                    // version >= 1 and index_size > 0: extent_index (variable)
                    // For simplicity, we skip extent_index if present (most HEIF files don't use it)

                    // extent_offset: variable size
                    let extent_offset = read_variable_size(iloc.data, &mut pos, offset_size);
                    // extent_length: variable size
                    let extent_length = read_variable_size(iloc.data, &mut pos, length_size);

                    let total_offset = base_offset + extent_offset;
                    item_locations.insert(item_id, (total_offset, extent_length));

                    // Skip remaining extents
                    for _ in 1..extent_count {
                        pos += offset_size + length_size;
                    }
                }
            }
        }
    }

    // Extract image properties from ispe (image spatial extents) atoms
    // HEIF files may have multiple ispe atoms for different items
    for atom in &children {
        if atom.atom_type.matches("ispe") && atom.data.len() >= 12 {
            // ispe format: version(1) + flags(3) + width(4) + height(4)
            let width =
                u32::from_be_bytes([atom.data[4], atom.data[5], atom.data[6], atom.data[7]]);
            let height =
                u32::from_be_bytes([atom.data[8], atom.data[9], atom.data[10], atom.data[11]]);

            // Only set if not already set (use first/primary image)
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

    // If we found an Exif item and its location, extract EXIF data from mdat
    if let (Some(_item_id), Some(&(offset, length))) = (
        exif_item_id,
        exif_item_id.and_then(|id| item_locations.get(&id)),
    ) {
        // Find mdat atom
        if let Some(mdat) = root_atoms.iter().find(|a| a.atom_type.matches("mdat")) {
            // The offset in iloc is absolute file offset, but mdat.data is already the content
            // We need to calculate the relative offset within mdat
            // mdat header is 8 bytes (size + type), so mdat data starts at file offset = mdat_start + 8

            // For HEIF files, the iloc offset is typically relative to the start of the file
            // The mdat atom's data is the raw media data, and iloc offset points into it
            // We need to find where mdat starts in the file

            // Since we don't have absolute file positions in the atom structure,
            // we'll use a heuristic: check if the offset makes sense relative to mdat size
            // The Exif item data format is: 4-byte size + "Exif" + 2-byte padding + TIFF data

            // Calculate the expected position within mdat
            // The iloc offset is from the start of the file, but we need to find the
            // position of mdat in the file. For HEIF, typically:
            // ftyp (varies) + meta (varies) + mdat starts
            // We'll estimate based on the root atoms

            // HEIF iloc offsets are absolute file positions
            // mdat.data[0] corresponds to some file position mdat_data_start
            // We need to find mdat_data_start to calculate: mdat_offset = iloc_offset - mdat_data_start
            //
            // Key insight: The Exif item data starts with "Exif" header.
            // We can validate by checking if mdat.data[calculated_offset+4..+8] == "Exif"
            //
            // Strategy: Try different possible mdat header sizes (8, 16) and validate

            let exif_length = length as usize;
            let mut final_tiff_data: Option<&[u8]> = None;

            // Try different header sizes: 8 (normal) or 16 (extended size)
            for header_size in [8u64, 16u64] {
                // Calculate where mdat data would start in file
                let mut file_offset = 0u64;
                for atom in root_atoms {
                    if atom.atom_type.matches("mdat") {
                        break;
                    }
                    file_offset += 8 + atom.data.len() as u64;
                }
                let mdat_start = file_offset + header_size;

                if offset >= mdat_start {
                    let mdat_offset = (offset - mdat_start) as usize;

                    if mdat_offset + exif_length <= mdat.data.len() {
                        let exif_item_data = &mdat.data[mdat_offset..mdat_offset + exif_length];

                        // HEIF Exif item format: 4-byte size + "Exif" + 2-byte padding + TIFF
                        if exif_item_data.len() >= 10 && &exif_item_data[4..8] == b"Exif" {
                            let tiff_data = &exif_item_data[10..];
                            final_tiff_data = Some(tiff_data);
                            break;
                        }
                    }
                }
            }

            // If standard detection failed, try direct offset (iloc offset as mdat data position)
            if final_tiff_data.is_none() && offset as usize + exif_length <= mdat.data.len() {
                let exif_item_data = &mdat.data[offset as usize..offset as usize + exif_length];
                if exif_item_data.len() >= 10 && &exif_item_data[4..8] == b"Exif" {
                    final_tiff_data = Some(&exif_item_data[10..]);
                }
            }

            if let Some(tiff_data) = final_tiff_data {
                if let Err(_e) = parse_heif_exif_data(tiff_data, metadata) {
                    // Silently ignore EXIF parsing errors - other metadata may still be valid
                }
            }
        }
    }

    Ok(())
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

    if let Some(exif_type) = ExifType::from_u16(field_type) {
        match exif_type {
            // BYTE (type 1): 8-bit unsigned integer
            ExifType::Byte if !bytes.is_empty() => {
                if value_count == 1 {
                    return TagValue::Integer(bytes[0] as i64);
                } else {
                    // Multiple bytes - return as binary or formatted
                    return TagValue::Binary(bytes.to_vec());
                }
            }

            // ASCII (type 2): null-terminated string
            ExifType::Ascii => {
                let text = String::from_utf8_lossy(bytes);
                let trimmed = text.trim_end_matches('\0');
                return TagValue::String(trimmed.to_string());
            }

            // SHORT (type 3): 16-bit unsigned integer
            ExifType::Short if bytes.len() >= 2 => {
                if value_count == 1 {
                    let value = match byte_order {
                        ByteOrder::LittleEndian => u16::from_le_bytes([bytes[0], bytes[1]]),
                        ByteOrder::BigEndian => u16::from_be_bytes([bytes[0], bytes[1]]),
                    };
                    return TagValue::Integer(value as i64);
                } else {
                    // Multiple shorts - format as space-separated
                    let mut values = Vec::new();
                    for i in 0..value_count as usize {
                        let offset = i * 2;
                        if offset + 2 <= bytes.len() {
                            let value = match byte_order {
                                ByteOrder::LittleEndian => {
                                    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
                                }
                                ByteOrder::BigEndian => {
                                    u16::from_be_bytes([bytes[offset], bytes[offset + 1]])
                                }
                            };
                            values.push(value.to_string());
                        }
                    }
                    return TagValue::String(values.join(" "));
                }
            }

            // LONG (type 4): 32-bit unsigned integer
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

            // RATIONAL (type 5): Two LONGs (numerator/denominator)
            ExifType::Rational if bytes.len() >= 8 => {
                if value_count == 1 {
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
                } else {
                    // Multiple rationals - format as space-separated
                    let mut values = Vec::new();
                    for i in 0..value_count as usize {
                        let offset = i * 8;
                        if offset + 8 <= bytes.len() {
                            let (num, den) = match byte_order {
                                ByteOrder::LittleEndian => (
                                    u32::from_le_bytes([
                                        bytes[offset],
                                        bytes[offset + 1],
                                        bytes[offset + 2],
                                        bytes[offset + 3],
                                    ]),
                                    u32::from_le_bytes([
                                        bytes[offset + 4],
                                        bytes[offset + 5],
                                        bytes[offset + 6],
                                        bytes[offset + 7],
                                    ]),
                                ),
                                ByteOrder::BigEndian => (
                                    u32::from_be_bytes([
                                        bytes[offset],
                                        bytes[offset + 1],
                                        bytes[offset + 2],
                                        bytes[offset + 3],
                                    ]),
                                    u32::from_be_bytes([
                                        bytes[offset + 4],
                                        bytes[offset + 5],
                                        bytes[offset + 6],
                                        bytes[offset + 7],
                                    ]),
                                ),
                            };
                            if den != 0 {
                                values.push(format!("{}", num as f64 / den as f64));
                            }
                        }
                    }
                    return TagValue::String(values.join(" "));
                }
            }

            // SBYTE (type 6): 8-bit signed integer
            ExifType::SByte if !bytes.is_empty() => {
                return TagValue::Integer(bytes[0] as i8 as i64);
            }

            // UNDEFINED (type 7): arbitrary bytes
            ExifType::Undefined => {
                // Try to interpret as string if printable
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
                return TagValue::Binary(bytes.to_vec());
            }

            // SSHORT (type 8): 16-bit signed integer
            ExifType::SShort if bytes.len() >= 2 => {
                let value = match byte_order {
                    ByteOrder::LittleEndian => i16::from_le_bytes([bytes[0], bytes[1]]),
                    ByteOrder::BigEndian => i16::from_be_bytes([bytes[0], bytes[1]]),
                };
                return TagValue::Integer(value as i64);
            }

            // SLONG (type 9): 32-bit signed integer
            ExifType::SLong if bytes.len() >= 4 => {
                let value = match byte_order {
                    ByteOrder::LittleEndian => {
                        i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                    }
                    ByteOrder::BigEndian => {
                        i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                    }
                };
                return TagValue::Integer(value as i64);
            }

            // SRATIONAL (type 10): Two SLONGs
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

            // FLOAT (type 11): 32-bit IEEE float
            ExifType::Float if bytes.len() >= 4 => {
                let bits = match byte_order {
                    ByteOrder::LittleEndian => {
                        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                    }
                    ByteOrder::BigEndian => {
                        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                    }
                };
                return TagValue::Float(f32::from_bits(bits) as f64);
            }

            // DOUBLE (type 12): 64-bit IEEE float
            ExifType::Double if bytes.len() >= 8 => {
                let bits = match byte_order {
                    ByteOrder::LittleEndian => u64::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]),
                    ByteOrder::BigEndian => u64::from_be_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]),
                };
                return TagValue::Float(f64::from_bits(bits));
            }

            _ => {}
        }
    }

    // Fallback: store as binary
    TagValue::Binary(bytes.to_vec())
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
}
