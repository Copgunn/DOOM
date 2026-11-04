//! M3h9 E1M1 policy: its identity, the superset relation to the movers policy and
//! the DSIM36 version plumbing. Expected behavior comes from the original source
//! reading in M3H9_REFERENCE_INVESTIGATION.md and the committed C evidence, never
//! from Rust output.
use super::*;
use crate::diagonal_room::Lumps;
use crate::identity::OriginalState;
use crate::player_movers_tests::{options, room as m3h8_room};

mod maps {
    include!("../../../tests/reference/m3h9_maps.rs");
}

/// A committed M3h9 room (`tests/reference/m3h9_cases.csv`, column `map`): 0–2
/// `scrollhall` (empty, a Zombieman on the ledge, an Imp in the strip), 3
/// `scrollblock`, 4 `dense8`, 5–9 `gallery` (the ten decorations alone, with a
/// Zombieman at (−240, −64), at (−536, −64), an Imp at (256, 0), at (256, −192)),
/// 10 `liftroom` (a lamp and a dead player on the lift), 11 `liftroom` (a pillar
/// under the door), 12 `census`.
pub(crate) fn room(index: usize) -> Lumps {
    Lumps::parse(maps::MAP_BYTES[index])
}
pub(crate) fn e1m1(lumps: &Lumps) -> Result<Simulation, OriginalRegistryError> {
    Simulation::original_player_e1m1_map(lumps.map(), options())
}

/// The thirteen committed M3h9 rooms parse; the census room (no line special, one
/// sector-1 light) loads under the E1M1 policy with a DSIM36 capture.
#[test]
fn committed_m3h9_rooms_parse_and_the_census_room_loads() {
    assert_eq!(maps::MAP_BYTES.len(), 13);
    for index in 0..maps::MAP_BYTES.len() {
        let lumps = room(index);
        assert!(lumps.try_map().is_ok(), "room {index}");
    }
    let sim = e1m1(&room(12)).unwrap();
    assert_eq!(sim.snapshot().version(), 36);
    assert_eq!(sim.movers.as_ref().unwrap().movers, 0);
}

#[test]
fn e1m1_policy_identity_is_wire_20_with_its_own_context() {
    let p = OriginalExecutionProfile::PlayerMonstersE1M1V1;
    assert_eq!(p.wire_id(), 20);
    assert_eq!(p.context_schema(), PLAYER_E1M1_CONTEXT_SCHEMA);
    assert_eq!(
        PLAYER_E1M1_CONTEXT_SCHEMA,
        b"PLAYER-MONSTERS-E1M1-1-MEDIUM-v1"
    );
    // A superset of the movers policy: every Stage A and Stage B predicate holds.
    assert!(p.has_e1m1() && p.has_movers() && p.has_specials() && p.has_real_map());
    assert!(p.has_diagonals() && p.has_heights() && p.has_map_scale());
    assert!(p.has_imps() && p.has_monsters() && p.has_world_pickups());
    assert!(p.has_player_body() && p.has_physical_player() && p.has_player_weapons());
    assert!(p.has_player_armor() && p.has_player_bonuses() && p.has_weapon_pack());
    assert!(p.has_player_commands());
    // The older policies gain nothing and keep their identities.
    let movers = OriginalExecutionProfile::PlayerMonstersMoversV1;
    let specials = OriginalExecutionProfile::PlayerMonstersSpecialsV1;
    assert!(!movers.has_e1m1() && !specials.has_e1m1());
    assert!(movers.has_movers() && !specials.has_movers());
    assert_eq!((movers.wire_id(), specials.wire_id()), (19, 18));
    assert_eq!(movers.context_schema(), PLAYER_MOVERS_CONTEXT_SCHEMA);
    assert_eq!(specials.context_schema(), PLAYER_SPECIALS_CONTEXT_SCHEMA);
    assert_ne!(p.context_schema(), movers.context_schema());
    let sim = e1m1(&m3h8_room(0)).unwrap();
    assert_eq!(sim.original_execution_profile(), Some(p));
    assert_eq!(sim.snapshot().version(), 36);
}

