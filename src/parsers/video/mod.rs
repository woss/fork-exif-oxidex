//! Video format parsers
//!
//! This module contains parsers for various video container formats.

#![allow(dead_code)]

pub mod mkv;
pub mod flv;
pub mod avi;
pub mod mts;

pub use mkv::MkvParser;
pub use flv::FlvParser;
pub use avi::AviParser;
pub use mts::MtsParser;
