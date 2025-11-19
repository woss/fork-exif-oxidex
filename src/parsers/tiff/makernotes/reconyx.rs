//! Reconyx Wildlife Camera MakerNote parser
//!
//! Parses Reconyx-specific EXIF MakerNote tags from trail/wildlife cameras.
//! Reconyx specializes in motion-triggered cameras for wildlife monitoring.
//!
//! ## Supported Models
//! - HyperFire Series (HF2X, HF2XC)
//! - UltraFire Series (XR6, XP9)
//! - MicroFire Series (MR5, MS8)
//! - PC900 (security)
//!
//! ## Key Features
//! - Motion trigger details
//! - Time-lapse interval
//! - Temperature (ambient)
//! - Battery voltage
//! - Moon phase
//! - Sequence number
//! - PIR (infrared) sensor data
//!
//! ## Architecture
//! Reconyx uses specialized metadata for wildlife monitoring applications.

#![allow(dead_code)]
#![allow(unused_imports)]

use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};
use once_cell::sync::Lazy;
use std::collections::HashMap;

use super::shared::array_extractors::{extract_i16_array, extract_string};
use super::shared::ifd_parser_base::{parse_ifd_entries, IfdParserConfig};
use super::shared::tag_registry::TagRegistry;
use super::shared::MakerNoteParser;

// Import macros
use crate::const_decoder;

// Reconyx MakerNote Tag IDs
const RECONYX_MODEL: u16 = 0x0001;
const RECONYX_SERIAL: u16 = 0x0002;
const RECONYX_FIRMWARE: u16 = 0x0003;
const RECONYX_TRIGGER_MODE: u16 = 0x0100;
const RECONYX_SEQUENCE_NUMBER: u16 = 0x0101;
const RECONYX_EVENT_NUMBER: u16 = 0x0102;
const RECONYX_TEMPERATURE: u16 = 0x0103;
const RECONYX_BATTERY_VOLTAGE: u16 = 0x0104;
const RECONYX_MOON_PHASE: u16 = 0x0105;
const RECONYX_TIMELAPSE_INTERVAL: u16 = 0x0106;
const RECONYX_PIR_READINGS: u16 = 0x0107;
const RECONYX_FLASH_OUTPUT: u16 = 0x0108;
const RECONYX_SENSOR_SENSITIVITY: u16 = 0x0109;
const RECONYX_MOTION_DETECT_LEVEL: u16 = 0x010A;

const RECONYX_SIGNATURE: &[u8] = b"Reconyx";

// ============================================================================
// Declarative Decoder Definitions
// ============================================================================

const_decoder!(
    TRIGGER_MODE,
    i16,
    [
        (0, "Time Lapse"),
        (1, "Motion Detection"),
        (2, "Time Lapse + Motion"),
    ]
);

const_decoder!(
    MOON_PHASE,
    i16,
    [
        (0, "New Moon"),
        (1, "Waxing Crescent"),
        (2, "First Quarter"),
        (3, "Waxing Gibbous"),
        (4, "Full Moon"),
        (5, "Waning Gibbous"),
        (6, "Last Quarter"),
        (7, "Waning Crescent"),
    ]
);

// ============================================================================
// Custom Formatters
// ============================================================================

fn format_temperature(value: i16) -> String {
    format!("{}°C", value)
}

fn format_voltage(value: i16) -> String {
    format!("{:.2} V", value as f64 / 1000.0)
}

fn format_interval(value: i16) -> String {
    format!("{} seconds", value)
}

// ============================================================================
// Tag Registry
// ============================================================================

static TAG_REGISTRY: Lazy<TagRegistry> = Lazy::new(|| {
    TagRegistry::with_capacity(14)
        .register_raw(RECONYX_MODEL, "Model")
        .register_raw(RECONYX_SERIAL, "SerialNumber")
        .register_raw(RECONYX_FIRMWARE, "FirmwareVersion")
        .register_simple_i16(RECONYX_TRIGGER_MODE, "TriggerMode", &TRIGGER_MODE)
        .register_simple_i16(RECONYX_MOON_PHASE, "MoonPhase", &MOON_PHASE)
        .register_raw(RECONYX_SEQUENCE_NUMBER, "SequenceNumber")
        .register_raw(RECONYX_EVENT_NUMBER, "EventNumber")
        .register_raw(RECONYX_TEMPERATURE, "Temperature")
        .register_raw(RECONYX_BATTERY_VOLTAGE, "BatteryVoltage")
        .register_raw(RECONYX_TIMELAPSE_INTERVAL, "TimelapseInterval")
        .register_raw(RECONYX_PIR_READINGS, "PIRReadings")
        .register_raw(RECONYX_FLASH_OUTPUT, "FlashOutput")
        .register_raw(RECONYX_SENSOR_SENSITIVITY, "SensorSensitivity")
        .register_raw(RECONYX_MOTION_DETECT_LEVEL, "MotionDetectLevel")
});

