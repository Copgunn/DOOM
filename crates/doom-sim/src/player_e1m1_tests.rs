//! M3h9 E1M1 policy: its identity, the superset relation to the movers policy and
//! the DSIM36 version plumbing. Expected behavior comes from the original source
//! reading in M3H9_REFERENCE_INVESTIGATION.md and the committed C evidence, never
//! from Rust output.
use super::*;
use crate::diagonal_room::Lumps;
use crate::identity::{OriginalActorType, OriginalState};
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

// ---- Step 6: static decorations -------------------------------------------------

use crate::e1m1_rules::{self, E1M1_DECORATIONS, E1M1_LINKED_THINGS, MAX_DECORATIONS};
use crate::planes::ClipOutcome;

/// The seven DoomEd numbers of the list (M3h9 note §2.5), in list order.
const DOOMED: [i32; 7] = [2028, 48, 35, 24, 15, 10, 12];
/// MT_BARREL (2035, decision E3) and a hanging corpse (49, `MT_MISC51`,
/// `MF_SPAWNCEILING|MF_NOGRAVITY`, decision E2): never decorations here.
const BARREL: i16 = 2035;
const HANGING_CORPSE: i16 = 49;

fn attack() -> [TicCommand; 4] {
    let mut c = [TicCommand::default(); 4];
    c[0].buttons = BT_ATTACK;
    c
}
fn life(s: &Simulation, id: ActorId) -> ActorLife {
    s.actor(id).unwrap().life().unwrap()
}
fn actors_of(s: &Simulation, ty: OriginalActorType) -> Vec<ActorId> {
    s.actors
        .order()
        .filter(|id| life(s, *id).actor_type.0 == ty.ordinal())
        .collect()
}
fn player_health(s: &Simulation) -> i32 {
    s.original_player.as_ref().unwrap().health
}

/// Decision E2: the list is exactly the seven E1M1 types of info.c, each satisfying
/// the static predicate with the table's spawn state, flags, radius and height;
/// barrels and hanging corpses fail it (shootable; spawn-ceiling and no-gravity).
#[test]
fn the_decoration_list_is_the_seven_static_e1m1_types() {
    assert_eq!(
        E1M1_DECORATIONS.map(|t| t.descriptor().doomed_number),
        DOOMED
    );
    let expected = [
        ("MT_MISC31", OriginalState::S_COLU, 16, 2),
        ("MT_MISC48", OriginalState::S_TECHPILLAR, 16, 2),
        ("MT_MISC50", OriginalState::S_CANDELABRA, 16, 2),
        ("MT_MISC71", OriginalState::S_GIBS, 20, 0),
        ("MT_MISC62", OriginalState::S_PLAY_DIE7, 20, 0),
        ("MT_MISC68", OriginalState::S_PLAY_XDIE9, 20, 0),
        ("MT_MISC69", OriginalState::S_PLAY_XDIE9, 20, 0),
    ];
    for (ty, (name, spawn, radius, flags)) in E1M1_DECORATIONS.into_iter().zip(expected) {
        let d = ty.descriptor();
        assert_eq!(ty.name(), name);
        assert!(e1m1_rules::static_decoration(ty), "{name}");
        assert_eq!(d.states.spawn, spawn);
        assert_eq!((d.radius, d.height), (radius * 65536, 16 * 65536));
        assert_eq!(
            (d.flags, d.spawn_health, d.pain_chance, d.mass),
            (flags, 1000, 0, 100)
        );
        let s = spawn.descriptor();
        assert_eq!((s.tics, s.action), (-1, None));
        // No state of the type has an action.
        for state in [
            d.states.see,
            d.states.pain,
            d.states.melee,
            d.states.missile,
            d.states.death,
            d.states.extreme_death,
            d.states.raise,
        ] {
            assert_eq!(state, OriginalState::S_NULL, "{name}");
        }
        assert!(e1m1_rules::validate_definition(ActorTypeId(ty.ordinal())).is_ok());
        assert!(e1m1_rules::spawn_state(ActorTypeId(ty.ordinal()), spawn));
        assert!(!e1m1_rules::spawn_state(
            ActorTypeId(ty.ordinal()),
            OriginalState::S_PLAY
        ));
    }
    let barrel = OriginalActorType::MT_BARREL;
    assert_eq!(barrel.descriptor().doomed_number, i32::from(BARREL));
    assert!(!e1m1_rules::static_decoration(barrel));
    let hanging = OriginalActorType::MT_MISC51;
    assert_eq!(
        hanging.descriptor().doomed_number,
        i32::from(HANGING_CORPSE)
    );
    assert_ne!(hanging.descriptor().flags & (256 | 512), 0);
    assert!(!e1m1_rules::static_decoration(hanging));
    // Only the list is admitted, even where the predicate would hold.
    for ty in OriginalActorType::all() {
        let listed = E1M1_DECORATIONS.contains(&ty);
        assert_eq!(
            e1m1_rules::decoration(ActorTypeId(ty.ordinal())).is_some(),
            listed
        );
        assert_eq!(
            e1m1_rules::validate_definition(ActorTypeId(ty.ordinal())).is_ok(),
            listed
        );
    }
    assert_eq!((MAX_DECORATIONS, E1M1_LINKED_THINGS), (32, 65));
    assert_eq!(
        E1M1_LINKED_THINGS,
        1 + MAX_PLAYER_MONSTERS + MAX_WORLD_PICKUPS + MAX_PLAYER_MONSTERS + MAX_DECORATIONS
    );
}

