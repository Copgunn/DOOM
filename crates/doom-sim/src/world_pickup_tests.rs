use super::*;
#[test]
fn independent_original_world_ammo() {
    assert_ne!(pickup_evidence::verify_parts(0), 0);
}

#[test]
fn independent_original_world_health() {
    assert_ne!(pickup_evidence::verify_parts(1), 0);
}
