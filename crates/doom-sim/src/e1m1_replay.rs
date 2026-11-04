//! The full replay of the committed M3h9 C scenarios (plan step 8): every one of
//! the 20 medium-skill scenarios (`m3h9_cases.csv`; `spawn_filter_easy` and
//! `spawn_filter_hard` are C-only rows of other skills) that the E1M1 policy loads
//! drives it through its declared setup and per-op commands, and at every op the C rows
//! (sector heights and `specialdata`, runtime line specials, use latch, player
//! state and exit, crossings, uses, use intercepts, `spechit`-blocked moves, the
//! thinker list) and the independent DSIM36 goldens (the PBE1 event frame, and at
//! every capture the digest, size, field count, the full bytes of the 15 pinned
//! fixtures and the v36 reader's round trip) are compared. The comparison bodies are
//! the M3h8 replay's ([`crate::movers_replay`]); expected values come only from the
//! C rows and the independent goldens, never from Rust output.
//!
//! Every medium scenario loads. `monster_hitscan_through_pillar` places its
//! Zombieman (radius 20) at (−536, −64) in a 48 × 48 alcove whose walls are four
//! units clear of its box; an earlier 48 × 40 alcove, whose walls the closed box
//! touched, was refused by the diagonal policies' spawn rule (center inside `Ω` and
//! the closed box meeting no wall, M3h5 note §4.1 item 4) while `P_SpawnMapThing`
//! checks no position, so the evidence was moved and the rule kept. The Rust-only
//! test below rebuilds that touching-box room in memory and asserts the refusal.
use crate::movers_replay::full::{
    Capture, Frames, Spec, Totals, parse_captures, parse_frames, replay_all_in,
};
use crate::movers_replay::{Case, Evidence, Rows, parse_cases, parse_rows, setup_map_with};
use crate::*;
use std::{collections::BTreeMap, sync::OnceLock};

/// Every M3h9 trace row keyed by (case, op).
pub(crate) fn rows() -> &'static Rows {
    static DATA: OnceLock<Rows> = OnceLock::new();
    DATA.get_or_init(|| parse_rows(include_bytes!("../../../tests/reference/m3h9_trace.dref")))
}
/// The 20 scenarios run at medium skill (`skill` 2). The two C-only census rows at
/// easy and hard are excluded, as the snapshot contract lists
/// (`c_only_scenarios_excluded`).
pub(crate) fn medium_cases() -> Vec<Case> {
    let csv = include_str!("../../../tests/reference/m3h9_cases.csv");
    let skills: Vec<_> = csv
        .lines()
        .skip(1)
        .map(|line| line.rsplit(',').nth(1).unwrap() == "2")
        .collect();
    parse_cases(csv, rows())
        .into_iter()
        .filter(|c| skills[c.which])
        .collect()
}
/// The 20 replayed scenarios: every one of [`medium_cases`].
pub(crate) fn cases() -> Vec<Case> {
    medium_cases()
}
/// The C driver's setup in the committed M3h9 room of `c` under the E1M1 policy.
pub(crate) fn setup(c: &Case) -> (Simulation, ActorId, Vec<ActorId>) {
    setup_map_with(
        c,
        crate::player_e1m1_tests::room(c.map).map(),
        Simulation::original_player_e1m1_map,
    )
}
/// The M3h9 E1M1 evidence.
pub(crate) const M3H9: Evidence = Evidence { rows, setup };

pub(crate) use crate::player_e1m1_tests::FIXTURES;
fn frames() -> &'static Frames {
    static DATA: OnceLock<Frames> = OnceLock::new();
    DATA.get_or_init(|| parse_frames(include_bytes!("../../../tests/reference/m3h9_events.dref")))
}
fn captures() -> BTreeMap<(usize, usize), Capture> {
    parse_captures(include_str!("../../../tests/reference/m3h9_snapshots.csv"))
}
/// The M3h9 set: DSIM36 read by `decode_e1m1`, rejected by `decode_movers`.
pub(crate) const M3H9_SPEC: Spec = Spec {
    evidence: M3H9,
    cases,
    frames,
    captures,
    fixtures: FIXTURES,
    decode: |b, l| Diagnostic::decode_e1m1(b, l),
    older: |b, l| Diagnostic::decode_movers(b, l),
    version: 36,
};