/// The admitted definition set of wire 20 is the movers set plus the seven
/// decorations, each in its spawn state alone; wires 18 and 19 reject the seven.
#[test]
fn e1m1_admits_exactly_the_movers_roles_plus_the_seven_decorations() {
    let new =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersE1M1V1).unwrap();
    let movers =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersMoversV1).unwrap();
    let specials =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersSpecialsV1).unwrap();
    for ty in OriginalActorType::all() {
        let identity = DefinitionIdentity::Original(ty);
        let listed = E1M1_DECORATIONS.contains(&ty);
        let old = movers.admit_definition(identity).is_ok();
        assert_eq!(
            old,
            specials.admit_definition(identity).is_ok(),
            "{}",
            ty.name()
        );
        assert_eq!(
            new.admit_definition(identity).is_ok(),
            old || listed,
            "{}",
            ty.name()
        );
        if listed {
            assert!(!old);
            let spawn = ty.states().spawn;
            assert_eq!(
                new.admit_state(identity, spawn).unwrap(),
                StateId(spawn.ordinal())
            );
            assert!(movers.admit_state(identity, spawn).is_err());
            for other in [
                OriginalState::S_PLAY,
                OriginalState::S_POSS_RUN1,
                OriginalState::S_NULL,
            ] {
                assert!(new.admit_state(identity, other).is_err(), "{}", ty.name());
            }
        }
    }
    // MT_MISC71 spawns in S_GIBS; S_GIBS is not a monster's or the player's state
    // under this reader-independent role check, as under the movers policy.
    assert!(
        new.admit_state(
            DefinitionIdentity::Original(OriginalActorType::MT_MISC71),
            OriginalState::S_GIBS
        )
        .is_ok()
    );
    assert!(
        new.admit_state(
            DefinitionIdentity::Original(OriginalActorType::MT_POSSESSED),
            OriginalState::S_GIBS
        )
        .is_err()
    );
}

/// `P_SpawnMapThing` (p_mobj.c:760–795) for the ten decorations of the gallery: each
/// is spawned by `P_SpawnMobj` with its info flags, radius, height, health 1000,
/// spawn state and `tics` −1 (no tics draw), one `lastlook` draw (`P_Random() % 4`),
/// at `ONFLOORZ`, with its angle and spawnpoint, in lump order; no counter moves.
#[test]
fn gallery_decorations_spawn_as_p_spawnmapthing_does() {
    let mut s = Simulation::original_player_e1m1_map(room(5).map(), options()).unwrap();
    let [p0, _] = s.rng.indices();
    let map = s.world.map().clone();
    let report = s.spawn_map_things().unwrap();
    assert_eq!(report.spawned.len(), 10);
    assert_eq!(report.filtered, 0);
    // One P_Random per spawn, none for tics (every spawn state has tics −1).
    assert_eq!(s.rng.indices()[0], p0.wrapping_add(10));
    let stats = s.statistics.unwrap();
    assert_eq!((stats.total_kills, stats.total_items), (0, 0));
    for (id, thing) in report.spawned.iter().zip(map.things()) {
        let a = s.actor(*id).unwrap();
        let l = a.life().unwrap();
        let ty = e1m1_rules::decoration(l.actor_type).unwrap();
        let d = ty.descriptor();
        assert_eq!(d.doomed_number, i32::from(thing.kind));
        assert_eq!(l.state, Some(StateId(d.states.spawn.ordinal())));
        assert_eq!(l.tics, -1);
        assert_eq!(l.health, 1000);
        assert_eq!(l.flags, d.flags);
        assert_eq!(a.radius(), Fixed::from_bits(d.radius));
        assert_eq!(a.height(), Fixed::from_map_units(16));
        assert_eq!(a.z(), a.constraints()[0]);
        assert!(a.lastlook() <= 3);
        assert_eq!(l.spawnpoint, Some(*thing));
        assert_eq!(l.target, None);
        assert_eq!(a.momentum(), [Fixed::ZERO; 2]);
        assert_eq!(
            (a.solidity == Solidity::Solid),
            d.flags & crate::original_player::MF_SOLID != 0
        );
        assert_eq!(
            a.spec.angle,
            ((i32::from(thing.angle) / 45) as u32).wrapping_mul(0x20000000)
        );
        e1m1_rules::validate_decoration(None, *a).unwrap();
    }
    // The pedestal decorations sit on their 32-unit floors.
    let candelabra = actors_of(&s, OriginalActorType::MT_MISC50)[0];
    assert_eq!(s.actor(candelabra).unwrap().z(), Fixed::from_map_units(32));
    let pool = actors_of(&s, OriginalActorType::MT_MISC71);
    assert_eq!(pool.len(), 2);
    assert_eq!(s.actor(pool[0]).unwrap().z(), Fixed::from_map_units(32));
    // The world then runs: every per-tic preflight admits the ten decorations.
    s.spawn_player_body(
        PlayerSlot::One,
        [Fixed::from_map_units(-128), Fixed::from_map_units(128)],
    )
    .unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    let before: Vec<_> = report
        .spawned
        .iter()
        .map(|id| *s.actor(*id).unwrap())
        .collect();
    for _ in 0..5 {
        s.advance([TicCommand::default(); 4]).unwrap();
    }
    for (id, b) in report.spawned.iter().zip(before) {
        assert_eq!(*s.actor(*id).unwrap(), b);
    }
}

