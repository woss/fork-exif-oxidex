//! Tag registry modules for MakerNote parsers
//!
//! This module contains TagRegistry definitions for each manufacturer,
//! providing declarative tag and array schema definitions.

// Temporarily commented out incomplete registries to allow incremental testing
// TODO: Re-enable after Canon, Nikon, Google migrations are complete
pub mod canon;
pub mod sony; // Sony migration complete (Task 6)
pub mod apple;
// pub mod google;

pub use canon::canon_registry;
pub use sony::sony_registry; // Sony migration complete (Task 6)
pub use apple::apple_registry;
// pub use google::google_registry;
