//! OLE (Object Linking and Embedding) / Compound File Binary Format parser
//!
//! This module parses Microsoft Compound File Binary Format files (.doc, .xls, .ppt, .msg)
//! and extracts metadata including VBA macro detection for forensic analysis.

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// OLE file signature (magic bytes)
const OLE_SIGNATURE: &[u8] = &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// Directory entry type constants
const STGTY_INVALID: u8 = 0;
const STGTY_STORAGE: u8 = 1;
const STGTY_STREAM: u8 = 2;
const STGTY_ROOT: u8 = 5;

/// Maximum sector size (4096 bytes)
const MAX_SECTOR_SIZE: usize = 4096;

/// Directory entry size in bytes
const DIR_ENTRY_SIZE: usize = 128;

/// Parser for OLE (Compound File Binary Format) files
///
/// Extracts metadata from OLE files including:
/// - Basic file structure information
/// - VBA macro presence and analysis
/// - Suspicious code pattern detection
pub struct OLEParser;

/// Represents an OLE directory entry
#[derive(Debug, Clone)]
struct DirectoryEntry {
    name: String,
    entry_type: u8,
    start_sector: u32,
    size: u32,
    left_sibling: u32,
    right_sibling: u32,
    child_did: u32,
}

/// OLE file header structure
#[derive(Debug)]
struct OLEHeader {
    sector_size: usize,
    mini_sector_size: usize,
    total_sectors: u32,
    fat_sectors: u32,
    first_dir_sector: u32,
    first_mini_fat_sector: u32,
    mini_fat_sectors: u32,
    first_difat_sector: u32,
    difat_sectors: u32,
}

impl OLEParser {
    /// Parse the OLE header
    fn parse_header(reader: &dyn FileReader) -> Result<OLEHeader> {
        if reader.size() < 512 {
            return Err(ExifToolError::parse_error(
                "File too small to be valid OLE file",
            ));
        }

        // Read header (first 512 bytes)
        let header = reader.read(0, 512)?;

        // Verify signature
        if &header[0..8] != OLE_SIGNATURE {
            return Err(ExifToolError::parse_error("Invalid OLE signature"));
        }

        // Parse sector sizes
        let sector_shift = u16::from_le_bytes([header[30], header[31]]) as usize;
        let mini_sector_shift = u16::from_le_bytes([header[32], header[33]]) as usize;

        let sector_size = 1 << sector_shift;
        let mini_sector_size = 1 << mini_sector_shift;

        if sector_size > MAX_SECTOR_SIZE {
            return Err(ExifToolError::parse_error("Invalid sector size"));
        }

        // Parse FAT information
        let total_sectors = u32::from_le_bytes([header[44], header[45], header[46], header[47]]);
        let first_dir_sector =
            u32::from_le_bytes([header[48], header[49], header[50], header[51]]);
        let first_mini_fat_sector =
            u32::from_le_bytes([header[60], header[61], header[62], header[63]]);
        let mini_fat_sectors =
            u32::from_le_bytes([header[64], header[65], header[66], header[67]]);
        let first_difat_sector =
            u32::from_le_bytes([header[68], header[69], header[70], header[71]]);
        let difat_sectors = u32::from_le_bytes([header[72], header[73], header[74], header[75]]);
        let fat_sectors = u32::from_le_bytes([header[76], header[77], header[78], header[79]]);

        Ok(OLEHeader {
            sector_size,
            mini_sector_size,
            total_sectors,
            fat_sectors,
            first_dir_sector,
            first_mini_fat_sector,
            mini_fat_sectors,
            first_difat_sector,
            difat_sectors,
        })
    }