/// Replays every scenario Rust replays against the C rows and the goldens.
pub(crate) fn replay_all() -> Totals {
    replay_all_in(&M3H9_SPEC)
}

fn rows_named(tag: &str) -> usize {
    let replayed: Vec<_> = cases().iter().map(|c| c.which).collect();
    rows()
        .iter()
        .filter(|((which, _), _)| replayed.contains(which))
        .flat_map(|(_, r)| r)
        .filter(|(t, _)| t == tag)
        .count()
}

#[test]
fn full_replay_matches_the_c_rows_and_the_independent_dsim36_goldens() {
    let t = replay_all();
    println!(
        "player_e1m1 full replay: cases={} ops={} event_frames={} event_records={} captures={} fixtures={} thinker_rows={} crossings={} uses={} blocked={} digest={:016x}",
        t.cases,
        t.ops,
        t.event_frames,
        t.event_records,
        t.captures,
        t.fixtures,
        t.thinker_rows,
        t.partial.crossings,
        t.partial.uses,
        t.partial.blocked,
        t.digest
    );
    // Every replayed case, op, frame, capture, fixture and row was reached.
    let replayed: Vec<_> = cases().iter().map(|c| c.which).collect();
    let frames = frames()
        .keys()
        .filter(|(w, _)| replayed.contains(w))
        .count();
    let captures = captures()
        .keys()
        .filter(|(w, _)| replayed.contains(w))
        .count();
    assert_eq!(t.cases, 20);
    assert_eq!(t.ops, frames);
    assert_eq!(t.event_frames, frames);
    assert_eq!(t.captures, captures);
    assert_eq!(t.captures, 337);
    assert_eq!(t.fixtures, FIXTURES.len());
    assert_eq!(t.fixtures, 15);
    assert_eq!(t.thinker_rows, rows_named("TK"));
    assert_eq!(t.button_rows, 0);
    assert_eq!(t.partial.crossings, rows_named("MX"));
    assert_eq!(t.partial.blocked, rows_named("MP"));
    assert_eq!(t.partial.uses, rows_named("MU") + rows_named("MI"));
}

/// The closed-box spawn rule is unchanged under wire 20: the committed alcove room
/// (gallery map 7) with its four alcove vertices moved back to y = −84 and y = −44
/// (the 48 × 40 alcove of the first C evidence, rebuilt in memory) puts the
/// Zombieman's closed box at (−536, −64), radius 20, on both alcove walls, and the
/// spawn batch is refused with `MonsterError::Arithmetic`, nothing spawned and the
/// snapshot unchanged. The committed 48 × 48 room spawns all eleven THINGs.
#[test]
fn a_zombieman_box_touching_the_alcove_walls_is_refused_at_spawn() {
    let c = medium_cases()
        .into_iter()
        .find(|c| c.label == "monster_hitscan_through_pillar")
        .unwrap();
    let options = || MapContextOptions {
        initial_rng: doom_types::DoomRng::from_indices(c.seed, 0),
        respawn_monsters: false,
    };
    let committed = crate::player_e1m1_tests::room(c.map);
    let mut s = Simulation::original_player_e1m1_map(committed.map(), options()).unwrap();
    assert_eq!(s.spawn_map_things().unwrap().spawned.len(), 11);
    let mut touching = committed.clone();
    let mut moved = 0;
    let count = i16::try_from(
        touching
            .lumps
            .iter()
            .find(|(n, _)| n == "VERTEXES")
            .unwrap()
            .1
            .len()
            / 4,
    )
    .unwrap();
    for index in 0..count {
        let [x, y] = touching.vertex(index);
        let to = match (x, y) {
            (-512 | -560, -88) => -84,
            (-512 | -560, -40) => -44,
            _ => continue,
        };
        touching.set_vertex(index, [x, to]);
        moved += 1;
    }
    assert_eq!(moved, 4);
    let mut s = Simulation::original_player_e1m1_map(touching.map(), options()).unwrap();
    let before = s.snapshot();
    let rng = s.rng();
    assert_eq!(
        s.spawn_map_things().map(|_| ()),
        Err(SimError::Monster(MonsterError::Arithmetic))
    );
    assert_eq!(s.snapshot(), before);
    assert_eq!(s.rng(), rng);
    assert_eq!(s.actor_order().count(), 0);
}

