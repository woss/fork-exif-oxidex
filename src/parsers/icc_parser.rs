//! ICC Profile parser for embedded color profiles
//!
//! This module handles parsing of ICC (International Color Consortium)
//! profiles embedded in various file formats (PDF, JPEG, PNG, TIFF, etc.).
//! ICC profiles describe color characteristics for accurate color reproduction
//! across different devices.
//!
//! # Architecture
//!
//! The parser uses a **registry-based approach** for maintainability and extensibility:
//! - **TagRegistry**: Table-driven tag definitions with signatures, names, and decoders
//! - **HeaderField**: Registry for header field locations and extractors
//! - **LookupTables**: Static lookup tables for enumerations (profile class, platform, etc.)
//!
//! This design reduces cyclomatic complexity, improves maintainability, and makes
//! it easy to add new tags or modify existing ones.
//!
//! # ICC Profile Structure
//!
//! An ICC profile consists of:
//! 1. **Profile Header** (128 bytes): Contains profile metadata
//! 2. **Tag Table**: List of tags with their signatures, offsets, and sizes
//! 3. **Tagged Element Data**: Actual tag data (descriptions, calibration data, etc.)

use crate::core::{FileReader, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use flate2::read::ZlibDecoder;
use std::collections::HashMap;
use std::io::Read;

// ============================================================================
// CORE REGISTRY STRUCTURES
// ============================================================================

/// Type of ICC tag data - determines which decoder to use
#[derive(Debug, Clone, Copy, PartialEq)]
enum TagType {
    /// Text description (desc/mluc)
    TextDescription,
    /// Simple text (text/mluc)
    Text,
    /// XYZ coordinate triple
    Xyz,
    /// Binary curve data
    Curve,
    /// Viewing conditions structure
    ViewingConditions,
    /// Measurement structure
    Measurement,
    /// 4-byte signature
    Signature,
}

/// Registry entry for an ICC tag
struct TagDef {
    /// 4-character ICC tag signature
    signature: &'static str,
    /// Human-readable tag name (added to metadata)
    name: &'static str,
    /// Type of data this tag contains
    tag_type: TagType,
}

/// Header field definition for structured parsing
struct HeaderField {
    /// Byte offset in ICC header
    offset: usize,
    /// Field name in metadata
    name: &'static str,
    /// Extractor function
    extract: fn(&[u8], usize, &mut HashMap<String, TagValue>) -> Result<()>,
}

/// Lookup table entry for mapping codes to names
struct LookupEntry {
    /// Code or signature to match
    code: &'static str,
    /// Human-readable name
    name: &'static str,
}

// ============================================================================
// TAG REGISTRY
// ============================================================================

/// Complete ICC tag registry
///
/// This table defines all supported ICC tags with their signatures, names,
/// and associated decoder types. Adding a new tag is as simple as adding
/// a new entry to this table.
static TAG_REGISTRY: &[TagDef] = &[
    // Text description tags
    TagDef { signature: "desc", name: "ProfileDescription", tag_type: TagType::TextDescription },
    TagDef { signature: "cprt", name: "ProfileCopyright", tag_type: TagType::Text },
    TagDef { signature: "dmnd", name: "DeviceMfgDesc", tag_type: TagType::TextDescription },
    TagDef { signature: "dmdd", name: "DeviceModelDesc", tag_type: TagType::TextDescription },
    TagDef { signature: "vued", name: "ViewingCondDesc", tag_type: TagType::TextDescription },

    // XYZ coordinate tags
    TagDef { signature: "wtpt", name: "MediaWhitePoint", tag_type: TagType::Xyz },
    TagDef { signature: "bkpt", name: "MediaBlackPoint", tag_type: TagType::Xyz },
    TagDef { signature: "rXYZ", name: "RedMatrixColumn", tag_type: TagType::Xyz },
    TagDef { signature: "gXYZ", name: "GreenMatrixColumn", tag_type: TagType::Xyz },
    TagDef { signature: "bXYZ", name: "BlueMatrixColumn", tag_type: TagType::Xyz },
    TagDef { signature: "lumi", name: "Luminance", tag_type: TagType::Xyz },

    // Curve tags (binary data)
    TagDef { signature: "rTRC", name: "RedToneReproductionCurve", tag_type: TagType::Curve },
    TagDef { signature: "gTRC", name: "GreenToneReproductionCurve", tag_type: TagType::Curve },
    TagDef { signature: "bTRC", name: "BlueToneReproductionCurve", tag_type: TagType::Curve },

    // Structured data tags
    TagDef { signature: "view", name: "ViewingConditions", tag_type: TagType::ViewingConditions },
    TagDef { signature: "meas", name: "Measurement", tag_type: TagType::Measurement },
    TagDef { signature: "tech", name: "Technology", tag_type: TagType::Signature },
];

// ============================================================================
// HEADER FIELD REGISTRY
// ============================================================================

/// Header field definitions for structured parsing
///
/// Each entry defines the offset, name, and extractor function for a
/// specific field in the 128-byte ICC header.
static HEADER_FIELDS: &[HeaderField] = &[
    HeaderField { offset: 4, name: "ProfileCMMType", extract: extract_cmm_type },
    HeaderField { offset: 8, name: "ProfileVersion", extract: extract_version },
    HeaderField { offset: 12, name: "ProfileClass", extract: extract_profile_class },
    HeaderField { offset: 16, name: "ColorSpaceData", extract: extract_color_space },
    HeaderField { offset: 20, name: "ProfileConnectionSpace", extract: extract_pcs },
    HeaderField { offset: 24, name: "ProfileDateTime", extract: extract_datetime },
    HeaderField { offset: 36, name: "ProfileFileSignature", extract: extract_signature },
    HeaderField { offset: 40, name: "PrimaryPlatform", extract: extract_platform },
    HeaderField { offset: 44, name: "CMMFlags", extract: extract_flags },
    HeaderField { offset: 48, name: "DeviceManufacturer", extract: extract_manufacturer },
    HeaderField { offset: 52, name: "DeviceModel", extract: extract_model },
    HeaderField { offset: 56, name: "DeviceAttributes", extract: extract_attributes },
    HeaderField { offset: 64, name: "RenderingIntent", extract: extract_rendering_intent },
    HeaderField { offset: 68, name: "ConnectionSpaceIlluminant", extract: extract_illuminant },
    HeaderField { offset: 80, name: "ProfileCreator", extract: extract_creator },
    HeaderField { offset: 84, name: "ProfileID", extract: extract_profile_id },
];

// ============================================================================
// LOOKUP TABLES
// ============================================================================

/// Profile class lookup table
static PROFILE_CLASSES: &[LookupEntry] = &[
    LookupEntry { code: "scnr", name: "Input Device Profile" },
    LookupEntry { code: "mntr", name: "Display Device Profile" },
    LookupEntry { code: "prtr", name: "Output Device Profile" },
    LookupEntry { code: "link", name: "DeviceLink Profile" },
    LookupEntry { code: "spac", name: "ColorSpace Profile" },
    LookupEntry { code: "abst", name: "Abstract Profile" },
    LookupEntry { code: "nmcl", name: "Named Color Profile" },
];

/// Platform lookup table
static PLATFORMS: &[LookupEntry] = &[
    LookupEntry { code: "APPL", name: "Apple Computer Inc." },
    LookupEntry { code: "MSFT", name: "Microsoft Corporation" },
    LookupEntry { code: "SGI", name: "Silicon Graphics Inc." },
    LookupEntry { code: "SUNW", name: "Sun Microsystems" },
];

/// Technology lookup table
static TECHNOLOGIES: &[LookupEntry] = &[
    LookupEntry { code: "fscn", name: "Film Scanner" },
    LookupEntry { code: "dcam", name: "Digital Camera" },
    LookupEntry { code: "rscn", name: "Reflective Scanner" },
    LookupEntry { code: "ijet", name: "Ink Jet Printer" },
    LookupEntry { code: "twax", name: "Thermal Wax Printer" },
    LookupEntry { code: "epho", name: "Electrophotographic Printer" },
    LookupEntry { code: "esta", name: "Electrostatic Printer" },
    LookupEntry { code: "dsub", name: "Dye Sublimation Printer" },
    LookupEntry { code: "rpho", name: "Photographic Paper Printer" },
    LookupEntry { code: "fprn", name: "Film Writer" },
    LookupEntry { code: "vidm", name: "Video Monitor" },
    LookupEntry { code: "vidc", name: "Video Camera" },
    LookupEntry { code: "pjtv", name: "Projection Television" },
    LookupEntry { code: "CRT", name: "Cathode Ray Tube Display" },
    LookupEntry { code: "PMD", name: "Passive Matrix Display" },
    LookupEntry { code: "AMD", name: "Active Matrix Display" },
    LookupEntry { code: "KPCD", name: "Photo CD" },
    LookupEntry { code: "imgs", name: "Photo Image Setter" },
    LookupEntry { code: "grav", name: "Gravure" },
    LookupEntry { code: "offs", name: "Offset Lithography" },
    LookupEntry { code: "silk", name: "Silkscreen" },
    LookupEntry { code: "flex", name: "Flexography" },
];

/// Rendering intent names (indexed by code 0-3)
static RENDERING_INTENTS: &[&str] = &[
    "Perceptual",
    "Relative Colorimetric",
    "Saturation",
    "Absolute Colorimetric",
];

/// Illuminant type names (indexed by code 1-8)
static ILLUMINANT_TYPES: &[&str] = &[
    "Unknown", // 0 - not used
    "D50",     // 1
    "D65",     // 2
    "D93",     // 3
    "F2",      // 4
    "D55",     // 5
    "A",       // 6
    "Equi-Power (E)", // 7
    "F8",      // 8
];

/// Observer types (indexed by code 1-2)
static OBSERVER_TYPES: &[&str] = &[
    "Unknown", // 0
    "CIE 1931", // 1
    "CIE 1964", // 2
];

/// Geometry types (indexed by code 0-2)
static GEOMETRY_TYPES: &[&str] = &[
    "Unknown",     // 0
    "0/45 or 45/0", // 1
    "0/d or d/0",   // 2
];

// ============================================================================
// PUBLIC API
// ============================================================================

/// Extracts ICC profile metadata from a PDF file.
///
/// This function searches for ICC profiles in the PDF's OutputIntents,
/// extracts the profile stream, decompresses if necessary, and parses
/// the ICC profile header and tags.
pub fn extract_icc_profile(reader: &dyn FileReader) -> Result<MetadataMap> {
    let mut metadata = MetadataMap::new();

    // Extract ICC profile from PDF
    let icc_data = extract_icc_from_pdf(reader)?;

    // Parse the ICC profile
    let icc_metadata = parse_icc_profile(&icc_data)?;

    // Add Profile: prefix to all tags
    for (key, value) in icc_metadata {
        metadata.insert(format!("Profile:{}", key), value);
    }

    if metadata.is_empty() {
        return Err(ExifToolError::parse_error("No ICC profile found in PDF"));
    }

    Ok(metadata)
}

/// Parses ICC profile binary data and extracts metadata.
///
/// This is the main entry point for parsing ICC profile data from any source
/// (JPEG APP2 segments, PDF streams, PNG chunks, etc.).
pub fn parse_icc_profile_data(data: &[u8]) -> Result<HashMap<String, TagValue>> {
    parse_icc_profile(data)
}

// ============================================================================
// CORE PARSING LOGIC
// ============================================================================

/// Main ICC profile parser - uses registry-based approach
fn parse_icc_profile(data: &[u8]) -> Result<HashMap<String, TagValue>> {
    if data.len() < 128 {
        return Err(ExifToolError::parse_error("ICC profile too small (< 128 bytes)"));
    }

    let mut metadata = HashMap::new();

    // Parse header using registry
    parse_header_registry(data, &mut metadata)?;

    // Parse tags using registry
    if data.len() > 128 {
        parse_tags_registry(data, &mut metadata)?;
    }

    Ok(metadata)
}

/// Parses ICC header using the header field registry
///
/// This function iterates through the header field registry and extracts
/// each field using its associated extractor function.
fn parse_header_registry(data: &[u8], metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    for field in HEADER_FIELDS {
        if data.len() >= field.offset + 4 {
            // Call the extractor function for this field
            (field.extract)(data, field.offset, metadata)?;
        }
    }
    Ok(())
}

/// Parses ICC tags using the tag registry
///
/// This function reads the tag table and dispatches each tag to its
/// appropriate decoder based on the tag type in the registry.
fn parse_tags_registry(data: &[u8], metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    if data.len() < 132 {
        return Ok(());
    }

    let tag_count = read_u32_be(data, 128)?;

    for i in 0..tag_count {
        let entry_offset = 132 + (i * 12) as usize;
        if entry_offset + 12 > data.len() {
            break;
        }

        let tag_signature = read_signature(data, entry_offset)?;
        let tag_offset = read_u32_be(data, entry_offset + 4)? as usize;
        let tag_size = read_u32_be(data, entry_offset + 8)? as usize;

        if tag_offset >= data.len() || tag_offset + tag_size > data.len() {
            continue;
        }

        let tag_data = &data[tag_offset..tag_offset + tag_size];

        // Look up tag in registry and decode
        decode_tag(&tag_signature.trim(), tag_data, tag_size, metadata);
    }

    Ok(())
}

/// Decodes a single tag using the tag registry
///
/// This function looks up the tag signature in the registry and calls
/// the appropriate decoder based on the tag type.
fn decode_tag(
    signature: &str,
    data: &[u8],
    size: usize,
    metadata: &mut HashMap<String, TagValue>,
) {
    // Find tag in registry
    let tag_def = TAG_REGISTRY.iter().find(|t| t.signature == signature);

    if let Some(def) = tag_def {
        // Decode based on tag type
        let result = match def.tag_type {
            TagType::TextDescription => {
                parse_text_description_type(data).ok()
                    .map(|s| TagValue::new_string(s))
            }
            TagType::Text => {
                parse_text_type(data).ok()
                    .map(|s| TagValue::new_string(s))
            }
            TagType::Xyz => {
                parse_xyz_type(data).ok()
                    .map(|(x, y, z)| TagValue::new_string(format!("{} {} {}", x, y, z)))
            }
            TagType::Curve => {
                Some(TagValue::new_string(format!(
                    "(Binary data {} bytes, use -b option to extract)", size
                )))
            }
            TagType::ViewingConditions => {
                parse_viewing_conditions(data).ok()
                    .and_then(|vc| decode_viewing_conditions(vc, metadata))
            }
            TagType::Measurement => {
                parse_measurement(data).ok()
                    .and_then(|m| decode_measurement(m, metadata))
            }
            TagType::Signature => {
                parse_signature_type(data).ok()
                    .map(|sig| {
                        let name = lookup_in_table(&TECHNOLOGIES, &sig);
                        TagValue::new_string(name.to_string())
                    })
            }
        };

        if let Some(value) = result {
            metadata.insert(def.name.to_string(), value);
        }
    }
}

/// Decodes viewing conditions into multiple metadata entries
fn decode_viewing_conditions(
    vc: HashMap<String, String>,
    metadata: &mut HashMap<String, TagValue>,
) -> Option<TagValue> {
    if let Some(illuminant) = vc.get("illuminant") {
        metadata.insert("ViewingCondIlluminant".to_string(),
                       TagValue::new_string(illuminant.clone()));
    }
    if let Some(surround) = vc.get("surround") {
        metadata.insert("ViewingCondSurround".to_string(),
                       TagValue::new_string(surround.clone()));
    }
    if let Some(illum_type) = vc.get("illuminant_type") {
        metadata.insert("ViewingCondIlluminantType".to_string(),
                       TagValue::new_string(illum_type.clone()));
    }
    None // This tag produces multiple entries, not a single value
}

/// Decodes measurement data into multiple metadata entries
fn decode_measurement(
    m: HashMap<String, String>,
    metadata: &mut HashMap<String, TagValue>,
) -> Option<TagValue> {
    if let Some(observer) = m.get("observer") {
        metadata.insert("MeasurementObserver".to_string(),
                       TagValue::new_string(observer.clone()));
    }
    if let Some(backing) = m.get("backing") {
        metadata.insert("MeasurementBacking".to_string(),
                       TagValue::new_string(backing.clone()));
    }
    if let Some(geometry) = m.get("geometry") {
        metadata.insert("MeasurementGeometry".to_string(),
                       TagValue::new_string(geometry.clone()));
    }
    if let Some(flare) = m.get("flare") {
        metadata.insert("MeasurementFlare".to_string(),
                       TagValue::new_string(flare.clone()));
    }
    if let Some(illuminant) = m.get("illuminant") {
        metadata.insert("MeasurementIlluminant".to_string(),
                       TagValue::new_string(illuminant.clone()));
    }
    None // This tag produces multiple entries, not a single value
}

// ============================================================================
// HEADER FIELD EXTRACTORS
// ============================================================================

/// Extracts CMM type from header
fn extract_cmm_type(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    let cmm_type = read_signature(data, offset)?;
    if !cmm_type.is_empty() {
        metadata.insert("ProfileCMMType".to_string(), TagValue::new_string(cmm_type));
    }
    Ok(())
}

/// Extracts profile version from header
fn extract_version(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    let version = format!(
        "{}.{}.{}",
        data.get(offset).copied().unwrap_or(0),
        (data.get(offset + 1).copied().unwrap_or(0) >> 4) & 0x0F,
        data.get(offset + 1).copied().unwrap_or(0) & 0x0F
    );
    metadata.insert("ProfileVersion".to_string(), TagValue::new_string(version));
    Ok(())
}

/// Extracts profile class from header
fn extract_profile_class(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    let class = read_signature(data, offset)?;
    let class_name = lookup_in_table(&PROFILE_CLASSES, &class);
    metadata.insert("ProfileClass".to_string(), TagValue::new_string(class_name.to_string()));
    Ok(())
}

/// Extracts color space from header
fn extract_color_space(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    let color_space = read_signature(data, offset)?.trim().to_string();
    metadata.insert("ColorSpaceData".to_string(), TagValue::new_string(color_space));
    Ok(())
}

/// Extracts Profile Connection Space from header
fn extract_pcs(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    let pcs = read_signature(data, offset)?.trim().to_string();
    metadata.insert("ProfileConnectionSpace".to_string(), TagValue::new_string(pcs));
    Ok(())
}

/// Extracts date/time from header
fn extract_datetime(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    if data.len() < offset + 12 {
        return Ok(());
    }

    let year = read_u16_be(data, offset)?;
    let month = read_u16_be(data, offset + 2)?;
    let day = read_u16_be(data, offset + 4)?;
    let hour = read_u16_be(data, offset + 6)?;
    let minute = read_u16_be(data, offset + 8)?;
    let second = read_u16_be(data, offset + 10)?;

    let datetime = format!(
        "{}:{:02}:{:02} {:02}:{:02}:{:02}",
        year, month, day, hour, minute, second
    );
    metadata.insert("ProfileDateTime".to_string(), TagValue::new_string(datetime));
    Ok(())
}

/// Extracts signature from header
fn extract_signature(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    let signature = read_signature(data, offset)?;
    metadata.insert("ProfileFileSignature".to_string(), TagValue::new_string(signature));
    Ok(())
}

/// Extracts primary platform from header
fn extract_platform(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    let platform = read_signature(data, offset)?;
    let platform_name = lookup_in_table(&PLATFORMS, &platform);
    if !platform_name.is_empty() {
        metadata.insert("PrimaryPlatform".to_string(), TagValue::new_string(platform_name.to_string()));
    }
    Ok(())
}

/// Extracts CMM flags from header
fn extract_flags(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    let flags = read_u32_be(data, offset)?;
    let embedded = if flags & 0x01 == 0 { "Not Embedded" } else { "Embedded" };
    let independent = if flags & 0x02 == 0 { "Independent" } else { "Dependent" };
    metadata.insert("CMMFlags".to_string(),
                   TagValue::new_string(format!("{}, {}", embedded, independent)));
    Ok(())
}

/// Extracts device manufacturer from header
fn extract_manufacturer(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    if data.len() >= offset + 4 {
        let manufacturer = read_signature(data, offset)?;
        if !manufacturer.trim().is_empty() {
            metadata.insert("DeviceManufacturer".to_string(), TagValue::new_string(manufacturer));
        }
    }
    Ok(())
}

/// Extracts device model from header
fn extract_model(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    if data.len() >= offset + 4 {
        let model = read_signature(data, offset)?;
        if !model.trim().is_empty() {
            metadata.insert("DeviceModel".to_string(), TagValue::new_string(model));
        }
    }
    Ok(())
}

/// Extracts device attributes from header
fn extract_attributes(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    if data.len() < offset + 8 {
        return Ok(());
    }

    let attrs = read_u64_be(data, offset)?;
    let reflective = if attrs & 0x01 == 0 { "Reflective" } else { "Transparency" };
    let glossy = if attrs & 0x02 == 0 { "Glossy" } else { "Matte" };
    let positive = if attrs & 0x04 == 0 { "Positive" } else { "Negative" };
    let color = if attrs & 0x08 == 0 { "Color" } else { "B&W" };

    metadata.insert("DeviceAttributes".to_string(),
                   TagValue::new_string(format!("{}, {}, {}, {}", reflective, glossy, positive, color)));
    Ok(())
}

/// Extracts rendering intent from header
fn extract_rendering_intent(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    if data.len() < offset + 4 {
        return Ok(());
    }

    let intent = read_u32_be(data, offset)?;
    let intent_name = RENDERING_INTENTS.get(intent as usize)
        .copied()
        .unwrap_or("Unknown");
    metadata.insert("RenderingIntent".to_string(), TagValue::new_string(intent_name.to_string()));
    Ok(())
}

/// Extracts connection space illuminant from header
fn extract_illuminant(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    if data.len() >= offset + 12 {
        let x = read_s15fixed16(data, offset)?;
        let y = read_s15fixed16(data, offset + 4)?;
        let z = read_s15fixed16(data, offset + 8)?;
        metadata.insert("ConnectionSpaceIlluminant".to_string(),
                       TagValue::new_string(format!("{} {} {}", x, y, z)));
    }
    Ok(())
}

/// Extracts profile creator from header
fn extract_creator(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    if data.len() >= offset + 4 {
        let creator = read_signature(data, offset)?;
        if !creator.trim().is_empty() {
            metadata.insert("ProfileCreator".to_string(), TagValue::new_string(creator));
        }
    }
    Ok(())
}

/// Extracts profile ID (MD5 hash) from header
fn extract_profile_id(data: &[u8], offset: usize, metadata: &mut HashMap<String, TagValue>) -> Result<()> {
    if data.len() >= offset + 16 {
        let id_bytes = &data[offset..offset + 16];
        if id_bytes.iter().all(|&b| b == 0) {
            metadata.insert("ProfileID".to_string(), TagValue::new_string("0".to_string()));
        } else {
            let id_hex = id_bytes.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();
            metadata.insert("ProfileID".to_string(), TagValue::new_string(id_hex));
        }
    }
    Ok(())
}

// ============================================================================
// LOOKUP TABLE HELPER
// ============================================================================

/// Generic lookup function for finding names in lookup tables
fn lookup_in_table<'a>(table: &'a [LookupEntry], code: &'a str) -> &'a str {
    let trimmed = code.trim();
    table.iter()
        .find(|entry| entry.code == trimmed)
        .map(|entry| entry.name)
        .unwrap_or(trimmed)
}

// ============================================================================
// ICC DATA TYPE PARSERS
// ============================================================================

/// Parses ICC textDescriptionType (old-style text)
fn parse_text_description_type(data: &[u8]) -> Result<String> {
    if data.len() < 12 {
        return Err(ExifToolError::parse_error("textDescriptionType too small"));
    }

    let type_sig = read_signature(data, 0)?;

    if type_sig.trim() == "desc" {
        let ascii_count = read_u32_be(data, 8)? as usize;
        if ascii_count > 0 && data.len() >= 12 + ascii_count {
            let text_bytes = &data[12..12 + ascii_count];
            let text_len = text_bytes.iter().position(|&b| b == 0)
                .unwrap_or(text_bytes.len());
            return Ok(String::from_utf8_lossy(&text_bytes[..text_len]).to_string());
        }
    } else if type_sig.trim() == "mluc" {
        return parse_mluc_type(data);
    }

    Err(ExifToolError::parse_error("Invalid text description type"))
}

/// Parses ICC multiLocalizedUnicodeType (modern text format)
fn parse_mluc_type(data: &[u8]) -> Result<String> {
    if data.len() < 16 {
        return Err(ExifToolError::parse_error("mluc type too small"));
    }

    let num_records = read_u32_be(data, 8)? as usize;
    if num_records == 0 {
        return Err(ExifToolError::parse_error("mluc has no records"));
    }

    if data.len() < 16 + 12 {
        return Err(ExifToolError::parse_error("mluc record table too small"));
    }

    let str_length = read_u32_be(data, 16 + 8)? as usize;
    let str_offset = read_u32_be(data, 16 + 12)? as usize;

    if str_offset + str_length > data.len() {
        return Err(ExifToolError::parse_error("mluc string out of bounds"));
    }

    let utf16_bytes = &data[str_offset..str_offset + str_length];
    let u16_vec: Vec<u16> = utf16_bytes
        .chunks(2)
        .filter_map(|chunk| {
            if chunk.len() == 2 {
                Some(u16::from_be_bytes([chunk[0], chunk[1]]))
            } else {
                None
            }
        })
        .collect();

    String::from_utf16(&u16_vec)
        .map_err(|_| ExifToolError::parse_error("Invalid UTF-16 in mluc string"))
}

/// Parses ICC textType (simple text)
fn parse_text_type(data: &[u8]) -> Result<String> {
    if data.len() < 8 {
        return Err(ExifToolError::parse_error("textType too small"));
    }

    let type_sig = read_signature(data, 0)?;

    if type_sig.trim() == "text" {
        let text_bytes = &data[8..];
        let text_len = text_bytes.iter().position(|&b| b == 0)
            .unwrap_or(text_bytes.len());
        return Ok(String::from_utf8_lossy(&text_bytes[..text_len]).to_string());
    } else if type_sig.trim() == "mluc" {
        return parse_mluc_type(data);
    }

    Err(ExifToolError::parse_error("Invalid text type"))
}

/// Parses ICC XYZType (XYZ color values)
fn parse_xyz_type(data: &[u8]) -> Result<(f64, f64, f64)> {
    if data.len() < 20 {
        return Err(ExifToolError::parse_error("XYZType too small"));
    }

    let x = read_s15fixed16(data, 8)?;
    let y = read_s15fixed16(data, 12)?;
    let z = read_s15fixed16(data, 16)?;

    Ok((x, y, z))
}

/// Parses ICC signatureType (4-byte signature)
fn parse_signature_type(data: &[u8]) -> Result<String> {
    if data.len() < 12 {
        return Err(ExifToolError::parse_error("signatureType too small"));
    }

    let sig = read_signature(data, 8)?;
    Ok(sig)
}

/// Parses ICC viewing conditions structure
fn parse_viewing_conditions(data: &[u8]) -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();

    if data.len() < 36 {
        return Err(ExifToolError::parse_error("Viewing conditions too small"));
    }

    let illum_x = read_s15fixed16(data, 8)?;
    let illum_y = read_s15fixed16(data, 12)?;
    let illum_z = read_s15fixed16(data, 16)?;
    result.insert("illuminant".to_string(), format!("{} {} {}", illum_x, illum_y, illum_z));

    let surr_x = read_s15fixed16(data, 20)?;
    let surr_y = read_s15fixed16(data, 24)?;
    let surr_z = read_s15fixed16(data, 28)?;
    result.insert("surround".to_string(), format!("{} {} {}", surr_x, surr_y, surr_z));

    if data.len() >= 36 {
        let illum_type = read_u32_be(data, 32)?;
        let illum_name = ILLUMINANT_TYPES.get(illum_type as usize)
            .copied()
            .unwrap_or("Unknown");
        result.insert("illuminant_type".to_string(), illum_name.to_string());
    }

    Ok(result)
}