/// The census room (forty records of every admitted kind with each skill-bit
/// combination, bit 16 and ambush) under medium: exactly the records whose
/// options have bit 2 and not bit 16 spawn (p_mobj.c:740–750), in lump order; a
/// decoration record's `MTF_AMBUSH` reaches its flags (792–793).
#[test]
fn the_census_room_spawns_the_medium_records_in_lump_order_with_ambush() {
    let mut s = Simulation::original_player_e1m1_map(room(12).map(), options()).unwrap();
    let map = s.world.map().clone();
    let report = s.spawn_map_things().unwrap();
    let expected: Vec<_> = map
        .things()
        .iter()
        .filter(|t| t.options & 16 == 0 && t.options & 2 != 0)
        .collect();
    assert_eq!(map.things().len(), 40);
    assert_eq!(report.spawned.len(), expected.len());
    assert_eq!(report.filtered, 40 - expected.len());
    let mut decorations = 0;
    let mut ambushed = 0;
    for (id, thing) in report.spawned.iter().zip(expected) {
        let l = life(&s, *id);
        assert_eq!(l.spawnpoint, Some(*thing));
        if let Some(d) = e1m1_rules::decoration(l.actor_type) {
            decorations += 1;
            assert_eq!(d.descriptor().doomed_number, i32::from(thing.kind));
            assert_eq!(l.flags & 32 != 0, thing.options & 8 != 0);
            ambushed += usize::from(thing.options & 8 != 0);
        }
    }
    assert!(
        decorations >= 7 && ambushed >= 1,
        "{decorations} {ambushed}"
    );
    // The spawn order is the lump order: decorations interleave with monsters and
    // pickups, and the per-tic preflight admits that order.
    s.spawn_player_body(
        PlayerSlot::One,
        [Fixed::from_map_units(-448), Fixed::from_map_units(-340)],
    )
    .unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    for _ in 0..4 {
        s.advance([TicCommand::default(); 4]).unwrap();
    }
    let limits = DiagnosticLimits::default();
    let bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
    let view = Diagnostic::decode_e1m1(&bytes, limits).unwrap();
    assert_eq!(view.encode(limits).unwrap(), bytes);
}

/// Decorations load under wire 20 alone: the movers and specials policies refuse
/// the gallery's first decoration (`UnsupportedType(2028)`) with the state
/// unchanged; a barrel and a hanging corpse are `UnsupportedType` under E1M1 too.
#[test]
fn decorations_are_unsupported_types_under_older_policies_and_barrels_everywhere() {
    for construct in [
        Simulation::original_player_movers_map as fn(_, _) -> _,
        Simulation::original_player_specials_map,
    ] {
        let mut s = construct(room(5).map(), options()).unwrap();
        let before = s.snapshot();
        assert_eq!(
            s.spawn_map_things().map(|_| ()),
            Err(SimError::MapSpawn(MapSpawnError::UnsupportedType(2028)))
        );
        assert_eq!(s.snapshot(), before);
    }
    for (kind, position) in [(BARREL, [-300, 100]), (HANGING_CORPSE, [-300, 100])] {
        let mut lumps = room(5);
        lumps.add_thing([position[0], position[1], 0, kind, 7]);
        let mut s = e1m1(&lumps).unwrap();
        let before = s.snapshot();
        assert_eq!(
            s.spawn_map_things().map(|_| ()),
            Err(SimError::MapSpawn(MapSpawnError::UnsupportedType(kind)))
        );
        assert_eq!(s.snapshot(), before);
    }
}

/// `MAX_DECORATIONS`: 32 lamps spawn; a 33rd is `DecorationLimit` before any spawn,
/// with the state unchanged and nothing counted.
#[test]
fn the_thirty_third_decoration_is_refused_with_the_state_unchanged() {
    let lamps = |count: usize| {
        let mut lumps = room(5);
        lumps.clear_things();
        for i in 0..count {
            let (x, y) = (-480 + 64 * (i % 16) as i16, -340 + 64 * (i / 16) as i16);
            lumps.add_thing([x, y, 0, 2028, 7]);
        }
        lumps
    };
    let mut s = e1m1(&lamps(32)).unwrap();
    let report = s.spawn_map_things().unwrap();
    assert_eq!(report.spawned.len(), 32);
    s.spawn_player_body(
        PlayerSlot::One,
        [Fixed::from_map_units(200), Fixed::from_map_units(100)],
    )
    .unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s.advance([TicCommand::default(); 4]).unwrap();
    let mut s = e1m1(&lamps(33)).unwrap();
    let before = s.snapshot();
    let [p0, _] = s.rng.indices();
    assert_eq!(
        s.spawn_map_things().map(|_| ()),
        Err(SimError::MapSpawn(MapSpawnError::DecorationLimit))
    );
    assert_eq!(s.snapshot(), before);
    assert_eq!(s.rng.indices()[0], p0);
    assert_eq!(s.actors.order().count(), 0);
}

/// The API refuses what the source never does to a decoration: a push (thrust needs
/// `MF_SHOOTABLE`) and a state entry (`tics` −1, never killed), state unchanged.
#[test]
fn a_decoration_refuses_a_push_and_a_state_entry() {
    let (mut s, _, things) = setup(5, [-128, 128]);
    let lamp = things[0];
    assert_eq!(
        life(&s, lamp).actor_type.0,
        OriginalActorType::MT_MISC31.ordinal()
    );
    let before = s.snapshot();
    assert_eq!(
        s.set_momentum(lamp, [Fixed::from_map_units(1), Fixed::ZERO]),
        Err(SimError::PlayerBody(PlayerBodyError::Unsupported))
    );
    assert!(s.enter_original_state(lamp, OriginalState::S_COLU).is_err());
    assert!(s.enter_original_state(lamp, OriginalState::S_NULL).is_err());
    assert_eq!(s.snapshot(), before);
    assert_eq!(
        s.original_actor_identity(lamp).unwrap(),
        (
            DefinitionIdentity::Original(OriginalActorType::MT_MISC31),
            Some(OriginalState::S_COLU)
        )
    );
}

/// `walk_into_lamp`: `PIT_CheckThing`'s last line (p_map.c:345) blocks a walker on a
/// solid non-shootable thing with no height test: the player stops a radius sum
/// (16 + 16) short of the lamp at (0, 128).
#[test]
fn a_lamp_blocks_the_player_with_no_height_test() {
    let (mut s, body, _) = setup(5, [-128, 128]);
    for _ in 1..=40 {
        s.advance(forward(25)).unwrap();
    }
    let [x, y] = position(&s, body);
    assert!(x <= -32 && x > -64, "{x} {y}");
    assert_eq!(y, 128);
    // `walk_into_lamp_from_ledge`: from the ledge (z 32) the lamp's top (16) is
    // below the player's feet and it blocks all the same.
    let (mut s, body, _) = setup(5, [-256, 300]);
    s.advance(turn(-16384)).unwrap();
    for _ in 2..=40 {
        s.advance(forward(25)).unwrap();
    }
    let [x, y] = position(&s, body);
    assert_eq!(s.actor(body).unwrap().z(), Fixed::from_map_units(32));
    assert!((226 + 32..300).contains(&y), "{x} {y}");
}