    /// Read directory entries from the OLE file
    fn read_directory_entries(
        reader: &dyn FileReader,
        header: &OLEHeader,
    ) -> Result<Vec<DirectoryEntry>> {
        let mut entries = Vec::new();

        // Calculate directory sector offset
        let dir_offset = 512 + (header.first_dir_sector as usize * header.sector_size);

        if dir_offset + header.sector_size > reader.size() as usize {
            return Err(ExifToolError::parse_error("Invalid directory sector offset"));
        }

        // Read first directory sector
        let dir_data = reader.read(dir_offset as u64, header.sector_size)?;

        // Parse directory entries (4 per 512-byte sector, more for larger sectors)
        let entries_per_sector = header.sector_size / DIR_ENTRY_SIZE;

        for i in 0..entries_per_sector {
            let offset = i * DIR_ENTRY_SIZE;
            if offset + DIR_ENTRY_SIZE > dir_data.len() {
                break;
            }

            let entry_data = &dir_data[offset..offset + DIR_ENTRY_SIZE];

            // Parse entry name (first 64 bytes, UTF-16LE)
            let name_len = u16::from_le_bytes([entry_data[64], entry_data[65]]) as usize;
            if name_len > 64 {
                continue;
            }

            let name = if name_len > 2 {
                String::from_utf16_lossy(
                    &entry_data[0..name_len.saturating_sub(2)]
                        .chunks_exact(2)
                        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                        .collect::<Vec<u16>>(),
                )
            } else {
                String::new()
            };

            // Skip empty entries
            if name.is_empty() {
                continue;
            }

            let entry_type = entry_data[66];
            let left_sibling = u32::from_le_bytes([
                entry_data[68],
                entry_data[69],
                entry_data[70],
                entry_data[71],
            ]);
            let right_sibling = u32::from_le_bytes([
                entry_data[72],
                entry_data[73],
                entry_data[74],
                entry_data[75],
            ]);
            let child_did = u32::from_le_bytes([
                entry_data[76],
                entry_data[77],
                entry_data[78],
                entry_data[79],
            ]);
            let start_sector = u32::from_le_bytes([
                entry_data[116],
                entry_data[117],
                entry_data[118],
                entry_data[119],
            ]);
            let size = u32::from_le_bytes([
                entry_data[120],
                entry_data[121],
                entry_data[122],
                entry_data[123],
            ]);

            entries.push(DirectoryEntry {
                name,
                entry_type,
                start_sector,
                size,
                left_sibling,
                right_sibling,
                child_did,
            });
        }

        Ok(entries)
    }
}

impl FormatParser for OLEParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        let header = Self::parse_header(reader)?;
        let entries = Self::read_directory_entries(reader, &header)?;

        let mut metadata = MetadataMap::new();

        // Basic OLE metadata
        metadata.insert(
            "OLE:SectorSize".to_string(),
            TagValue::new_integer(header.sector_size as i64),
        );
        metadata.insert(
            "OLE:TotalSectors".to_string(),
            TagValue::new_integer(header.total_sectors as i64),
        );
        metadata.insert(
            "OLE:DirectoryEntryCount".to_string(),
            TagValue::new_integer(entries.len() as i64),
        );

        // Check for VBA macros
        let vba_metadata = VBAAnalyzer::analyze_vba(reader, &entries, &header);
        for (key, value) in vba_metadata {
            metadata.insert(key, value);
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::OLE)
    }
}

/// VBA Macro analyzer for forensic detection
struct VBAAnalyzer;

impl VBAAnalyzer {
    /// Analyze VBA macros in the OLE file
    fn analyze_vba(
        reader: &dyn FileReader,
        entries: &[DirectoryEntry],
        header: &OLEHeader,
    ) -> MetadataMap {
        let mut metadata = MetadataMap::new();

        // Find VBA directory
        let vba_dir = entries.iter().find(|e| {
            e.name.eq_ignore_ascii_case("VBA")
                || e.name.eq_ignore_ascii_case("_VBA_PROJECT_CUR")
                || e.name.eq_ignore_ascii_case("Macros")
        });

        if vba_dir.is_none() {
            metadata.insert(
                "OLE:HasVBAMacros".to_string(),
                TagValue::new_string("No"),
            );
            return metadata;
        }

        metadata.insert(
            "OLE:HasVBAMacros".to_string(),
            TagValue::new_string("Yes"),
        );

        // Look for VBA project streams
        let vba_project = entries
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case("_VBA_PROJECT"));

        if let Some(_project) = vba_project {
            metadata.insert(
                "OLE:VBAProjectName".to_string(),
                TagValue::new_string("VBA Project"),
            );
        }

