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
/// 13–16 `floors`, 17–18 `dense`, 19 `hall` and 20 `lift` (the between-tic chase
/// entries), 21 `pit`, exactly the committed bytes the C harness reads:
/// the builder names `SW1COMM` as the front middle texture of every 11/63 line
/// (§7 rule 9), which the harness resolves through its declared texture table.
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
    for index in 0..maps::MAP_BYTES.len() {
        let lumps = room(index);
        let sim = movers(&lumps).unwrap_or_else(|e| panic!("room {index}: {e:?}"));
        // The runtime line specials start as the map's.
        let map = lumps.map();
        // The public bounds are the admission's.
        let admission = sim.movers.as_ref().unwrap();
        assert_eq!(
            crate::mover_bounds(&map),
            Ok(crate::MoverBounds {
                reach: admission.reach,
                delta: admission.delta,
                movers: admission.movers,
            })
        );
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

/// Review finding 2: `P_FindHighestFloorSurrounding` folds from −500 (p_spec.c:297–316),
/// so in the `pit` room (map 21: turbo target T, sector 3, whose only neighbour N sits
/// at −508) the destination is −500 + 8 = −492, not −508 + 8. With T lowered to −510
/// the destination −492 is an 18-unit jump-up (Δ 18; the neighbour fold would give
/// −500 and Δ 10), and with T at −500 itself the destination equals the floor: no + 8,
/// no movement (the old rule, which required every neighbour floor to equal the own
/// floor, added 8).
#[test]
fn movers_fold_the_turbo_destination_from_minus_500() {
    let pit = movers(&room(21)).unwrap();
    let admission = pit.movers.as_ref().unwrap();
    assert_eq!(admission.reach, [-508, 128]);
    assert_eq!(admission.delta, 0);
    let mut lumps = room(21);
    set_heights(&mut lumps, 3, -510, 128);
    let bounds = crate::mover_bounds(&lumps.map()).unwrap();
    assert_eq!((bounds.reach, bounds.delta), ([-510, 128], 18));
    set_heights(&mut lumps, 3, -500, 128);
    let bounds = crate::mover_bounds(&lumps.map()).unwrap();
    assert_eq!((bounds.reach, bounds.delta), ([-508, 128], 8));
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
        set_front_texture(&mut lumps, line, 2, b"SW1COMM\0");
    }
    movers(&lumps).unwrap();
    set_special(&mut lumps, walls[15], 63);
    set_front_texture(&mut lumps, walls[15], 2, b"SW1COMM\0");
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

fn set_front_texture(lumps: &mut Lumps, line: usize, part: usize, name: &[u8; 8]) {
    let front = usize::from(line_field(lumps, line, 10));
    lump(lumps, "SIDEDEFS")[30 * front + 4 + 8 * part..30 * front + 12 + 8 * part]
        .copy_from_slice(name);
}

/// Rule 9 (decision N9a): the committed switchyard names `SW1COMM` as the front
/// middle texture of its S1 11 and SR 63 lines and is admitted; with those textures
/// put back to the builder's plain `TESTWALL` (sides `-`, `-`, `TESTWALL`) it is
/// rejected. An episode-1 name in any one of the three textures, in any ASCII
/// case and in either column (`SW1`, `SW2`), admits; an episode-2 name (absent from
/// the shareware `switchlist`), a near miss or a `-` name rejects with
/// `MapScaleError::SwitchTexture`, before any simulation exists. A DR line (1)
/// needs no switch texture (`EV_VerticalDoor` never calls `P_ChangeSwitchTexture`).
#[test]
fn movers_reject_a_switch_line_without_an_episode_one_switch_texture() {
    let switch = SimError::MapScale(MapScaleError::SwitchTexture);
    movers(&room(6)).unwrap();
    let plain = || {
        let mut lumps = room(6);
        for line in [1, 10] {
            set_front_texture(&mut lumps, line, 2, b"TESTWALL");
        }
        lumps
    };
    rejects(movers(&plain()), switch);
    // Lines 1 (SR 63) and 10 (S1 11) of the switchyard.
    for line in [1, 10] {
        let other = 11 - line;
        for (part, name, admitted) in [
            (0, b"SW1BRCOM", true),
            (1, b"sw2strtn", true),
            (2, b"Sw1Exit\0", true),
            (2, b"SW1BLUE\0", false),
            (2, b"SW1COMMX", false),
            (0, b"-SW1COMM", false),
            (0, b"SW1COM\0\0", false),
        ] {
            let mut lumps = plain();
            set_front_texture(&mut lumps, other, 2, b"SW1COMM\0");
            set_front_texture(&mut lumps, line, part, name);
            let map = lumps.map();
            let before = map.clone();
            let result = Simulation::original_player_movers_map(map.clone(), options());
            if admitted {
                assert!(result.is_ok(), "{line} {part} {name:?}");
            } else {
                rejects(result, switch);
            }
            assert_eq!(map, before);
        }
    }
    // The DR doors of the switchyard keep their plain textures and load.
    let lumps = room(6);
    let map = lumps.map();
    assert!(
        map.linedefs()
            .iter()
            .any(|l| l.special == mover_rules::LINE_DOOR_DR)
    );
}

/// `P_ChangeSwitchTexture` scans `switchlist` in slot order and tests top, middle,
/// bottom at each slot (p_switch.c:224–262): the first slot any texture matches
/// wins, so a later slot on the top loses to an earlier one on the bottom; the
/// partner of every episode-1 slot is in the same prefix, so a used switch stays
/// found. The C harness's switch (middle only) matches its middle texture.
#[test]
fn the_switch_match_follows_the_source_scan_order() {
    let name = |b: &[u8]| {
        let mut n = [0u8; 8];
        n[..b.len()].copy_from_slice(b);
        doom_types::LumpName::from_bytes(n)
    };
    let dash = name(b"-");
    // [upper, lower, middle].
    assert_eq!(
        mover_rules::switch_match(&[dash, dash, name(b"SW1COMM")]),
        Some((10, 1))
    );
    assert_eq!(
        mover_rules::switch_match(&[name(b"SW1STRTN"), name(b"SW1BRCOM"), dash]),
        Some((0, 2))
    );
    assert_eq!(
        mover_rules::switch_match(&[name(b"SW2BRCOM"), dash, name(b"SW1BRCOM")]),
        Some((0, 1))
    );
    assert_eq!(
        mover_rules::switch_match(&[name(b"SW1BRCOM"), dash, name(b"SW1BRCOM")]),
        Some((0, 0))
    );
    assert_eq!(
        mover_rules::switch_match(&[dash, dash, name(b"TESTWALL")]),
        None
    );
    for (slot, n) in mover_rules::EPISODE1_SWITCHES.iter().flatten().enumerate() {
        assert_eq!(
            mover_rules::switch_match(&[dash, dash, name(n)]),
            Some((slot, 1))
        );
    }
}

/// DSIM35 frame bytes with field `path` replaced by `value` (`None` removes it).
fn dsim35_with(bytes: &[u8], path: &str, value: Option<&[u8]>) -> Vec<u8> {
    let view = Diagnostic::decode_movers(bytes, DiagnosticLimits::default()).unwrap();
    let mut fields = Vec::new();
    let mut at = 16;
    while at < bytes.len() {
        let n = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        let name = std::str::from_utf8(&bytes[at + 4..at + 4 + n]).unwrap();
        at += 4 + n;
        let m = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()) as usize;
        let v = &bytes[at + 8..at + 8 + m];
        at += 8 + m;
        assert_eq!(view.field(name), Some(v));
        if name == path {
            if let Some(value) = value {
                fields.push((name, value.to_vec()));
            }
        } else {
            fields.push((name, v.to_vec()));
        }
    }
    let mut out = b"DSIM".to_vec();
    out.extend(35_u32.to_le_bytes());
    out.extend((fields.len() as u64).to_le_bytes());
    for (name, v) in fields {
        out.extend((name.len() as u32).to_le_bytes());
        out.extend(name.as_bytes());
        out.extend((v.len() as u64).to_le_bytes());
        out.extend(v);
    }
    out
}

