//! Specialized format parsers

pub mod dwg;
pub mod dxf;
pub mod elf;
pub mod evtx;
pub mod fits;
pub mod gltf;
pub mod hdf5;
pub mod lnk;
pub mod macho;
pub mod obj;
pub mod pcap;
pub mod plist;
pub mod prefetch;
pub mod registry;
pub mod sqlite;
pub mod stl;
pub mod x509;

pub use dwg::DWGParser;
pub use dxf::DXFParser;
pub use elf::ELFParser;
pub use evtx::EVTXParser;
pub use fits::FITSParser;
pub use gltf::GLTFParser;
pub use hdf5::HDF5Parser;
pub use lnk::LNKParser;
pub use macho::MachOParser;
pub use obj::OBJParser;
pub use pcap::PCAPParser;
pub use plist::PlistParser;
pub use prefetch::PrefetchParser;
pub use registry::RegistryParser;
pub use sqlite::SQLiteParser;
pub use stl::STLParser;
pub use x509::X509Parser;
