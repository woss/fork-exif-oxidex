//! Tag registry modules for MakerNote parsers
//!
//! This module contains TagRegistry definitions for each manufacturer,
//! providing declarative tag and array schema definitions.

// Temporarily commented out incomplete registries to allow incremental testing
// TODO: Re-enable after Canon, Nikon, Google migrations are complete
pub mod canon;
pub mod sony; // Sony migration complete (Task 6)
pub mod apple;
pub mod captureone; // Capture One migration complete (Batch 4, Task 4.2)
pub mod nikoncapture; // Nikon Capture migration complete (Batch 4, Task 4.3)
// pub mod google;

// Batch 1: Traditional Camera Manufacturers
pub mod olympus; // Olympus migration (Batch 1, Task 1.1)
pub mod panasonic; // Panasonic migration (Batch 1, Task 1.2)
pub mod pentax; // Pentax migration (Batch 1, Task 1.3)
pub mod fujifilm; // Fujifilm migration (Batch 1, Task 1.4)
pub mod leica; // Leica migration (Batch 1, Task 1.5)

// Batch 2: Smartphone manufacturers
pub mod microsoft; // Microsoft migration complete (Batch 2, Task 2.1)
pub mod samsung; // Samsung migration complete (Batch 2, Task 2.2)
pub mod qualcomm; // Qualcomm migration complete (Batch 2, Task 2.3)

// Batch 3: Specialty Device Manufacturers
pub mod dji; // DJI migration complete (Batch 3, Task 3.1)
pub mod gopro; // GoPro migration (Batch 3, Task 3.2)
pub mod flir; // FLIR migration (Batch 3, Task 3.3)
pub mod lytro; // Lytro migration (Batch 3, Task 3.4)

// Batch 5: Legacy and Niche Manufacturers
// Sub-Batch 5.1: Traditional Camera Manufacturers
pub mod sigma; // Sigma migration (Batch 5, Sub-Batch 5.1)
pub mod minolta; // Minolta migration (Batch 5, Sub-Batch 5.1)
pub mod ricoh; // Ricoh migration (Batch 5, Sub-Batch 5.1)
pub mod casio; // Casio migration (Batch 5, Sub-Batch 5.1)
pub mod kodak; // Kodak migration (Batch 5, Sub-Batch 5.1)

// Sub-Batch 5.2: Medium Format and Specialty Manufacturers
pub mod phaseone; // Phase One migration (Batch 5, Sub-Batch 5.2)
pub mod leaf; // Leaf migration (Batch 5, Sub-Batch 5.2)
pub mod red; // RED migration (Batch 5, Sub-Batch 5.2)
pub mod parrot; // Parrot migration (Batch 5, Sub-Batch 5.2)

pub use canon::canon_registry;
pub use sony::sony_registry; // Sony migration complete (Task 6)
pub use apple::apple_registry;
// pub use google::google_registry;

// Batch 1 exports
pub use olympus::olympus_registry;
pub use panasonic::panasonic_registry;
pub use pentax::pentax_registry;
pub use fujifilm::fujifilm_registry;
pub use leica::leica_registry;

// Batch 2 exports
pub use microsoft::microsoft_registry;
pub use samsung::samsung_registry;
pub use qualcomm::qualcomm_registry;

// Batch 3 exports
pub use dji::dji_registry;
pub use gopro::gopro_registry;
pub use flir::flir_registry;
pub use lytro::lytro_registry;

// Batch 5 Sub-Batch 5.1 exports
pub use sigma::sigma_registry;
pub use minolta::minolta_registry;
pub use ricoh::ricoh_registry;
pub use casio::casio_registry;
pub use kodak::kodak_registry;

// Batch 5 Sub-Batch 5.2 exports
pub use phaseone::phaseone_registry;
pub use leaf::leaf_registry;
pub use red::red_registry;
pub use parrot::parrot_registry;
