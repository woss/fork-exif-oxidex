//! SVG (Scalable Vector Graphics) parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use crate::parsers::xmp::parse_xmp;

/// Maximum bytes to read from SVG file for parsing (SVG headers are at the start)
const MAX_READ_SIZE: usize = 65536; // 64KB

/// Parser for SVG (Scalable Vector Graphics) files
///
/// Extracts metadata from SVG XML-based vector graphics files including dimensions,
/// title, description, and other attributes.
pub struct SVGParser;

impl SVGParser {
    /// Verifies the SVG file by checking for the presence of "<svg" tag in the header
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        let read_size = (reader.size() as usize).min(1000);
        if read_size < 4 {
            return Ok(false);
        }
        let header = reader.read(0, read_size)?;
        let text = std::str::from_utf8(header).unwrap_or("");
        Ok(text.contains("<svg"))
    }

    /// Extracts an attribute value from an XML tag
    /// Handles both single and double quotes: width="100" or width='100'
    fn extract_attribute(text: &str, attr_name: &str) -> Option<String> {
        let patterns = [
            format!("{}=\"", attr_name),
            format!("{}='", attr_name),
            format!("{}=\"", attr_name),
        ];

        for pattern in &patterns {
            if let Some(start) = text.find(pattern) {
                let value_start = start + pattern.len();
                let quote = pattern.chars().last()?;
                if let Some(end) = text[value_start..].find(quote) {
                    return Some(text[value_start..value_start + end].to_string());
                }
            }
        }
        None
    }

    /// Extracts text content from an XML element
    /// Example: <title>My SVG</title> returns "My SVG"
    fn extract_element_content(text: &str, element: &str) -> Option<String> {
        let open_tag = format!("<{}>", element);
        let close_tag = format!("</{}>", element);

        if let Some(start) = text.find(&open_tag) {
            let content_start = start + open_tag.len();
            if let Some(end) = text[content_start..].find(&close_tag) {
                let content = text[content_start..content_start + end].trim();
                return if !content.is_empty() {
                    Some(content.to_string())
                } else {
                    None
                };
            }
        }
        None
    }

    /// Parses dimension value, preserving units like "px", "em", "in", "%"
    /// ExifTool keeps units intact, so we should too
    fn parse_dimension(value: &str) -> Option<String> {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            Some(trimmed.to_string())
        } else {
            None
        }
    }

    /// Parses viewBox attribute: "minX minY width height"
    fn parse_viewbox(viewbox: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = viewbox.split_whitespace().collect();
        if parts.len() == 4 {
            Some((parts[2].to_string(), parts[3].to_string()))
        } else {
            None
        }
    }

    /// Checks if SVG contains animation elements
    fn is_animated(text: &str) -> bool {
        text.contains("<animate") || text.contains("<animateTransform")
    }

    /// Counts SVG elements (shape elements, text, etc.)
    /// Counts common SVG elements: rect, circle, ellipse, line, polyline, polygon, path, text, image, use, g
    fn count_svg_elements(text: &str) -> i64 {
        let mut count = 0i64;
        let elements = [
            "<rect",
            "<circle",
            "<ellipse",
            "<line",
            "<polyline",
            "<polygon",
            "<path",
            "<text",
            "<image",
            "<use",
            "<g ",
        ];

        for element in &elements {
            // Count occurrences of each element tag
            let mut start = 0;
            while let Some(pos) = text[start..].find(element) {
                count += 1;
                start = start + pos + element.len();
            }
        }

        count
    }

    /// Extracts dc:creator content, handling RDF bags/sequences
    /// Handles formats like:
    /// - Simple: <dc:creator>Name</dc:creator>
    /// - RDF Bag: <dc:creator><rdf:Bag><rdf:li>Name1</rdf:li><rdf:li>Name2</rdf:li></rdf:Bag></dc:creator>
    /// - RDF Seq: <dc:creator><rdf:Seq><rdf:li>Name</rdf:li></rdf:Seq></dc:creator>
    fn extract_dc_creator(text: &str) -> Option<String> {
        // First try to find dc:creator element
        let start_tag = "<dc:creator>";
        let end_tag = "</dc:creator>";

        let start = text.find(start_tag)?;
        let content_start = start + start_tag.len();
        let end = text[content_start..].find(end_tag)? + content_start;
        let content = &text[content_start..end];

        // Try to extract rdf:li elements (handles both Bag and Seq)
        let li_values: Vec<String> = Self::extract_all_rdf_li(content);

        if !li_values.is_empty() {
            // ExifTool formats multiple creators as ["name1","name2"]
            if li_values.len() == 1 {
                Some(li_values[0].clone())
            } else {
                Some(format!(
                    "[{}]",
                    li_values
                        .iter()
                        .map(|s| format!("\"{}\"", s))
                        .collect::<Vec<_>>()
                        .join(",")
                ))
            }
        } else {
            // Simple content without RDF structure
            let trimmed = content.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('<') {
                Some(trimmed.to_string())
            } else {
                None
            }
        }
    }

    /// Extract all rdf:li values from content
    fn extract_all_rdf_li(content: &str) -> Vec<String> {
        let mut values = Vec::new();
        let mut pos = 0;

        while let Some(start) = content[pos..].find("<rdf:li>") {
            let value_start = pos + start + 8; // len of "<rdf:li>"
            if let Some(end) = content[value_start..].find("</rdf:li>") {
                let value = content[value_start..value_start + end].trim();
                if !value.is_empty() {
                    values.push(value.to_string());
                }
                pos = value_start + end + 10; // len of "</rdf:li>"
            } else {
                break;
            }
        }

        values
    }

    /// Extract embedded XMP metadata from SVG
    /// SVG can contain XMP in <x:xmpmeta> or <rdf:RDF> elements
    fn extract_xmp(text: &str, metadata: &mut MetadataMap) {
        // Look for x:xmpmeta element
        if let Some(start) = text.find("<x:xmpmeta") {
            if let Some(end) = text[start..].find("</x:xmpmeta>") {
                let xmp_data = &text[start..start + end + 12];
                if let Ok(xmp_tuples) = parse_xmp(xmp_data.as_bytes()) {
                    for (key, value) in xmp_tuples {
                        metadata.insert(key, TagValue::new_string(value));
                    }
                }
            }
        }
        // Also look for standalone rdf:RDF inside metadata element
        else if let Some(meta_start) = text.find("<metadata") {
            if let Some(meta_end) = text[meta_start..].find("</metadata>") {
                let meta_content = &text[meta_start..meta_start + meta_end + 11];
                if let Some(rdf_start) = meta_content.find("<rdf:RDF") {
                    if let Some(rdf_end) = meta_content[rdf_start..].find("</rdf:RDF>") {
                        let rdf_data = &meta_content[rdf_start..rdf_start + rdf_end + 10];
                        // Wrap in xmpmeta for parser
                        let wrapped = format!("<x:xmpmeta>{}</x:xmpmeta>", rdf_data);
                        if let Ok(xmp_tuples) = parse_xmp(wrapped.as_bytes()) {
                            for (key, value) in xmp_tuples {
                                metadata.insert(key, TagValue::new_string(value));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Extract Dublin Core elements that map to XMP tags
    fn extract_dublin_core(text: &str, metadata: &mut MetadataMap) {
        // dc:date -> XMP:Date
        if let Some(dc_date) = Self::extract_element_content(text, "dc:date") {
            metadata.insert("XMP:Date".to_string(), TagValue::new_string(dc_date));
        }

        // dc:format -> XMP:Format
        if let Some(dc_format) = Self::extract_element_content(text, "dc:format") {
            metadata.insert("XMP:Format".to_string(), TagValue::new_string(dc_format));
        }

        // dc:language -> XMP:Language
        if let Some(dc_lang) = Self::extract_element_content(text, "dc:language") {
            metadata.insert("XMP:Language".to_string(), TagValue::new_string(dc_lang));
        }

        // dc:publisher -> XMP:Publisher
        if let Some(dc_pub) = Self::extract_element_content(text, "dc:publisher") {
            metadata.insert("XMP:Publisher".to_string(), TagValue::new_string(dc_pub));
        }

        // rdf:about -> XMP:About
        if let Some(about) = Self::extract_attribute(text, "rdf:about") {
            metadata.insert("XMP:About".to_string(), TagValue::new_string(about));
        }
    }

    /// Extract SVG-specific description metadata
    fn extract_svg_desc_metadata(text: &str, metadata: &mut MetadataMap) {
        // Look for desc elements with specific structure
        // <desc role="xxxTitle">content</desc>
        let mut pos = 0;
        while let Some(desc_start) = text[pos..].find("<desc") {
            let desc_abs_start = pos + desc_start;

            // Find end of opening tag
            if let Some(tag_end) = text[desc_abs_start..].find('>') {
                let tag_content = &text[desc_abs_start..desc_abs_start + tag_end + 1];

                // Look for closing tag
                if let Some(close) = text[desc_abs_start + tag_end..].find("</desc>") {
                    let content =
                        &text[desc_abs_start + tag_end + 1..desc_abs_start + tag_end + close];

                    // Extract role attribute
                    if let Some(role) = Self::extract_attribute(tag_content, "role") {
                        let tag_name = format!("SVG:Desc{}", capitalize_first(&role));
                        metadata.insert(tag_name, TagValue::new_string(content.trim().to_string()));
                    }

                    pos = desc_abs_start + tag_end + close + 7;
                } else {
                    pos = desc_abs_start + 1;
                }
            } else {
                break;
            }
        }
    }
}

/// Capitalize first letter of string
fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

impl FormatParser for SVGParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid SVG signature"));
        }

        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("SVG".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        // Read up to 64KB for parsing (SVG metadata is in the header)
        let read_size = std::cmp::min(reader.size() as usize, MAX_READ_SIZE);
        let content = reader.read(0, read_size)?;
        let text = std::str::from_utf8(content).unwrap_or("");

        // Extract <svg> tag (find first occurrence)
        if let Some(svg_start) = text.find("<svg") {
            let svg_end = text[svg_start..]
                .find('>')
                .map(|pos| svg_start + pos)
                .unwrap_or(text.len());
            let svg_tag = &text[svg_start..svg_end];

            // Extract width and height
            if let Some(width) = Self::extract_attribute(svg_tag, "width")
                && let Some(parsed) = Self::parse_dimension(&width)
            {
                metadata.insert("ImageWidth".to_string(), TagValue::String(parsed.clone()));
                // Also add SVG:Width for Worker 26 compatibility
                metadata.insert("SVG:Width".to_string(), TagValue::new_string(parsed));
            }

            if let Some(height) = Self::extract_attribute(svg_tag, "height")
                && let Some(parsed) = Self::parse_dimension(&height)
            {
                metadata.insert("ImageHeight".to_string(), TagValue::String(parsed.clone()));
                // Also add SVG:Height for Worker 26 compatibility
                metadata.insert("SVG:Height".to_string(), TagValue::new_string(parsed));
            }

            // Extract viewBox for dimensions if width/height not present
            if let Some(viewbox) = Self::extract_attribute(svg_tag, "viewBox") {
                metadata.insert(
                    "SVG:ViewBox".to_string(),
                    TagValue::new_string(viewbox.clone()),
                );

                // If no width/height, try to extract from viewBox
                if !metadata.contains_key("ImageWidth")
                    && let Some((vb_width, vb_height)) = Self::parse_viewbox(&viewbox)
                {
                    metadata.insert("ImageWidth".to_string(), TagValue::String(vb_width));
                    metadata.insert("ImageHeight".to_string(), TagValue::String(vb_height));
                }
            }

            // Extract xmlns (namespace) - ExifTool calls this "Xmlns"
            if let Some(xmlns) = Self::extract_attribute(svg_tag, "xmlns") {
                metadata.insert("SVG:Xmlns".to_string(), TagValue::String(xmlns));
            }

            // Extract version - ExifTool calls this "SVGVersion" or "Version"
            if let Some(version) = Self::extract_attribute(svg_tag, "version") {
                metadata.insert(
                    "SVG:SVGVersion".to_string(),
                    TagValue::String(version.clone()),
                );
                // Also add SVG:Version for Worker 26 compatibility
                metadata.insert("SVG:Version".to_string(), TagValue::new_string(version));
            }

            // Extract preserveAspectRatio
            if let Some(preserve) = Self::extract_attribute(svg_tag, "preserveAspectRatio") {
                metadata.insert(
                    "SVG:PreserveAspectRatio".to_string(),
                    TagValue::new_string(preserve),
                );
            }
        }

        // Extract title
        if let Some(title) = Self::extract_element_content(text, "title") {
            metadata.insert("Title".to_string(), TagValue::String(title));
        }

        // Extract description
        if let Some(desc) = Self::extract_element_content(text, "desc") {
            metadata.insert("Description".to_string(), TagValue::String(desc));
        }

        // Extract embedded XMP metadata first
        Self::extract_xmp(text, &mut metadata);

        // Extract Dublin Core metadata if present
        if text.contains("dc:") {
            if let Some(dc_title) = Self::extract_element_content(text, "dc:title") {
                metadata.insert("XMP:Title".to_string(), TagValue::String(dc_title));
            }
            if let Some(dc_creator) = Self::extract_dc_creator(text) {
                metadata.insert("XMP:Creator".to_string(), TagValue::String(dc_creator));
            }
            if let Some(dc_desc) = Self::extract_element_content(text, "dc:description") {
                metadata.insert("XMP:Description".to_string(), TagValue::String(dc_desc));
            }

            // Extract additional Dublin Core elements
            Self::extract_dublin_core(text, &mut metadata);
        }

        // Extract SVG-specific desc metadata with roles
        Self::extract_svg_desc_metadata(text, &mut metadata);

        // Check if animated
        if Self::is_animated(text) {
            metadata.insert(
                "SVG:Animated".to_string(),
                TagValue::String("true".to_string()),
            );
        }

        // Count SVG elements (shapes, text, etc.) for Worker 26
        let element_count = Self::count_svg_elements(text);
        if element_count > 0 {
            metadata.insert(
                "SVG:ElementCount".to_string(),
                TagValue::new_integer(element_count),
            );
        }

        // Check for <defs> definitions
        let has_definitions = text.contains("<defs");
        metadata.insert(
            "SVG:HasDefinitions".to_string(),
            TagValue::new_string(if has_definitions { "true" } else { "false" }),
        );

        // Check for <metadata> element
        let has_metadata = text.contains("<metadata");
        metadata.insert(
            "SVG:HasMetadata".to_string(),
            TagValue::new_string(if has_metadata { "true" } else { "false" }),
        );

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::SVG)
    }
}

/// Parses metadata from SVG files.
///
/// This is a convenience wrapper around SVGParser that provides a functional API.
pub fn parse_svg_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = SVGParser;
    parser.parse(reader).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::BufferedReader;

    #[test]
    fn test_svg_basic_parsing() {
        let svg_data = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" version="1.1" width="200" height="150">
  <title>Test SVG</title>
  <desc>A test description</desc>
  <rect x="10" y="10" width="100" height="50"/>
</svg>"#;

        let reader = BufferedReader::from_bytes(svg_data.as_bytes());
        let parser = SVGParser;
        let metadata = parser.parse(&reader).unwrap();

        assert_eq!(metadata.get("FileType").unwrap().as_string(), Some("SVG"));
        assert_eq!(metadata.get("ImageWidth").unwrap().as_string(), Some("200"));
        assert_eq!(
            metadata.get("ImageHeight").unwrap().as_string(),
            Some("150")
        );
        assert_eq!(metadata.get("Title").unwrap().as_string(), Some("Test SVG"));
        assert_eq!(
            metadata.get("Description").unwrap().as_string(),
            Some("A test description")
        );
        assert_eq!(
            metadata.get("SVG:Xmlns").unwrap().as_string(),
            Some("http://www.w3.org/2000/svg")
        );
        assert_eq!(
            metadata.get("SVG:SVGVersion").unwrap().as_string(),
            Some("1.1")
        );
    }

    #[test]
    fn test_svg_viewbox() {
        let svg_data = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 200"></svg>"#;

        let reader = BufferedReader::from_bytes(svg_data.as_bytes());
        let parser = SVGParser;
        let metadata = parser.parse(&reader).unwrap();

        assert_eq!(metadata.get("ImageWidth").unwrap().as_string(), Some("100"));
        assert_eq!(
            metadata.get("ImageHeight").unwrap().as_string(),
            Some("200")
        );
        assert_eq!(
            metadata.get("SVG:ViewBox").unwrap().as_string(),
            Some("0 0 100 200")
        );
    }

    #[test]
    fn test_svg_dimension_units() {
        let svg_data = r#"<svg width="300px" height="200em"></svg>"#;

        let reader = BufferedReader::from_bytes(svg_data.as_bytes());
        let parser = SVGParser;
        let metadata = parser.parse(&reader).unwrap();

        // Units should be preserved to match ExifTool behavior
        assert_eq!(
            metadata.get("ImageWidth").unwrap().as_string(),
            Some("300px")
        );
        assert_eq!(
            metadata.get("ImageHeight").unwrap().as_string(),
            Some("200em")
        );
    }

    #[test]
    fn test_svg_animated() {
        let svg_data = r#"<svg>
  <rect x="10" y="10" width="50" height="50">
    <animate attributeName="x" from="10" to="100" dur="1s"/>
  </rect>
</svg>"#;

        let reader = BufferedReader::from_bytes(svg_data.as_bytes());
        let parser = SVGParser;
        let metadata = parser.parse(&reader).unwrap();

        assert_eq!(
            metadata.get("SVG:Animated").unwrap().as_string(),
            Some("true")
        );
    }

    #[test]
    fn test_svg_dublin_core() {
        let svg_data = r#"<svg xmlns:dc="http://purl.org/dc/elements/1.1/">
  <metadata>
    <dc:title>DC Title</dc:title>
    <dc:creator>DC Creator</dc:creator>
    <dc:description>DC Description</dc:description>
  </metadata>
</svg>"#;

        let reader = BufferedReader::from_bytes(svg_data.as_bytes());
        let parser = SVGParser;
        let metadata = parser.parse(&reader).unwrap();

        assert_eq!(
            metadata.get("XMP:Title").unwrap().as_string(),
            Some("DC Title")
        );
        assert_eq!(
            metadata.get("XMP:Creator").unwrap().as_string(),
            Some("DC Creator")
        );
        assert_eq!(
            metadata.get("XMP:Description").unwrap().as_string(),
            Some("DC Description")
        );
    }

    #[test]
    fn test_svg_dublin_core_rdf_bag() {
        // Test RDF Bag structure for multiple creators
        let svg_data = r#"<svg xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <metadata>
    <dc:creator>
      <rdf:Bag>
        <rdf:li>Irving Bird</rdf:li>
        <rdf:li>Mary Lambert</rdf:li>
      </rdf:Bag>
    </dc:creator>
  </metadata>
</svg>"#;

        let reader = BufferedReader::from_bytes(svg_data.as_bytes());
        let parser = SVGParser;
        let metadata = parser.parse(&reader).unwrap();

        assert_eq!(
            metadata.get("XMP:Creator").unwrap().as_string(),
            Some("[\"Irving Bird\",\"Mary Lambert\"]")
        );
    }

    #[test]
    fn test_svg_invalid() {
        let invalid_data = b"Not an SVG file";
        let reader = BufferedReader::from_bytes(invalid_data);
        let parser = SVGParser;

        let result = parser.parse(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_svg_single_quotes() {
        let svg_data = r#"<svg width='100' height='200'></svg>"#;

        let reader = BufferedReader::from_bytes(svg_data.as_bytes());
        let parser = SVGParser;
        let metadata = parser.parse(&reader).unwrap();

        assert_eq!(metadata.get("ImageWidth").unwrap().as_string(), Some("100"));
        assert_eq!(
            metadata.get("ImageHeight").unwrap().as_string(),
            Some("200")
        );
    }
}
