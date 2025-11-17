//! Video format parsers
//!
//! This module contains parsers for various video container formats.

#![allow(dead_code)]

pub mod avi;
pub mod flv;
pub mod mkv;
pub mod mts;
pub mod webm;

pub use avi::{parse_avi_metadata, AviParser};
pub use flv::{parse_flv_metadata, FlvParser};
pub use mkv::{parse_mkv_metadata, MkvParser};
pub use mts::{parse_mts_metadata, MtsParser};
pub use webm::{parse_webm_metadata, WebmParser};