/// Parses ICC measurement structure
fn parse_measurement(data: &[u8]) -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();

    if data.len() < 36 {
        return Err(ExifToolError::parse_error("Measurement data too small"));
    }

    let observer = read_u32_be(data, 8)?;
    let observer_name = OBSERVER_TYPES.get(observer as usize)
        .copied()
        .unwrap_or("Unknown");
    result.insert("observer".to_string(), observer_name.to_string());

    let back_x = read_s15fixed16(data, 12)?;
    let back_y = read_s15fixed16(data, 16)?;
    let back_z = read_s15fixed16(data, 20)?;
    result.insert("backing".to_string(), format!("{} {} {}", back_x, back_y, back_z));

    let geometry = read_u32_be(data, 24)?;
    let geometry_name = GEOMETRY_TYPES.get(geometry as usize)
        .copied()
        .unwrap_or("Unknown");
    result.insert("geometry".to_string(), geometry_name.to_string());

    if data.len() >= 32 {
        let flare = read_u16fixed16(data, 28)?;
        result.insert("flare".to_string(), format!("{}%", flare * 100.0));
    }

    if data.len() >= 36 {
        let illuminant = read_u32_be(data, 32)?;
        let illuminant_name = ILLUMINANT_TYPES.get(illuminant as usize)
            .copied()
            .unwrap_or("Unknown");
        result.insert("illuminant".to_string(), illuminant_name.to_string());
    }

    Ok(result)
}

