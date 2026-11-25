//! Kills finale_melt.rs:701:28 (`palette > 13` to `>=` in `SessionDisplayStateV4::decode`), which
//! the record had listed as killed by a test of the line above it (the steps-shown bound).
use super::SessionDisplayStateV4;
use super::reader_tests::{MELT, melting};

/// The melt's palette byte is 0..=13 (the 14 palettes of PLAYPAL): 13 decodes, 14 is refused.
#[test]
fn decode_takes_palette_13_and_refuses_14() {
    let mut b = melting(0).encode();
    assert_eq!(b[MELT], 1, "melt tag");
    b[MELT + 2] = 13;
    assert!(SessionDisplayStateV4::decode(&b).is_ok());
    b[MELT + 2] = 14;
    let refused = Err(crate::StateDecodeError::Invalid("melt palette"));
    assert_eq!(SessionDisplayStateV4::decode(&b), refused);
}
