//! Format detection via magic byte analysis
//!
//! This module provides format detection capabilities for determining file types
//! by examining magic bytes (file signatures) at the beginning of files.
//!
//! # Architectural Role
//!
//! The format detector is part of the **infrastructure layer** and serves as the
//! entry point for the parsing pipeline. It uses the `FileReader` port to read
//! magic bytes and returns a `FileFormat` enum variant that routes to the
//! appropriate format parser.
//!
//! # Supported Formats
//!
//! The detector currently identifies:
//! - JPEG: 0xFF 0xD8 0xFF
//! - TIFF (Little-Endian): 0x49 0x49 0x2A 0x00
//! - TIFF (Big-Endian): 0x4D 0x4D 0x00 0x2A
//! - PNG: 0x89 0x50 0x4E 0x47
//! - FLAC: 0x66 0x4C 0x61 0x43 ("fLaC")
//! - PDF: 0x25 0x50 0x44 0x46
//! - QuickTime/MP4: "ftyp" at bytes 4-7
//!
//! Unknown formats return `FileFormat::Unknown`.
//!
//! # Examples
//!
//! ```no_run
//! use oxidex::parsers::format_detector::detect_format;
//! use oxidex::io::MMapReader;
//! use std::path::Path;
//!
//! # fn example() -> std::io::Result<()> {
//! let reader = MMapReader::new(Path::new("image.jpg"))?;
//! let format = detect_format(&reader)?;
//! println!("Detected format: {}", format);
//! # Ok(())
//! # }
//! ```

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader};
use crate::parsers::raw;
use std::io;