/// `walk_over_gore`: the flags-0 corpses and gore are passed by the first flag test
/// of `PIT_CheckThing` (p_map.c:258–259): the player walks through the row at
/// y = −300.
#[test]
fn gore_and_corpses_are_passed_through() {
    let (mut s, body, _) = setup(5, [-220, -300]);
    for _ in 1..=60 {
        s.advance(forward(25)).unwrap();
    }
    let [x, y] = position(&s, body);
    assert!(x > 120 + 20, "{x} {y}");
    // finesine[0] is 25, not 0 (tables.c): an eastward thrust drifts north by a
    // fraction of a unit per tic, as in the source.
    assert!((-301..=-299).contains(&y), "{x} {y}");
}

/// `monster_path_around_pillar`: a chasing Zombieman's `P_Move` fails on the solid
/// pillar at (−160, −64) (p_map.c:345), `P_NewChaseDir` draws and it goes around:
/// the pillar is a blocking actor of a refused attempt, and the monster later
/// passes it.
#[test]
fn a_monster_is_blocked_by_a_pillar_and_walks_around_it() {
    let (mut s, body, things) = setup(6, [200, -64]);
    let pillar = things[2];
    assert_eq!(
        life(&s, pillar).actor_type.0,
        OriginalActorType::MT_MISC48.ordinal()
    );
    let zombie = things[10];
    s.set_actor_target(zombie, Some(body)).unwrap();
    s.enter_original_state(zombie, OriginalState::S_POSS_RUN1)
        .unwrap();
    let mut blocked = 0;
    for _ in 1..=120 {
        s.advance([TicCommand::default(); 4]).unwrap();
        blocked += s
            .events()
            .iter()
            .filter(|e| {
                matches!(e, Event::SectorMovement { actor, attempt }
                    if *actor == zombie && attempt.movement.blocked_by_actor == Some(pillar))
            })
            .count();
    }
    assert!(blocked > 0);
    assert!(position(&s, zombie)[0] > -160, "{:?}", position(&s, zombie));
    assert_eq!(position(&s, pillar), [-160, -64]);
}

/// `fireball_hits_candelabra` and `fireball_over_corpse`: the missile branch of
/// `PIT_CheckThing` returns `!(thing->flags & MF_SOLID)` for a non-shootable thing
/// (p_map.c:304–305): the fireball explodes on the candelabra and flies through the
/// pool of blood to the player.
#[test]
fn a_fireball_explodes_on_a_candelabra_and_passes_over_a_corpse() {
    let (mut s, body, things) = setup(8, [-256, 0]);
    let imp = things[10];
    s.set_actor_target(imp, Some(body)).unwrap();
    s.enter_original_state(imp, OriginalState::S_TROO_ATK1)
        .unwrap();
    let mut exploded = Vec::new();
    for _ in 1..=50 {
        s.advance([TicCommand::default(); 4]).unwrap();
        for e in s.events() {
            if let Event::Projectile(ProjectileEvent::Exploded(id)) = e {
                exploded.push(position(&s, *id));
            }
        }
    }
    assert_eq!(exploded.len(), 1, "{exploded:?}");
    // Stopped at the candelabra at (0, 0): the half-step is kept short of it.
    assert!(exploded[0][0] > 0 && exploded[0][0] < 64, "{exploded:?}");
    assert_eq!(player_health(&s), 100);
    let (mut s, body, things) = setup(9, [-256, -192]);
    let imp = things[10];
    let pool = things[5];
    assert_eq!(
        life(&s, pool).actor_type.0,
        OriginalActorType::MT_MISC71.ordinal()
    );
    assert_eq!(position(&s, pool), [0, -192]);
    s.set_actor_target(imp, Some(body)).unwrap();
    s.enter_original_state(imp, OriginalState::S_TROO_ATK1)
        .unwrap();
    let mut exploded = Vec::new();
    for _ in 1..=50 {
        s.advance([TicCommand::default(); 4]).unwrap();
        for e in s.events() {
            if let Event::Projectile(ProjectileEvent::Exploded(id)) = e {
                exploded.push(position(&s, *id));
            }
        }
    }
    // At the C scenario's last tic the fireball is past the pool, still flying.
    assert!(exploded.is_empty(), "{exploded:?}");
    let shots = actors_of(&s, OriginalActorType::MT_TROOPSHOT);
    assert_eq!(shots.len(), 1);
    assert!(
        position(&s, shots[0])[0] < -40,
        "{:?}",
        position(&s, shots[0])
    );
    assert_eq!(s.actor(shots[0]).unwrap().z(), Fixed::from_map_units(32));
    assert_eq!(player_health(&s), 100);
    // It reaches the player.
    for _ in 51..=80 {
        s.advance([TicCommand::default(); 4]).unwrap();
    }
    assert!(player_health(&s) < 100);
}

