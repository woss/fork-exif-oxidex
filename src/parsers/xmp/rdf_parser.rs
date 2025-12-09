//! RDF/XML parsing for XMP
//!
//! This module handles parsing of RDF/XML data using quick-xml.
//! It extracts simple string properties from XMP metadata while
//! gracefully skipping complex structures (bags, sequences, structs).
//!
//! # XMP Structure
//!
//! Standard XMP has this structure:
//! ```xml
//! <x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="XMP Core 5.1.0">
//!   <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
//!     <rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/">
//!       <xmp:Creator>John Doe</xmp:Creator>
//!       <xmp:ModifyDate>2023-05-15</xmp:ModifyDate>
//!     </rdf:Description>
//!   </rdf:RDF>
//! </x:xmpmeta>
//! ```
//!
//! # Extracted Tags
//!
//! This parser extracts:
//! - **XMP:XMPToolkit** - from `x:xmptk` attribute on `x:xmpmeta` element
//! - **XMP:About** - from `rdf:about` attribute on `rdf:Description` element
//! - **Property elements** - like `<xmp:Creator>value</xmp:Creator>`
//! - **Property attributes** - XMP shorthand form on `rdf:Description`
//!
//! # Example
//!
//! ```no_run
//! use oxidex::parsers::xmp::rdf_parser::parse_xmp;
//!
//! let xml = br#"
//!     <x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Image::ExifTool 12.46">
//!       <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
//!         <rdf:Description xmlns:xmp="http://ns.adobe.com/xap/1.0/">
//!           <xmp:Creator>John Doe</xmp:Creator>
//!           <xmp:Rating>5</xmp:Rating>
//!         </rdf:Description>
//!       </rdf:RDF>
//!     </x:xmpmeta>
//! "#;
//!
//! let result = parse_xmp(xml).unwrap();
//! assert!(result.len() >= 3); // XMPToolkit + Creator + Rating
//! ```

use crate::error::{ExifToolError, Result};
use crate::parsers::xmp::namespace_resolver::NamespaceResolver;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

/// Parses XMP metadata from RDF/XML format.
///
/// This function extracts simple string properties from XMP metadata.
/// Complex structures (rdf:Bag, rdf:Seq, rdf:Alt, nested structs) are
/// currently skipped and not parsed.
///
/// # Parameters
///
/// - `xml_bytes`: Raw XML data containing XMP metadata
///
/// # Returns
///
/// Vector of (tag_name, value) pairs where tag_name includes namespace
/// prefix in the format "XMP:PropertyName" (e.g., "XMP:Creator", "XMP:Rights").
///
/// # Errors
///
/// Returns `ParseError` if XML is malformed or cannot be parsed.
///
/// # Example
///
/// ```no_run
/// use oxidex::parsers::xmp::rdf_parser::parse_xmp;
///
/// let xml = br#"
///     <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
///              xmlns:xmp="http://ns.adobe.com/xap/1.0/">
///       <rdf:Description>
///         <xmp:Creator>John Doe</xmp:Creator>
///       </rdf:Description>
///     </rdf:RDF>
/// "#;
///
/// let result = parse_xmp(xml).unwrap();
/// assert_eq!(result.len(), 1);
/// assert_eq!(result[0], ("XMP:Creator".to_string(), "John Doe".to_string()));
/// ```
pub fn parse_xmp(xml_bytes: &[u8]) -> Result<Vec<(String, String)>> {
    let mut reader = Reader::from_reader(xml_bytes);
    reader.config_mut().trim_text(true); // Trim whitespace from text nodes

    let mut resolver = NamespaceResolver::new();
    let mut results = Vec::new();
    let mut buf = Vec::new();

    // State tracking
    let mut inside_description = false;
    let mut current_property: Option<String> = None;
    let mut current_value = String::new();
    let mut depth = 0;
    let mut property_depth = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                depth += 1;

                let tag_name = extract_tag_name(&e)?;

                // Register any new namespaces from this element first
                register_namespaces_from_element(&e, &mut resolver)?;

                // Check for x:xmpmeta element and extract XMPToolkit
                if is_xmpmeta(&tag_name) {
                    extract_xmpmeta_attributes(&e, &mut results)?;
                }
                // Check if this is an rdf:Description element
                else if is_rdf_description(&tag_name, &resolver) {
                    inside_description = true;
                    // Extract rdf:about and property attributes from Description
                    extract_description_attributes(&e, &resolver, &mut results)?;
                } else if inside_description && current_property.is_none() {
                    // This is a property element inside rdf:Description
                    // Check if it's a complex structure we should skip
                    if is_simple_property(&tag_name, &resolver) {
                        current_property = Some(tag_name.to_string());
                        current_value.clear();
                        property_depth = depth;
                    }
                }
            }

            Ok(Event::End(e)) => {
                let tag_name = extract_tag_name_from_bytes(e.name().as_ref())?;

                if is_rdf_description(&tag_name, &resolver) {
                    inside_description = false;
                } else if let Some(ref prop) = current_property
                    && depth == property_depth
                {
                    // End of current property - extract tag name and value
                    if !current_value.trim().is_empty() {
                        let prefixed_name = format_tag_name(prop, &resolver);
                        results.push((prefixed_name, current_value.trim().to_string()));
                    }
                    current_property = None;
                    current_value.clear();
                }
                depth -= 1;
            }

            Ok(Event::Text(e)) => {
                // Collect text content if we're inside a property
                if current_property.is_some()
                    && let Ok(text) = e.xml_content()
                {
                    current_value.push_str(&text);
                }
            }

            Ok(Event::Empty(e)) => {
                let tag_name = extract_tag_name(&e)?;

                // Register namespaces from empty elements
                register_namespaces_from_element(&e, &mut resolver)?;

                // Handle self-closing x:xmpmeta
                if is_xmpmeta(&tag_name) {
                    extract_xmpmeta_attributes(&e, &mut results)?;
                }
                // Handle self-closing rdf:Description (shorthand form)
                else if is_rdf_description(&tag_name, &resolver) {
                    extract_description_attributes(&e, &resolver, &mut results)?;
                }
            }

            Ok(Event::Eof) => break,

            Ok(_) => {} // Ignore other events (comments, PI, etc.)

            Err(e) => {
                return Err(ExifToolError::parse_error(format!(
                    "Invalid XMP XML structure: {}",
                    e
                )));
            }
        }

        buf.clear();
    }

    Ok(results)
}