// ============================================================================
// PDF EXTRACTION FUNCTIONS
// ============================================================================

/// Extracts the raw ICC profile data from a PDF file
fn extract_icc_from_pdf(reader: &dyn FileReader) -> Result<Vec<u8>> {
    let file_size = reader.size();

    // Read the last 1024 bytes to find trailer
    let tail_size = std::cmp::min(1024, file_size as usize);
    let tail_offset = file_size.saturating_sub(tail_size as u64);
    let tail_data = reader.read(tail_offset, tail_size)?;

    let xref_offset = find_xref_offset(tail_data)?;

    // Read xref table and trailer region
    let xref_size = std::cmp::min(8192, file_size.saturating_sub(xref_offset) as usize);
    let xref_data = reader.read(xref_offset, xref_size)?;

    let xref_map = parse_xref_table(xref_data)?;
    let root_ref = find_root_reference(xref_data)?;

    let root_offset = *xref_map.get(&root_ref)
        .ok_or_else(|| ExifToolError::parse_error(
            format!("Root object {} not found in xref table", root_ref)))?;

    // Read Root/Catalog object
    let root_size = std::cmp::min(8192, file_size.saturating_sub(root_offset) as usize);
    let root_data = reader.read(root_offset, root_size)?;

    // Find ICC profile reference
    let output_profile_ref = find_output_profile_reference(root_data)
        .or_else(|_| find_icc_based_reference(reader, &xref_map))?;

    let profile_offset = *xref_map.get(&output_profile_ref)
        .ok_or_else(|| ExifToolError::parse_error(
            format!("ICC profile object {} not found in xref table", output_profile_ref)))?;

    // Read ICC profile stream object
    let profile_size = std::cmp::min(131072, file_size.saturating_sub(profile_offset) as usize);
    let profile_data = reader.read(profile_offset, profile_size)?;

    extract_and_decompress_stream(profile_data)
}

