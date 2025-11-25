//! Pentax lens database
//!
//! This module is a compatibility shim for the consolidated lens data.
//! Prefer using `oxidex::parsers::tiff::makernotes::lens_data::pentax` directly.

#![allow(missing_docs)]

use super::lens_data::pentax;

pub fn lookup_lens_name(lens_id: u16) -> Option<String> {
    pentax::lookup(lens_id).map(|s| s.to_string())
}
