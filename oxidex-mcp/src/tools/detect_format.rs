use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct DetectFormatParams {
    path: String,
}

/// Represents metadata group support information
#[derive(Debug)]
struct MetadataGroup {
    name: &'static str,
    readable: bool,
    writable: bool,
}

/// Represents format detection result for a single file
#[derive(Debug)]
struct FormatInfo {
    format_name: String,
    mime_type: String,
    metadata_groups: Vec<MetadataGroup>,
    supported_operations: Vec<&'static str>,
}

pub async fn handle(arguments: Value) -> Result<String> {
    let params: DetectFormatParams =
        serde_json::from_value(arguments).context("Invalid arguments for detect_format")?;

    // Validate path
    crate::utils::validate_path(&params.path)?;

    // Check if it's a glob pattern
    let is_glob = params.path.contains('*') || params.path.contains('?');

    if is_glob {
        handle_glob_pattern(&params.path).await
    } else {
        handle_single_file(&params.path).await
    }
}

async fn handle_single_file(path: &str) -> Result<String> {
    let path_buf = PathBuf::from(path);

    if !path_buf.exists() {
        return Ok(format!("File not found: {}", path));
    }

    // Detect format using OxiDex format detector
    match detect_file_format(&path_buf) {
        Ok(format_info) => {
            let filename = path_buf
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path);
            Ok(format_single_file(filename, &format_info))
        }
        Err(e) => Ok(crate::format::format_error(path, &e.to_string())),
    }
}