/// `pistol_through_lamp`: `PTR_AimTraverse` and `PTR_ShootTraverse` skip a
/// non-shootable thing (p_map.c:1013–1014): the puff spawns at the east wall, not
/// at the lamp at (0, 128).
#[test]
fn a_hitscan_passes_through_a_lamp() {
    let (mut s, _, _) = setup(5, [-256, 128]);
    let mut puffs = Vec::new();
    for op in 1..=40 {
        s.advance(if op == 20 { attack() } else { forward(0) })
            .unwrap();
        puffs.extend(spawned_puffs(&s));
    }
    assert_eq!(puffs.len(), 1, "{puffs:?}");
    assert!(puffs[0][0] > 400, "{puffs:?}");
}
/// The positions of the `MT_PUFF`s spawned in the last tic (they live 16–19 tics).
fn spawned_puffs(s: &Simulation) -> Vec<[i32; 2]> {
    s.events()
        .iter()
        .filter_map(|e| match e {
            Event::Spawned(id)
                if s.actor(*id)
                    .and_then(Actor::life)
                    .is_some_and(|l| l.actor_type.0 == OriginalActorType::MT_PUFF.ordinal()) =>
            {
                Some(position(s, *id))
            }
            _ => None,
        })
        .collect()
}

/// `monster_hitscan_through_pillar`: a pinned Zombieman's bullets pass the pillar
/// between it and the player (`PTR_ShootTraverse` skips a non-shootable thing,
/// p_map.c:1013–1014): every puff is east of the pillar, or the player is hit. The
/// committed alcove room (7) is replayed against the C rows by `e1m1_replay`; this
/// Rust-only variant pins a Zombieman in the gallery (6) with a lane of lamps added
/// north and south of it, a lamp closing its west end and a second pillar closing
/// its east end: it can only walk east and west (so `movecount` keeps being reset
/// and `A_Chase` reaches its missile check), and every shot it takes at the player
/// passes the pillar.
#[test]
fn a_monster_hitscan_passes_through_a_pillar() {
    let mut lumps = room(6);
    for x in (-404..=-212).step_by(32) {
        lumps.add_thing([x, -100, 0, 2028, 7]);
        lumps.add_thing([x, -28, 0, 2028, 7]);
    }
    lumps.add_thing([-404, -64, 0, 2028, 7]);
    lumps.add_thing([-204, -64, 0, 48, 7]);
    let mut s = e1m1(&lumps).unwrap();
    let things = s.spawn_map_things().unwrap().spawned;
    assert_eq!(things.len(), 11 + 16);
    let body = s
        .spawn_player_body(
            PlayerSlot::One,
            [Fixed::from_map_units(200), Fixed::from_map_units(-64)],
        )
        .unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s.set_initial_player_reaction_time(PlayerSlot::One, 0)
        .unwrap();
    let zombie = things[10];
    assert_eq!(position(&s, zombie), [-240, -64]);
    s.set_actor_target(zombie, Some(body)).unwrap();
    s.enter_original_state(zombie, OriginalState::S_POSS_RUN1)
        .unwrap();
    let mut shots = 0;
    let mut puffs = Vec::new();
    for _ in 1..=600 {
        s.advance([TicCommand::default(); 4]).unwrap();
        assert_eq!(position(&s, zombie)[1], -64);
        shots += s
            .events()
            .iter()
            .filter(|e| {
                matches!(e, Event::ActorState(lifecycle::ActorStateEvent::Sound { sound, .. })
                    if *sound == lifecycle::source_sound_id("sfx_pistol"))
            })
            .count();
        puffs.extend(spawned_puffs(&s));
    }
    assert!(shots > 0);
    assert!((-368..=-240).contains(&position(&s, zombie)[0]));
    for puff in &puffs {
        assert!(puff[0] > -204 + 16, "{puffs:?}");
    }
    assert!(player_health(&s) < 100 || !puffs.is_empty());
}

/// `decorations_ride_lift`: the lamp and the dead player on the lift follow its
/// floor down and up through `P_ChangeSector` and `P_ThingHeightClip`
/// (p_map.c:1261–1265, 542–546): every clip fits, `z` tracks the sector floor.
#[test]
fn a_lamp_and_a_corpse_ride_the_lift() {
    let (mut s, _, things) = setup(10, [256, 64]);
    let [lamp, corpse] = [things[0], things[1]];
    assert_eq!(
        life(&s, lamp).actor_type.0,
        OriginalActorType::MT_MISC31.ordinal()
    );
    assert_eq!(
        life(&s, corpse).actor_type.0,
        OriginalActorType::MT_MISC62.ordinal()
    );
    let lift = 3;
    assert_eq!(s.sectors()[lift].floor, Fixed::from_map_units(64));
    s.advance(turn(16384)).unwrap();
    let mut lowest = i32::MAX;
    let mut clips = 0;
    for op in 2..=190 {
        s.advance(forward(if op <= 9 { 25 } else { 0 })).unwrap();
        for e in s.events() {
            if let Event::Mover(MoverEvent::ThingClipped { actor, outcome, .. }) = e {
                if [lamp, corpse].contains(actor) {
                    assert_eq!(*outcome, ClipOutcome::Fits);
                    clips += 1;
                }
            }
        }
        let floor = s.sectors()[lift].floor;
        for id in [lamp, corpse] {
            let a = s.actor(id).unwrap();
            assert_eq!(a.z(), floor, "op {op}");
            assert_eq!(a.constraints()[0], floor);
        }
        lowest = lowest.min(floor.bits() / 65536);
    }
    assert!(clips > 0);
    assert_eq!(lowest, 2);
    assert_eq!(s.sectors()[lift].floor, Fixed::from_map_units(64));
}

