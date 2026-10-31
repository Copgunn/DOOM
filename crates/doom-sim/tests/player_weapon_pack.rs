//! Source-derived canonical four-weapon integration.
#[path = "../../../tests/support/m3g8.rs"]
mod source;
#[test]
fn core_fallback_fist_switching_match_independent_original_c() {
    source::verify();
}

#[test]
fn shotgun_current_sequential_pellets_match_independent_original_c() {
    source::verify_parts(1);
}

#[test]
fn all_four_weapons_and_integrated_combat_match_independent_original_c() {
    source::verify_parts(3);
}
