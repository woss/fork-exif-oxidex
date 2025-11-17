//! Specialized format parsers

pub mod dwg;
pub mod dxf;
pub mod elf;
pub mod fits;
pub mod gltf;
pub mod hdf5;
pub mod macho;
pub mod obj;
pub mod stl;

pub use dwg::DWGParser;
pub use dxf::DXFParser;
pub use elf::ELFParser;
pub use fits::FITSParser;
pub use gltf::GLTFParser;
pub use hdf5::HDF5Parser;
pub use macho::MachOParser;
pub use obj::OBJParser;
pub use stl::STLParser;
