use super::*;
#[test]
fn independent_original_armor_actors() {
    assert_ne!(armor_evidence::verify_range(0..8), 0);
}
