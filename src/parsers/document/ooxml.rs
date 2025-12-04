//! Office Open XML (DOCX, XLSX, PPTX) format parsers

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
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
            core_file.read_to_string(&mut xml_content).map_err(|e| {
                ExifToolError::parse_error(format!("Failed to read core.xml: {}", e))
            })?;

            parse_core_properties(&xml_content, &mut metadata)?;
        }

        // Parse app.xml for application properties
        if let Ok(mut app_file) = archive.by_name("docProps/app.xml") {
            let mut xml_content = String::new();
            app_file.read_to_string(&mut xml_content).map_err(|e| {
                ExifToolError::parse_error(format!("Failed to read app.xml: {}", e))
            })?;

            parse_app_properties(&xml_content, &mut metadata)?;
        }

        // Parse custom.xml for custom properties
        if let Ok(mut custom_file) = archive.by_name("docProps/custom.xml") {
            let mut xml_content = String::new();
            custom_file.read_to_string(&mut xml_content).map_err(|e| {
                ExifToolError::parse_error(format!("Failed to read custom.xml: {}", e))
            })?;

            parse_custom_properties(&xml_content, &mut metadata)?;
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

        if let Ok(mut custom_file) = archive.by_name("docProps/custom.xml") {
            let mut xml_content = String::new();
            custom_file.read_to_string(&mut xml_content).ok();
            parse_custom_properties(&xml_content, &mut metadata)?;
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

        if let Ok(mut custom_file) = archive.by_name("docProps/custom.xml") {
            let mut xml_content = String::new();
            custom_file.read_to_string(&mut xml_content).ok();
            parse_custom_properties(&xml_content, &mut metadata)?;
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
                            "lastModifiedBy" => "OOXML:LastModifiedBy",
                            "revision" => "OOXML:RevisionNumber",
                            "lastPrinted" => "OOXML:LastPrinted",
                            "category" => "OOXML:Category",
                            "contentStatus" => "OOXML:ContentStatus",
                            _ => {
                                buf.clear();
                                continue;
                            }
                        };
                        metadata
                            .insert(tag_name.to_string(), TagValue::new_string(text.to_string()));
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(ExifToolError::parse_error(format!(
                    "XML parse error: {}",
                    e
                )))
            }
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
                            "Manager" => "OOXML:Manager",
                            "Template" => "OOXML:Template",
                            "HyperlinkBase" => "OOXML:HyperlinkBase",
                            "HiddenSlides" => "OOXML:HiddenSlides",
                            "PresentationFormat" => "OOXML:PresentationFormat",
                            "AppVersion" => "OOXML:AppVersion",
                            "DocSecurity" => "OOXML:DocSecurity",
                            "TotalTime" => {
                                // Convert minutes to human-readable format
                                if let Ok(minutes) = text.parse::<u64>() {
                                    let formatted = format_edit_time(minutes);
                                    metadata.insert(
                                        "OOXML:TotalEditTime".to_string(),
                                        TagValue::new_string(formatted),
                                    );
                                }
                                buf.clear();
                                continue;
                            }
                            _ => {
                                buf.clear();
                                continue;
                            }
                        };
                        metadata
                            .insert(tag_name.to_string(), TagValue::new_string(text.to_string()));
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