/// `door_closes_on_pillar`: the door closing onto the 16-high pillar reaches
/// `PIT_ChangeSector`'s `MF_SHOOTABLE` test (p_map.c:1290–1294) and returns with
/// `nofit` untouched: the door closes fully, the pillar keeps `z == floorz`.
#[test]
fn a_door_closes_on_a_pillar_without_nofit() {
    let (mut s, _, things) = setup(11, [470, 64]);
    let pillar = things[0];
    assert_eq!(
        life(&s, pillar).actor_type.0,
        OriginalActorType::MT_MISC48.ordinal()
    );
    let door = 5;
    assert_eq!(s.sectors()[door].ceiling, Fixed::from_map_units(124));
    let mut stuck = 0;
    for op in 1..=80 {
        s.advance(if matches!(op, 3 | 6) {
            use_button()
        } else {
            forward(0)
        })
        .unwrap();
        for e in s.events() {
            match e {
                Event::Mover(MoverEvent::ThingClipped {
                    actor,
                    outcome,
                    z,
                    floor,
                    ..
                }) if *actor == pillar => {
                    assert_ne!(*outcome, ClipOutcome::NoFit);
                    assert_eq!(z, floor);
                    stuck += usize::from(*outcome == ClipOutcome::LeftStuck);
                }
                Event::Mover(MoverEvent::SectorChanged { sector, nofit })
                    if usize::from(*sector) == door =>
                {
                    assert!(!nofit);
                }
                _ => {}
            }
        }
    }
    assert!(stuck > 0);
    assert_eq!(s.sectors()[door].ceiling, s.sectors()[door].floor);
    let a = s.actor(pillar).unwrap();
    assert_eq!(a.z(), Fixed::from_map_units(2));
    assert_eq!(a.height(), Fixed::from_map_units(16));
    assert_eq!(life(&s, pillar).health, 1000);
}

/// DSIM36 with decorations: a gallery capture decodes and round-trips, its ten
/// decoration records pass the type-keyed witness (`MT_MISC71` in `S_GIBS` with
/// height 16, radius 20, health 1000), and a tampered record is refused.
#[test]
fn decode_e1m1_validates_decoration_records() {
    let (mut s, _, _) = setup(5, [-128, 128]);
    s.advance([TicCommand::default(); 4]).unwrap();
    let snapshot = s.snapshot();
    let limits = DiagnosticLimits::default();
    let bytes = snapshot.diagnostic_bytes(limits).unwrap();
    let view = Diagnostic::decode_e1m1(&bytes, limits).unwrap();
    assert_eq!(view.encode(limits).unwrap(), bytes);
    type Edit<'a> = &'a dyn Fn(&str, &[u8]) -> Option<Vec<u8>>;
    let encode = |edit: Edit<'_>| {
        let mut out = b"DSIM".to_vec();
        out.extend(36u32.to_le_bytes());
        let fields: Vec<_> = snapshot.fields().collect();
        out.extend((fields.len() as u64).to_le_bytes());
        for f in fields {
            let value = edit(f.path(), f.value()).unwrap_or_else(|| f.value().to_vec());
            out.extend((f.path().len() as u32).to_le_bytes());
            out.extend(f.path().as_bytes());
            out.extend((value.len() as u64).to_le_bytes());
            out.extend(value);
        }
        out
    };
    assert_eq!(encode(&|_, _| None), bytes);
    // The pool of blood (S_GIBS) is slot 5 in lump order: decoration or giblet?
    let gibs = OriginalActorType::MT_MISC71.ordinal().to_le_bytes();
    let slot = (0..10)
        .find(|i| view.field(&format!("actor.{i}.type")) == Some(&gibs))
        .unwrap();
    for (field, bad) in [
        ("tics", 0i32.to_le_bytes().to_vec()),
        ("health", 999i32.to_le_bytes().to_vec()),
        ("height", 0i32.to_le_bytes().to_vec()),
        ("radius", (16 * 65536i32).to_le_bytes().to_vec()),
        ("momx", 65536i32.to_le_bytes().to_vec()),
        ("flags", 2u32.to_le_bytes().to_vec()),
        (
            "state",
            OriginalState::S_PLAY_DIE7.ordinal().to_le_bytes().to_vec(),
        ),
    ] {
        let path = format!("actor.{slot}.{field}");
        let forged = encode(&|p, _| (p == path).then(|| bad.clone()));
        assert_eq!(
            Diagnostic::decode_e1m1(&forged, limits).map(|_| ()),
            Err(DiagnosticError::Identity),
            "{field}"
        );
    }
    // A monster type in S_GIBS keeps the giblet rule (height 0, radius 0).
    let lamp = OriginalActorType::MT_MISC31.ordinal().to_le_bytes();
    let slot = (0..10)
        .find(|i| view.field(&format!("actor.{i}.type")) == Some(&lamp))
        .unwrap();
    let path = format!("actor.{slot}.type");
    let forged = encode(&|p, _| {
        (p == path).then(|| {
            OriginalActorType::MT_POSSESSED
                .ordinal()
                .to_le_bytes()
                .to_vec()
        })
    });
    assert!(Diagnostic::decode_e1m1(&forged, limits).is_err());
}

/// The committed full-byte DSIM36 fixtures of the independent projection
/// (`m3h9_snapshot_contract.json`).
pub(crate) const FIXTURES: &[(&str, &[u8])] = &[
    (
        "m3h9_state_0_19.bin",
        include_bytes!("../../../tests/reference/m3h9_state_0_19.bin"),
    ),
    (
        "m3h9_state_12_38.bin",
        include_bytes!("../../../tests/reference/m3h9_state_12_38.bin"),
    ),
    (
        "m3h9_state_15_48.bin",
        include_bytes!("../../../tests/reference/m3h9_state_15_48.bin"),
    ),
    (
        "m3h9_state_16_67.bin",
        include_bytes!("../../../tests/reference/m3h9_state_16_67.bin"),
    ),
    (
        "m3h9_state_16_80.bin",
        include_bytes!("../../../tests/reference/m3h9_state_16_80.bin"),
    ),
    (
        "m3h9_state_17_0.bin",
        include_bytes!("../../../tests/reference/m3h9_state_17_0.bin"),
    ),
    (
        "m3h9_state_17_4.bin",
        include_bytes!("../../../tests/reference/m3h9_state_17_4.bin"),
    ),
    (
        "m3h9_state_1_6.bin",
        include_bytes!("../../../tests/reference/m3h9_state_1_6.bin"),
    ),
    (
        "m3h9_state_2_3.bin",
        include_bytes!("../../../tests/reference/m3h9_state_2_3.bin"),
    ),
    (
        "m3h9_state_4_45.bin",
        include_bytes!("../../../tests/reference/m3h9_state_4_45.bin"),
    ),
    (
        "m3h9_state_5_60.bin",
        include_bytes!("../../../tests/reference/m3h9_state_5_60.bin"),
    ),
    (
        "m3h9_state_6_14.bin",
        include_bytes!("../../../tests/reference/m3h9_state_6_14.bin"),
    ),
    (
        "m3h9_state_6_60.bin",
        include_bytes!("../../../tests/reference/m3h9_state_6_60.bin"),
    ),
    (
        "m3h9_state_7_20.bin",
        include_bytes!("../../../tests/reference/m3h9_state_7_20.bin"),
    ),
    (
        "m3h9_state_9_60.bin",
        include_bytes!("../../../tests/reference/m3h9_state_9_60.bin"),
    ),
];