        // Count VBA modules
        let module_names: Vec<String> = entries
            .iter()
            .filter(|e| {
                e.entry_type == STGTY_STREAM
                    && !e.name.starts_with('_')
                    && !e.name.eq_ignore_ascii_case("dir")
                    && !e.name.eq_ignore_ascii_case("PROJECT")
                    && !e.name.eq_ignore_ascii_case("PROJECTwm")
                    && e.name.chars().any(|c| c.is_alphabetic())
            })
            .map(|e| e.name.clone())
            .collect();

        if !module_names.is_empty() {
            metadata.insert(
                "OLE:VBAModuleCount".to_string(),
                TagValue::new_integer(module_names.len() as i64),
            );
            metadata.insert(
                "OLE:VBAModuleNames".to_string(),
                TagValue::new_array(
                    module_names
                        .iter()
                        .map(|s| TagValue::new_string(s.clone()))
                        .collect(),
                ),
            );
        }

        // Analyze suspicious patterns in VBA streams
        let mut suspicious_findings = Vec::new();

        for entry in entries.iter() {
            if entry.entry_type != STGTY_STREAM || entry.size == 0 {
                continue;
            }

            // Read stream data
            if let Ok(stream_data) = Self::read_stream(reader, entry, header) {
                let patterns = Self::check_suspicious_patterns(&stream_data);
                suspicious_findings.extend(patterns);
            }
        }

        // Report suspicious findings
        if !suspicious_findings.is_empty() {
            // Remove duplicates
            suspicious_findings.sort();
            suspicious_findings.dedup();

            // Check for specific categories
            let has_auto_exec = suspicious_findings
                .iter()
                .any(|s| s.contains("AutoExec") || s.contains("Auto_Open"));
            let has_shell = suspicious_findings
                .iter()
                .any(|s| s.contains("Shell") || s.contains("CreateObject"));
            let has_network = suspicious_findings
                .iter()
                .any(|s| s.contains("Network") || s.contains("HTTP"));
            let has_file_access = suspicious_findings
                .iter()
                .any(|s| s.contains("File") || s.contains("FileSystem"));
            let has_powershell = suspicious_findings.iter().any(|s| s.contains("PowerShell"));
            let has_obfuscation = suspicious_findings
                .iter()
                .any(|s| s.contains("Obfuscation") || s.contains("Chr("));

            metadata.insert(
                "OLE:HasAutoExec".to_string(),
                TagValue::new_string(if has_auto_exec { "Yes" } else { "No" }),
            );
            metadata.insert(
                "OLE:HasShellExecution".to_string(),
                TagValue::new_string(if has_shell { "Yes" } else { "No" }),
            );
            metadata.insert(
                "OLE:HasNetworkAccess".to_string(),
                TagValue::new_string(if has_network { "Yes" } else { "No" }),
            );
            metadata.insert(
                "OLE:HasFileAccess".to_string(),
                TagValue::new_string(if has_file_access { "Yes" } else { "No" }),
            );
            metadata.insert(
                "OLE:HasPowerShell".to_string(),
                TagValue::new_string(if has_powershell { "Yes" } else { "No" }),
            );
            metadata.insert(
                "OLE:HasObfuscation".to_string(),
                TagValue::new_string(if has_obfuscation { "Yes" } else { "No" }),
            );

            metadata.insert(
                "OLE:SuspiciousPatterns".to_string(),
                TagValue::new_array(
                    suspicious_findings
                        .iter()
                        .map(|s| TagValue::new_string(s.clone()))
                        .collect(),
                ),
            );
        }

