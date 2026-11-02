//! M3h6 real-map scale: source replay of every committed scenario in rooms of
//! classic-map size, the real-map admission (format bounds, the arithmetic and
//! slide-window certificates, starts as records), the intercept witness, the
//! hitscan buffer regression and the recoverable paths. Expected behavior comes
//! from the original source reading in M3H6_REFERENCE_INVESTIGATION.md, never
//! from Rust output.
use super::*;

// Source replay: every committed scenario against the compiled original rows,
// the independent DSIM33 captures and ordered events; step equals advance.
#[test]
fn real_map_source_case_00_town_patrol() {
    real_map_evidence::verify_case(0);
}
#[test]
fn real_map_source_case_01_town_fight() {
    real_map_evidence::verify_case(1);
}
#[test]
fn real_map_source_case_02_town_sight_far() {
    real_map_evidence::verify_case(2);
}
#[test]
fn real_map_source_case_03_town_slide_chamfer() {
    real_map_evidence::verify_case(3);
}
#[test]
fn real_map_source_case_04_town_window_shot() {
    real_map_evidence::verify_case(4);
}
#[test]
fn real_map_source_case_05_town_fireball() {
    real_map_evidence::verify_case(5);
}
#[test]
fn real_map_source_case_06_gallery_shot() {
    real_map_evidence::verify_case(6);
}
#[test]
fn real_map_source_case_07_gallery_aim_pickups() {
    real_map_evidence::verify_case(7);
}
#[test]
fn real_map_source_case_08_gallery_monster_shot() {
    real_map_evidence::verify_case(8);
}
#[test]
fn real_map_source_case_09_gallery_shotgun() {
    real_map_evidence::verify_case(9);
}
#[test]
fn real_map_source_case_10_extent_shot_long_wall() {
    real_map_evidence::verify_case(10);
}
#[test]
fn real_map_source_case_11_extent_sight_diagonal() {
    real_map_evidence::verify_case(11);
}
#[test]
fn real_map_source_case_12_extent_slide_long_wall() {
    real_map_evidence::verify_case(12);
}
#[test]
fn real_map_source_case_13_extent_fireball_far() {
    real_map_evidence::verify_case(13);
}
#[test]
fn real_map_source_case_14_dense_slide() {
    real_map_evidence::verify_case(14);
}
#[test]
fn real_map_source_case_15_dense_slide_steep() {
    real_map_evidence::verify_case(15);
}
#[test]
fn real_map_source_case_16_dense_shot() {
    real_map_evidence::verify_case(16);
}
#[test]
fn real_map_source_case_17_wide_pursuit() {
    real_map_evidence::verify_case(17);
}
#[test]
fn real_map_source_case_18_wide_sight() {
    real_map_evidence::verify_case(18);
}
#[test]
fn real_map_source_case_19_wide_shot_corridor() {
    real_map_evidence::verify_case(19);
}