/// Finds the /OutputIntents -> /DestOutputProfile reference in the Catalog
fn find_output_profile_reference(root_data: &[u8]) -> Result<u32> {
    let output_intents_pos = find_bytes(root_data, b"/OutputIntents")
        .ok_or_else(|| ExifToolError::parse_error("No /OutputIntents found in Catalog (no ICC profile)"))?;

    let after_output = &root_data[output_intents_pos..];
    let dest_profile_pos = find_bytes(after_output, b"/DestOutputProfile")
        .ok_or_else(|| ExifToolError::parse_error("No /DestOutputProfile found in OutputIntents"))?;

    let after_dest = &after_output[dest_profile_pos + 18..];
    let after_dest_str = std::str::from_utf8(&after_dest[..std::cmp::min(100, after_dest.len())])
        .unwrap_or("");

    parse_object_ref(after_dest_str)
}

/// Finds ICC profile reference via /ICCBased in the PDF
fn find_icc_based_reference(reader: &dyn FileReader, _xref_map: &HashMap<u32, u64>) -> Result<u32> {
    let file_size = reader.size();
    let search_size = std::cmp::min(65536, file_size as usize);
    let search_data = reader.read(0, search_size)?;

    let icc_based_pos = find_bytes(search_data, b"/ICCBased")
        .ok_or_else(|| ExifToolError::parse_error("No /ICCBased reference found (no ICC profile)"))?;

    let after_icc = &search_data[icc_based_pos + 9..];
    let after_icc_str = String::from_utf8_lossy(&after_icc[..std::cmp::min(100, after_icc.len())]);

    parse_object_ref(&after_icc_str)
}

