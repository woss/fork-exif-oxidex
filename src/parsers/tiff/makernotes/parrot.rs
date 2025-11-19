//! Parrot Drone MakerNote parser
//!
//! Parses Parrot-specific EXIF MakerNote tags from consumer and professional drones.
//! Parrot manufactures the Anafi, Bebop, and other drone series.
//!
//! ## Supported Models
//! - Anafi (4K, USA, Thermal)
//! - Anafi AI
//! - Bebop 2
//! - Bebop Drone
//! - Disco FPV
//! - Bluegrass (agriculture)
//!
//! ## Key Features
//! - GPS coordinates and altitude
//! - Flight speed and direction
//! - Gimbal angles
//! - Battery level
//! - WiFi signal strength
//! - Flight mode
//! - Camera settings
//!
//! ## Architecture
//! Similar to DJI but with simplified metadata structure.

#![allow(dead_code)]
#![allow(unused_imports)]

use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};
use once_cell::sync::Lazy;
use std::collections::HashMap;

use super::shared::array_extractors::{extract_i16_array, extract_i32_value, extract_string};
use super::shared::ifd_parser_base::{parse_ifd_entries, IfdParserConfig};
use super::shared::tag_registry::TagRegistry;
use super::shared::MakerNoteParser;

// Import macros for declarative decoder definitions
use crate::const_decoder;

// Parrot MakerNote Tag IDs
const PARROT_MODEL: u16 = 0x0001;
const PARROT_SERIAL: u16 = 0x0002;
const PARROT_VERSION: u16 = 0x0003;
const PARROT_GPS_LAT: u16 = 0x0100;
const PARROT_GPS_LON: u16 = 0x0101;
const PARROT_ALTITUDE: u16 = 0x0102;
const PARROT_SPEED: u16 = 0x0103;
const PARROT_DIRECTION: u16 = 0x0104;
const PARROT_GIMBAL_PITCH: u16 = 0x0105;
const PARROT_GIMBAL_ROLL: u16 = 0x0106;
const PARROT_GIMBAL_YAW: u16 = 0x0107;
const PARROT_BATTERY: u16 = 0x0108;
const PARROT_WIFI_SIGNAL: u16 = 0x0109;
const PARROT_FLIGHT_MODE: u16 = 0x010A;
const PARROT_DISTANCE: u16 = 0x010B;

// Parrot signature
const PARROT_SIGNATURE: &[u8] = b"Parrot";

// ============================================================================
// Declarative Decoder Definitions
// ============================================================================

// Flight Mode decoder - Different autonomous and manual flight modes
const_decoder!(
    FLIGHT_MODE,
    i16,
    [
        (0, "Manual"),
        (1, "GPS"),
        (2, "Follow Me"),
        (3, "Return Home"),
    ]
);

// ============================================================================
// Custom Formatters
// ============================================================================

// Formats GPS coordinates (stored as 1/10,000,000 degrees)
// # Arguments
// * `value` - GPS coordinate in 1/10,000,000 degree units
// # Returns
// Formatted coordinate with 7 decimal places
fn format_gps_coord(value: i32) -> String {
    format!("{:.7}", value as f64 / 10_000_000.0)
}

// Formats altitude (stored as centimeters)
// # Arguments
// * `value` - Altitude in centimeters
// # Returns
// Formatted altitude in meters with 2 decimal places
fn format_altitude(value: i16) -> String {
    format!("{:.2} m", value as f64 / 100.0)
}

// Formats speed (stored as deciseconds per meter)
// # Arguments
// * `value` - Speed in 1/10 m/s units
// # Returns
// Formatted speed in m/s with 1 decimal place
fn format_speed(value: i16) -> String {
    format!("{:.1} m/s", value as f64 / 10.0)
}

// Formats gimbal angle (stored as decidegrees)
// # Arguments
// * `value` - Angle in 1/10 degree units
// # Returns
// Formatted angle in degrees with 1 decimal place
fn format_gimbal_angle(value: i16) -> String {
    format!("{:.1}°", value as f64 / 10.0)
}

// ============================================================================
// Tag Registry
// ============================================================================

// Lazy-initialized tag registry for Parrot-specific tags
static TAG_REGISTRY: Lazy<TagRegistry> = Lazy::new(|| {
    TagRegistry::with_capacity(15)
        // String tags
        .register_raw(PARROT_MODEL, "Model")
        .register_raw(PARROT_SERIAL, "SerialNumber")
        .register_raw(PARROT_VERSION, "Version")
        // GPS tags (handled separately due to i32 type)
        .register_raw(PARROT_GPS_LAT, "GPSLatitude")
        .register_raw(PARROT_GPS_LON, "GPSLongitude")
        // Flight mode decoder
        .register_simple_i16(PARROT_FLIGHT_MODE, "FlightMode", &FLIGHT_MODE)
        // Custom formatted tags (handled separately)
        .register_raw(PARROT_ALTITUDE, "Altitude")
        .register_raw(PARROT_SPEED, "Speed")
        .register_raw(PARROT_DIRECTION, "Direction")
        .register_raw(PARROT_GIMBAL_PITCH, "GimbalPitch")
        .register_raw(PARROT_GIMBAL_ROLL, "GimbalRoll")
        .register_raw(PARROT_GIMBAL_YAW, "GimbalYaw")
        .register_raw(PARROT_BATTERY, "BatteryLevel")
        .register_raw(PARROT_WIFI_SIGNAL, "WiFiSignal")
        .register_raw(PARROT_DISTANCE, "HomeDistance")
});

