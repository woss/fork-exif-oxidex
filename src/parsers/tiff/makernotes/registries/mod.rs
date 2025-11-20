//! Tag registry modules for MakerNote parsers
//!
//! This module contains TagRegistry definitions for each manufacturer,
//! providing declarative tag and array schema definitions.

pub mod canon;
pub mod nikon;
pub mod sony;
pub mod apple;
pub mod google;

pub use canon::canon_registry;
pub use nikon::nikon_registry;
pub use sony::sony_registry;
pub use apple::apple_registry;
pub use google::google_registry;