/// Parses an object reference like "5 0 R" and returns the object number
fn parse_object_ref(s: &str) -> Result<u32> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() {
        return Err(ExifToolError::parse_error("No object reference found"));
    }

    if let Some(idx) = parts.iter().position(|&p| p == "R" || p.starts_with('R')) {
        if idx >= 2 {
            return parts[idx - 2].parse::<u32>()
                .map_err(|_| ExifToolError::parse_error("Invalid object number in reference"));
        } else if idx >= 1 {
            return parts[0].parse::<u32>()
                .map_err(|_| ExifToolError::parse_error("Invalid object number in reference"));
        }
    }

    for part in &parts {
        if let Ok(num) = part.parse::<u32>() {
            return Ok(num);
        }
    }

    Err(ExifToolError::parse_error("Invalid object reference format"))
}

/// Extracts and decompresses a stream from a PDF object
fn extract_and_decompress_stream(obj_data: &[u8]) -> Result<Vec<u8>> {
    let stream_start_pos = find_bytes(obj_data, b"stream")
        .ok_or_else(|| ExifToolError::parse_error("No stream marker found in object"))?;

    let mut stream_offset = stream_start_pos + 6;

    // Skip newline after "stream"
    if obj_data.len() > stream_offset {
        if obj_data[stream_offset] == b'\r' {
            stream_offset += 1;
        }
        if obj_data.len() > stream_offset && obj_data[stream_offset] == b'\n' {
            stream_offset += 1;
        }
    }

    let endstream_pos = find_bytes(obj_data, b"endstream")
        .ok_or_else(|| ExifToolError::parse_error("No endstream marker found - stream may be truncated"))?;

    let stream_data = &obj_data[stream_offset..endstream_pos];

    // Check if stream is compressed
    let header_data = &obj_data[..stream_start_pos];
    let is_compressed = find_bytes(header_data, b"/FlateDecode").is_some()
        || find_bytes(header_data, b"/Fl").is_some();

    if is_compressed {
        let mut decoder = ZlibDecoder::new(stream_data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| ExifToolError::parse_error(format!("Failed to decompress FlateDecode stream: {}", e)))?;
        Ok(decompressed)
    } else {
        Ok(stream_data.to_vec())
    }
}

