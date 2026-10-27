//! Immutable classic DOOM maps decoded from bounded [`doom_wad`] archives.
//!
//! Validates structural references, BSP acyclicity, REJECT coverage and BLOCKMAP
//! lists. Does not spawn actors, resolve textures, or prove geometric correctness
//! of a node builder. Unsupported extended formats are rejected.

mod error;
mod parse;
mod types;

pub use error::{ErrorKind, MapError};
pub use parse::{Map, MapLimits, is_map_marker};
pub use types::{
    Blockmap, BspChild, LineDef, Node, Sector, Seg, SideDef, Subsector, Thing, Vertex,
};