/// Every earlier reader rejects 35, `decode_specials` (34) included, and the v35
/// reader rejects 34; a capture of the movers policy is version 35, policy 19.
#[test]
fn dsim35_readers_reject_other_versions_and_the_movers_capture_is_35() {
    let limits = DiagnosticLimits::default();
    let v35: &[u8] = include_bytes!("../../../tests/reference/m3h8_state_8_300.bin");
    let v34: &[u8] = include_bytes!("../../../tests/reference/m3h7_state_0_0.bin");
    type Reader =
        fn(&'static [u8], DiagnosticLimits) -> Result<Diagnostic<'static>, DiagnosticError>;
    let readers: [Reader; 15] = [
        Diagnostic::decode,
        Diagnostic::decode_v20,
        Diagnostic::decode_player,
        Diagnostic::decode_pistol,
        Diagnostic::decode_weapons,
        Diagnostic::decode_pickups,
        Diagnostic::decode_armor,
        Diagnostic::decode_bonuses,
        Diagnostic::decode_monsters,
        Diagnostic::decode_imps,
        Diagnostic::decode_map_scale,
        Diagnostic::decode_heights,
        Diagnostic::decode_diagonal,
        Diagnostic::decode_real_map,
        Diagnostic::decode_specials,
    ];
    for read in readers {
        assert_eq!(read(v35, limits).err(), Some(DiagnosticError::Version(35)));
    }
    assert!(Diagnostic::decode_specials(v34, limits).is_ok());
    assert_eq!(
        Diagnostic::decode_movers(v34, limits).err(),
        Some(DiagnosticError::Version(34))
    );
    let view = Diagnostic::decode_movers(v35, limits).unwrap();
    assert_eq!(view.version(), 35);
    assert_eq!(view.field("profile"), Some(19_u32.to_le_bytes().as_slice()));
    let sim = movers(&room(0)).unwrap();
    let snap = sim.snapshot();
    assert_eq!(snap.version(), 35);
    let bytes = snap.diagnostic_bytes(limits).unwrap();
    assert_eq!(
        Diagnostic::decode_movers(&bytes, limits)
            .unwrap()
            .encode(limits)
            .unwrap(),
        bytes
    );
}

/// The v35 reader checks the policy identity and the DSIM35 fields: a frame naming
/// another policy or context, a button outside its domain or on a non-SR line, a
/// runtime special a line cannot take, a missing button slot or line special,
/// and non-boolean `use_down`/`level.exit` are rejected.
#[test]
fn dsim35_reader_rejects_policy_and_field_violations() {
    let limits = DiagnosticLimits::default();
    let v35: &[u8] = include_bytes!("../../../tests/reference/m3h8_state_8_300.bin");
    let reject = |path: &str, value: Option<&[u8]>| {
        Diagnostic::decode_movers(&dsim35_with(v35, path, value), limits).err()
    };
    assert!(Diagnostic::decode_movers(&dsim35_with(v35, "tic", None), limits).is_err());
    assert_eq!(
        reject("profile", Some(&18_u32.to_le_bytes())),
        Some(DiagnosticError::Schema)
    );
    assert_eq!(
        reject(
            "context.schema",
            Some(b"PLAYER-MONSTERS-SPECIALS-1-MEDIUM-v1")
        ),
        Some(DiagnosticError::Schema)
    );
    // Slot 0 runs on SR line 1 in this capture.
    let view = Diagnostic::decode_movers(v35, limits).unwrap();
    assert_eq!(
        view.field("button.0.line"),
        Some(1_u16.to_le_bytes().as_slice())
    );
    assert_eq!(
        reject("button.0.btimer", Some(&36_i32.to_le_bytes())),
        Some(DiagnosticError::Identity)
    );
    assert_eq!(
        reject("button.0.line", Some(&10_u16.to_le_bytes())),
        Some(DiagnosticError::Identity)
    );
    assert_eq!(
        reject("button.0.soundorg", Some(&3_u16.to_le_bytes())),
        Some(DiagnosticError::Identity)
    );
    assert_eq!(
        reject("button.0.btimer", Some(&0_i32.to_le_bytes())),
        Some(DiagnosticError::Structure)
    );
    assert_eq!(
        reject("button.15.btimer", None),
        Some(DiagnosticError::Structure)
    );
    // Line 1 is SR 63 (never cleared); line 10 is the S1 exit (may read 0).
    assert_eq!(
        reject("line.1.special", Some(&0_i16.to_le_bytes())),
        Some(DiagnosticError::Identity)
    );
    assert!(
        Diagnostic::decode_movers(
            &dsim35_with(v35, "line.10.special", Some(&0_i16.to_le_bytes())),
            limits
        )
        .is_ok()
    );
    assert_eq!(
        reject("line.10.special", None),
        Some(DiagnosticError::Structure)
    );
    assert_eq!(
        reject("original_player.0.use_down", Some(&[2])),
        Some(DiagnosticError::Structure)
    );
    assert_eq!(
        reject("level.exit", Some(&[2])),
        Some(DiagnosticError::Structure)
    );
}

/// Loads `map` under the movers policy, expecting `expected`, and asserts that the
/// refused load left its input untouched (the admission reads the map only).
fn rejects_map(map: doom_map::Map, expected: SimError) {
    let before = map.clone();
    rejects(
        Simulation::original_player_movers_map(map.clone(), options()),
        expected,
    );
    assert_eq!(map, before);
}
/// The first one-sided linedef of `lumps`.
fn one_sided_line(lumps: &mut Lumps) -> usize {
    (0..lumps.line_count())
        .find(|&l| line_field(lumps, l, 12) == u16::MAX)
        .unwrap()
}
/// Whether `special` is one of the seven admitted tag/use/cross types (§7 rule 1).
fn admitted_type(special: i16) -> bool {
    use mover_rules::*;
    [
        LINE_DOOR_DR,
        LINE_DOOR_SR,
        LINE_DOOR_BLAZE,
        LINE_LIFT,
        LINE_FLOOR_TURBO,
        LINE_FLOOR_LOWEST,
        LINE_EXIT,
    ]
    .contains(&special)
}

/// §7 rule 1 by family (M3h8 note §15, deferred types): each family's original
/// line types (p_spec.c `P_CrossSpecialLine`, `P_ShootSpecialLine`, p_switch.c
/// `P_UseSpecialLine`) put on a two-sided DR line of `doorway` and on a one-sided
/// wall are rejected with `PursuitError::Specials`, the map untouched; the same
/// room with the line unchanged loads. Then every other value of the field in
/// `-1..=300` and both `i16` extremes, on both sides, except the admitted types and
/// a one-sided 48 (Stage A).
#[test]
fn movers_reject_every_deferred_line_family_on_either_side() {
    let families: [(&str, &[i16]); 14] = [
        ("crushers", &[6, 25, 49, 55, 56, 57, 73, 74, 77, 141]),
        (
            "locked doors",
            &[26, 27, 28, 32, 33, 34, 99, 133, 134, 135, 136, 137],
        ),
        (
            "other doors",
            &[
                2, 3, 4, 16, 29, 31, 42, 50, 61, 75, 76, 86, 90, 103, 105, 106, 107, 108, 110, 111,
                112, 113, 114, 115, 116, 117, 118,
            ],
        ),
        ("teleports", &[39, 97, 125, 126]),
        ("stairs", &[7, 8, 100, 127]),
        ("donuts", &[9]),
        ("raiseToTexture", &[30, 96]),
        ("perpetual and stopping plats", &[53, 54, 87, 89]),
        (
            "other plats",
            &[
                10, 14, 15, 20, 21, 22, 47, 62, 66, 67, 68, 95, 120, 121, 122, 123,
            ],
        ),
        (
            "other floors",
            &[
                5, 18, 19, 23, 37, 45, 58, 59, 60, 64, 65, 69, 70, 71, 82, 83, 84, 91, 92, 93, 94,
                98, 101, 102, 119, 128, 129, 130, 131, 132, 140,
            ],
        ),
        ("ceilings", &[40, 41, 43, 44, 72]),
        ("light lines", &[12, 13, 17, 35, 79, 80, 81, 104, 138, 139]),
        ("gun lines", &[24, 46, 47]),
        ("other exits", &[51, 52, 124]),
    ];
    let mut lumps = room(0);
    let wall = one_sided_line(&mut lumps);
    movers(&lumps).unwrap();
    let specials = SimError::Pursuit(PursuitError::Specials);
    let check = |special: i16| {
        for line in [2, wall] {
            let mut lumps = room(0);
            set_special(&mut lumps, line, special);
            if special == 48 && line == wall {
                // A one-sided scroller stays admitted (Stage A).
                movers(&lumps).unwrap();
            } else {
                rejects_map(lumps.map(), specials);
            }
        }
    };
    for (family, types) in families {
        for &special in types {
            assert!(!admitted_type(special), "{family} {special}");
            check(special);
        }
    }
    for special in (-1..=300).chain([i16::MIN, i16::MAX]) {
        if special != 0 && !admitted_type(special) {
            check(special);
        }
    }
}

/// Decision N1 (iii), provisional: a two-sided scroller (48) stays rejected under
/// the movers policy whether or not the line blocks, while the same special on a
/// one-sided wall loads.
#[test]
fn movers_keep_rejecting_a_two_sided_scroller() {
    for flags in [None, Some(1)] {
        let mut lumps = room(0);
        set_special(&mut lumps, 2, 48);
        if let Some(f) = flags {
            let old = line_field(&mut lumps, 2, 4);
            set_line_field(&mut lumps, 2, 4, old | f);
        }
        rejects_map(lumps.map(), SimError::Pursuit(PursuitError::Specials));
    }
    let mut lumps = room(0);
    let wall = one_sided_line(&mut lumps);
    set_special(&mut lumps, wall, 48);
    movers(&lumps).unwrap();
}

/// Rule 4, the exact bound: a reachable height of 513 (the turbo destination of
/// `floors` sector 5 beside a neighbour floor of 505) is rejected; 512 loads (the
/// `movers_reject_reachable_heights_beyond_512` control).
#[test]
fn movers_reject_a_reachable_height_of_513() {
    let mut lumps = room(13);
    set_heights(&mut lumps, 6, 505, 512);
    rejects_map(lumps.map(), SimError::MapScale(MapScaleError::MoverHeights));
}

/// Rule 3 on a synthetic one-sector room (the M3h4 strip builder): a sector tagged
/// by a one-sided 109, 36, 38 or 88 line has no `getNextSector` neighbour (no
/// two-sided line) and is rejected; the same line in the two-sector strip, whose
/// portal gives the target a neighbour, loads.
#[test]
fn movers_reject_a_mover_without_a_two_sided_neighbour() {
    use crate::heights_room::{Fault, HeightsRoom};
    let one = HeightsRoom::new(&[-512, 512], &[[0, 128]]);
    let two = HeightsRoom::new(&[-512, 0, 512], &[[0, 128], [0, 128]]);
    let load = |room: &HeightsRoom| Simulation::original_player_movers_map(room.map(), options());
    load(&one).unwrap();
    for special in [109, 36, 38, 88] {
        let faults = [Fault::TaggedSpecial(1, special, 1), Fault::SectorTag(0, 1)];
        rejects_map(
            one.clone_with(&faults).map(),
            SimError::MapScale(MapScaleError::MoverNeighbor),
        );
        let sim = load(&two.clone_with(&faults)).unwrap();
        assert_eq!(sim.movers.as_ref().unwrap().movers, 1, "{special}");
    }
}

/// Rule 4c on a synthetic strip: a two-sided line with the door sector on both
/// sides makes the door its own neighbour (its closed top would be `floor − 4`) and
/// is rejected; the same line in the other, unmoved sector, or in a floor target
/// (rule 4c names doors only), loads.
#[test]
fn movers_reject_a_self_referencing_door_line() {
    use crate::heights_room::{Fault, HeightsRoom};
    let two = HeightsRoom::new(&[-512, 0, 512], &[[0, 128], [0, 128]]);
    let load = |room: &HeightsRoom| Simulation::original_player_movers_map(room.map(), options());
    let door = [Fault::TaggedSpecial(1, 109, 1), Fault::SectorTag(0, 1)];
    load(&two.clone_with(&door)).unwrap();
    let mut faults = door.to_vec();
    faults.push(Fault::SelfReferencing(0));
    rejects_map(
        two.clone_with(&faults).map(),
        SimError::MapScale(MapScaleError::SelfReferencingDoor),
    );
    faults[2] = Fault::SelfReferencing(1);
    load(&two.clone_with(&faults)).unwrap();
    let floor = [
        Fault::TaggedSpecial(1, 38, 1),
        Fault::SectorTag(0, 1),
        Fault::SelfReferencing(0),
    ];
    load(&two.clone_with(&floor)).unwrap();
}

/// Rule 5, `MAXPLATS`: a synthetic strip of 32 sectors whose first 30 or 31 carry
/// the tag of a one-sided WR 88 line; 30 plat sectors load, 31 are rejected.
#[test]
fn movers_reject_thirty_one_plat_sectors() {
    use crate::heights_room::{Fault, HeightsRoom};
    let spans: Vec<i16> = (0..=32).map(|k| -512 + 32 * k).collect();
    let strip = HeightsRoom::new(&spans, &[[0, 128]; 32]);
    let tagged = |n: usize| {
        let mut faults = vec![Fault::TaggedSpecial(1, 88, 1)];
        faults.extend((0..n).map(|s| Fault::SectorTag(s, 1)));
        strip.clone_with(&faults)
    };
    let sim = Simulation::original_player_movers_map(tagged(30).map(), options()).unwrap();
    assert_eq!(sim.movers.as_ref().unwrap().movers, 30);
    rejects_map(
        tagged(31).map(),
        SimError::MapScale(MapScaleError::MoverCount),
    );
}

/// Rule 5, `MAXLINEANIMS` (Stage A, kept by the movers policy): a synthetic room
/// whose walls are cut into 32-unit pieces; 64 one-sided scrollers load, 65 are
/// rejected with `PursuitError::Specials`.
#[test]
fn movers_reject_sixty_five_scrollers() {
    use crate::heights_room::{Fault, HeightsRoom};
    let mut walls = HeightsRoom::new(&[-512, 512], &[[0, 128]]);
    walls.segment = 32;
    let scrollers = |n: usize| {
        let faults: Vec<_> = (0..n).map(|l| Fault::TaggedSpecial(l, 48, 0)).collect();
        walls.clone_with(&faults)
    };
    let map = scrollers(64).map();
    assert!(map.linedefs().len() >= 65);
    assert_eq!(
        map.linedefs().iter().filter(|l| l.special == 48).count(),
        64
    );
    Simulation::original_player_movers_map(map, options()).unwrap();
    rejects_map(
        scrollers(65).map(),
        SimError::Pursuit(PursuitError::Specials),
    );
}