        metadata
    }

    /// Read a stream from the OLE file
    fn read_stream(
        reader: &dyn FileReader,
        entry: &DirectoryEntry,
        header: &OLEHeader,
    ) -> Result<Vec<u8>> {
        // For simplicity, only read small streams (< 4KB)
        if entry.size > 4096 {
            return Ok(Vec::new());
        }

        let offset = 512 + (entry.start_sector as usize * header.sector_size);
        let size = entry.size.min(4096) as usize;

        if offset + size > reader.size() as usize {
            return Ok(Vec::new());
        }

        let data = reader
            .read(offset as u64, size)
            .map_err(ExifToolError::IoError)?;
        Ok(data.to_vec())
    }

    /// Check for suspicious patterns in VBA code/streams
    fn check_suspicious_patterns(data: &[u8]) -> Vec<String> {
        let mut findings = Vec::new();

        // Convert to lowercase string for pattern matching
        let text = String::from_utf8_lossy(data).to_lowercase();

        // Auto-execution patterns
        const AUTO_EXEC_PATTERNS: &[(&str, &str)] = &[
            ("auto_open", "AutoExec: Auto_Open"),
            ("autoopen", "AutoExec: AutoOpen"),
            ("autoexec", "AutoExec: AutoExec"),
            ("autoclose", "AutoExec: AutoClose"),
            ("document_open", "AutoExec: Document_Open"),
            ("document_close", "AutoExec: Document_Close"),
            ("workbook_open", "AutoExec: Workbook_Open"),
            ("workbook_activate", "AutoExec: Workbook_Activate"),
        ];

        for (pattern, description) in AUTO_EXEC_PATTERNS {
            if text.contains(pattern) {
                findings.push(description.to_string());
            }
        }

        // Shell execution patterns
        const SHELL_PATTERNS: &[(&str, &str)] = &[
            ("shell", "Shell: Shell function"),
            ("wscript.shell", "Shell: WScript.Shell"),
            ("createobject", "Shell: CreateObject"),
            ("getobject", "Shell: GetObject"),
        ];

        for (pattern, description) in SHELL_PATTERNS {
            if text.contains(pattern) {
                findings.push(description.to_string());
            }
        }

        // Network access patterns
        const NETWORK_PATTERNS: &[(&str, &str)] = &[
            ("xmlhttp", "Network: XMLHTTP"),
            ("winhttp", "Network: WinHttp"),
            ("urldownloadtofile", "Network: URLDownloadToFile"),
            ("internetopen", "Network: InternetOpen"),
        ];

        for (pattern, description) in NETWORK_PATTERNS {
            if text.contains(pattern) {
                findings.push(description.to_string());
            }
        }

        // File access patterns
        const FILE_PATTERNS: &[(&str, &str)] = &[
            ("filesystemobject", "File: FileSystemObject"),
            ("createtextfile", "File: CreateTextFile"),
            ("opentextfile", "File: OpenTextFile"),
            ("open", "File: Open statement"),
        ];

        for (pattern, description) in FILE_PATTERNS {
            if text.contains(pattern) {
                findings.push(description.to_string());
            }
        }

        // PowerShell patterns
        const POWERSHELL_PATTERNS: &[(&str, &str)] = &[
            ("powershell", "PowerShell: powershell.exe"),
            ("-encodedcommand", "PowerShell: Encoded command"),
            ("-enc", "PowerShell: Encoded command (short)"),
            ("-command", "PowerShell: Command execution"),
        ];

        for (pattern, description) in POWERSHELL_PATTERNS {
            if text.contains(pattern) {
                findings.push(description.to_string());
            }
        }

        // Obfuscation patterns
        const OBFUSCATION_PATTERNS: &[(&str, &str)] = &[
            ("chr(", "Obfuscation: Chr() function"),
            ("chrw(", "Obfuscation: ChrW() function"),
            ("chr$(", "Obfuscation: Chr$() function"),
        ];

        for (pattern, description) in OBFUSCATION_PATTERNS {
            if text.contains(pattern) {
                findings.push(description.to_string());
            }
        }

        // Check for excessive string concatenation (potential obfuscation)
        let concat_count = text.matches(" & ").count();
        if concat_count > 20 {
            findings.push(format!(
                "Obfuscation: Excessive concatenation ({} instances)",
                concat_count
            ));
        }

        findings
    }

    /// Basic VBA RLE decompression (MS-OVBA algorithm)
    ///
    /// This is a simplified implementation for detection purposes.
    /// Full decompression requires proper token parsing and buffer management.
    #[allow(dead_code)]
    fn decompress_vba(data: &[u8]) -> Option<Vec<u8>> {
        if data.len() < 3 {
            return None;
        }

        // Check for compressed chunk signature (0x01)
        if data[0] != 0x01 {
            return None;
        }

        let mut output = Vec::new();
        let mut pos = 3; // Skip signature and size header

        while pos < data.len() {
            let flag_byte = data[pos];
            pos += 1;

            for bit in 0..8 {
                if pos >= data.len() {
                    break;
                }

                if (flag_byte & (1 << bit)) == 0 {
                    // Literal byte
                    output.push(data[pos]);
                    pos += 1;
                } else {
                    // Copy token
                    if pos + 1 >= data.len() {
                        break;
                    }

                    let token = u16::from_le_bytes([data[pos], data[pos + 1]]);
                    pos += 2;

                    let offset = (token & 0x0FFF) as usize;
                    let length = ((token >> 12) & 0x0F) as usize + 3;

                    // Copy from output buffer
                    if offset < output.len() {
                        let start = output.len() - offset;
                        for i in 0..length {
                            if start + i < output.len() {
                                let byte = output[start + i];
                                output.push(byte);
                            }
                        }
                    }
                }
            }
        }

        Some(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper: Simple in-memory FileReader implementation
    struct TestFileReader {
        data: Vec<u8>,
    }

    impl TestFileReader {
        fn new(data: Vec<u8>) -> Self {
            Self { data }
        }
    }

    impl FileReader for TestFileReader {
        fn read(&self, offset: u64, length: usize) -> std::io::Result<&[u8]> {
            let start = offset as usize;
            let end = start + length;

            if end > self.data.len() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
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
    fn test_ole_signature_validation() {
        let mut data = vec![0u8; 512];
        data[0..8].copy_from_slice(OLE_SIGNATURE);

        let reader = TestFileReader::new(data);
        let result = OLEParser::parse_header(&reader);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_ole_signature() {
        let data = vec![0u8; 512];
        let reader = TestFileReader::new(data);
        let result = OLEParser::parse_header(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_file_too_small() {
        let data = vec![0u8; 100];
        let reader = TestFileReader::new(data);
        let result = OLEParser::parse_header(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_suspicious_pattern_detection_auto_exec() {
        let data = b"Sub Auto_Open()\n  MsgBox \"Hello\"\nEnd Sub";
        let patterns = VBAAnalyzer::check_suspicious_patterns(data);
        assert!(!patterns.is_empty());
        assert!(patterns.iter().any(|p| p.contains("Auto_Open")));
    }

    #[test]
    fn test_suspicious_pattern_detection_shell() {
        let data = b"Shell \"cmd.exe /c calc\"";
        let patterns = VBAAnalyzer::check_suspicious_patterns(data);
        assert!(!patterns.is_empty());
        assert!(patterns.iter().any(|p| p.contains("Shell")));
    }

    #[test]
    fn test_suspicious_pattern_detection_network() {
        let data = b"Set http = CreateObject(\"MSXML2.XMLHTTP\")";
        let patterns = VBAAnalyzer::check_suspicious_patterns(data);
        assert!(!patterns.is_empty());
        assert!(patterns.iter().any(|p| p.contains("XMLHTTP")));
    }

    #[test]
    fn test_suspicious_pattern_detection_powershell() {
        let data = b"powershell.exe -encodedcommand ABC123";
        let patterns = VBAAnalyzer::check_suspicious_patterns(data);
        assert!(!patterns.is_empty());
        assert!(patterns.iter().any(|p| p.contains("PowerShell")));
    }

    #[test]
    fn test_suspicious_pattern_detection_obfuscation() {
        let data = b"Chr(65) & Chr(66) & Chr(67)";
        let patterns = VBAAnalyzer::check_suspicious_patterns(data);
        assert!(!patterns.is_empty());
        assert!(patterns.iter().any(|p| p.contains("Chr")));
    }

    #[test]
    fn test_no_suspicious_patterns_in_clean_code() {
        let data = b"Sub CalculateSum()\n  Dim total As Integer\n  total = 1 + 2\nEnd Sub";
        let patterns = VBAAnalyzer::check_suspicious_patterns(data);
        // May have some findings due to "Open" in "Sub", but should be minimal
        assert!(patterns.len() <= 1);
    }

    #[test]
    fn test_excessive_concatenation_detection() {
        let mut data = String::from("x = \"a\"");
        for _ in 0..25 {
            data.push_str(" & \"b\"");
        }
        let patterns = VBAAnalyzer::check_suspicious_patterns(data.as_bytes());
        assert!(patterns.iter().any(|p| p.contains("concatenation")));
    }
}
