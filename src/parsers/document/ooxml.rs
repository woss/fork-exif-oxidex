//! Office Open XML (DOCX, XLSX, PPTX) format parsers

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::io::{Cursor, Read};
use zip::ZipArchive;

/// DOCX parser
pub struct DocxParser;

impl FormatParser for DocxParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        let mut metadata = MetadataMap::new();

        // Read as ZIP
        let size = reader.size() as usize;
        let file_data = reader.read(0, size)?;
        let cursor = Cursor::new(file_data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| ExifToolError::parse_error(format!("Not a valid DOCX: {}", e)))?;

        // Check for DOCX-specific files
        let has_content_types = archive.by_name("[Content_Types].xml").is_ok();
        let has_word_doc = archive.by_name("word/document.xml").is_ok();

        if !has_content_types || !has_word_doc {
            return Err(ExifToolError::parse_error("Not a valid DOCX file"));
        }

        // Parse core.xml for metadata
        if let Ok(mut core_file) = archive.by_name("docProps/core.xml") {
            let mut xml_content = String::new();
            core_file.read_to_string(&mut xml_content)
                .map_err(|e| ExifToolError::parse_error(format!("Failed to read core.xml: {}", e)))?;

            parse_core_properties(&xml_content, &mut metadata)?;
        }

        // Parse app.xml for application properties
        if let Ok(mut app_file) = archive.by_name("docProps/app.xml") {
            let mut xml_content = String::new();
            app_file.read_to_string(&mut xml_content)
                .map_err(|e| ExifToolError::parse_error(format!("Failed to read app.xml: {}", e)))?;

            parse_app_properties(&xml_content, &mut metadata)?;
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::DOCX)
    }
}

/// XLSX parser
pub struct XlsxParser;

impl FormatParser for XlsxParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        let mut metadata = MetadataMap::new();
        let size = reader.size() as usize;
        let file_data = reader.read(0, size)?;
        let cursor = Cursor::new(file_data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| ExifToolError::parse_error(format!("Not a valid XLSX: {}", e)))?;

        if archive.by_name("xl/workbook.xml").is_err() {
            return Err(ExifToolError::parse_error("Not a valid XLSX file"));
        }

        // Parse metadata from docProps
        if let Ok(mut core_file) = archive.by_name("docProps/core.xml") {
            let mut xml_content = String::new();
            core_file.read_to_string(&mut xml_content).ok();
            parse_core_properties(&xml_content, &mut metadata)?;
        }

        if let Ok(mut app_file) = archive.by_name("docProps/app.xml") {
            let mut xml_content = String::new();
            app_file.read_to_string(&mut xml_content).ok();
            parse_app_properties(&xml_content, &mut metadata)?;
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::XLSX)
    }
}

/// PPTX parser
pub struct PptxParser;

impl FormatParser for PptxParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        let mut metadata = MetadataMap::new();
        let size = reader.size() as usize;
        let file_data = reader.read(0, size)?;
        let cursor = Cursor::new(file_data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| ExifToolError::parse_error(format!("Not a valid PPTX: {}", e)))?;

        if archive.by_name("ppt/presentation.xml").is_err() {
            return Err(ExifToolError::parse_error("Not a valid PPTX file"));
        }

        // Parse metadata
        if let Ok(mut core_file) = archive.by_name("docProps/core.xml") {
            let mut xml_content = String::new();
            core_file.read_to_string(&mut xml_content).ok();
            parse_core_properties(&xml_content, &mut metadata)?;
        }

        if let Ok(mut app_file) = archive.by_name("docProps/app.xml") {
            let mut xml_content = String::new();
            app_file.read_to_string(&mut xml_content).ok();
            parse_app_properties(&xml_content, &mut metadata)?;
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::PPTX)
    }
}

/// Parse core.xml properties (Dublin Core metadata)
fn parse_core_properties(xml: &str, metadata: &mut MetadataMap) -> Result<()> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut current_element = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_element = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                if let Ok(text) = e.xml_content() {
                    if !text.is_empty() {
                        let tag_name = match current_element.as_str() {
                            "title" => "OOXML:Title",
                            "creator" => "OOXML:Creator",
                            "subject" => "OOXML:Subject",
                            "description" => "OOXML:Description",
                            "created" => "OOXML:CreateDate",
                            "modified" => "OOXML:ModifyDate",
                            _ => {
                                buf.clear();
                                continue;
                            }
                        };
                        metadata.insert(tag_name.to_string(), TagValue::new_string(text.to_string()));
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ExifToolError::parse_error(format!("XML parse error: {}", e))),
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

/// Parse app.xml properties
fn parse_app_properties(xml: &str, metadata: &mut MetadataMap) -> Result<()> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut current_element = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_element = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                if let Ok(text) = e.xml_content() {
                    if !text.is_empty() {
                        let tag_name = match current_element.as_str() {
                            "Application" => "OOXML:Application",
                            "Pages" => "OOXML:Pages",
                            "Words" => "OOXML:Words",
                            "Characters" => "OOXML:Characters",
                            "Company" => "OOXML:Company",
                            _ => {
                                buf.clear();
                                continue;
                            }
                        };
                        metadata.insert(tag_name.to_string(), TagValue::new_string(text.to_string()));
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_core_properties() {
        let xml = r#"<?xml version="1.0"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties">
    <dc:title>Test Document</dc:title>
    <dc:creator>John Doe</dc:creator>
    <dc:subject>Testing</dc:subject>
</cp:coreProperties>"#;

        let mut metadata = MetadataMap::new();
        let result = parse_core_properties(xml, &mut metadata);
        assert!(result.is_ok());
        assert!(metadata.contains_key("OOXML:Title"));
    }

    #[test]
    fn test_parse_app_properties() {
        let xml = r#"<?xml version="1.0"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">
    <Application>Microsoft Word</Application>
    <Pages>10</Pages>
</Properties>"#;

        let mut metadata = MetadataMap::new();
        let result = parse_app_properties(xml, &mut metadata);
        assert!(result.is_ok());
    }
}
