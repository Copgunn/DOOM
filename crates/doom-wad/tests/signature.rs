//! Signature boundary tests, not archive-parsing coverage.

mod common;

use doom_wad::WadKind;

#[test]
fn recognizes_both_signatures_from_synthetic_archives() {
    for (magic, expected) in [(*b"IWAD", WadKind::Iwad), (*b"PWAD", WadKind::Pwad)] {
        let bytes = common::empty_wad(magic);
        assert_eq!(
            WadKind::from_magic(bytes[..4].try_into().unwrap()),
            Some(expected)
        );
    }
}

#[test]
fn rejects_malformed_signatures() {
    // Exact magic is required; case folding is a lump lookup concern.
    for magic in [*b"iwad", *b"PWAX", *b"WAD\0", [0; 4], [255; 4]] {
        let bytes = common::empty_wad(magic);
        assert_eq!(WadKind::from_magic(bytes[..4].try_into().unwrap()), None);
    }
}
