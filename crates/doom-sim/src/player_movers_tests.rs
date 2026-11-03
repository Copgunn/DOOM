//! M3h8 doors, lifts, floors, switches and the exit: the movers admission and its
//! rejections, the policy identity and the sector-thinker budget. Expected behavior
//! comes from the original source reading in M3H8_REFERENCE_INVESTIGATION.md and the
//! committed C evidence, never from Rust output.
use super::*;
use crate::diagonal_room::Lumps;

mod maps {
    include!("../../../tests/reference/m3h8_maps.rs");
}
const DENSE9: &[u8] = include_bytes!("../../../tests/reference/m3h8_dense9.bin");

pub(crate) fn options() -> MapContextOptions {
    MapContextOptions {
        initial_rng: DoomRng::from_indices(6, 0),
        respawn_monsters: false,
    }
}
/// A committed M3h8 room: 0–3 `doorway`, 4–5 `hall`, 6 `switchyard`, 7–12 `lift`,
/// 13–16 `floors`, 17–18 `dense`.
pub(crate) fn room(index: usize) -> Lumps {
    Lumps::parse(maps::MAP_BYTES[index])
}
fn lump<'a>(lumps: &'a mut Lumps, name: &str) -> &'a mut Vec<u8> {
    &mut lumps.lumps.iter_mut().find(|(n, _)| n == name).unwrap().1
}
fn set_line_field(lumps: &mut Lumps, line: usize, offset: usize, value: u16) {
    let l = lump(lumps, "LINEDEFS");
    l[14 * line + offset..14 * line + offset + 2].copy_from_slice(&value.to_le_bytes());
}
fn line_field(lumps: &mut Lumps, line: usize, offset: usize) -> u16 {
    let l = lump(lumps, "LINEDEFS");
    u16::from_le_bytes([l[14 * line + offset], l[14 * line + offset + 1]])
}
fn set_special(lumps: &mut Lumps, line: usize, special: i16) {
    set_line_field(lumps, line, 6, special as u16);
}
fn set_tag(lumps: &mut Lumps, line: usize, tag: i16) {
    set_line_field(lumps, line, 8, tag as u16);
}
fn set_heights(lumps: &mut Lumps, sector: usize, floor: i16, ceiling: i16) {
    let s = lump(lumps, "SECTORS");
    s[26 * sector..26 * sector + 2].copy_from_slice(&floor.to_le_bytes());
    s[26 * sector + 2..26 * sector + 4].copy_from_slice(&ceiling.to_le_bytes());
}
fn set_ceiling_flat(lumps: &mut Lumps, sector: usize, name: &[u8; 8]) {
    lump(lumps, "SECTORS")[26 * sector + 12..26 * sector + 20].copy_from_slice(name);
}
pub(crate) fn movers(lumps: &Lumps) -> Result<Simulation, OriginalRegistryError> {
    Simulation::original_player_movers_map(lumps.map(), options())
}
fn rejects(result: Result<Simulation, OriginalRegistryError>, expected: SimError) {
    match result {
        Err(OriginalRegistryError::Runtime(error)) => assert_eq!(error, expected),
        Err(other) => panic!("unexpected error {other:?}, expected {expected:?}"),
        Ok(_) => panic!("admitted, expected {expected:?}"),
    }
}

#[test]
fn movers_policy_identity_is_wire_19_with_its_own_context() {
    let p = OriginalExecutionProfile::PlayerMonstersMoversV1;
    assert_eq!(p.wire_id(), 19);
    assert_eq!(p.context_schema(), PLAYER_MOVERS_CONTEXT_SCHEMA);
    assert_eq!(
        PLAYER_MOVERS_CONTEXT_SCHEMA,
        b"PLAYER-MONSTERS-MOVERS-1-MEDIUM-v1"
    );
    // A superset of the specials policy; the older policy gains nothing.
    assert!(p.has_specials() && p.has_real_map() && p.has_movers());
    let older = OriginalExecutionProfile::PlayerMonstersSpecialsV1;
    assert!(!older.has_movers());
    assert_eq!(older.wire_id(), 18);
    let sim = movers(&room(0)).unwrap();
    assert_eq!(sim.original_execution_profile(), Some(p));
}