/// Every committed M3h8 room loads under the E1M1 policy with the same movers
/// admission, line specials and sector-thinker budget as under the movers policy.
#[test]
fn e1m1_admits_every_committed_movers_room_as_a_superset() {
    for index in 0..crate::player_movers_tests::ROOM_COUNT {
        let lumps = m3h8_room(index);
        let new = e1m1(&lumps).unwrap_or_else(|e| panic!("room {index}: {e:?}"));
        let old = crate::player_movers_tests::movers(&lumps).unwrap();
        assert_eq!(new.movers, old.movers);
        assert_eq!(new.line_specials, old.line_specials);
        assert_eq!(new.actors.sector_capacity(), old.actors.sector_capacity());
        assert_eq!(new.actors.visit_bound(), old.actors.visit_bound());
    }
    for c in crate::specials_evidence::cases() {
        let sim = Simulation::original_player_e1m1_map(
            crate::specials_evidence::case_map(c.map),
            options(),
        )
        .unwrap_or_else(|e| panic!("{}: {e:?}", c.label));
        assert_eq!(sim.movers.as_ref().unwrap().movers, 0);
    }
}

/// The fields of a DSIM36 capture, minus the two that name the policy.
fn policy_free_fields(snapshot: &Snapshot) -> Vec<(String, Vec<u8>)> {
    snapshot
        .fields()
        .filter(|f| !matches!(f.path(), "profile" | "context.schema"))
        .map(|f| (f.path().to_string(), f.value().to_vec()))
        .collect()
}

/// Every committed M3h8 scenario replayed under the E1M1 policy produces, at every
/// op, the DSIM35 capture of the movers policy except the version byte, the wire id
/// and the context (decision E8: the layouts are equal until decorations appear),
/// and the same event stream.
#[test]
fn e1m1_replays_every_movers_scenario_with_the_dsim35_layout() {
    use crate::movers_replay as replay;
    let limits = DiagnosticLimits::default();
    let mut captures = 0;
    for c in replay::cases() {
        let map = m3h8_room(c.map).map();
        let (mut old, old_body, old_things) = replay::setup_map(&c, map.clone());
        let (mut new, new_body, new_things) =
            replay::setup_map_with(&c, map, Simulation::original_player_e1m1_map);
        assert_eq!(
            (old_body, old_things.clone()),
            (new_body, new_things.clone())
        );
        for op in 0..=c.tics {
            if op != 0 {
                replay::controlled(&mut old, &c, old_body, &old_things, op);
                replay::controlled(&mut new, &c, new_body, &new_things, op);
                let command = replay::command(&c, op);
                old.advance(command)
                    .unwrap_or_else(|e| panic!("{} op{op} movers: {e}", c.label));
                new.advance(command)
                    .unwrap_or_else(|e| panic!("{} op{op} e1m1: {e}", c.label));
            }
            let (a, b) = (old.snapshot(), new.snapshot());
            assert_eq!((a.version(), b.version()), (35, 36), "{} op{op}", c.label);
            assert_eq!(
                policy_free_fields(&a),
                policy_free_fields(&b),
                "{} op{op}",
                c.label
            );
            assert_eq!(old.events(), new.events(), "{} op{op}", c.label);
            assert_eq!(old.pending_events(), new.pending_events());
            let bytes = b.diagnostic_bytes(limits).unwrap();
            let view = Diagnostic::decode_e1m1(&bytes, limits)
                .unwrap_or_else(|e| panic!("{} op{op}: {e:?}", c.label));
            assert_eq!(view.field("profile"), Some(20u32.to_le_bytes().as_slice()));
            assert_eq!(
                view.field("context.schema"),
                Some(PLAYER_E1M1_CONTEXT_SCHEMA)
            );
            assert_eq!(view.encode(limits).unwrap(), bytes);
            assert_eq!(
                Diagnostic::decode_movers(&bytes, limits).map(|_| ()),
                Err(DiagnosticError::Version(36))
            );
            captures += 1;
        }
    }
    assert!(captures > 1000, "{captures} captures compared");
}