fn medium_case(label: &str) -> Case {
    medium_cases()
        .into_iter()
        .find(|c| c.label == label)
        .unwrap()
}

/// `fireball_through_blocking_scroller` (review evidence): `PIT_CheckLine` skips the
/// `ML_BLOCKING` test for a missile (p_map.c:214–221), so the Imp's fireball crosses
/// the blocking 48 line of `scrollblock`: the replay reaches the last op, and the one
/// crossing the C rows record (`MX`, special 48) is the fireball's
/// `MoverEvent::Crossed` with the special unchanged (`P_CrossSpecialLine` returns at
/// once for `MT_TROOPSHOT`).
#[test]
fn a_fireball_crosses_a_blocking_two_sided_scroller() {
    let c = medium_case("fireball_through_blocking_scroller");
    let mut crossings = Vec::new();
    let (s, _) = crate::movers_replay::replay_in(&M3H9, &c, c.tics, |s, _| {
        for e in s.events() {
            if let Event::Mover(MoverEvent::Crossed {
                actor,
                line,
                before,
                after,
                ..
            }) = *e
            {
                crossings.push((actor, line, before, after));
            }
        }
    });
    assert_eq!(crossings.len(), 1);
    let (actor, line, before, after) = crossings[0];
    assert_eq!((before, after), (48, 48));
    assert_eq!(s.line_specials[usize::from(line)], 48);
    let ty = s.actor(actor).unwrap().life().unwrap().actor_type;
    assert_eq!(
        ty.0,
        crate::identity::OriginalActorType::MT_TROOPSHOT.ordinal()
    );
}

/// `pillar_straddles_door_step` (review evidence): the techno pillar's box straddles
/// the line between a door (floor 0) and a step (floor 8). The door closes fully onto
/// it (`PIT_ChangeSector` returns at the `MF_SHOOTABLE` test, p_map.c:1290–1294) and
/// `P_ThingHeightClip` leaves its floorz (8) above its ceilingz while the door is
/// shut, with `z == floorz`. Under wire 20 that inert state is admitted (the
/// decoration stays in V′ case (b)); the replay runs every op through the reopening
/// and the DSIM36 capture of a shut-door tic decodes.
#[test]
fn a_door_shut_on_a_straddling_pillar_leaves_its_floor_above_its_ceiling() {
    let c = medium_case("pillar_straddles_door_step");
    let mut inverted = Vec::new();
    let mut shut = None;
    let (s, _) = crate::movers_replay::replay_in(&M3H9, &c, c.tics, |s, op| {
        let a = s
            .actor_order()
            .filter_map(|id| s.actor(id))
            .find(|a| {
                a.life().is_some_and(|l| {
                    l.actor_type.0 == crate::identity::OriginalActorType::MT_MISC48.ordinal()
                })
            })
            .unwrap();
        if a.floor() > a.ceiling() {
            assert_eq!(a.z(), a.floor());
            assert_eq!(a.floor(), Fixed::from_map_units(8));
            inverted.push(op);
            if a.ceiling() == Fixed::ZERO && shut.is_none() {
                shut = Some(
                    s.snapshot()
                        .diagnostic_bytes(DiagnosticLimits::default())
                        .unwrap(),
                );
            }
        }
    });
    assert_eq!(s.tic(), c.tics as u64);
    // C: inverted from op 64 (ceiling 6) through the shut door (ops 68–89) to op 92
    // (ceiling 6 on the way up).
    assert_eq!(inverted.first(), Some(&64));
    assert_eq!(inverted.last(), Some(&92));
    assert_eq!(inverted.len(), 92 - 64 + 1);
    let bytes = shut.unwrap();
    Diagnostic::decode_e1m1(&bytes, DiagnosticLimits::default()).unwrap();
}