// ============================================================================
// Parser Implementation
// ============================================================================

#[derive(Default)]
pub struct ReconyxParser;

impl ReconyxParser {
    pub fn new() -> Self {
        ReconyxParser
    }

    fn parse_entry(
        &self,
        entry: &IfdEntry,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) {
        let tag_id = entry.tag_id;

        // Handle string tags
        match tag_id {
            RECONYX_MODEL | RECONYX_SERIAL | RECONYX_FIRMWARE => {
                if let Some(s) = extract_string(entry, data, byte_order) {
                    if let Some(name) = TAG_REGISTRY.get_tag_name(tag_id) {
                        tags.insert(format!("Reconyx:{}", name), s);
                    }
                }
                return;
            }
            _ => {}
        }

        // Handle i16 array tags
        if let Some(array) = extract_i16_array(entry, data, byte_order) {
            if let Some(&value) = array.first() {
                let tag_name = match TAG_REGISTRY.get_tag_name(tag_id) {
                    Some(name) => name,
                    None => return,
                };

                let formatted_value = match tag_id {
                    RECONYX_TRIGGER_MODE | RECONYX_MOON_PHASE => {
                        TAG_REGISTRY.decode_i16(tag_id, value)
                    }
                    RECONYX_SEQUENCE_NUMBER
                    | RECONYX_EVENT_NUMBER
                    | RECONYX_PIR_READINGS
                    | RECONYX_SENSOR_SENSITIVITY
                    | RECONYX_MOTION_DETECT_LEVEL => value.to_string(),
                    RECONYX_TEMPERATURE => format_temperature(value),
                    RECONYX_BATTERY_VOLTAGE => format_voltage(value),
                    RECONYX_TIMELAPSE_INTERVAL => format_interval(value),
                    RECONYX_FLASH_OUTPUT => format!("{}%", value),
                    _ => return,
                };

                tags.insert(format!("Reconyx:{}", tag_name), formatted_value);
            }
        }
    }
}

impl MakerNoteParser for ReconyxParser {
    fn manufacturer_name(&self) -> &'static str {
        "Reconyx"
    }

    fn tag_prefix(&self) -> &'static str {
        "Reconyx:"
    }

    fn validate_header(&self, data: &[u8]) -> bool {
        data.len() >= 8 && (data.starts_with(RECONYX_SIGNATURE) || data.len() >= 8)
    }

    fn parse(
        &self,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) -> Result<(), String> {
        let config = IfdParserConfig {
            signature: Some(RECONYX_SIGNATURE),
            signature_offset: 7,
            max_entries: 200,
        };

        parse_ifd_entries(data, byte_order, &config, |entry, parse_data| {
            self.parse_entry(entry, parse_data, byte_order, tags);
        })
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconyx_parser_creation() {
        let parser = ReconyxParser::new();
        assert_eq!(parser.manufacturer_name(), "Reconyx");
        assert_eq!(parser.tag_prefix(), "Reconyx:");
    }

    #[test]
    fn test_trigger_mode_decoder() {
        assert_eq!(TRIGGER_MODE.decode(0), "Time Lapse");
        assert_eq!(TRIGGER_MODE.decode(1), "Motion Detection");
    }

    #[test]
    fn test_moon_phase_decoder() {
        assert_eq!(MOON_PHASE.decode(0), "New Moon");
        assert_eq!(MOON_PHASE.decode(4), "Full Moon");
    }

    #[test]
    fn test_format_voltage() {
        assert_eq!(format_voltage(6000), "6.00 V");
    }

    #[test]
    fn test_tag_registry() {
        assert_eq!(TAG_REGISTRY.get_tag_name(RECONYX_MODEL), Some("Model"));
        assert!(TAG_REGISTRY.has_tag(RECONYX_TEMPERATURE));
    }
}