/// DSIM36 is read by `decode_e1m1` alone; the DSIM35 reader and every older one
/// reject it, and `decode_e1m1` rejects 35 and every older version.
#[test]
fn dsim36_is_rejected_by_every_older_reader_and_decode_e1m1_reads_36_only() {
    let limits = DiagnosticLimits::default();
    let c = crate::movers_replay::cases().remove(0);
    let (mut new, ..) = crate::movers_replay::setup_map_with(
        &c,
        m3h8_room(c.map).map(),
        Simulation::original_player_e1m1_map,
    );
    new.advance([TicCommand::default(); 4]).unwrap();
    let v36 = new.snapshot().diagnostic_bytes(limits).unwrap();
    let (mut old, ..) = crate::movers_replay::setup(&c);
    old.advance([TicCommand::default(); 4]).unwrap();
    let v35 = old.snapshot().diagnostic_bytes(limits).unwrap();
    assert_eq!(old.snapshot().version(), 35);
    for reader in [
        Diagnostic::decode,
        Diagnostic::decode_v20,
        Diagnostic::decode_player,
        Diagnostic::decode_bonuses,
        Diagnostic::decode_monsters,
        Diagnostic::decode_imps,
        Diagnostic::decode_map_scale,
        Diagnostic::decode_heights,
        Diagnostic::decode_diagonal,
        Diagnostic::decode_real_map,
        Diagnostic::decode_specials,
        Diagnostic::decode_movers,
    ] {
        assert_eq!(
            reader(&v36, limits).map(|_| ()),
            Err(DiagnosticError::Version(36))
        );
    }
    assert_eq!(
        Diagnostic::decode_e1m1(&v35, limits).map(|_| ()),
        Err(DiagnosticError::Version(35))
    );
    // An older capture with its version rewritten to 36 fails the schema check: the
    // profile and context of version 36 are wire 20's.
    let mut forged = v35.clone();
    forged[4..8].copy_from_slice(&36u32.to_le_bytes());
    assert_eq!(
        Diagnostic::decode_e1m1(&forged, limits).map(|_| ()),
        Err(DiagnosticError::Schema)
    );
    Diagnostic::decode_movers(&v35, limits).unwrap();
    Diagnostic::decode_e1m1(&v36, limits).unwrap();
}

const DENSE9: &[u8] = include_bytes!("../../../tests/reference/m3h9_dense9.bin");
/// The committed rooms with a two-sided 48 line: `scrollhall` (0–2, non-blocking),
/// `scrollblock` (3, `ML_BLOCKING`) and `dense8` (4, eight in one window).
const SCROLLER_ROOMS: [usize; 5] = [0, 1, 2, 3, 4];

fn rejects(result: Result<Simulation, OriginalRegistryError>, expected: SimError) {
    match result {
        Err(OriginalRegistryError::Runtime(error)) => assert_eq!(error, expected),
        Err(other) => panic!("unexpected error {other:?}, expected {expected:?}"),
        Ok(_) => panic!("admitted, expected {expected:?}"),
    }
}