/// Extracts the tag name from a BytesStart event.
fn extract_tag_name(element: &BytesStart) -> Result<String> {
    let name = element.name();
    let name_str = std::str::from_utf8(name.as_ref())
        .map_err(|e| ExifToolError::parse_error(format!("Invalid UTF-8 in tag name: {}", e)))?;
    Ok(name_str.to_string())
}

/// Extracts the tag name from any element (helper for End events).
fn extract_tag_name_from_bytes(name_bytes: &[u8]) -> Result<String> {
    let name_str = std::str::from_utf8(name_bytes)
        .map_err(|e| ExifToolError::parse_error(format!("Invalid UTF-8 in tag name: {}", e)))?;
    Ok(name_str.to_string())
}

/// Checks if a tag name represents an x:xmpmeta element.
///
/// The xmpmeta element wraps XMP data and may contain the XMPToolkit attribute.
fn is_xmpmeta(tag_name: &str) -> bool {
    // Check for x:xmpmeta or xmpmeta (with or without prefix)
    tag_name == "x:xmpmeta" || tag_name == "xmpmeta"
}

/// Extracts XMPToolkit from x:xmpmeta element attributes.
///
/// The XMPToolkit value comes from the x:xmptk attribute on the x:xmpmeta element:
/// `<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Image::ExifTool 12.46">`
fn extract_xmpmeta_attributes(
    element: &BytesStart,
    results: &mut Vec<(String, String)>,
) -> Result<()> {
    for attr in element.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).map_err(|e| {
            ExifToolError::parse_error(format!("Invalid UTF-8 in attribute key: {}", e))
        })?;

        // Check for x:xmptk or xmptk attribute (XMP Toolkit version)
        if key == "x:xmptk" || key == "xmptk" {
            let value = std::str::from_utf8(&attr.value).map_err(|e| {
                ExifToolError::parse_error(format!("Invalid UTF-8 in XMPToolkit value: {}", e))
            })?;

            // Only add non-empty XMPToolkit values
            if !value.trim().is_empty() {
                results.push(("XMP:XMPToolkit".to_string(), value.trim().to_string()));
            }
        }
    }
    Ok(())
}

