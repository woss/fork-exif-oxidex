//! Audio format parsers
//!
//! This module contains parsers for various audio formats.

#![allow(dead_code)]

pub mod flac;
pub mod mp3;
pub mod aac;
pub mod wav;
pub mod ogg;
pub mod opus;
pub mod ape;

pub use flac::FlacParser;
pub use mp3::Mp3Parser;
pub use aac::AacParser;
pub use wav::WavParser;
pub use ogg::OggParser;
pub use opus::OpusParser;
pub use ape::ApeParser;