/// Decision E1: a two-sided 48, blocking or not, loads under wire 20 alone; the
/// movers and specials policies keep returning `PursuitError::Specials` (rule 1,
/// decision N1), and the admission is otherwise the movers admission.
#[test]
fn two_sided_scrollers_load_under_e1m1_only() {
    for index in SCROLLER_ROOMS {
        let lumps = room(index);
        let map = lumps.map();
        let two_sided_48 = map
            .linedefs()
            .iter()
            .filter(|l| l.special == 48 && l.sides[1].is_some())
            .count();
        assert!(two_sided_48 > 0, "room {index}");
        rejects(
            Simulation::original_player_movers_map(lumps.map(), options()),
            SimError::Pursuit(PursuitError::Specials),
        );
        rejects(
            Simulation::original_player_specials_map(lumps.map(), options()),
            SimError::Pursuit(PursuitError::Specials),
        );
        let sim = e1m1(&lumps).unwrap_or_else(|e| panic!("room {index}: {e:?}"));
        // The runtime line specials start as the map's: every 48 is a runtime 48.
        assert_eq!(
            sim.line_specials,
            map.linedefs().iter().map(|l| l.special).collect::<Vec<_>>()
        );
        assert_eq!(
            sim.line_specials.iter().filter(|s| **s == 48).count(),
            map.linedefs().iter().filter(|l| l.special == 48).count()
        );
    }
    // scrollblock: the blocking line keeps ML_BLOCKING and its special.
    let block = room(3).map();
    let line = block.linedefs().iter().find(|l| l.special == 48).unwrap();
    assert_eq!(line.flags & 1, 1);
    assert!(line.sides[1].is_some());
    // Rule 1 is relaxed for 48 alone: another two-sided special still rejects.
    let mut lumps = room(0);
    let index = (0..lumps.line_count())
        .find(|&i| {
            let l = lumps.lumps.iter().find(|(n, _)| n == "LINEDEFS").unwrap();
            i16::from_le_bytes([l.1[14 * i + 6], l.1[14 * i + 7]]) == 48
        })
        .unwrap();
    lumps.set_line_special(index, 49);
    rejects(e1m1(&lumps), SimError::Pursuit(PursuitError::Specials));
}

/// Rules 5 and 6 are unchanged: the nine-scroller lane exceeds the `spechit` window
/// under the E1M1 policy too (`MapScaleError::SpecialWindow`), as the eight-line
/// lane is the largest the C harness runs; the movers policy rejects it by rule 1.
#[test]
fn dense9_is_rejected_by_the_special_window_rule_under_e1m1() {
    let lumps = Lumps::parse(DENSE9);
    assert_eq!(crate::mover_rules::special_window_lines(&lumps.map()), 9);
    rejects(
        e1m1(&lumps),
        SimError::MapScale(MapScaleError::SpecialWindow),
    );
    rejects(
        Simulation::original_player_movers_map(lumps.map(), options()),
        SimError::Pursuit(PursuitError::Specials),
    );
    assert_eq!(crate::mover_rules::special_window_lines(&room(4).map()), 8);
    e1m1(&room(4)).unwrap();
}

/// The C driver's setup for a committed M3h9 room under the E1M1 policy: THINGs,
/// the player body at `position` (facing east), the default weapons, then
/// `P_SpawnSpecials`; seed 6 as every replayed scenario.
pub(crate) fn setup(index: usize, position: [i16; 2]) -> (Simulation, ActorId, Vec<ActorId>) {
    let mut s = Simulation::original_player_e1m1_map(room(index).map(), options()).unwrap();
    let things = s.spawn_map_things().unwrap().spawned;
    let body = s
        .spawn_player_body(PlayerSlot::One, position.map(Fixed::from_map_units))
        .unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s.set_initial_player_reaction_time(PlayerSlot::One, 0)
        .unwrap();
    (s, body, things)
}
fn forward(forward: i8) -> [TicCommand; 4] {
    let mut c = [TicCommand::default(); 4];
    c[0].forward = forward;
    c
}
fn turn(angle_turn: i16) -> [TicCommand; 4] {
    let mut c = [TicCommand::default(); 4];
    c[0].angle_turn = angle_turn;
    c
}
fn use_button() -> [TicCommand; 4] {
    let mut c = [TicCommand::default(); 4];
    c[0].buttons = BT_USE;
    c
}
/// Advances one tic and appends its events to `log` (`events()` holds the last
/// tic's events only).
fn step(s: &mut Simulation, log: &mut Vec<Event>, command: [TicCommand; 4]) {
    s.advance(command).unwrap();
    log.extend_from_slice(s.events());
}
fn mover_events(log: &[Event]) -> Vec<MoverEvent> {
    log.iter()
        .filter_map(|e| match e {
            Event::Mover(m) => Some(*m),
            _ => None,
        })
        .collect()
}
fn sounds(log: &[Event]) -> Vec<u16> {
    log.iter()
        .filter_map(|e| match e {
            Event::ActorState(lifecycle::ActorStateEvent::Sound { sound, .. }) => Some(*sound),
            _ => None,
        })
        .collect()
}
fn position(s: &Simulation, id: ActorId) -> [i32; 2] {
    s.actor(id).unwrap().position().map(|v| v.bits() / 65536)
}