/// Finds the /Root reference from trailer
fn find_root_reference(xref_data: &[u8]) -> Result<u32> {
    let xref_str = std::str::from_utf8(xref_data)
        .map_err(|_| ExifToolError::parse_error("xref data contains invalid UTF-8"))?;

    let trailer_pos = xref_str.find("trailer")
        .ok_or_else(|| ExifToolError::parse_error("trailer not found in PDF"))?;

    let after_trailer = &xref_str[trailer_pos..];
    let root_pos = after_trailer.find("/Root")
        .ok_or_else(|| ExifToolError::parse_error("No /Root reference in trailer"))?;

    let after_root = &after_trailer[root_pos + 5..];
    parse_object_ref(after_root)
}

/// Finds startxref offset from PDF tail
fn find_xref_offset(tail_data: &[u8]) -> Result<u64> {
    let tail_str = std::str::from_utf8(tail_data)
        .map_err(|_| ExifToolError::parse_error("PDF tail contains invalid UTF-8"))?;

    let startxref_pos = tail_str.rfind("startxref")
        .ok_or_else(|| ExifToolError::parse_error("startxref not found in PDF"))?;

    let after_keyword = &tail_str[startxref_pos + 9..];
    let num_str: String = after_keyword.chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect();

    num_str.parse::<u64>()
        .map_err(|_| ExifToolError::parse_error("Invalid xref offset after startxref"))
}