/// Detects the file format by examining magic bytes.
///
/// This function reads the first 16 bytes of the file (or fewer if the file is smaller)
/// and matches them against known format signatures. Format detection is performed
/// by checking byte sequences in order from most specific to least specific.
///
/// # Arguments
///
/// * `reader` - A file reader providing access to file contents via the FileReader port
///
/// # Returns
///
/// * `Ok(FileFormat)` - The detected format, or `FileFormat::Unknown` if unrecognized
/// * `Err(io::Error)` - An I/O error occurred while reading the file
///
/// # Error Handling
///
/// This function gracefully handles files smaller than 16 bytes by reading only the
/// available bytes and attempting format detection with the partial data. Empty files
/// return `Ok(FileFormat::Unknown)`.
///
/// # Magic Byte Sequences
///
/// - **JPEG**: `FF D8 FF` (3 bytes) - JPEG Start of Image marker
/// - **TIFF (LE)**: `49 49 2A 00` (4 bytes) - "II" + magic number 42 (little-endian)
/// - **TIFF (BE)**: `4D 4D 00 2A` (4 bytes) - "MM" + magic number 42 (big-endian)
/// - **PNG**: `89 50 4E 47` (4 bytes) - PNG signature (first 4 of 8 bytes)
/// - **PDF**: `25 50 44 46` (4 bytes) - "%PDF" ASCII signature
///
/// # Examples
///
/// ```no_run
/// use oxidex::parsers::format_detector::detect_format;
/// use oxidex::io::MMapReader;
/// use oxidex::core::FileFormat;
/// use std::path::Path;
///
/// # fn example() -> std::io::Result<()> {
/// let reader = MMapReader::new(Path::new("photo.jpg"))?;
/// let format = detect_format(&reader)?;
///
/// match format {
///     FileFormat::JPEG => println!("JPEG image detected"),
///     FileFormat::PNG => println!("PNG image detected"),
///     FileFormat::TIFF => println!("TIFF image detected"),
///     FileFormat::PDF => println!("PDF document detected"),
///     FileFormat::Unknown => println!("Unknown or unsupported format"),
///     _ => println!("Other format detected"),
/// }
/// # Ok(())
/// # }
/// ```
pub fn detect_format(reader: &dyn FileReader) -> io::Result<FileFormat> {
    // Attempt to read 600 bytes for magic byte detection (increased to support MTS which needs 3 packets)
    // If the file is smaller, read only what's available
    let magic_bytes = match reader.read(0, 600) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
            // File is smaller than 600 bytes, try reading available bytes
            let size = reader.size() as usize;
            if size == 0 {
                // Empty file - cannot determine format
                return Ok(FileFormat::Unknown);
            }
            // Read whatever is available
            reader.read(0, size)?
        }
        Err(e) => return Err(e),
    };

    // Check for empty result (edge case)
    if magic_bytes.is_empty() {
        return Ok(FileFormat::Unknown);
    }

    // Check for raw formats first (before TIFF check)
    // Many raw formats are TIFF-based, so we need to check for raw-specific signatures
    // before falling back to generic TIFF detection
    //
    // Note: Without filename context, we can only detect raw formats with unique magic bytes
    // (CR2, CR3, RAF, X3F, MRW). TIFF-based raw formats without distinctive markers
    // (NEF, ARW, DNG, etc.) will be detected as TIFF and require filename-based detection
    // at a higher level.

    // Canon CR2 has "CR\x02\x00" marker at offset 8
    if magic_bytes.len() >= 12
        && magic_bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00])
        && &magic_bytes[8..12] == b"CR\x02\x00"
    {
        return Ok(FileFormat::CameraRaw(raw::RawFormat::CanonCR2));
    }

    // Canon CR3 uses ISO Base Media Format with "ftypcrx " marker
    if magic_bytes.len() >= 12 && &magic_bytes[4..12] == b"ftypcrx " {
        return Ok(FileFormat::CameraRaw(raw::RawFormat::CanonCR3));
    }

    // Fujifilm RAF has "FUJIFILMCCD-RAW " signature
    if magic_bytes.len() >= 16 && &magic_bytes[0..16] == b"FUJIFILMCCD-RAW " {
        return Ok(FileFormat::CameraRaw(raw::RawFormat::FujifilmRAF));
    }

    // Sigma X3F has "FOVb" signature
    if magic_bytes.len() >= 4 && &magic_bytes[0..4] == b"FOVb" {
        return Ok(FileFormat::CameraRaw(raw::RawFormat::SigmaX3F));
    }

    // Minolta MRW has "\x00MRM" signature
    if magic_bytes.len() >= 4 && &magic_bytes[0..4] == b"\x00MRM" {
        return Ok(FileFormat::CameraRaw(raw::RawFormat::MinoltaMRW));
    }

    // Check formats in order from most specific to least specific
    // Start with 4-byte signatures, then 3-byte signatures

    // TIFF Little-Endian: 0x49 0x49 0x2A 0x00 ("II" + 42 in LE)
    // Note: Many raw formats (NEF, ARW, DNG, PEF, etc.) are TIFF-based
    // and will be detected here. Higher-level code should use filename to distinguish.
    if magic_bytes.len() >= 4 && magic_bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00]) {
        return Ok(FileFormat::TIFF);
    }

    // TIFF Big-Endian: 0x4D 0x4D 0x00 0x2A ("MM" + 42 in BE)
    // Note: NEF and some other raw formats use this signature
    if magic_bytes.len() >= 4 && magic_bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A]) {
        return Ok(FileFormat::TIFF);
    }

    // Panasonic RW2: 0x49 0x49 0x55 0x00 ("II" + 0x55 instead of 42)
    // RW2 files use a TIFF variant with magic number 0x55 (85) instead of 0x2A (42)
    if magic_bytes.len() >= 4 && magic_bytes.starts_with(&[0x49, 0x49, 0x55, 0x00]) {
        return Ok(FileFormat::TIFF);
    }

    // Olympus ORF: 0x49 0x49 0x52 0x4F ("II" + "RO" instead of 42)
    // ORF files use "IIRO" header where "RO" stands for "Raw Olympus"
    // The "RO" (0x52 0x4F) replaces the standard TIFF magic number 42 (0x2A)
    if magic_bytes.len() >= 4 && magic_bytes.starts_with(&[0x49, 0x49, 0x52, 0x4F]) {
        return Ok(FileFormat::TIFF);
    }

    // Olympus ORF (variant): 0x49 0x49 0x52 0x53 ("II" + "RS" instead of 42)
    // Older Olympus compact cameras (C5050Z, C5060WZ, C7070WZ, SP350, SP500UZ, SP510UZ, SP550UZ, SP565UZ, SP570UZ, C8080WZ)
    // use "IIRS" header. The "RS" (0x52 0x53) replaces the standard TIFF magic number
    if magic_bytes.len() >= 4 && magic_bytes.starts_with(&[0x49, 0x49, 0x52, 0x53]) {
        return Ok(FileFormat::TIFF);
    }

    // Olympus ORF (Big-Endian variant): 0x4D 0x4D 0x4F 0x52 ("MM" + "OR")
    // Some Olympus cameras use big-endian byte order with "MMOR" header
    if magic_bytes.len() >= 4 && magic_bytes.starts_with(&[0x4D, 0x4D, 0x4F, 0x52]) {
        return Ok(FileFormat::TIFF);
    }

    // PNG: 0x89 0x50 0x4E 0x47 (first 4 bytes of 8-byte signature)
    if magic_bytes.len() >= 4 && magic_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return Ok(FileFormat::PNG);
    }

    // FLAC: "fLaC" signature
    if magic_bytes.len() >= 4 && &magic_bytes[0..4] == b"fLaC" {
        return Ok(FileFormat::FLAC);
    }

    // MP3: ID3v2 tag or MPEG frame sync
    // ID3v2: "ID3" signature at start
    if magic_bytes.len() >= 3 && &magic_bytes[0..3] == b"ID3" {
        return Ok(FileFormat::MP3);
    }
    // MPEG frame sync: 0xFF 0xFB or 0xFF 0xFA (11 bits set)
    if magic_bytes.len() >= 2 && magic_bytes[0] == 0xFF && (magic_bytes[1] & 0xE0) == 0xE0 {
        return Ok(FileFormat::MP3);
    }

    // FLV: "FLV" signature
    if magic_bytes.len() >= 3 && &magic_bytes[0..3] == b"FLV" {
        return Ok(FileFormat::FLV);
    }

    // MTS/M2TS: MPEG-TS sync byte pattern (0x47 repeating every 188 or 192 bytes)
    // Check for standard TS (188 bytes)
    if magic_bytes.len() >= 564  // 3 packets
        && magic_bytes[0] == 0x47
        && magic_bytes[188] == 0x47
        && magic_bytes[376] == 0x47
    {
        return Ok(FileFormat::MTS);
    }
    // Check for M2TS (192 bytes with 4-byte timestamp)
    if magic_bytes.len() >= 576  // 3 packets
        && magic_bytes[4] == 0x47
        && magic_bytes[196] == 0x47
        && magic_bytes[388] == 0x47
    {
        return Ok(FileFormat::MTS);
    }

    // AAC: ADTS sync word (0xFFF in first 12 bits)
    // 0xFF 0xF1 or 0xFF 0xF9 are common ADTS patterns
    if magic_bytes.len() >= 2
        && magic_bytes[0] == 0xFF
        && (magic_bytes[1] == 0xF1 || magic_bytes[1] == 0xF9)
    {
        return Ok(FileFormat::AAC);
    }

    // APE: "MAC " signature
    if magic_bytes.len() >= 4 && &magic_bytes[0..4] == b"MAC " {
        return Ok(FileFormat::APE);
    }

    // MKV/Matroska/WebM: EBML signature 0x1A 0x45 0xDF 0xA3
    // Note: Both MKV and WebM use EBML, need to parse DocType to distinguish
    // For now, detect as MKV (can be refined later with DocType parsing)
    if magic_bytes.len() >= 4 && &magic_bytes[0..4] == b"\x1A\x45\xDF\xA3" {
        // TODO: Parse EBML header to check DocType ("matroska" vs "webm")
        return Ok(FileFormat::MKV);
    }

    // OGG: "OggS" signature (used by Vorbis and Opus)
    // Need to peek into first page to distinguish Opus from Vorbis
    if magic_bytes.len() >= 4 && &magic_bytes[0..4] == b"OggS" {
        // Check for Opus signature ("OpusHead" at typical offset)
        if magic_bytes.len() >= 36 && &magic_bytes[28..36] == b"OpusHead" {
            return Ok(FileFormat::OPUS);
        }
        // Default to OGG Vorbis
        return Ok(FileFormat::OGG);
    }

    // RIFF-based formats (WAV, AVI)
    // RIFF header: "RIFF" + 4 bytes size + format type
    if magic_bytes.len() >= 12 && &magic_bytes[0..4] == b"RIFF" {
        let format_type = &magic_bytes[8..12];
        if format_type == b"WAVE" {
            return Ok(FileFormat::WAV);
        } else if format_type == b"AVI " {
            return Ok(FileFormat::AVI);
        }
    }

    // PDF: 0x25 0x50 0x44 0x46 ("%PDF")
    if magic_bytes.len() >= 4 && magic_bytes.starts_with(&[0x25, 0x50, 0x44, 0x46]) {
        return Ok(FileFormat::PDF);
    }

    // QuickTime/MP4: Check for "ftyp" atom at bytes 4-7
    // MP4/MOV files have structure: [4 bytes size][4 bytes type "ftyp"]
    // Common types after ftyp: "isom", "mp42", "mp41", "M4V ", "qt  ", etc.
    if magic_bytes.len() >= 8 && &magic_bytes[4..8] == b"ftyp" {
        return Ok(FileFormat::QuickTime);
    }

    // PE (Portable Executable): 0x4D 0x5A ("MZ") DOS signature
    // Must verify this is actually a PE file, not just an old DOS executable
    if magic_bytes.len() >= 64 && magic_bytes.starts_with(&[0x4D, 0x5A]) {
        // Read e_lfanew field at offset 0x3C (4 bytes, little-endian)
        // This points to the PE signature
        if magic_bytes.len() >= 0x40 {
            let e_lfanew_bytes = &magic_bytes[0x3C..0x40];
            let e_lfanew = u32::from_le_bytes([
                e_lfanew_bytes[0],
                e_lfanew_bytes[1],
                e_lfanew_bytes[2],
                e_lfanew_bytes[3],
            ]) as u64;

            // Verify PE signature exists at e_lfanew offset
            // PE signature is "PE\0\0" (0x50 0x45 0x00 0x00)
            if e_lfanew < reader.size() && e_lfanew + 4 <= reader.size() {
                if let Ok(pe_sig) = reader.read(e_lfanew, 4) {
                    if pe_sig == [0x50, 0x45, 0x00, 0x00] {
                        return Ok(FileFormat::PE);
                    }
                }
            }
        }
    }

    // ZIP-based formats: Check for "PK" signature (0x50 0x4B)
    // This includes ZIP, DOCX, XLSX, PPTX, Pages, Numbers, Keynote, EPUB
    if magic_bytes.len() >= 2 && magic_bytes.starts_with(&[0x50, 0x4B]) {
        // Need to read more to distinguish ZIP-based formats
        // Try to detect Office Open XML formats by checking for specific files
        let size = reader.size() as usize;
        if let Ok(all_data) = reader.read(0, size) {
            use std::io::Cursor;
            use zip::ZipArchive;

            if let Ok(mut archive) = ZipArchive::new(Cursor::new(all_data)) {
                // Check for EPUB (mimetype file)
                if archive.by_name("mimetype").is_ok() {
                    return Ok(FileFormat::EPUB);
                }

                // Check for DOCX
                if archive.by_name("word/document.xml").is_ok() {
                    return Ok(FileFormat::DOCX);
                }

                // Check for XLSX
                if archive.by_name("xl/workbook.xml").is_ok() {
                    return Ok(FileFormat::XLSX);
                }

                // Check for PPTX
                if archive.by_name("ppt/presentation.xml").is_ok() {
                    return Ok(FileFormat::PPTX);
                }

                // Check for Pages
                if archive.by_name("Index/Document.iwa").is_ok() {
                    return Ok(FileFormat::Pages);
                }

                // Check for Numbers
                if archive.by_name("Index/Document.iwa").is_ok()
                    && archive.by_name("Index/Tables/").is_ok() {
                    return Ok(FileFormat::Numbers);
                }

                // Check for Keynote
                if archive.by_name("Index/Presentation.iwa").is_ok() {
                    return Ok(FileFormat::Keynote);
                }

                // Generic ZIP archive
                return Ok(FileFormat::ZIP);
            }
        }

        // If we can't read it as a ZIP, still recognize it as ZIP
        return Ok(FileFormat::ZIP);
    }

    // JPEG: 0xFF 0xD8 0xFF (SOI marker + start of next marker)
    // Note: JPEG signature is 3 bytes, checked after 4-byte signatures
    if magic_bytes.len() >= 3 && magic_bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Ok(FileFormat::JPEG);
    }

    // No known format matched
    Ok(FileFormat::Unknown)
}

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
            let end = start.saturating_add(length).min(self.data.len());

            if start > self.data.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "offset beyond end of data",
                ));
            }

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
    fn test_detect_jpeg() {
        // Valid JPEG magic bytes: FF D8 FF
        let data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::JPEG);
    }

    #[test]
    fn test_detect_tiff_little_endian() {
        // TIFF Little-Endian: 49 49 2A 00 ("II" + 42 in LE)
        let data = vec![0x49, 0x49, 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::TIFF);
    }

    #[test]
    fn test_detect_tiff_big_endian() {
        // TIFF Big-Endian: 4D 4D 00 2A ("MM" + 42 in BE)
        let data = vec![0x4D, 0x4D, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x08];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::TIFF);
    }

    #[test]
    fn test_detect_png() {
        // PNG signature (first 8 bytes, but we only check first 4)
        let data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::PNG);
    }

    #[test]
    fn test_detect_pdf() {
        // PDF signature: "%PDF"
        let data = vec![0x25, 0x50, 0x44, 0x46, 0x2D, 0x31, 0x2E, 0x34];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::PDF);
    }

    #[test]
    fn test_detect_unknown() {
        // Random bytes that don't match any format
        let data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::Unknown);
    }

    #[test]
    fn test_empty_file() {
        // Empty file (0 bytes)
        let data = vec![];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::Unknown);
    }

    #[test]
    fn test_file_too_small_one_byte() {
        // File with only 1 byte (smaller than any magic byte sequence)
        let data = vec![0xFF];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::Unknown);
    }

    #[test]
    fn test_file_too_small_two_bytes() {
        // File with 2 bytes (still too small for any format)
        let data = vec![0xFF, 0xD8];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::Unknown);
    }

    #[test]
    fn test_short_file_matches_jpeg() {
        // File with exactly 3 bytes that match JPEG
        let data = vec![0xFF, 0xD8, 0xFF];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::JPEG);
    }

    #[test]
    fn test_short_file_matches_pdf() {
        // File with exactly 4 bytes that match PDF
        let data = vec![0x25, 0x50, 0x44, 0x46];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::PDF);
    }

    #[test]
    fn test_jpeg_with_padding() {
        // JPEG with additional data after magic bytes
        let mut data = vec![0xFF, 0xD8, 0xFF, 0xE1];
        data.extend_from_slice(&[0x00; 20]); // Add padding
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::JPEG);
    }

    #[test]
    fn test_tiff_little_endian_minimal() {
        // Minimal TIFF LE header (exactly 4 bytes)
        let data = vec![0x49, 0x49, 0x2A, 0x00];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::TIFF);
    }

    #[test]
    fn test_tiff_big_endian_minimal() {
        // Minimal TIFF BE header (exactly 4 bytes)
        let data = vec![0x4D, 0x4D, 0x00, 0x2A];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::TIFF);
    }

    #[test]
    fn test_png_full_signature() {
        // Full 8-byte PNG signature
        let data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::PNG);
    }

    #[test]
    fn test_partial_match_not_detected() {
        // Bytes that partially match JPEG but not completely
        let data = vec![0xFF, 0xD8, 0x00, 0x00]; // Missing third 0xFF
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::Unknown);
    }

    #[test]
    fn test_pdf_with_version() {
        // PDF with version string "%PDF-1.7"
        let data = vec![0x25, 0x50, 0x44, 0x46, 0x2D, 0x31, 0x2E, 0x37, 0x0A];
        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::PDF);
    }

    #[test]
    fn test_detect_pe_mz_signature() {
        // PE files start with "MZ" (0x4D 0x5A) DOS signature
        // Followed by DOS stub and NT headers
        let mut data = vec![0x4D, 0x5A]; // MZ signature
        data.extend_from_slice(&[0x90, 0x00]); // e_cblp
        data.extend_from_slice(&[0x03, 0x00]); // e_cp
                                               // Add padding to reach e_lfanew at offset 0x3C
        data.resize(0x3C, 0x00);
        data.extend_from_slice(&[0x80, 0x00, 0x00, 0x00]); // e_lfanew = 0x80
                                                           // Add padding and PE signature at offset 0x80
        data.resize(0x80, 0x00);
        data.extend_from_slice(&[0x50, 0x45, 0x00, 0x00]); // "PE\0\0" signature

        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::PE);
    }

    #[test]
    fn test_detect_pe_with_nt_signature() {
        // Complete PE file with NT signature
        let mut data = vec![0x4D, 0x5A]; // MZ
        data.resize(0x3C, 0x00);
        data.extend_from_slice(&[0x40, 0x00, 0x00, 0x00]); // e_lfanew = 0x40
        data.resize(0x40, 0x00);
        data.extend_from_slice(&[0x50, 0x45, 0x00, 0x00]); // "PE\0\0" signature

        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        assert_eq!(format, FileFormat::PE);
    }

    #[test]
    fn test_detect_non_pe_mz_file() {
        // Old DOS executable without PE signature
        let mut data = vec![0x4D, 0x5A]; // MZ
        data.resize(64, 0x00); // DOS header but no PE

        let reader = TestReader::new(data);
        let format = detect_format(&reader).unwrap();
        // Should be Unknown since no valid PE signature
        assert_eq!(format, FileFormat::Unknown);
    }
}
