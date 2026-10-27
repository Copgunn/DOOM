//! Classic binary map vocabulary for the Rust rewrite.
//!
//! M0 provides only a vertex value type. Map decoding, reference validation,
//! BSP interpretation, and map loading are M1 work.

/// A classic map vertex in signed map units, before fixed-point conversion.
///
/// This is a value representation, not a packed on-disk structure. Decode
/// little-endian fields explicitly rather than casting input bytes to structs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Vertex {
    /// Horizontal map coordinate.
    pub x: i16,
    /// Vertical map coordinate.
    pub y: i16,
}
