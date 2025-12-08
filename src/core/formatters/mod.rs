//! Value formatters for ExifTool-compatible output
//!
//! This module contains formatters that transform raw metadata values into
//! human-readable strings matching ExifTool's output format.

pub mod cfa_pattern;
pub mod exposure_program;
pub mod gps_altitude_ref;
pub mod gps_direction_ref;
pub mod gps_lat_lon_ref;
pub mod gps_processing_method;
pub mod gps_speed_ref;
pub mod gps_status;
pub mod interop_version;
pub mod numeric_precision;
pub mod scene_type;
pub mod unit_suffixes;

// Re-export main formatting functions for convenience
pub use cfa_pattern::decode_cfa_pattern;
pub use exposure_program::format_exposure_program;
pub use gps_altitude_ref::{format_gps_altitude_ref, format_gps_altitude_ref_byte};
pub use gps_direction_ref::format_gps_direction_ref;
pub use gps_lat_lon_ref::{format_gps_lat_ref, format_gps_lon_ref};
pub use gps_processing_method::decode_gps_processing_method;
pub use gps_speed_ref::format_gps_speed_ref;
pub use gps_status::format_gps_status;
pub use interop_version::decode_version_bytes;
pub use numeric_precision::{
    format_exif_rational, format_icc_value, format_integer_precision_values,
    format_three_decimal_values, is_icc_matrix_tag, is_integer_precision_tag, is_three_decimal_tag,
};
pub use scene_type::decode_scene_type;
pub use unit_suffixes::format_with_unit;
