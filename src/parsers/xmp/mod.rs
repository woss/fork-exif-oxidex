//! XMP (Extensible Metadata Platform) parser
//!
//! Handles RDF/XML parsing and XMP namespace resolution.
//!
//! This module provides functionality to parse XMP metadata from RDF/XML
//! format. It supports:
//! - Namespace resolution for standard XMP namespaces (xmp, dc, exif, etc.)
//! - Extraction of simple string properties
//! - Edit history extraction for forensic tamper detection
//! - Document ID and version tracking metadata
//! - Graceful handling of malformed XML
//!
//! Complex XMP structures (bags, sequences, structs) are currently skipped,
//! except for xmpMM:History which is fully parsed for forensic analysis.
//!
//! # Example
//!
//! ```no_run
//! use oxidex::parsers::xmp::parse_xmp;
//!
//! let xml = br#"
//!     <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
//!              xmlns:xmp="http://ns.adobe.com/xap/1.0/">
//!       <rdf:Description>
//!         <xmp:Creator>John Doe</xmp:Creator>
//!       </rdf:Description>
//!     </rdf:RDF>
//! "#;
//!
//! let result = parse_xmp(xml).unwrap();
//! assert_eq!(result.len(), 1);
//! ```

pub mod history_parser;
pub mod namespace_mapping;
pub mod namespace_resolver;
pub mod rdf_parser;

use crate::core::{FileReader, MetadataMap, TagValue};
use crate::error::Result;

// Re-export main parsing function for convenience
pub use history_parser::{parse_xmp_history, XmpHistoryEntry};
pub use namespace_mapping::namespace_to_family;
pub use namespace_resolver::NamespaceResolver;
pub use rdf_parser::parse_xmp;

/// Parses a standalone XMP sidecar file.
///
/// This function reads an XMP sidecar file (.xmp) and extracts all metadata.
pub fn parse_xmp_file(reader: &dyn FileReader) -> Result<MetadataMap> {
    let mut metadata = MetadataMap::new();

    // Read the entire XMP file
    let size = reader.size() as usize;
    let xmp_data = reader.read(0, size)?;

    // Parse the XMP data
    let xmp_tags = parse_xmp(xmp_data)?;

    // Add all XMP tags to metadata
    for (key, value) in xmp_tags {
        metadata.insert(key, TagValue::new_string(value));
    }

    Ok(metadata)
}
