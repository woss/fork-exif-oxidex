//! Video format parsers
//!
//! This module contains parsers for various video container formats.

#![allow(dead_code)]

pub mod avi;
pub mod flv;
pub mod mkv;
pub mod mts;
pub mod webm;

pub use avi::AviParser;
pub use flv::FlvParser;
pub use mkv::MkvParser;
pub use mts::MtsParser;
pub use webm::WebmParser;