/// `use_scroll_wall_stops_trace`: `BT_USE` from (−8, 0) toward the door at x = 48:
/// `PTR_UseTraverse` stops at the 48 line (p_map.c:1113–1122), whose
/// `P_UseSpecialLine` reaches the trailing `return true` with no effect
/// (p_switch.c, no case 48): the special stays 48, the door stays shut (no mover
/// started), no `sfx_noway`.
#[test]
fn a_player_use_through_a_two_sided_scroller_stops_with_no_effect() {
    let (mut s, body, _) = setup(0, [-8, 0]);
    let mut log = Vec::new();
    for _ in 0..2 {
        step(&mut s, &mut log, [TicCommand::default(); 4]);
    }
    log.clear();
    step(&mut s, &mut log, use_button());
    let events = mover_events(&log);
    assert_eq!(
        events
            .iter()
            .find(|e| matches!(e, MoverEvent::SpecialUse { .. })),
        Some(&MoverEvent::SpecialUse {
            user: body,
            line: s
                .world
                .map()
                .linedefs()
                .iter()
                .position(|l| {
                    l.special == 48 && {
                        let a = s.world.map().vertices()[usize::from(l.vertices[0])];
                        let b = s.world.map().vertices()[usize::from(l.vertices[1])];
                        a.x == 0 && b.x == 0 && a.y.min(b.y) <= 0 && 0 <= a.y.max(b.y)
                    }
                })
                .unwrap() as u16,
            side: 0,
            before: 48,
            result: true,
            after: 48,
        })
    );
    assert!(events.iter().any(|e| matches!(
        e,
        MoverEvent::UseIntercept {
            continues: false,
            ..
        }
    )));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, MoverEvent::Started { .. }))
    );
    assert_eq!(sounds(&log), Vec::<u16>::new());
    assert!(s.line_specials.iter().all(|s| matches!(s, 0 | 1 | 48)));
    // The door sector (4) stays closed.
    assert_eq!(s.sectors()[4].ceiling, s.sectors()[4].floor);
}

/// `scroll_cross_noop`: the player walks east across the non-blocking 48 line: it
/// enters `spechit`, the crossing loop calls `P_CrossSpecialLine` (p_spec.c:503–534,
/// no case 48) and nothing changes; the line keeps its special.
#[test]
fn a_player_crossing_a_two_sided_scroller_is_a_no_op() {
    let (mut s, body, _) = setup(0, [-40, 0]);
    let mut log = Vec::new();
    for op in 1..=40 {
        step(&mut s, &mut log, forward(if op <= 6 { 25 } else { 0 }));
    }
    // The center crossed x = 0 (the C row: the player stops at 8).
    assert!(position(&s, body)[0] > 0, "{:?}", position(&s, body));
    let crossings: Vec<_> = mover_events(&log)
        .into_iter()
        .filter(|e| matches!(e, MoverEvent::Crossed { .. }))
        .collect();
    assert_eq!(crossings.len(), 1);
    assert!(matches!(
        crossings[0],
        MoverEvent::Crossed {
            actor,
            before: 48,
            after: 48,
            ..
        } if actor == body
    ));
    assert_eq!(s.line_specials.iter().filter(|s| **s == 48).count(), 2);
}