/// Extracts XMP properties from rdf:Description element attributes.
///
/// This handles two types of attributes:
/// 1. rdf:about - the subject URI, extracted as XMP:About
/// 2. Property shorthand - XMP properties written as attributes (e.g., xmp:Rating="5")
///
/// Example:
/// ```xml
/// <rdf:Description rdf:about="uuid:faf5bdd5-ba3d-11da-ad31-d33d75182f1b"
///                  xmp:CreateDate="2023-01-15T10:30:00"
///                  xmp:ModifyDate="2023-01-20T14:00:00">
/// ```
fn extract_description_attributes(
    element: &BytesStart,
    resolver: &NamespaceResolver,
    results: &mut Vec<(String, String)>,
) -> Result<()> {
    for attr in element.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).map_err(|e| {
            ExifToolError::parse_error(format!("Invalid UTF-8 in attribute key: {}", e))
        })?;

        let value = std::str::from_utf8(&attr.value).map_err(|e| {
            ExifToolError::parse_error(format!("Invalid UTF-8 in attribute value: {}", e))
        })?;

        // Skip empty values
        if value.trim().is_empty() {
            continue;
        }

        // Skip namespace declarations (xmlns:xxx)
        if key.starts_with("xmlns") {
            continue;
        }

        // Handle rdf:about attribute (the subject URI)
        if key == "rdf:about" {
            results.push(("XMP:About".to_string(), value.trim().to_string()));
            continue;
        }

        // Skip other rdf: attributes (rdf:parseType, rdf:resource, etc.)
        if key.starts_with("rdf:") {
            continue;
        }

        // Handle XMP property shorthand (properties as attributes)
        // These are namespace-prefixed attributes like xmp:Rating="5"
        if key.contains(':') {
            let prefixed_name = format_tag_name(key, resolver);
            results.push((prefixed_name, value.trim().to_string()));
        }
    }
    Ok(())
}

/// Checks if a tag name represents an rdf:Description element.
fn is_rdf_description(tag_name: &str, resolver: &NamespaceResolver) -> bool {
    if let Some(prefix) = NamespaceResolver::extract_prefix(tag_name) {
        let local_name = NamespaceResolver::extract_local_name(tag_name);
        if local_name == "Description"
            && let Some(uri) = resolver.resolve_prefix(prefix)
        {
            return uri == "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
        }
    }
    false
}

/// Checks if a property is a simple property (not a complex structure).
///
/// We skip complex RDF structures like:
/// - rdf:Bag, rdf:Seq, rdf:Alt (collections)
/// - Nested rdf:Description (structs)
fn is_simple_property(tag_name: &str, resolver: &NamespaceResolver) -> bool {
    if let Some(prefix) = NamespaceResolver::extract_prefix(tag_name) {
        let local_name = NamespaceResolver::extract_local_name(tag_name);

        // Check if it's an RDF namespace element
        if let Some(uri) = resolver.resolve_prefix(prefix)
            && uri == "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
        {
            // Skip RDF structural elements
            return !matches!(
                local_name,
                "Bag" | "Seq" | "Alt" | "Description" | "RDF" | "li"
            );
        }

        // It's a property in a non-RDF namespace (xmp, dc, exif, etc.)
        return true;
    }

    // No namespace prefix - treat as simple property
    true
}

/// Registers namespace declarations from an element's attributes.
fn register_namespaces_from_element(
    element: &BytesStart,
    resolver: &mut NamespaceResolver,
) -> Result<()> {
    for attr in element.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).map_err(|e| {
            ExifToolError::parse_error(format!("Invalid UTF-8 in attribute key: {}", e))
        })?;

        // Check for xmlns:prefix="uri" declarations
        if let Some(prefix) = key.strip_prefix("xmlns:") {
            let uri = std::str::from_utf8(&attr.value).map_err(|e| {
                ExifToolError::parse_error(format!("Invalid UTF-8 in namespace URI: {}", e))
            })?;

            resolver.register_namespace(prefix, uri);
        } else if key == "xmlns" {
            // Default namespace
            let uri = std::str::from_utf8(&attr.value).map_err(|e| {
                ExifToolError::parse_error(format!("Invalid UTF-8 in default namespace URI: {}", e))
            })?;
            resolver.register_namespace("", uri);
        }
    }
    Ok(())
}