#[test]
fn movers_admit_every_committed_room_and_the_specials_policy_rejects_them() {
    for (index, bytes) in maps::MAP_BYTES.iter().enumerate() {
        let lumps = Lumps::parse(bytes);
        let sim = movers(&lumps).unwrap_or_else(|e| panic!("room {index}: {e:?}"));
        // The runtime line specials start as the map's.
        let map = lumps.map();
        assert_eq!(
            sim.line_specials,
            map.linedefs().iter().map(|l| l.special).collect::<Vec<_>>()
        );
        rejects(
            Simulation::original_player_specials_map(map, options()),
            SimError::Pursuit(PursuitError::Specials),
        );
    }
}

#[test]
fn movers_admit_the_older_committed_rooms_as_a_superset() {
    for c in crate::specials_evidence::cases() {
        let sim = Simulation::original_player_movers_map(
            crate::specials_evidence::case_map(c.map),
            options(),
        )
        .unwrap_or_else(|e| panic!("{}: {e:?}", c.label));
        // No mover sector: the slot class is the light sectors alone.
        assert_eq!(sim.movers.as_ref().unwrap().movers, 0);
        assert!(sim.line_specials.iter().all(|s| matches!(s, 0 | 48)));
    }
}

/// Sector-thinker slots `L + 2M` and visits `320 + L + 2M` (decision N8).
#[test]
fn movers_bound_the_sector_thinker_slots_by_lights_plus_twice_the_movers() {
    // doorway: door sectors 2, 3, 4 (DR back sectors), no light sector.
    let sim = movers(&room(0)).unwrap();
    assert_eq!(sim.movers.as_ref().unwrap().movers, 3);
    assert_eq!(sim.actors.sector_capacity(), 6);
    assert_eq!(sim.actors.visit_bound(), 326);
    // floors: 36 → 5 and 7, 38 → 9, 109 and the DR line share door sector 11.
    let sim = movers(&room(13)).unwrap();
    let admission = sim.movers.as_ref().unwrap();
    assert_eq!(admission.movers, 4);
    assert_eq!(sim.actors.sector_capacity(), 8);
    assert_eq!(sim.actors.visit_bound(), 328);
    use crate::mover_rules::Family::*;
    assert_eq!(admission.family[5], Some(Floor));
    assert_eq!(admission.family[7], Some(Floor));
    assert_eq!(admission.family[9], Some(Floor));
    assert_eq!(admission.family[11], Some(Door));
    assert_eq!(admission.family[0], None);
    // lift: both 88 lines tag plat sector 3; dense: all eight tag plat sector 9.
    assert_eq!(movers(&room(7)).unwrap().actors.sector_capacity(), 2);
    assert_eq!(movers(&room(17)).unwrap().actors.sector_capacity(), 2);
}

/// The §5 fixpoint: the turbo jump-up of `floors` (36 beside a floor 32 units
/// higher: 64 + 8 over 32) and the lift's 4-unit rise give `Δ`, and the reachable
/// range covers the jump-up destination.
#[test]
fn movers_reachable_heights_and_delta_follow_the_fixpoint() {
    let sim = movers(&room(13)).unwrap();
    let admission = sim.movers.as_ref().unwrap();
    // Sector 5 (floor 32) has neighbours 0 (floor 0) and 6 (floor 64): the turbo
    // destination 72 is a 40-unit jump-up, the largest one-tic rise.
    assert_eq!(admission.delta, 40);
    assert_eq!(admission.reach, [0, 192]);
    let lift = movers(&room(7)).unwrap();
    assert_eq!(lift.movers.as_ref().unwrap().delta, 4);
    let door = movers(&room(0)).unwrap();
    assert_eq!(door.movers.as_ref().unwrap().delta, 2);
    assert_eq!(door.movers.as_ref().unwrap().reach, [0, 128]);
}

#[test]
fn movers_reject_every_unadmitted_line_special_and_a_two_sided_scroller() {
    // Line 2 of `doorway` is a two-sided DR line; any other door, lift, floor,
    // crusher, teleport or exit type, and a two-sided 48 (decision N1 iii), is
    // rejected at load.
    for special in [2, 6, 26, 31, 39, 46, 48, 51, 53, 62, 97, 117, 124, 141] {
        let mut lumps = room(0);
        set_special(&mut lumps, 2, special);
        rejects(movers(&lumps), SimError::Pursuit(PursuitError::Specials));
    }
}

#[test]
fn movers_reject_a_one_sided_door_line() {
    // `switchyard` line 1 is the one-sided SR switch; as a DR line its
    // EV_VerticalDoor would read sides[-1].
    let mut lumps = room(6);
    set_special(&mut lumps, 1, 1);
    rejects(movers(&lumps), SimError::MapScale(MapScaleError::DoorLine));
}