/// `blocking_scroller_wall`: a 48 line with `ML_BLOCKING` blocks the player walking
/// west and the chasing Zombieman walking east in `PIT_CheckLine` before the
/// special test (p_map.c:220–244): nothing enters `spechit`, no crossing, no
/// `P_Move` failure branch with specials.
#[test]
fn a_blocking_two_sided_scroller_blocks_and_never_enters_spechit() {
    let (mut s, body, things) = setup(3, [48, 64]);
    let zombie = things[0];
    s.set_actor_target(zombie, Some(body)).unwrap();
    s.enter_original_state(zombie, OriginalState::S_POSS_RUN1)
        .unwrap();
    let mut log = Vec::new();
    step(&mut s, &mut log, turn(-32768));
    for _ in 2..=60 {
        step(&mut s, &mut log, forward(25));
    }
    // The player's center stays east of the line by its radius, the monster's west.
    assert!(position(&s, body)[0] >= 16, "{:?}", position(&s, body));
    assert!(position(&s, zombie)[0] <= -20, "{:?}", position(&s, zombie));
    let events = mover_events(&log);
    assert!(!events.iter().any(|e| matches!(
        e,
        MoverEvent::Crossed { .. }
            | MoverEvent::SpechitBlocked { .. }
            | MoverEvent::SpecialUse { .. }
    )));
}

/// `dense_spechit_8`: a position check whose box covers all eight 48 lines fills
/// `spechit` to its source size (`MAXSPECIALCROSS`) and no more; every crossing is
/// a no-op and the tic is never refused.
#[test]
fn eight_two_sided_scrollers_in_one_window_fill_spechit_without_overflow() {
    let (mut s, body, _) = setup(4, [200, 64]);
    let mut log = Vec::new();
    for op in 1..=60 {
        step(&mut s, &mut log, forward(if op <= 12 { 25 } else { 0 }));
    }
    // The center passed the last line at x = 268.
    assert!(position(&s, body)[0] > 268, "{:?}", position(&s, body));
    let crossed: Vec<u16> = mover_events(&log)
        .into_iter()
        .filter_map(|e| match e {
            MoverEvent::Crossed {
                before: 48,
                after: 48,
                line,
                ..
            } => Some(line),
            _ => None,
        })
        .collect();
    let mut distinct = crossed.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!((crossed.len(), distinct.len()), (8, 8));
}

/// `scroll_spechit_nodir`: the Zombieman on the ledge whose move into the hall is
/// refused with the A|L 48 line in `spechit` resets `movedir` to `DI_NODIR` and
/// `P_UseSpecialLine(actor, line, 0)` returns false (p_enemy.c:310–324; the monster
/// filter of p_switch.c:301–319 admits 1, 32, 33, 34 only).
#[test]
fn a_monster_blocked_with_a_scroller_in_spechit_resets_its_direction() {
    let (mut s, body, things) = setup(1, [-200, 72]);
    let zombie = things[0];
    s.set_actor_target(zombie, Some(body)).unwrap();
    s.enter_original_state(zombie, OriginalState::S_POSS_RUN1)
        .unwrap();
    let mut log = Vec::new();
    for _ in 1..=60 {
        step(&mut s, &mut log, [TicCommand::default(); 4]);
    }
    let events = mover_events(&log);
    let blocked = events
        .iter()
        .filter(|e| matches!(e, MoverEvent::SpechitBlocked { actor, count: 1 } if *actor == zombie))
        .count();
    assert!(blocked > 0);
    assert!(events.iter().any(|e| matches!(
        e,
        MoverEvent::SpecialUse {
            user,
            side: 0,
            before: 48,
            result: false,
            after: 48,
            ..
        } if *user == zombie
    )));
}
