//! M3h4 two-sided lines, static heights and vertical movement: source replay of
//! every committed scenario, the heights admission, the vertical invariant and the
//! recoverable paths. Expected behavior comes from the original source reading in
//! M3H4_REFERENCE_INVESTIGATION.md, never from Rust output.
use super::*;

// Source replay: every committed scenario against the compiled original rows,
// the independent DSIM31 captures and ordered events; step equals advance.
#[test]
fn heights_source_case_00_stairs_up() {
    heights_evidence::verify_case(0);
}
#[test]
fn heights_source_case_01_stairs_down() {
    heights_evidence::verify_case(1);
}
#[test]
fn heights_source_case_02_too_high_step() {
    heights_evidence::verify_case(2);
}
#[test]
fn heights_source_case_03_imp_pursuit_stairs() {
    heights_evidence::verify_case(3);
}
#[test]
fn heights_source_case_04_demon_pursuit_stairs() {
    heights_evidence::verify_case(4);
}
#[test]
fn heights_source_case_05_monster_refuses_riser() {
    heights_evidence::verify_case(5);
}
#[test]
fn heights_source_case_06_autoaim_up_step() {
    heights_evidence::verify_case(6);
}
#[test]
fn heights_source_case_07_autoaim_down_step() {
    heights_evidence::verify_case(7);
}
#[test]
fn heights_source_case_08_drop_landing_oof() {
    heights_evidence::verify_case(8);
}
#[test]
fn heights_source_case_09_monster_refuses_ledge() {
    heights_evidence::verify_case(9);
}
#[test]
fn heights_source_case_10_corpse_pushed_off_ledge() {
    heights_evidence::verify_case(10);
}
#[test]
fn heights_source_case_11_fall_forward_from_below() {
    heights_evidence::verify_case(11);
}
#[test]
fn heights_source_case_12_fireball_down_ledge_floor() {
    heights_evidence::verify_case(12);
}
#[test]
fn heights_source_case_13_window_sight_pit_then_step() {
    heights_evidence::verify_case(13);
}
#[test]
fn heights_source_case_14_window_lower_wall_puff() {
    heights_evidence::verify_case(14);
}
#[test]
fn heights_source_case_15_window_upper_wall_puff() {
    heights_evidence::verify_case(15);
}
#[test]
fn heights_source_case_16_noise_through_window() {
    heights_evidence::verify_case(16);
}
#[test]
fn heights_source_case_17_noise_through_soundblock() {
    heights_evidence::verify_case(17);
}
#[test]
fn heights_source_case_18_reject_bit() {
    heights_evidence::verify_case(18);
}
#[test]
fn heights_source_case_19_closed_opening_blocks_all() {
    heights_evidence::verify_case(19);
}
#[test]
fn heights_source_case_20_fence_slide() {
    heights_evidence::verify_case(20);
}
#[test]
fn heights_source_case_21_fence_blocks_monsters_only() {
    heights_evidence::verify_case(21);
}
#[test]
fn heights_source_case_22_crawlspace_refusal() {
    heights_evidence::verify_case(22);
}
#[test]
fn heights_source_case_23_fireball_up_ceiling() {
    heights_evidence::verify_case(23);
}
