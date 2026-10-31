//! Independent original-source pistol encounter comparisons.
#[path = "../../../tests/support/m3g7.rs"]
mod source;
#[test]
fn original_pistol_and_two_way_encounter_match_independent_c() {
    source::verify();
}
