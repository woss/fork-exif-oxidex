//! Nikon tag registry with array schemas
//!
//! This module will contain TagRegistry definitions for Nikon MakerNotes,
//! providing declarative tag and array schema definitions.
//!
//! Status: Stub - To be implemented in Task 5

use super::super::shared::tag_registry::TagRegistry;

/// Create Nikon tag registry with all tag definitions and array schemas
///
/// Returns a TagRegistry configured for Nikon MakerNote parsing.
/// This is a stub implementation that will be populated in Task 5.
pub fn nikon_registry() -> TagRegistry {
    TagRegistry::new()
}
