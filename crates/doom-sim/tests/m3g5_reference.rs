//! New compiled original-C player roster/Look/Chase observations and independent bytes.
#[path = "../../../tests/support/m3g5.rs"]
mod source;
#[test]
fn real_player_demon_acquisition_pursuit_and_full_events_match_original_c() {
    let _ = source::verify();
}