/// Formats a tag name to match ExifTool's XMP output conventions.
///
/// ExifTool uses a simplified "XMP:" prefix for most common XMP properties,
/// regardless of their namespace. This function uses namespace URI resolution
/// to determine the correct family prefix.
///
/// XMP properties are returned with these prefixes:
/// - dc:title -> XMP:Title (Dublin Core uses simplified XMP: prefix and Title-case)
/// - dc:rights -> XMP:Rights (Dublin Core uses simplified XMP: prefix and Title-case)
/// - xmp:Creator -> XMP:Creator (Core XMP uses simplified XMP: prefix)
/// - exif:Make -> XMP-exif:Make (EXIF namespace uses XMP-exif: prefix)
/// - tiff:Model -> XMP-tiff:Model (TIFF namespace uses XMP-tiff: prefix)
fn format_tag_name(qname: &str, resolver: &NamespaceResolver) -> String {
    use super::namespace_mapping::namespace_to_family;

    let mut local_name = NamespaceResolver::extract_local_name(qname).to_string();

    // Extract namespace prefix from the qualified name
    if let Some(prefix) = NamespaceResolver::extract_prefix(qname) {
        // Resolve the namespace URI from the prefix
        let family_prefix = if let Some(namespace_uri) = resolver.resolve_prefix(prefix) {
            // Use namespace mapping to get ExifTool family prefix
            namespace_to_family(namespace_uri).unwrap_or("XMP")
        } else {
            // Unknown namespace - use generic XMP prefix
            "XMP"
        };

        // Dublin Core (dc) namespace uses Title-case for property names
        // Convert first letter to uppercase for dc: elements to match ExifTool
        if prefix == "dc" && !local_name.is_empty() {
            // Capitalize first letter
            local_name = capitalize_first_letter(&local_name);
        }

        // Format with the appropriate family prefix
        format!("{}:{}", family_prefix, local_name)
    } else {
        // No namespace prefix - use generic "XMP:" prefix
        format!("XMP:{}", local_name)
    }
}