/// Parses xref table and builds object offset map
fn parse_xref_table(xref_data: &[u8]) -> Result<HashMap<u32, u64>> {
    let xref_str = std::str::from_utf8(xref_data)
        .map_err(|_| ExifToolError::parse_error("xref table contains invalid UTF-8"))?;

    let xref_pos = xref_str.find("xref")
        .ok_or_else(|| ExifToolError::parse_error("xref table not found"))?;

    let after_xref = &xref_str[xref_pos + 4..];
    let mut xref_map = HashMap::new();
    let lines: Vec<&str> = after_xref.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("trailer") {
            break;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 {
            if let (Ok(start_obj), Ok(count)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                for j in 0..count {
                    i += 1;
                    if i >= lines.len() {
                        break;
                    }

                    let entry_line = lines[i].trim();
                    let entry_parts: Vec<&str> = entry_line.split_whitespace().collect();

                    if entry_parts.len() >= 3 {
                        if let Ok(offset) = entry_parts[0].parse::<u64>() {
                            if entry_parts[2] == "n" {
                                let obj_num = start_obj + j;
                                xref_map.insert(obj_num, offset);
                            }
                        }
                    }
                }
            }
        }

        i += 1;
    }

    if xref_map.is_empty() {
        return Err(ExifToolError::parse_error("No valid xref entries found"));
    }

    Ok(xref_map)
}