#[test]
fn movers_reject_a_sector_in_two_families() {
    // `floors` line 4 (W1 36) retagged onto the blazeOpen door sector 11.
    let mut lumps = room(13);
    set_tag(&mut lumps, 4, 5);
    rejects(
        movers(&lumps),
        SimError::MapScale(MapScaleError::MoverFamily),
    );
    // Door kinds sharing a sector stay admitted (decision N5): a DR line's back
    // sector targeted by the 109 line already is (sector 11).
    movers(&room(13)).unwrap();
}

#[test]
fn movers_reject_reachable_heights_beyond_512() {
    // The turbo target 5's neighbour 6 at floor 506: the destination 514 leaves
    // ±512 although every static height is inside it.
    let mut lumps = room(13);
    set_heights(&mut lumps, 6, 506, 512);
    rejects(
        movers(&lumps),
        SimError::MapScale(MapScaleError::MoverHeights),
    );
    // At floor 504 the destination is 512, still admitted.
    let mut lumps = room(13);
    set_heights(&mut lumps, 6, 504, 512);
    let sim = movers(&lumps).unwrap();
    assert_eq!(sim.movers.as_ref().unwrap().reach[1], 512);
}

#[test]
fn movers_reject_an_inverting_door() {
    // Door sector 3 (floor 32) beside room 0 lowered to ceiling 20: its smallest
    // possible top, 16, lies below its floor.
    let mut lumps = room(0);
    set_heights(&mut lumps, 0, 0, 20);
    rejects(
        movers(&lumps),
        SimError::MapScale(MapScaleError::InvertingDoor),
    );
}

#[test]
fn movers_reject_seventeen_reusable_switches() {
    // `switchyard` one-sided walls become SR switches (tag 1, the door sector 4):
    // 16 are admitted, 17 exceed MAXBUTTONS.
    let walls = [0, 2, 3, 5, 6, 8, 9, 11, 12, 14, 16, 18, 19, 20, 21, 22, 23];
    let one_sided = |lumps: &mut Lumps, line: usize| line_field(lumps, line, 12) == u16::MAX;
    let mut lumps = room(6);
    let walls: Vec<usize> = walls
        .into_iter()
        .filter(|&l| one_sided(&mut lumps, l))
        .collect();
    assert!(walls.len() >= 16, "{walls:?}");
    // Line 1 is already an SR switch.
    for &line in &walls[..15] {
        set_special(&mut lumps, line, 63);
        set_tag(&mut lumps, line, 1);
    }
    movers(&lumps).unwrap();
    set_special(&mut lumps, walls[15], 63);
    rejects(
        movers(&lumps),
        SimError::MapScale(MapScaleError::MoverCount),
    );
}

#[test]
fn movers_reject_nine_special_lines_in_one_window() {
    // The Rust-only `dense9` lane: nine two-sided special lines in one 2 × 2
    // window; the committed `dense` room holds exactly eight.
    let lumps = Lumps::parse(DENSE9);
    assert_eq!(mover_rules::special_window_lines(&lumps.map()), 9);
    rejects(
        movers(&lumps),
        SimError::MapScale(MapScaleError::SpecialWindow),
    );
    assert_eq!(mover_rules::special_window_lines(&room(17).map()), 8);
}

#[test]
fn movers_reject_a_sky_ceiling_over_a_mover() {
    // Decision N4: a door sector with an F_SKY1 ceiling; a sky ceiling elsewhere
    // stays admitted (Stage A).
    let mut lumps = room(0);
    set_ceiling_flat(&mut lumps, 2, b"F_SKY1\0\0");
    rejects(movers(&lumps), SimError::MapScale(MapScaleError::SkyMover));
    let mut lumps = room(0);
    set_ceiling_flat(&mut lumps, 0, b"f_sky1\0\0");
    movers(&lumps).unwrap();
}

/// Tag 0 is faithful: a tag-0 tagged line acts on every untagged sector, so a
/// tag-0 lift in `doorway` makes every untagged sector a plat, and the door
/// sectors then belong to two families.
#[test]
fn movers_evaluate_tag_zero_on_every_untagged_sector() {
    let mut lumps = room(6);
    // `switchyard` line 10 is the exit; a tag-0 lift on a one-sided wall targets
    // every untagged sector, including DR door sector 3.
    set_special(&mut lumps, 0, 88);
    set_tag(&mut lumps, 0, 0);
    rejects(
        movers(&lumps),
        SimError::MapScale(MapScaleError::MoverFamily),
    );
}
