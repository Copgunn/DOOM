//! Pins the committed BLOCKMAP corner WAD bytes.

use doom_test_evidence::sha256;

#[test]
fn blockmap_corner_fixture_bytes() {
    let bytes = include_bytes!("../../../tests/reference/blockmap_corner.wad");

    assert_eq!(bytes.len(), 46_426);
    assert_eq!(
        sha256(bytes),
        "d464b2d2e58be8bccee75d4b9637684c94c7531c02fb8b4d7a1f59f8fe58cdc6"
    );
}
