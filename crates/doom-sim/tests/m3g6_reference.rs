//! Original compiled-C fields/events and independently encoded DSIM22 policy6.
#[path = "../../../tests/support/m3g6.rs"]
mod source;
#[test]
fn autonomous_player_demon_melee_matches_independent_source_and_encoder() {
    source::verify();
}