/// The fifteen pinned DSIM36 fixtures of the independent projection
/// (`m3h9_snapshot_contract.json`): each decodes under `decode_e1m1` alone and
/// re-encodes to the same bytes; the Rust writer's bytes for the same states are
/// compared with them by the full replay (`e1m1_replay`).
#[test]
fn the_pinned_dsim36_fixtures_decode_under_decode_e1m1_alone() {
    let limits = DiagnosticLimits::default();
    assert_eq!(FIXTURES.len(), 15);
    for (name, bytes) in FIXTURES {
        let view =
            Diagnostic::decode_e1m1(bytes, limits).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(view.encode(limits).unwrap(), *bytes, "{name}");
        assert_eq!(
            Diagnostic::decode_movers(bytes, limits).map(|_| ()),
            Err(DiagnosticError::Version(36)),
            "{name}"
        );
    }
}

/// The fields of a DSIM capture in order (path, value).
fn dsim_fields(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let word = |at: usize, n: usize| {
        let mut v = [0u8; 8];
        v[..n].copy_from_slice(&bytes[at..at + n]);
        u64::from_le_bytes(v) as usize
    };
    let mut at = 16;
    (0..word(8, 8))
        .map(|_| {
            let n = word(at, 4);
            let path = std::str::from_utf8(&bytes[at + 4..at + 4 + n])
                .unwrap()
                .to_owned();
            at += 4 + n;
            let n = word(at, 8);
            let value = bytes[at + 8..at + 8 + n].to_vec();
            at += 8 + n;
            (path, value)
        })
        .collect()
}
/// `bytes` re-encoded with `edit` applied to every field (None keeps it).
fn forge(bytes: &[u8], edit: impl Fn(&str, &[u8]) -> Option<Vec<u8>>) -> Vec<u8> {
    let fields = dsim_fields(bytes);
    let mut out = bytes[..8].to_vec();
    out.extend((fields.len() as u64).to_le_bytes());
    for (path, value) in fields {
        let value = edit(&path, &value).unwrap_or(value);
        out.extend((path.len() as u32).to_le_bytes());
        out.extend(path.as_bytes());
        out.extend((value.len() as u64).to_le_bytes());
        out.extend(value);
    }
    out
}
fn fixture(name: &str) -> &'static [u8] {
    FIXTURES.iter().find(|(n, _)| *n == name).unwrap().1
}
/// The slot of the first occupied actor of type `ty` in a capture.
fn slot_of(bytes: &[u8], ty: OriginalActorType) -> usize {
    let fields = dsim_fields(bytes);
    let value = |p: String| fields.iter().find(|(f, _)| *f == p).map(|(_, v)| v.clone());
    (0..)
        .find(|i| {
            value(format!("actor.{i}.occupied")) == Some(vec![1])
                && value(format!("actor.{i}.type")) == Some(ty.ordinal().to_le_bytes().to_vec())
        })
        .unwrap()
}