/// Capitalizes the first letter of a string
fn capitalize_first_letter(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_xmp() {
        let xml = br#"
            <x:xmpmeta xmlns:x="adobe:ns:meta/">
              <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
                <rdf:Description xmlns:xmp="http://ns.adobe.com/xap/1.0/">
                  <xmp:Creator>John Doe</xmp:Creator>
                  <xmp:Rating>5</xmp:Rating>
                </rdf:Description>
              </rdf:RDF>
            </x:xmpmeta>
        "#;

        let result = parse_xmp(xml).unwrap();
        assert!(
            result.len() >= 2,
            "Expected at least 2 properties, got {}",
            result.len()
        );

        // Check that Creator and Rating are present with simplified XMP: prefix
        let creators: Vec<_> = result
            .iter()
            .filter(|(name, _)| name == "XMP:Creator")
            .collect();
        assert_eq!(creators.len(), 1);
        assert_eq!(creators[0].1, "John Doe");

        let ratings: Vec<_> = result
            .iter()
            .filter(|(name, _)| name == "XMP:Rating")
            .collect();
        assert_eq!(ratings.len(), 1);
        assert_eq!(ratings[0].1, "5");
    }

    #[test]
    fn test_parse_multiple_namespaces() {
        let xml = br#"
            <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                     xmlns:xmp="http://ns.adobe.com/xap/1.0/"
                     xmlns:dc="http://purl.org/dc/elements/1.1/"
                     xmlns:exif="http://ns.adobe.com/exif/1.0/">
              <rdf:Description>
                <xmp:Creator>Jane Smith</xmp:Creator>
                <dc:title>My Photo</dc:title>
                <dc:rights>Copyright 2024</dc:rights>
                <exif:Make>Canon</exif:Make>
                <exif:Model>EOS R5</exif:Model>
                <xmp:ModifyDate>2024-01-15</xmp:ModifyDate>
              </rdf:Description>
            </rdf:RDF>
        "#;

        let result = parse_xmp(xml).unwrap();
        assert!(
            result.len() >= 5,
            "Expected at least 5 properties, got {}",
            result.len()
        );

        // Verify properties from all 3 namespaces (xmp, dc, exif)
        let prop_names: Vec<String> = result.iter().map(|(name, _)| name.clone()).collect();

        // Check for xmp properties with simplified XMP: prefix
        assert!(
            prop_names.iter().any(|n| n == "XMP:Creator"),
            "Missing XMP:Creator"
        );
        assert!(
            prop_names.iter().any(|n| n == "XMP:ModifyDate"),
            "Missing XMP:ModifyDate"
        );

        // Check for dc properties (Dublin Core uses simplified XMP: prefix and Title-case)
        assert!(
            prop_names.iter().any(|n| n == "XMP:Title"),
            "Missing XMP:Title (dc:title)"
        );
        assert!(
            prop_names.iter().any(|n| n == "XMP:Rights"),
            "Missing XMP:Rights (dc:rights)"
        );

        // Check for exif properties (EXIF namespace uses XMP-exif: prefix)
        assert!(
            prop_names.iter().any(|n| n == "XMP-exif:Make"),
            "Missing XMP-exif:Make (exif:Make)"
        );
        assert!(
            prop_names.iter().any(|n| n == "XMP-exif:Model"),
            "Missing XMP-exif:Model (exif:Model)"
        );
    }

    #[test]
    fn test_malformed_xml_returns_error() {
        // quick-xml is lenient with structure, but will fail on invalid UTF-8 in tag names
        // Create XML with invalid UTF-8 sequence in a tag name
        let mut xml = Vec::new();
        xml.extend_from_slice(b"<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description><");
        xml.push(0xFF); // Invalid UTF-8 start byte
        xml.push(0xFE); // Invalid UTF-8 continuation
        xml.extend_from_slice(b":test>value</test></rdf:Description></rdf:RDF>");

        let result = parse_xmp(&xml);

        // Should error due to invalid UTF-8 in tag name
        assert!(
            result.is_err(),
            "Expected error for malformed XML with invalid UTF-8"
        );

        // Verify we got a ParseError
        match result {
            Err(ExifToolError::ParseError { .. }) => {
                // Success - got the expected error type
            }
            Ok(_) => panic!("Expected error for malformed XML, got Ok"),
            Err(e) => panic!("Expected ParseError, got {:?}", e),
        }
    }

    #[test]
    fn test_empty_xml_returns_empty_vector() {
        let xml = br#"
            <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
              <rdf:Description />
            </rdf:RDF>
        "#;

        let result = parse_xmp(xml).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_skip_complex_structures() {
        let xml = br#"
            <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                     xmlns:dc="http://purl.org/dc/elements/1.1/">
              <rdf:Description>
                <dc:creator>Simple Creator</dc:creator>
                <dc:subject>
                  <rdf:Bag>
                    <rdf:li>keyword1</rdf:li>
                    <rdf:li>keyword2</rdf:li>
                  </rdf:Bag>
                </dc:subject>
                <dc:title>Simple Title</dc:title>
              </rdf:Description>
            </rdf:RDF>
        "#;

        let result = parse_xmp(xml).unwrap();

        // Should have simple properties but not the complex Bag structure
        let prop_names: Vec<String> = result.iter().map(|(name, _)| name.clone()).collect();
        assert!(prop_names.iter().any(|n| n == "XMP:Creator"));
        assert!(prop_names.iter().any(|n| n == "XMP:Title"));

        // The Bag contents should not be present as individual items
        assert!(!prop_names.iter().any(|n| n.contains("keyword")));
    }

    #[test]
    fn test_whitespace_trimming() {
        let xml = br#"
            <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                     xmlns:xmp="http://ns.adobe.com/xap/1.0/">
              <rdf:Description>
                <xmp:Creator>
                  John Doe
                </xmp:Creator>
              </rdf:Description>
            </rdf:RDF>
        "#;

        let result = parse_xmp(xml).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0],
            ("XMP:Creator".to_string(), "John Doe".to_string())
        );
    }

    #[test]
    fn test_utf8_content() {
        // Use a regular string literal and convert to bytes to support UTF-8
        let xml = r#"
            <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                     xmlns:dc="http://purl.org/dc/elements/1.1/">
              <rdf:Description>
                <dc:creator>Jose Garcia</dc:creator>
                <dc:title>Nandu en la Patagonia</dc:title>
                <dc:rights>Copyright 2024</dc:rights>
              </rdf:Description>
            </rdf:RDF>
        "#;

        let result = parse_xmp(xml.as_bytes()).unwrap();
        assert_eq!(result.len(), 3);

        // Verify content is preserved
        assert!(result.iter().any(|(_, v)| v.contains("Jose Garcia")));
        assert!(result.iter().any(|(_, v)| v.contains("Nandu")));
        assert!(result.iter().any(|(_, v)| v.contains("Copyright")));
    }

    #[test]
    fn test_multiple_descriptions() {
        let xml = br#"
            <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                     xmlns:xmp="http://ns.adobe.com/xap/1.0/"
                     xmlns:dc="http://purl.org/dc/elements/1.1/">
              <rdf:Description>
                <xmp:Creator>First Creator</xmp:Creator>
              </rdf:Description>
              <rdf:Description>
                <dc:title>First Title</dc:title>
              </rdf:Description>
            </rdf:RDF>
        "#;

        let result = parse_xmp(xml).unwrap();
        assert_eq!(result.len(), 2);

        // Should handle properties from both Description blocks
        let creators: Vec<_> = result
            .iter()
            .filter(|(name, _)| name == "XMP:Creator")
            .collect();
        assert_eq!(creators.len(), 1);

        let titles: Vec<_> = result
            .iter()
            .filter(|(name, _)| name == "XMP:Title")
            .collect();
        assert_eq!(titles.len(), 1);
    }

    #[test]
    fn test_xmp_toolkit_extraction() {
        // Test extraction of XMPToolkit from x:xmpmeta element
        let xml = br#"
            <x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Image::ExifTool 12.46">
              <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
                <rdf:Description xmlns:xmp="http://ns.adobe.com/xap/1.0/">
                  <xmp:Creator>John Doe</xmp:Creator>
                </rdf:Description>
              </rdf:RDF>
            </x:xmpmeta>
        "#;

        let result = parse_xmp(xml).unwrap();

        // Should have XMPToolkit and Creator
        let prop_names: Vec<String> = result.iter().map(|(name, _)| name.clone()).collect();
        assert!(
            prop_names.iter().any(|n| n == "XMP:XMPToolkit"),
            "Missing XMP:XMPToolkit. Found: {:?}",
            prop_names
        );
        assert!(
            prop_names.iter().any(|n| n == "XMP:Creator"),
            "Missing XMP:Creator"
        );

        // Verify XMPToolkit value
        let toolkit = result
            .iter()
            .find(|(name, _)| name == "XMP:XMPToolkit")
            .map(|(_, v)| v.as_str());
        assert_eq!(toolkit, Some("Image::ExifTool 12.46"));
    }

    #[test]
    fn test_rdf_about_extraction() {
        // Test extraction of rdf:about attribute from rdf:Description
        let xml = br#"
            <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
              <rdf:Description rdf:about="uuid:faf5bdd5-ba3d-11da-ad31-d33d75182f1b"
                               xmlns:xmp="http://ns.adobe.com/xap/1.0/">
                <xmp:Creator>John Doe</xmp:Creator>
              </rdf:Description>
            </rdf:RDF>
        "#;

        let result = parse_xmp(xml).unwrap();

        // Should have About and Creator
        let prop_names: Vec<String> = result.iter().map(|(name, _)| name.clone()).collect();
        assert!(
            prop_names.iter().any(|n| n == "XMP:About"),
            "Missing XMP:About. Found: {:?}",
            prop_names
        );
        assert!(
            prop_names.iter().any(|n| n == "XMP:Creator"),
            "Missing XMP:Creator"
        );

        // Verify About value
        let about = result
            .iter()
            .find(|(name, _)| name == "XMP:About")
            .map(|(_, v)| v.as_str());
        assert_eq!(about, Some("uuid:faf5bdd5-ba3d-11da-ad31-d33d75182f1b"));
    }

    #[test]
    fn test_shorthand_attributes() {
        // Test extraction of XMP properties from rdf:Description attributes (shorthand form)
        let xml = br#"
            <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                     xmlns:xmp="http://ns.adobe.com/xap/1.0/"
                     xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/">
              <rdf:Description rdf:about=""
                               xmp:CreateDate="2023-01-15T10:30:00"
                               xmp:ModifyDate="2023-01-20T14:00:00"
                               photoshop:DateCreated="2023-01-15">
              </rdf:Description>
            </rdf:RDF>
        "#;

        let result = parse_xmp(xml).unwrap();

        // Should have shorthand properties extracted
        let prop_names: Vec<String> = result.iter().map(|(name, _)| name.clone()).collect();
        assert!(
            prop_names.iter().any(|n| n == "XMP:CreateDate"),
            "Missing XMP:CreateDate. Found: {:?}",
            prop_names
        );
        assert!(
            prop_names.iter().any(|n| n == "XMP:ModifyDate"),
            "Missing XMP:ModifyDate"
        );
        assert!(
            prop_names.iter().any(|n| n == "XMP-photoshop:DateCreated"),
            "Missing XMP-photoshop:DateCreated. Found: {:?}",
            prop_names
        );

        // Verify values
        let create_date = result
            .iter()
            .find(|(name, _)| name == "XMP:CreateDate")
            .map(|(_, v)| v.as_str());
        assert_eq!(create_date, Some("2023-01-15T10:30:00"));
    }

    #[test]
    fn test_self_closing_description_with_attributes() {
        // Test self-closing rdf:Description with shorthand properties
        let xml = br#"
            <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                     xmlns:xmp="http://ns.adobe.com/xap/1.0/">
              <rdf:Description rdf:about="test.jpg"
                               xmp:Rating="5"
                               xmp:Label="Yellow" />
            </rdf:RDF>
        "#;

        let result = parse_xmp(xml).unwrap();

        let prop_names: Vec<String> = result.iter().map(|(name, _)| name.clone()).collect();
        assert!(
            prop_names.iter().any(|n| n == "XMP:About"),
            "Missing XMP:About. Found: {:?}",
            prop_names
        );
        assert!(
            prop_names.iter().any(|n| n == "XMP:Rating"),
            "Missing XMP:Rating"
        );
        assert!(
            prop_names.iter().any(|n| n == "XMP:Label"),
            "Missing XMP:Label"
        );

        let rating = result
            .iter()
            .find(|(name, _)| name == "XMP:Rating")
            .map(|(_, v)| v.as_str());
        assert_eq!(rating, Some("5"));
    }

    #[test]
    fn test_full_xmp_packet_structure() {
        // Test a complete XMP packet with all features
        let xml = br#"
            <x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Adobe XMP Core 5.6-c140">
              <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
                <rdf:Description rdf:about=""
                                 xmlns:xmp="http://ns.adobe.com/xap/1.0/"
                                 xmlns:dc="http://purl.org/dc/elements/1.1/"
                                 xmp:CreateDate="2023-01-15T10:30:00+05:30"
                                 xmp:ModifyDate="2023-01-20T14:00:00Z">
                  <dc:creator>John Doe</dc:creator>
                  <dc:title>My Photo</dc:title>
                </rdf:Description>
              </rdf:RDF>
            </x:xmpmeta>
        "#;

        let result = parse_xmp(xml).unwrap();

        // Verify all expected tags are present
        let prop_names: Vec<String> = result.iter().map(|(name, _)| name.clone()).collect();

        // XMPToolkit from x:xmpmeta
        assert!(
            prop_names.iter().any(|n| n == "XMP:XMPToolkit"),
            "Missing XMP:XMPToolkit. Found: {:?}",
            prop_names
        );

        // Shorthand attributes from rdf:Description
        assert!(
            prop_names.iter().any(|n| n == "XMP:CreateDate"),
            "Missing XMP:CreateDate"
        );
        assert!(
            prop_names.iter().any(|n| n == "XMP:ModifyDate"),
            "Missing XMP:ModifyDate"
        );

        // Child element properties
        assert!(
            prop_names.iter().any(|n| n == "XMP:Creator"),
            "Missing XMP:Creator (dc:creator)"
        );
        assert!(
            prop_names.iter().any(|n| n == "XMP:Title"),
            "Missing XMP:Title (dc:title)"
        );

        // Verify XMPToolkit value
        let toolkit = result
            .iter()
            .find(|(name, _)| name == "XMP:XMPToolkit")
            .map(|(_, v)| v.as_str());
        assert_eq!(toolkit, Some("Adobe XMP Core 5.6-c140"));
    }

    #[test]
    fn test_empty_rdf_about_is_skipped() {
        // Test that empty rdf:about values are not included
        let xml = br#"
            <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
              <rdf:Description rdf:about=""
                               xmlns:xmp="http://ns.adobe.com/xap/1.0/">
                <xmp:Creator>John Doe</xmp:Creator>
              </rdf:Description>
            </rdf:RDF>
        "#;

        let result = parse_xmp(xml).unwrap();

        // Should only have Creator, not an empty About
        let prop_names: Vec<String> = result.iter().map(|(name, _)| name.clone()).collect();
        assert!(
            !prop_names.iter().any(|n| n == "XMP:About"),
            "Should not include empty XMP:About. Found: {:?}",
            prop_names
        );
        assert!(
            prop_names.iter().any(|n| n == "XMP:Creator"),
            "Missing XMP:Creator"
        );
    }
}