/// Parse custom.xml properties (user-defined metadata)
fn parse_custom_properties(xml: &str, metadata: &mut MetadataMap) -> Result<()> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut current_property_name = String::new();
    let mut in_property = false;
    let mut in_value = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let element_name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();

                if element_name == "property" {
                    in_property = true;
                    // Extract property name from attribute
                    for attr in e.attributes().flatten() {
                        let key_bytes = attr.key.local_name();
                        let key = String::from_utf8_lossy(key_bytes.as_ref());
                        if key == "name" {
                            current_property_name =
                                String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                } else if in_property
                    && (element_name == "lpwstr" || element_name == "i4" || element_name == "bool")
                {
                    in_value = true;
                }
            }
            Ok(Event::Text(e)) => {
                if in_value && !current_property_name.is_empty() {
                    if let Ok(text) = e.xml_content() {
                        let tag_name = format!("OOXML:Custom:{}", current_property_name);
                        metadata.insert(tag_name, TagValue::new_string(text.to_string()));
                    }
                }
            }
            Ok(Event::End(e)) => {
                let element_name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                if element_name == "property" {
                    in_property = false;
                    current_property_name.clear();
                } else if element_name == "lpwstr" || element_name == "i4" || element_name == "bool"
                {
                    in_value = false;
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

/// Format edit time from minutes to human-readable string
fn format_edit_time(minutes: u64) -> String {
    if minutes == 0 {
        return "0 minutes".to_string();
    }

    let hours = minutes / 60;
    let remaining_minutes = minutes % 60;

    match (hours, remaining_minutes) {
        (0, m) => format!("{} minute{}", m, if m == 1 { "" } else { "s" }),
        (h, 0) => format!("{} hour{}", h, if h == 1 { "" } else { "s" }),
        (h, m) => format!(
            "{} hour{} {} minute{}",
            h,
            if h == 1 { "" } else { "s" },
            m,
            if m == 1 { "" } else { "s" }
        ),
    }
}

/// Standalone function to parse DOCX metadata
///
/// This function provides a convenient way to parse DOCX metadata without
/// directly instantiating the DocxParser struct.
pub fn parse_docx_metadata(
    reader: &dyn crate::core::FileReader,
) -> std::result::Result<MetadataMap, String> {
    let parser = DocxParser;
    parser
        .parse(reader)
        .map_err(|e| format!("DOCX parse error: {}", e))
}

/// Standalone function to parse XLSX metadata
///
/// This function provides a convenient way to parse XLSX metadata without
/// directly instantiating the XlsxParser struct.
pub fn parse_xlsx_metadata(
    reader: &dyn crate::core::FileReader,
) -> std::result::Result<MetadataMap, String> {
    let parser = XlsxParser;
    parser
        .parse(reader)
        .map_err(|e| format!("XLSX parse error: {}", e))
}

/// Standalone function to parse PPTX metadata
///
/// This function provides a convenient way to parse PPTX metadata without
/// directly instantiating the PptxParser struct.
pub fn parse_pptx_metadata(
    reader: &dyn crate::core::FileReader,
) -> std::result::Result<MetadataMap, String> {
    let parser = PptxParser;
    parser
        .parse(reader)
        .map_err(|e| format!("PPTX parse error: {}", e))
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
    fn test_parse_core_properties_forensic() {
        let xml = r#"<?xml version="1.0"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:dcterms="http://purl.org/dc/terms/">
    <dc:title>Forensic Test</dc:title>
    <dc:creator>John Doe</dc:creator>
    <cp:lastModifiedBy>Jane Smith</cp:lastModifiedBy>
    <cp:revision>42</cp:revision>
    <dcterms:created>2024-01-15T10:30:00Z</dcterms:created>
    <dcterms:modified>2024-01-20T15:45:00Z</dcterms:modified>
    <cp:lastPrinted>2024-01-18T09:00:00Z</cp:lastPrinted>
    <cp:category>Confidential</cp:category>
    <cp:contentStatus>Draft</cp:contentStatus>
</cp:coreProperties>"#;

        let mut metadata = MetadataMap::new();
        let result = parse_core_properties(xml, &mut metadata);
        assert!(result.is_ok());

        assert_eq!(
            metadata.get("OOXML:LastModifiedBy").unwrap().as_string(),
            Some("Jane Smith")
        );
        assert_eq!(
            metadata.get("OOXML:RevisionNumber").unwrap().as_string(),
            Some("42")
        );
        assert_eq!(
            metadata.get("OOXML:Category").unwrap().as_string(),
            Some("Confidential")
        );
        assert_eq!(
            metadata.get("OOXML:ContentStatus").unwrap().as_string(),
            Some("Draft")
        );
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

    #[test]
    fn test_parse_app_properties_forensic() {
        let xml = r#"<?xml version="1.0"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">
    <Application>Microsoft Office Word</Application>
    <AppVersion>16.0000</AppVersion>
    <Company>Acme Corp</Company>
    <Manager>Bob Johnson</Manager>
    <Template>Normal.dotm</Template>
    <TotalTime>45</TotalTime>
    <HyperlinkBase>http://example.com</HyperlinkBase>
    <DocSecurity>0</DocSecurity>
</Properties>"#;

        let mut metadata = MetadataMap::new();
        let result = parse_app_properties(xml, &mut metadata);
        assert!(result.is_ok());

        assert_eq!(
            metadata.get("OOXML:Application").unwrap().as_string(),
            Some("Microsoft Office Word")
        );
        assert_eq!(
            metadata.get("OOXML:AppVersion").unwrap().as_string(),
            Some("16.0000")
        );
        assert_eq!(
            metadata.get("OOXML:Company").unwrap().as_string(),
            Some("Acme Corp")
        );
        assert_eq!(
            metadata.get("OOXML:Manager").unwrap().as_string(),
            Some("Bob Johnson")
        );
        assert_eq!(
            metadata.get("OOXML:Template").unwrap().as_string(),
            Some("Normal.dotm")
        );
        assert_eq!(
            metadata.get("OOXML:TotalEditTime").unwrap().as_string(),
            Some("45 minutes")
        );
        assert_eq!(
            metadata.get("OOXML:HyperlinkBase").unwrap().as_string(),
            Some("http://example.com")
        );
        assert_eq!(
            metadata.get("OOXML:DocSecurity").unwrap().as_string(),
            Some("0")
        );
    }

    #[test]
    fn test_parse_app_properties_powerpoint() {
        let xml = r#"<?xml version="1.0"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">
    <Application>Microsoft Office PowerPoint</Application>
    <HiddenSlides>3</HiddenSlides>
    <PresentationFormat>On-screen Show (4:3)</PresentationFormat>
</Properties>"#;

        let mut metadata = MetadataMap::new();
        let result = parse_app_properties(xml, &mut metadata);
        assert!(result.is_ok());

        assert_eq!(
            metadata.get("OOXML:HiddenSlides").unwrap().as_string(),
            Some("3")
        );
        assert_eq!(
            metadata
                .get("OOXML:PresentationFormat")
                .unwrap()
                .as_string(),
            Some("On-screen Show (4:3)")
        );
    }

    #[test]
    fn test_parse_custom_properties() {
        let xml = r#"<?xml version="1.0"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/custom-properties">
    <property fmtid="{D5CDD505-2E9C-101B-9397-08002B2CF9AE}" pid="2" name="ProjectID">
        <vt:lpwstr>PROJ-12345</vt:lpwstr>
    </property>
    <property fmtid="{D5CDD505-2E9C-101B-9397-08002B2CF9AE}" pid="3" name="Classification">
        <vt:lpwstr>Internal Use Only</vt:lpwstr>
    </property>
    <property fmtid="{D5CDD505-2E9C-101B-9397-08002B2CF9AE}" pid="4" name="ReviewCount">
        <vt:i4>5</vt:i4>
    </property>
</Properties>"#;

        let mut metadata = MetadataMap::new();
        let result = parse_custom_properties(xml, &mut metadata);
        assert!(result.is_ok());

        assert_eq!(
            metadata.get("OOXML:Custom:ProjectID").unwrap().as_string(),
            Some("PROJ-12345")
        );
        assert_eq!(
            metadata
                .get("OOXML:Custom:Classification")
                .unwrap()
                .as_string(),
            Some("Internal Use Only")
        );
        assert_eq!(
            metadata
                .get("OOXML:Custom:ReviewCount")
                .unwrap()
                .as_string(),
            Some("5")
        );
    }

    #[test]
    fn test_format_edit_time() {
        assert_eq!(format_edit_time(0), "0 minutes");
        assert_eq!(format_edit_time(1), "1 minute");
        assert_eq!(format_edit_time(5), "5 minutes");
        assert_eq!(format_edit_time(45), "45 minutes");
        assert_eq!(format_edit_time(60), "1 hour");
        assert_eq!(format_edit_time(90), "1 hour 30 minutes");
        assert_eq!(format_edit_time(120), "2 hours");
        assert_eq!(format_edit_time(150), "2 hours 30 minutes");
        assert_eq!(format_edit_time(301), "5 hours 1 minute");
    }
}