/// Every DSIM36 decoration check rejects a capture mutated in that one respect
/// (STATE_FORMAT "DSIM v36"): the pinned `walk_into_lamp` fixture at op 20 (the ten
/// gallery decorations) with one field of the floor lamp (`MT_MISC31`, solid) or the
/// pool of blood (`MT_MISC71`, flags 0, `S_GIBS`) changed.
#[test]
fn decode_e1m1_rejects_each_mutated_decoration_check() {
    let limits = DiagnosticLimits::default();
    let bytes = fixture("m3h9_state_7_20.bin");
    Diagnostic::decode_e1m1(bytes, limits).unwrap();
    assert_eq!(forge(bytes, |_, _| None), bytes);
    let lamp = slot_of(bytes, OriginalActorType::MT_MISC31);
    let gibs = slot_of(bytes, OriginalActorType::MT_MISC71);
    let i32v = |v: i32| v.to_le_bytes().to_vec();
    let u16v = |v: u16| v.to_le_bytes().to_vec();
    let fields = dsim_fields(bytes);
    let field = |slot: usize, name: &str| {
        let path = format!("actor.{slot}.{name}");
        fields.iter().find(|(p, _)| *p == path).unwrap().1.clone()
    };
    let floor = i32::from_le_bytes(field(lamp, "floor").try_into().unwrap());
    let mut target = vec![1];
    target.extend(field(lamp, "previous")[1..].iter().copied());
    let lamp_flags = u32::from_le_bytes(field(lamp, "flags").try_into().unwrap());
    let cases: Vec<(&str, usize, &str, Vec<u8>)> = vec![
        (
            "not one of the seven types",
            lamp,
            "type",
            u16v(OriginalActorType::MT_BARREL.ordinal()),
        ),
        (
            "another decoration's spawn state",
            lamp,
            "state",
            u16v(OriginalState::S_TECHPILLAR.ordinal()),
        ),
        ("tics other than -1", lamp, "tics", i32v(0)),
        ("health other than 1000", lamp, "health", i32v(999)),
        ("radius of another type", lamp, "radius", i32v(20 << 16)),
        ("height other than 16", lamp, "height", i32v(0)),
        (
            "flags without MF_SOLID",
            lamp,
            "flags",
            (lamp_flags & !2).to_le_bytes().to_vec(),
        ),
        (
            "flags with MF_SHOOTABLE",
            lamp,
            "flags",
            (lamp_flags | 4).to_le_bytes().to_vec(),
        ),
        (
            "flags with MF_SOLID on gore",
            gibs,
            "flags",
            2u32.to_le_bytes().to_vec(),
        ),
        ("z above the floor", lamp, "z", i32v(floor + 1)),
        ("momx", lamp, "momx", i32v(1)),
        ("momy", lamp, "momy", i32v(-1)),
        ("momz", lamp, "momz", i32v(1 << 16)),
        ("a target", lamp, "target", target),
        ("reaction time", lamp, "reaction_time", i32v(7)),
        ("threshold", lamp, "threshold", i32v(1)),
        ("movedir", lamp, "movedir", vec![1]),
        ("movecount", lamp, "movecount", i32v(1)),
        ("a player", lamp, "player", vec![0]),
        ("pending removal", lamp, "pending_removal", vec![1]),
        (
            "gore in S_GIBS with the crunch height",
            gibs,
            "height",
            i32v(0),
        ),
        (
            "gore in S_GIBS with the crunch radius",
            gibs,
            "radius",
            i32v(0),
        ),
    ];
    for (what, slot, name, value) in cases {
        let path = format!("actor.{slot}.{name}");
        let forged = forge(bytes, |p, _| (p == path).then(|| value.clone()));
        assert_ne!(forged, bytes, "{what}");
        assert!(
            Diagnostic::decode_e1m1(&forged, limits).is_err(),
            "{what} accepted"
        );
    }
    // Adding MF_AMBUSH (an ambush THING record) is admitted.
    let path = format!("actor.{lamp}.flags");
    let ambush = forge(bytes, |p, _| {
        (p == path).then(|| (lamp_flags | 0x20).to_le_bytes().to_vec())
    });
    Diagnostic::decode_e1m1(&ambush, limits).unwrap();
}

/// Lump order: the decorations of `thinker.order` read their spawnpoints as a
/// subsequence of the THINGS lump. Swapping two decoration records of the lump, or
/// changing one's options, leaves the order unmatched.
#[test]
fn decode_e1m1_rejects_decorations_out_of_lump_order() {
    let limits = DiagnosticLimits::default();
    let bytes = fixture("m3h9_state_7_20.bin");
    let things = dsim_fields(bytes)
        .into_iter()
        .find(|(p, _)| p == "map.things")
        .unwrap()
        .1;
    // Records 0 and 2 are the lamp at (0, 128) and the pillar at (-160, -64).
    let record = |i: usize| things[10 * i..10 * i + 10].to_vec();
    assert_eq!(i16::from_le_bytes([things[6], things[7]]), 2028);
    assert_eq!(i16::from_le_bytes([things[26], things[27]]), 48);
    let mut swapped = things.clone();
    swapped[..10].copy_from_slice(&record(2));
    swapped[20..30].copy_from_slice(&record(0));
    let forged = forge(bytes, |p, _| (p == "map.things").then(|| swapped.clone()));
    assert_eq!(
        Diagnostic::decode_e1m1(&forged, limits).map(|_| ()),
        Err(DiagnosticError::Identity)
    );
    let mut options = things.clone();
    options[8] ^= 1;
    let forged = forge(bytes, |p, _| (p == "map.things").then(|| options.clone()));
    assert_eq!(
        Diagnostic::decode_e1m1(&forged, limits).map(|_| ()),
        Err(DiagnosticError::Identity)
    );
}

/// At most `MAX_DECORATIONS` per capture: 32 lamps and a health bonus load; the
/// bonus record rewritten into a 33rd lamp (type, state, sprite, frame, radius,
/// height, flags, health, tics) is refused with `DiagnosticError::Limit`.
#[test]
fn decode_e1m1_rejects_a_thirty_third_decoration() {
    let limits = DiagnosticLimits::default();
    let mut lumps = room(5);
    lumps.clear_things();
    for i in 0..32 {
        let (x, y) = (-480 + 64 * (i % 16) as i16, -340 + 64 * (i / 16) as i16);
        lumps.add_thing([x, y, 0, 2028, 7]);
    }
    lumps.add_thing([200, 0, 0, 2014, 7]);
    let mut s = e1m1(&lumps).unwrap();
    assert_eq!(s.spawn_map_things().unwrap().spawned.len(), 33);
    s.spawn_player_body(
        PlayerSlot::One,
        [Fixed::from_map_units(200), Fixed::from_map_units(100)],
    )
    .unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    let bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
    Diagnostic::decode_e1m1(&bytes, limits).unwrap();
    let bonus = slot_of(&bytes, OriginalActorType::MT_MISC2);
    let lamp = slot_of(&bytes, OriginalActorType::MT_MISC31);
    let fields = dsim_fields(&bytes);
    let copied: Vec<_> = [
        "type", "state", "sprite", "frame", "radius", "height", "flags", "health", "tics",
    ]
    .into_iter()
    .map(|name| {
        let from = format!("actor.{lamp}.{name}");
        (
            format!("actor.{bonus}.{name}"),
            fields.iter().find(|(p, _)| *p == from).unwrap().1.clone(),
        )
    })
    .collect();
    let forged = forge(&bytes, |p, _| {
        copied
            .iter()
            .find(|(path, _)| path == p)
            .map(|(_, v)| v.clone())
    });
    assert_eq!(
        Diagnostic::decode_e1m1(&forged, limits).map(|_| ()),
        Err(DiagnosticError::Limit)
    );
}