async fn handle_glob_pattern(pattern: &str) -> Result<String> {
    let files = crate::utils::expand_glob(pattern)?;

    if files.is_empty() {
        return Ok(format!(
            "No files matched pattern '{}' in current directory",
            pattern
        ));
    }

    // Process files
    let mut results = Vec::new();
    let mut failures = Vec::new();

    for path in files {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        match detect_file_format(&path) {
            Ok(format_info) => results.push((filename, format_info)),
            Err(e) => failures.push((filename, e.to_string())),
        }
    }

    if results.is_empty() && !failures.is_empty() {
        Ok(format!(
            "Could not detect format for any files:\n{}",
            failures
                .iter()
                .map(|(f, e)| format!("✗ {}: {}", f, e))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    } else {
        Ok(format_multiple_files(results, failures))
    }
}

/// Detect format of a file and return format information
fn detect_file_format(path: &PathBuf) -> Result<FormatInfo> {
    // Use OxiDex MMapReader and format detector
    let reader = oxidex::io::MMapReader::new(path)?;
    let format = oxidex::parsers::format_detector::detect_format(&reader)?;

    // Get format information
    let format_name = format.name().to_string();
    let mime_type = get_mime_type(&format);
    let metadata_groups = get_metadata_groups(&format);
    let supported_operations = get_supported_operations(&format);

    Ok(FormatInfo {
        format_name,
        mime_type,
        metadata_groups,
        supported_operations,
    })
}

/// Get MIME type for a file format
fn get_mime_type(format: &oxidex::core::FileFormat) -> String {
    use oxidex::core::FileFormat;

    match format {
        // Image formats
        FileFormat::JPEG => "image/jpeg",
        FileFormat::PNG => "image/png",
        FileFormat::GIF => "image/gif",
        FileFormat::BMP => "image/bmp",
        FileFormat::TIFF => "image/tiff",
        FileFormat::WebP => "image/webp",
        FileFormat::HEIF => "image/heif",
        FileFormat::AVIF => "image/avif",
        FileFormat::JXL => "image/jxl",
        FileFormat::BPG => "image/bpg",
        FileFormat::EXR => "image/x-exr",
        FileFormat::FLIF => "image/flif",
        FileFormat::SVG => "image/svg+xml",
        FileFormat::ICO => "image/x-icon",
        FileFormat::PSD => "image/vnd.adobe.photoshop",
        FileFormat::CasioCAM => "image/x-casio-cam",
        FileFormat::RAW | FileFormat::CameraRaw(_) => "image/x-raw",

        // Video formats
        FileFormat::QuickTime => "video/quicktime",
        FileFormat::MKV => "video/x-matroska",
        FileFormat::WEBM => "video/webm",
        FileFormat::FLV => "video/x-flv",
        FileFormat::AVI => "video/x-msvideo",
        FileFormat::MTS => "video/mp2t",

        // Audio formats
        FileFormat::MP3 => "audio/mpeg",
        FileFormat::FLAC => "audio/flac",
        FileFormat::AAC => "audio/aac",
        FileFormat::WAV => "audio/wav",
        FileFormat::OGG => "audio/ogg",
        FileFormat::OPUS => "audio/opus",
        FileFormat::APE => "audio/x-ape",

        // Document formats
        FileFormat::PDF => "application/pdf",
        FileFormat::DOCX => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        FileFormat::XLSX => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        FileFormat::PPTX => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        FileFormat::Pages => "application/x-iwork-pages-sffpages",
        FileFormat::Numbers => "application/x-iwork-numbers-sffnumbers",
        FileFormat::Keynote => "application/x-iwork-keynote-sffkey",
        FileFormat::EPUB => "application/epub+zip",

        // Archive formats
        FileFormat::ZIP => "application/zip",
        FileFormat::RAR => "application/vnd.rar",
        FileFormat::SevenZ => "application/x-7z-compressed",
        FileFormat::ISO => "application/x-iso9660-image",
        FileFormat::TAR => "application/x-tar",
        FileFormat::GZ => "application/gzip",

        // Font formats
        FileFormat::TTF => "font/ttf",
        FileFormat::OTF => "font/otf",
        FileFormat::WOFF => "font/woff",
        FileFormat::WOFF2 => "font/woff2",

        // Executable formats
        FileFormat::PE => "application/x-msdownload",
        FileFormat::ELF => "application/x-elf",
        FileFormat::MachO => "application/x-mach-binary",

        // Specialized formats
        FileFormat::DWG => "application/acad",
        FileFormat::DXF => "application/dxf",
        FileFormat::STL => "model/stl",
        FileFormat::OBJ => "model/obj",
        FileFormat::GLTF => "model/gltf+json",
        FileFormat::FITS => "application/fits",
        FileFormat::HDF5 => "application/x-hdf",
        FileFormat::VCF => "text/vcard",
        FileFormat::LNK => "application/x-ms-shortcut",

        FileFormat::Unknown => "application/octet-stream",
    }
    .to_string()
}

/// Get metadata groups supported by a file format
///
/// This function returns the metadata groups (EXIF, XMP, IPTC, etc.) that are
/// supported by the given file format, along with read/write capabilities.
fn get_metadata_groups(format: &oxidex::core::FileFormat) -> Vec<MetadataGroup> {
    use oxidex::core::FileFormat;

    match format {
        // JPEG supports EXIF, XMP, IPTC, JFIF, ICC Profile
        FileFormat::JPEG => vec![
            MetadataGroup {
                name: "EXIF",
                readable: true,
                writable: true,
            },
            MetadataGroup {
                name: "XMP",
                readable: true,
                writable: true,
            },
            MetadataGroup {
                name: "IPTC",
                readable: true,
                writable: true,
            },
            MetadataGroup {
                name: "JFIF",
                readable: true,
                writable: false,
            },
            MetadataGroup {
                name: "ICC Profile",
                readable: true,
                writable: false,
            },
        ],

        // TIFF supports EXIF, XMP, IPTC, ICC Profile
        FileFormat::TIFF => vec![
            MetadataGroup {
                name: "EXIF",
                readable: true,
                writable: true,
            },
            MetadataGroup {
                name: "XMP",
                readable: true,
                writable: true,
            },
            MetadataGroup {
                name: "IPTC",
                readable: true,
                writable: true,
            },
            MetadataGroup {
                name: "ICC Profile",
                readable: true,
                writable: false,
            },
        ],

        // PNG supports XMP, ICC Profile
        FileFormat::PNG => vec![
            MetadataGroup {
                name: "XMP",
                readable: true,
                writable: true,
            },
            MetadataGroup {
                name: "ICC Profile",
                readable: true,
                writable: false,
            },
            MetadataGroup {
                name: "PNG",
                readable: true,
                writable: false,
            },
        ],

        // PDF supports XMP
        FileFormat::PDF => vec![
            MetadataGroup {
                name: "XMP",
                readable: true,
                writable: false,
            },
            MetadataGroup {
                name: "PDF",
                readable: true,
                writable: false,
            },
        ],

        // QuickTime/MP4 supports QuickTime metadata
        FileFormat::QuickTime => vec![
            MetadataGroup {
                name: "QuickTime",
                readable: true,
                writable: false,
            },
            MetadataGroup {
                name: "XMP",
                readable: true,
                writable: false,
            },
        ],

        // HEIF supports EXIF, XMP
        FileFormat::HEIF => vec![
            MetadataGroup {
                name: "EXIF",
                readable: true,
                writable: false,
            },
            MetadataGroup {
                name: "XMP",
                readable: true,
                writable: false,
            },
        ],

        // Camera RAW formats support EXIF, XMP, MakerNotes
        FileFormat::RAW | FileFormat::CameraRaw(_) => vec![
            MetadataGroup {
                name: "EXIF",
                readable: true,
                writable: false,
            },
            MetadataGroup {
                name: "XMP",
                readable: true,
                writable: false,
            },
            MetadataGroup {
                name: "MakerNotes",
                readable: true,
                writable: false,
            },
        ],

        // Audio formats support ID3, Vorbis Comments, etc.
        FileFormat::MP3 => vec![MetadataGroup {
            name: "ID3",
            readable: true,
            writable: false,
        }],

        FileFormat::FLAC | FileFormat::OGG | FileFormat::OPUS => vec![MetadataGroup {
            name: "Vorbis Comments",
            readable: true,
            writable: false,
        }],

        FileFormat::WAV => vec![
            MetadataGroup {
                name: "RIFF",
                readable: true,
                writable: false,
            },
            MetadataGroup {
                name: "ID3",
                readable: true,
                writable: false,
            },
        ],

        // Video formats
        FileFormat::MKV | FileFormat::WEBM => vec![MetadataGroup {
            name: "Matroska",
            readable: true,
            writable: false,
        }],

        FileFormat::FLV => vec![MetadataGroup {
            name: "FLV",
            readable: true,
            writable: false,
        }],

        FileFormat::AVI => vec![MetadataGroup {
            name: "RIFF",
            readable: true,
            writable: false,
        }],

        FileFormat::MTS => vec![MetadataGroup {
            name: "MPEG-TS",
            readable: true,
            writable: false,
        }],

        // Document formats
        FileFormat::DOCX | FileFormat::XLSX | FileFormat::PPTX => vec![MetadataGroup {
            name: "Office Open XML",
            readable: true,
            writable: false,
        }],

        FileFormat::EPUB => vec![MetadataGroup {
            name: "EPUB",
            readable: true,
            writable: false,
        }],

        // Archive formats
        FileFormat::ZIP => vec![MetadataGroup {
            name: "ZIP",
            readable: true,
            writable: false,
        }],

        FileFormat::RAR => vec![MetadataGroup {
            name: "RAR",
            readable: true,
            writable: false,
        }],

        FileFormat::SevenZ => vec![MetadataGroup {
            name: "7z",
            readable: true,
            writable: false,
        }],

        FileFormat::TAR => vec![MetadataGroup {
            name: "TAR",
            readable: true,
            writable: false,
        }],

        FileFormat::ISO => vec![MetadataGroup {
            name: "ISO 9660",
            readable: true,
            writable: false,
        }],

        // Font formats
        FileFormat::TTF | FileFormat::OTF | FileFormat::WOFF | FileFormat::WOFF2 => vec![
            MetadataGroup {
                name: "Font",
                readable: true,
                writable: false,
            },
        ],

        // Executable formats
        FileFormat::PE => vec![MetadataGroup {
            name: "PE",
            readable: true,
            writable: false,
        }],

        FileFormat::ELF => vec![MetadataGroup {
            name: "ELF",
            readable: true,
            writable: false,
        }],

        FileFormat::MachO => vec![MetadataGroup {
            name: "Mach-O",
            readable: true,
            writable: false,
        }],

        // Text formats
        FileFormat::VCF => vec![MetadataGroup {
            name: "vCard",
            readable: true,
            writable: false,
        }],

        FileFormat::LNK => vec![MetadataGroup {
            name: "Windows Shortcut",
            readable: true,
            writable: false,
        }],

        // Specialized formats (minimal or no metadata)
        FileFormat::GIF
        | FileFormat::BMP
        | FileFormat::WebP
        | FileFormat::CasioCAM
        | FileFormat::AAC
        | FileFormat::APE
        | FileFormat::Pages
        | FileFormat::Numbers
        | FileFormat::Keynote
        | FileFormat::GZ
        | FileFormat::AVIF
        | FileFormat::JXL
        | FileFormat::BPG
        | FileFormat::EXR
        | FileFormat::FLIF
        | FileFormat::SVG
        | FileFormat::ICO
        | FileFormat::PSD
        | FileFormat::DWG
        | FileFormat::DXF
        | FileFormat::STL
        | FileFormat::OBJ
        | FileFormat::GLTF
        | FileFormat::FITS
        | FileFormat::HDF5 => vec![MetadataGroup {
            name: "File",
            readable: true,
            writable: false,
        }],

        FileFormat::Unknown => vec![],
    }
}

/// Get supported operations for a file format
///
/// Returns a list of operations that OxiDex can perform on this format.
fn get_supported_operations(format: &oxidex::core::FileFormat) -> Vec<&'static str> {
    use oxidex::core::FileFormat;

    // All supported formats can extract metadata
    let mut operations = vec!["Extract metadata"];

    // Check if format supports writing
    match format {
        FileFormat::JPEG | FileFormat::TIFF | FileFormat::PNG => {
            operations.push("Write metadata");
            operations.push("Copy metadata");
        }
        _ => {}
    }

    // All formats support searching
    operations.push("Search metadata");

    // All formats support analysis
    operations.push("Analyze metadata");

    operations
}

/// Format a single file's format information as human-readable text
fn format_single_file(filename: &str, info: &FormatInfo) -> String {
    let mut output = format!("{}:\n", filename);
    output.push_str(&format!("  Format: {}\n", info.format_name));
    output.push_str(&format!("  MIME Type: {}\n", info.mime_type));

    if !info.metadata_groups.is_empty() {
        output.push_str("\n  Supported Metadata:\n");
        for group in &info.metadata_groups {
            let access = if group.writable {
                "read/write"
            } else {
                "read only"
            };
            output.push_str(&format!("    ✓ {} ({})\n", group.name, access));
        }
    } else {
        output.push_str("\n  Supported Metadata:\n");
        output.push_str("    (No metadata support detected)\n");
    }

    if !info.supported_operations.is_empty() {
        output.push_str("\n  Supported Operations:\n");
        for operation in &info.supported_operations {
            output.push_str(&format!("    ✓ {}\n", operation));
        }
    }

    output
}

/// Format multiple files' format information
fn format_multiple_files(
    results: Vec<(String, FormatInfo)>,
    failures: Vec<(String, String)>,
) -> String {
    if results.is_empty() && failures.is_empty() {
        return "No files found.".to_string();
    }

    let mut output = format!("Detected formats for {} file(s):\n\n", results.len());

    for (filename, info) in results {
        output.push_str(&format_single_file(&filename, &info));
        output.push('\n');
    }

    if !failures.is_empty() {
        output.push_str("\nFailures:\n");
        for (filename, error) in failures {
            output.push_str(&format!("✗ {}: {}\n", filename, error));
        }
    }

    output.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_mime_type() {
        use oxidex::core::FileFormat;

        assert_eq!(get_mime_type(&FileFormat::JPEG), "image/jpeg");
        assert_eq!(get_mime_type(&FileFormat::PNG), "image/png");
        assert_eq!(get_mime_type(&FileFormat::PDF), "application/pdf");
        assert_eq!(get_mime_type(&FileFormat::MP3), "audio/mpeg");
        assert_eq!(get_mime_type(&FileFormat::FLAC), "audio/flac");
        assert_eq!(get_mime_type(&FileFormat::Unknown), "application/octet-stream");
    }

    #[test]
    fn test_get_metadata_groups_jpeg() {
        use oxidex::core::FileFormat;

        let groups = get_metadata_groups(&FileFormat::JPEG);
        assert!(!groups.is_empty());

        // Check that JPEG has EXIF support
        let has_exif = groups.iter().any(|g| g.name == "EXIF");
        assert!(has_exif, "JPEG should support EXIF metadata");

        // Check that JPEG has XMP support
        let has_xmp = groups.iter().any(|g| g.name == "XMP");
        assert!(has_xmp, "JPEG should support XMP metadata");

        // Check that JPEG has IPTC support
        let has_iptc = groups.iter().any(|g| g.name == "IPTC");
        assert!(has_iptc, "JPEG should support IPTC metadata");
    }

    #[test]
    fn test_get_metadata_groups_png() {
        use oxidex::core::FileFormat;

        let groups = get_metadata_groups(&FileFormat::PNG);
        assert!(!groups.is_empty());

        // Check that PNG has XMP support
        let has_xmp = groups.iter().any(|g| g.name == "XMP");
        assert!(has_xmp, "PNG should support XMP metadata");
    }

    #[test]
    fn test_get_metadata_groups_unknown() {
        use oxidex::core::FileFormat;

        let groups = get_metadata_groups(&FileFormat::Unknown);
        assert!(groups.is_empty(), "Unknown format should have no metadata groups");
    }

    #[test]
    fn test_get_supported_operations() {
        use oxidex::core::FileFormat;

        // JPEG should support all operations
        let jpeg_ops = get_supported_operations(&FileFormat::JPEG);
        assert!(jpeg_ops.contains(&"Extract metadata"));
        assert!(jpeg_ops.contains(&"Write metadata"));
        assert!(jpeg_ops.contains(&"Copy metadata"));
        assert!(jpeg_ops.contains(&"Search metadata"));
        assert!(jpeg_ops.contains(&"Analyze metadata"));

        // PNG should support write operations
        let png_ops = get_supported_operations(&FileFormat::PNG);
        assert!(png_ops.contains(&"Extract metadata"));
        assert!(png_ops.contains(&"Write metadata"));

        // MP3 should only support extract/search/analyze
        let mp3_ops = get_supported_operations(&FileFormat::MP3);
        assert!(mp3_ops.contains(&"Extract metadata"));
        assert!(mp3_ops.contains(&"Search metadata"));
        assert!(!mp3_ops.contains(&"Write metadata"));
    }

    #[test]
    fn test_format_single_file() {
        let info = FormatInfo {
            format_name: "JPEG".to_string(),
            mime_type: "image/jpeg".to_string(),
            metadata_groups: vec![
                MetadataGroup {
                    name: "EXIF",
                    readable: true,
                    writable: true,
                },
                MetadataGroup {
                    name: "XMP",
                    readable: true,
                    writable: false,
                },
            ],
            supported_operations: vec!["Extract metadata", "Write metadata"],
        };

        let output = format_single_file("test.jpg", &info);

        // Verify output contains expected content
        assert!(output.contains("test.jpg:"));
        assert!(output.contains("Format: JPEG"));
        assert!(output.contains("MIME Type: image/jpeg"));
        assert!(output.contains("EXIF (read/write)"));
        assert!(output.contains("XMP (read only)"));
        assert!(output.contains("Extract metadata"));
        assert!(output.contains("Write metadata"));
    }

    #[test]
    fn test_format_single_file_no_metadata() {
        let info = FormatInfo {
            format_name: "Unknown".to_string(),
            mime_type: "application/octet-stream".to_string(),
            metadata_groups: vec![],
            supported_operations: vec!["Extract metadata"],
        };

        let output = format_single_file("unknown.bin", &info);

        // Verify output contains expected content
        assert!(output.contains("unknown.bin:"));
        assert!(output.contains("Format: Unknown"));
        assert!(output.contains("(No metadata support detected)"));
    }

    #[test]
    fn test_format_multiple_files() {
        let results = vec![
            (
                "photo1.jpg".to_string(),
                FormatInfo {
                    format_name: "JPEG".to_string(),
                    mime_type: "image/jpeg".to_string(),
                    metadata_groups: vec![],
                    supported_operations: vec![],
                },
            ),
            (
                "document.pdf".to_string(),
                FormatInfo {
                    format_name: "PDF".to_string(),
                    mime_type: "application/pdf".to_string(),
                    metadata_groups: vec![],
                    supported_operations: vec![],
                },
            ),
        ];

        let failures = vec![("bad.file".to_string(), "Could not detect format".to_string())];

        let output = format_multiple_files(results, failures);

        // Verify output contains all files
        assert!(output.contains("Detected formats for 2 file(s)"));
        assert!(output.contains("photo1.jpg"));
        assert!(output.contains("document.pdf"));
        assert!(output.contains("Failures:"));
        assert!(output.contains("bad.file"));
    }

    #[tokio::test]
    async fn test_handle_file_not_found() {
        let result = handle_single_file("/nonexistent/path/to/file.jpg").await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("File not found"));
    }
}
