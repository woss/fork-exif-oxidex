//! Video format parsers
//!
//! This module contains parsers for various video container formats.

#![allow(dead_code)]

pub mod avi;
pub mod flv;
pub mod mkv;
pub mod mts;
pub mod webm;

pub use avi::{AviParser, parse_avi_metadata};
pub use flv::{FlvParser, parse_flv_metadata};
pub use mkv::{MkvParser, parse_mkv_metadata};
pub use mts::{MtsParser, parse_mts_metadata};
pub use webm::{WebmParser, parse_webm_metadata};
