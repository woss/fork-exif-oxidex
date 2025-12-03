//! SVG (Scalable Vector Graphics) parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

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

    /// Parses dimension value, stripping units like "px", "em", "%"
    fn parse_dimension(value: &str) -> Option<String> {
        let trimmed = value.trim();
        // Try to extract numeric part
        let numeric: String = trimmed
            .chars()
            .take_while(|c| c.is_numeric() || *c == '.')
            .collect();

        if !numeric.is_empty() {
            Some(numeric)
        } else {
            Some(trimmed.to_string())
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
            if let Some(width) = Self::extract_attribute(svg_tag, "width") {
                if let Some(parsed) = Self::parse_dimension(&width) {
                    metadata.insert("ImageWidth".to_string(), TagValue::String(parsed));
                }
            }

            if let Some(height) = Self::extract_attribute(svg_tag, "height") {
                if let Some(parsed) = Self::parse_dimension(&height) {
                    metadata.insert("ImageHeight".to_string(), TagValue::String(parsed));
                }
            }

            // Extract viewBox for dimensions if width/height not present
            if let Some(viewbox) = Self::extract_attribute(svg_tag, "viewBox") {
                metadata.insert("SVG:ViewBox".to_string(), TagValue::String(viewbox.clone()));

                // If no width/height, try to extract from viewBox
                if !metadata.contains_key("ImageWidth") {
                    if let Some((vb_width, vb_height)) = Self::parse_viewbox(&viewbox) {
                        metadata.insert("ImageWidth".to_string(), TagValue::String(vb_width));
                        metadata.insert("ImageHeight".to_string(), TagValue::String(vb_height));
                    }
                }
            }

            // Extract xmlns (namespace)
            if let Some(xmlns) = Self::extract_attribute(svg_tag, "xmlns") {
                metadata.insert("SVG:Namespace".to_string(), TagValue::String(xmlns));
            }

            // Extract version
            if let Some(version) = Self::extract_attribute(svg_tag, "version") {
                metadata.insert("SVG:Version".to_string(), TagValue::String(version));
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

        // Extract Dublin Core metadata if present
        if text.contains("dc:") {
            if let Some(dc_title) = Self::extract_element_content(text, "dc:title") {
                metadata.insert("XMP:Title".to_string(), TagValue::String(dc_title));
            }
            if let Some(dc_creator) = Self::extract_element_content(text, "dc:creator") {
                metadata.insert("XMP:Creator".to_string(), TagValue::String(dc_creator));
            }
            if let Some(dc_desc) = Self::extract_element_content(text, "dc:description") {
                metadata.insert("XMP:Description".to_string(), TagValue::String(dc_desc));
            }
        }

        // Check if animated
        if Self::is_animated(text) {
            metadata.insert(
                "SVG:Animated".to_string(),
                TagValue::String("true".to_string()),
            );
        }

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
            metadata.get("SVG:Namespace").unwrap().as_string(),
            Some("http://www.w3.org/2000/svg")
        );
        assert_eq!(
            metadata.get("SVG:Version").unwrap().as_string(),
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

        assert_eq!(metadata.get("ImageWidth").unwrap().as_string(), Some("300"));
        assert_eq!(
            metadata.get("ImageHeight").unwrap().as_string(),
            Some("200")
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