// ============================================================================
// Parser Implementation
// ============================================================================

/// Parser for Parrot MakerNotes
#[derive(Default)]
pub struct ParrotParser;

impl ParrotParser {
    /// Creates a new Parrot parser instance
    pub fn new() -> Self {
        ParrotParser
    }

    /// Parses a single IFD entry and extracts the tag value
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
            PARROT_MODEL | PARROT_SERIAL | PARROT_VERSION => {
                if let Some(s) = extract_string(entry, data, byte_order) {
                    if let Some(name) = TAG_REGISTRY.get_tag_name(tag_id) {
                        tags.insert(format!("Parrot:{}", name), s);
                    }
                }
                return;
            }
            _ => {}
        }

        // Handle GPS coordinates (i32 type)
        match tag_id {
            PARROT_GPS_LAT | PARROT_GPS_LON => {
                if let Some(val) = extract_i32_value(entry, data, byte_order) {
                    if let Some(name) = TAG_REGISTRY.get_tag_name(tag_id) {
                        tags.insert(format!("Parrot:{}", name), format_gps_coord(val));
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
                    PARROT_FLIGHT_MODE => TAG_REGISTRY.decode_i16(tag_id, value),
                    PARROT_ALTITUDE => format_altitude(value),
                    PARROT_SPEED => format_speed(value),
                    PARROT_DIRECTION => format!("{}°", value),
                    PARROT_GIMBAL_PITCH | PARROT_GIMBAL_ROLL => format_gimbal_angle(value),
                    PARROT_GIMBAL_YAW => format!("{}°", value),
                    PARROT_BATTERY => format!("{}%", value),
                    PARROT_WIFI_SIGNAL => format!("{} dBm", value),
                    PARROT_DISTANCE => format!("{} m", value),
                    _ => return,
                };

                tags.insert(format!("Parrot:{}", tag_name), formatted_value);
            }
        }
    }
}

impl MakerNoteParser for ParrotParser {
    fn manufacturer_name(&self) -> &'static str {
        "Parrot"
    }

    fn tag_prefix(&self) -> &'static str {
        "Parrot:"
    }

    fn validate_header(&self, data: &[u8]) -> bool {
        data.len() >= 8 && (data.starts_with(PARROT_SIGNATURE) || data.len() >= 8)
    }

    fn parse(
        &self,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) -> Result<(), String> {
        let config = IfdParserConfig {
            signature: Some(PARROT_SIGNATURE),
            signature_offset: 6, // Skip "Parrot" signature
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
    fn test_parrot_parser_creation() {
        let parser = ParrotParser::new();
        assert_eq!(parser.manufacturer_name(), "Parrot");
        assert_eq!(parser.tag_prefix(), "Parrot:");
    }

    #[test]
    fn test_flight_mode_decoder() {
        assert_eq!(FLIGHT_MODE.decode(0), "Manual");
        assert_eq!(FLIGHT_MODE.decode(1), "GPS");
        assert_eq!(FLIGHT_MODE.decode(3), "Return Home");
    }

    #[test]
    fn test_format_gps_coord() {
        assert_eq!(format_gps_coord(123456789), "12.3456789");
        assert_eq!(format_gps_coord(-87654321), "-8.7654321");
    }

    #[test]
    fn test_format_altitude() {
        assert_eq!(format_altitude(1250), "12.50 m");
        assert_eq!(format_altitude(0), "0.00 m");
    }

    #[test]
    fn test_format_speed() {
        assert_eq!(format_speed(55), "5.5 m/s");
        assert_eq!(format_speed(100), "10.0 m/s");
    }

    #[test]
    fn test_format_gimbal_angle() {
        assert_eq!(format_gimbal_angle(450), "45.0°");
        assert_eq!(format_gimbal_angle(-300), "-30.0°");
    }

    #[test]
    fn test_tag_registry() {
        assert_eq!(TAG_REGISTRY.get_tag_name(PARROT_MODEL), Some("Model"));
        assert_eq!(
            TAG_REGISTRY.get_tag_name(PARROT_FLIGHT_MODE),
            Some("FlightMode")
        );
        assert!(TAG_REGISTRY.has_tag(PARROT_ALTITUDE));
    }
}