/// Finds a byte sequence in a larger byte slice
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

// ============================================================================
// BINARY DATA READERS
// ============================================================================

/// Reads a 4-byte big-endian unsigned integer
fn read_u32_be(data: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > data.len() {
        return Err(ExifToolError::parse_error("Offset out of bounds"));
    }
    Ok(u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

/// Reads a 2-byte big-endian unsigned integer
fn read_u16_be(data: &[u8], offset: usize) -> Result<u16> {
    if offset + 2 > data.len() {
        return Err(ExifToolError::parse_error("Offset out of bounds"));
    }
    Ok(u16::from_be_bytes([data[offset], data[offset + 1]]))
}

/// Reads an 8-byte big-endian unsigned integer
fn read_u64_be(data: &[u8], offset: usize) -> Result<u64> {
    if offset + 8 > data.len() {
        return Err(ExifToolError::parse_error("Offset out of bounds"));
    }
    Ok(u64::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ]))
}

/// Reads a 4-byte signature as a trimmed ASCII string
fn read_signature(data: &[u8], offset: usize) -> Result<String> {
    if offset + 4 > data.len() {
        return Err(ExifToolError::parse_error("Offset out of bounds"));
    }
    let bytes = &data[offset..offset + 4];
    Ok(String::from_utf8_lossy(bytes).to_string())
}

/// Reads a signed 15.16 fixed-point number and converts to f64
fn read_s15fixed16(data: &[u8], offset: usize) -> Result<f64> {
    let value = read_u32_be(data, offset)? as i32;
    let integer_part = (value >> 16) as i16 as f64;
    let fractional_part = (value & 0xFFFF) as f64 / 65536.0;
    Ok(integer_part + fractional_part)
}

/// Reads an unsigned 16.16 fixed-point number and converts to f64
fn read_u16fixed16(data: &[u8], offset: usize) -> Result<f64> {
    let value = read_u32_be(data, offset)?;
    let integer_part = (value >> 16) as f64;
    let fractional_part = (value & 0xFFFF) as f64 / 65536.0;
    Ok(integer_part + fractional_part)
}
