//! M3h9b barrels policy (wire 21): its identity, the superset relation to the E1M1
//! policy and the DSIM37 version plumbing (plan step 4). Expected behavior comes
//! from the original source reading in M3H9B_REFERENCE_INVESTIGATION.md and the
//! committed M3h9 C evidence, never from Rust output: a capture under wire 21 is
//! the wire-20 capture of the same scenario except the four fields that name the
//! policy and its slot count (decision F9).
use super::*;
use crate::e1m1_replay;
use crate::movers_replay::{self as replay, Case};
use crate::player_e1m1_tests::room;
use std::collections::BTreeMap;

const BARRELS: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersBarrelsV1;
const E1M1: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersE1M1V1;

/// The four fields in which a DSIM37 capture differs from the DSIM36 capture of
/// the same state (decision F9).
const POLICY_FIELDS: [&str; 4] = ["version", "profile", "context.schema", "store.max_slots"];

/// [`POLICY_FIELDS`], sorted as [`differing`] reports them.
fn policy_fields() -> Vec<String> {
    let mut out = POLICY_FIELDS.map(String::from).to_vec();
    out.sort();
    out
}

/// The capture as a field map, with the version as a pseudo-field.
fn field_map(snapshot: &Snapshot) -> BTreeMap<String, Vec<u8>> {
    let mut out: BTreeMap<String, Vec<u8>> = snapshot
        .fields()
        .map(|f| (f.path().to_string(), f.value().to_vec()))
        .collect();
    assert!(
        out.insert("version".into(), snapshot.version().to_le_bytes().to_vec())
            .is_none()
    );
    out
}

/// The fields of `a` and `b` that differ (present in one only, or unequal).
fn differing(a: &Snapshot, b: &Snapshot) -> Vec<String> {
    let (a, b) = (field_map(a), field_map(b));
    let mut keys: Vec<_> = a.keys().chain(b.keys()).cloned().collect();
    keys.sort();
    keys.dedup();
    keys.into_iter().filter(|k| a.get(k) != b.get(k)).collect()
}

fn barrels_setup(c: &Case) -> (Simulation, ActorId, Vec<ActorId>) {
    replay::setup_map_with(
        c,
        room(c.map).map(),
        Simulation::original_player_barrels_map,
    )
}

#[test]
fn barrels_policy_identity_is_wire_21_with_its_own_context_and_384_slots() {
    assert_eq!(BARRELS.wire_id(), 21);
    assert_eq!(BARRELS.context_schema(), PLAYER_BARRELS_CONTEXT_SCHEMA);
    assert_eq!(
        PLAYER_BARRELS_CONTEXT_SCHEMA,
        b"PLAYER-MONSTERS-BARRELS-1-MEDIUM-v1"
    );
    // A superset of the E1M1 policy: every Stage A, Stage B and E1M1 predicate.
    let p = BARRELS;
    assert!(p.has_barrels() && p.has_e1m1() && p.has_movers() && p.has_specials());
    assert!(p.has_real_map() && p.has_diagonals() && p.has_heights() && p.has_map_scale());
    assert!(p.has_imps() && p.has_monsters() && p.has_world_pickups());
    assert!(p.has_player_body() && p.has_physical_player() && p.has_player_weapons());
    assert!(p.has_player_armor() && p.has_player_bonuses() && p.has_weapon_pack());
    assert!(p.has_player_commands());
    // No older policy gains the barrels predicate; their identities are frozen.
    assert!(!E1M1.has_barrels());
    assert!(!OriginalExecutionProfile::PlayerMonstersMoversV1.has_barrels());
    assert_eq!(E1M1.wire_id(), 20);
    assert_eq!(E1M1.context_schema(), PLAYER_E1M1_CONTEXT_SCHEMA);
    assert_ne!(BARRELS.context_schema(), E1M1.context_schema());
    // The slot count is a policy value: 384 under wire 21 only.
    assert_eq!(actor_slots(BARRELS), BARRELS_ACTOR_SLOTS);
    assert_eq!(BARRELS_ACTOR_SLOTS, 384);
    assert_eq!(actor_slots(E1M1), MAP_SCALE_ACTOR_SLOTS);
    assert_eq!(MAP_SCALE_ACTOR_SLOTS, 320);
    let sim = Simulation::original_player_barrels_map(room(12).map(), replay_options()).unwrap();
    assert_eq!(sim.original_execution_profile(), Some(BARRELS));
    assert_eq!(sim.snapshot().version(), 37);
    assert_eq!(sim.actors.max_slots, 384);
    let old = crate::player_e1m1_tests::e1m1(&room(12)).unwrap();
    assert_eq!(old.actors.max_slots, 320);
    assert_eq!(sim.actors.visit_bound(), old.actors.visit_bound() + 64);
}

fn replay_options() -> MapContextOptions {
    crate::player_movers_tests::options()
}

/// The constructor demands exactly the policy's slot count: 320 or any other
/// limit is refused for wire 21, and 384 for wire 20.
#[test]
fn the_slot_count_is_an_equality_per_policy() {
    for (policy, wrong) in [(BARRELS, 320), (BARRELS, 385), (E1M1, 384), (E1M1, 321)] {
        let result = Simulation::with_original_registry(
            room(12).map(),
            wrong,
            OriginalExecutionRegistry::new(policy).unwrap(),
            ActorOptions::default(),
            replay_options(),
        );
        assert_eq!(
            result.map(|_| ()),
            Err(SimError::Store(StoreError::InvalidLimit)),
            "{policy:?} {wrong}"
        );
    }
}

/// Every committed M3h9 room loads under wire 21 exactly when it loads under wire
/// 20, with the same movers admission and line specials.
#[test]
fn every_committed_m3h9_room_loads_under_wire_21_as_under_wire_20() {
    for index in 0..15 {
        let lumps = room(index);
        let old = crate::player_e1m1_tests::e1m1(&lumps);
        let new = Simulation::original_player_barrels_map(lumps.map(), replay_options());
        assert_eq!(old.is_ok(), new.is_ok(), "room {index}");
        if let (Ok(old), Ok(new)) = (old, new) {
            assert_eq!(old.movers, new.movers, "room {index}");
            assert_eq!(old.line_specials, new.line_specials, "room {index}");
            assert_eq!(new.snapshot().version(), 37);
        }
    }
}

/// Every committed M3h9 medium scenario replayed under wire 21 produces, at every
/// op, the wire-20 capture except `version` (37), `profile` (21), `context.schema`
/// and `store.max_slots` (384), and the same event stream. Each DSIM37 capture is
/// read by `decode_barrels` alone, round-trips, and is rejected by `decode_e1m1`
/// and `decode_movers` with `DiagnosticError::Version(37)`.
#[test]
fn barrels_replays_every_m3h9_scenario_with_the_dsim36_layout() {
    let limits = DiagnosticLimits::default();
    let mut captures = 0;
    let cases = e1m1_replay::cases();
    let ops: usize = cases.iter().map(|c| c.tics + 1).sum();
    assert_eq!(cases.len(), 20);
    for c in cases {
        let (mut old, old_body, old_things) = e1m1_replay::setup(&c);
        let (mut new, new_body, new_things) = barrels_setup(&c);
        assert_eq!(
            (old_body, &old_things),
            (new_body, &new_things),
            "{}",
            c.label
        );
        for op in 0..=c.tics {
            if op != 0 {
                replay::controlled(&mut old, &c, old_body, &old_things, op);
                replay::controlled(&mut new, &c, new_body, &new_things, op);
                let command = replay::command_in(e1m1_replay::rows(), &c, op);
                old.advance(command)
                    .unwrap_or_else(|e| panic!("{} op{op} e1m1: {e}", c.label));
                new.advance(command)
                    .unwrap_or_else(|e| panic!("{} op{op} barrels: {e}", c.label));
            }
            let (a, b) = (old.snapshot(), new.snapshot());
            assert_eq!((a.version(), b.version()), (36, 37), "{} op{op}", c.label);
            assert_eq!(differing(&a, &b), policy_fields(), "{} op{op}", c.label);
            assert_eq!(old.events(), new.events(), "{} op{op}", c.label);
            assert_eq!(old.pending_events(), new.pending_events());
            let bytes = b.diagnostic_bytes(limits).unwrap();
            let view = Diagnostic::decode_barrels(&bytes, limits)
                .unwrap_or_else(|e| panic!("{} op{op}: {e:?}", c.label));
            assert_eq!(view.field("profile"), Some(21u32.to_le_bytes().as_slice()));
            assert_eq!(
                view.field("context.schema"),
                Some(PLAYER_BARRELS_CONTEXT_SCHEMA)
            );
            assert_eq!(
                view.field("store.max_slots"),
                Some(384u32.to_le_bytes().as_slice())
            );
            assert_eq!(view.encode(limits).unwrap(), bytes);
            for reader in [Diagnostic::decode_e1m1, Diagnostic::decode_movers] {
                assert_eq!(
                    reader(&bytes, limits).map(|_| ()),
                    Err(DiagnosticError::Version(37))
                );
            }
            captures += 1;
        }
    }
    // Every op of every scenario, op 0 (the setup) included.
    assert_eq!(captures, ops);
    assert!(captures > 1000, "{captures} captures compared");
}

/// DSIM37 is read by `decode_barrels` alone; every older reader rejects it, and
/// `decode_barrels` rejects 36. A wire-20 capture relabeled 37 fails the schema
/// check, and a `store.max_slots` of 385 exceeds the DSIM37 bound while 384 is
/// the bound itself; a DSIM36 capture never admits 321.
#[test]
fn dsim37_is_rejected_by_every_older_reader_and_bounds_its_slots_at_384() {
    let limits = DiagnosticLimits::default();
    let c = e1m1_replay::cases().remove(0);
    let (mut new, ..) = barrels_setup(&c);
    new.advance([TicCommand::default(); 4]).unwrap();
    let v37 = new.snapshot().diagnostic_bytes(limits).unwrap();
    let (mut old, ..) = e1m1_replay::setup(&c);
    old.advance([TicCommand::default(); 4]).unwrap();
    let v36 = old.snapshot().diagnostic_bytes(limits).unwrap();
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
        Diagnostic::decode_e1m1,
    ] {
        assert_eq!(
            reader(&v37, limits).map(|_| ()),
            Err(DiagnosticError::Version(37))
        );
    }
    assert_eq!(
        Diagnostic::decode_barrels(&v36, limits).map(|_| ()),
        Err(DiagnosticError::Version(36))
    );
    let mut forged = v36.clone();
    forged[4..8].copy_from_slice(&37u32.to_le_bytes());
    assert_eq!(
        Diagnostic::decode_barrels(&forged, limits).map(|_| ()),
        Err(DiagnosticError::Schema)
    );
    let patch = |bytes: &[u8], path: &str, value: &[u8]| {
        let mut out = bytes.to_vec();
        let needle = [&(path.len() as u32).to_le_bytes()[..], path.as_bytes()].concat();
        let at = out.windows(needle.len()).position(|w| w == needle).unwrap() + needle.len();
        let len = u64::from_le_bytes(out[at..at + 8].try_into().unwrap()) as usize;
        assert_eq!(len, value.len());
        out[at + 8..at + 8 + len].copy_from_slice(value);
        out
    };
    assert_eq!(
        Diagnostic::decode_barrels(
            &patch(&v37, "store.max_slots", &385u32.to_le_bytes()),
            limits
        )
        .map(|_| ()),
        Err(DiagnosticError::Limit)
    );
    assert_eq!(
        Diagnostic::decode_e1m1(
            &patch(&v36, "store.max_slots", &321u32.to_le_bytes()),
            limits
        )
        .map(|_| ()),
        Err(DiagnosticError::Limit)
    );
    Diagnostic::decode_e1m1(&v36, limits).unwrap();
    Diagnostic::decode_barrels(&v37, limits).unwrap();
}

// --- Plan step 5: pickups to 64, every THING angle, the spawn rule S′ ---------------

const IMP: i16 = 3001;
const FLOOR_LAMP: i16 = 2028;
const HEALTH_BONUS: i16 = 2014;

/// A committed M3h9 room with its THINGs replaced by `things`
/// (`[x, y, angle, type, options]`).
fn with_things(index: usize, things: &[[i16; 5]]) -> crate::diagonal_room::Lumps {
    let mut lumps = room(index);
    lumps.clear_things();
    for thing in things {
        lumps.add_thing(*thing);
    }
    lumps
}

/// `spawn_map_things` under `construct`: the spawned count, or the error with the
/// canonical state, the RNG and the actor list unchanged.
fn spawn_under(
    lumps: &crate::diagonal_room::Lumps,
    construct: fn(doom_map::Map, MapContextOptions) -> Result<Simulation, OriginalRegistryError>,
) -> Result<Vec<ActorId>, SimError> {
    let mut s = construct(lumps.map(), replay_options()).unwrap();
    let (before, rng) = (s.snapshot(), s.rng());
    let result = s.spawn_map_things().map(|r| r.spawned);
    if result.is_err() {
        assert_eq!(s.snapshot(), before);
        assert_eq!(s.rng(), rng);
        assert_eq!(s.actor_order().count(), 0);
    }
    result
}

fn barrels(lumps: &crate::diagonal_room::Lumps) -> Result<Vec<ActorId>, SimError> {
    spawn_under(lumps, Simulation::original_player_barrels_map)
}
fn e1m1(lumps: &crate::diagonal_room::Lumps) -> Result<Vec<ActorId>, SimError> {
    spawn_under(lumps, Simulation::original_player_e1m1_map)
}

/// The gallery room (map 5): its west wall `x = −512` is one-sided from `y = −384`
/// to the alcove at `y = −88`; every THING below stands at `y = −200`, so its box
/// meets that wall alone. `d` is the center's distance in front of the wall.
const GALLERY: usize = 5;
const WALL_X: i16 = -512;
const ARITHMETIC: SimError = SimError::Monster(MonsterError::Arithmetic);

/// S′ for a mover (decision F3): an Imp (radius 20) whose box overlaps the wall
/// by 4 units has `d` 16 ≥ 31 − 20 = 11 and spawns under wire 21; `d` 11 is the
/// bound itself; `d` 10 is refused. Under wire 20 the closed-box rule refuses
/// every overlap, and the Imp at `d` 16 too.
#[test]
fn s_prime_admits_an_imp_overlapping_a_wall_by_its_margin_only() {
    let at = |d: i16| with_things(GALLERY, &[[WALL_X + d, -200, 0, IMP, 7]]);
    assert_eq!(barrels(&at(16)).unwrap().len(), 1);
    assert_eq!(barrels(&at(11)).unwrap().len(), 1);
    assert_eq!(barrels(&at(10)), Err(ARITHMETIC));
    assert_eq!(barrels(&at(1)), Err(ARITHMETIC));
    assert_eq!(e1m1(&at(16)), Err(ARITHMETIC));
    assert_eq!(e1m1(&at(20)), Err(ARITHMETIC));
    // Clear of the wall (closed box one unit away), both policies spawn it.
    assert_eq!(e1m1(&at(21)).unwrap().len(), 1);
    assert_eq!(barrels(&at(21)).unwrap().len(), 1);
}

/// S′ for the player body (radius 16, margin 15): overlapping the wall by 4 units
/// (`d` 12) is refused by `player_spawn_guard`; `d` 15 is admitted under wire 21
/// and refused under wire 20 (its closed box overlaps).
#[test]
fn s_prime_refuses_a_player_body_embedded_below_its_margin() {
    let body = |construct: fn(
        doom_map::Map,
        MapContextOptions,
    ) -> Result<Simulation, OriginalRegistryError>,
                d: i16| {
        let mut s = construct(room(GALLERY).map(), replay_options()).unwrap();
        s.spawn_map_things().unwrap();
        let before = s.snapshot();
        let result = s.spawn_player_body(
            PlayerSlot::One,
            [WALL_X + d, -200].map(Fixed::from_map_units),
        );
        if result.is_err() {
            assert_eq!(s.snapshot(), before);
        }
        result.map(|_| ())
    };
    let binding = Err(SimError::PlayerBody(PlayerBodyError::Binding));
    assert_eq!(body(Simulation::original_player_barrels_map, 12), binding);
    assert_eq!(body(Simulation::original_player_barrels_map, 14), binding);
    assert_eq!(body(Simulation::original_player_barrels_map, 15), Ok(()));
    assert_eq!(body(Simulation::original_player_e1m1_map, 15), binding);
    assert_eq!(body(Simulation::original_player_e1m1_map, 16), binding);
    assert_eq!(body(Simulation::original_player_e1m1_map, 17), Ok(()));
}

/// Statics need only their center (decision F3): a floor lamp (radius 16) with the
/// wall exactly on its box edge and a health bonus (radius 20) overlapping the wall
/// by 8 units spawn under wire 21; wire 20 refuses both.
#[test]
fn s_prime_admits_statics_touching_or_overlapping_a_wall() {
    let lamp = with_things(GALLERY, &[[WALL_X + 16, -200, 0, FLOOR_LAMP, 7]]);
    let bonus = with_things(GALLERY, &[[WALL_X + 12, -200, 0, HEALTH_BONUS, 7]]);
    let both = with_things(
        GALLERY,
        &[
            [WALL_X + 16, -200, 0, FLOOR_LAMP, 7],
            [WALL_X + 1, -260, 0, HEALTH_BONUS, 7],
        ],
    );
    assert_eq!(barrels(&lamp).unwrap().len(), 1);
    assert_eq!(barrels(&bonus).unwrap().len(), 1);
    assert_eq!(barrels(&both).unwrap().len(), 2);
    assert_eq!(e1m1(&lamp), Err(ARITHMETIC));
    assert_eq!(e1m1(&bonus), Err(ARITHMETIC));
}

/// The center must have odd parity under every kind: a health bonus at (−540,
/// −200) is inside the vertex rectangle (the alcove reaches `x = −560`) but outside
/// the walls (even parity) and is refused; so is a center outside the rectangle,
/// and a controlled dropped pickup outside the walls.
#[test]
fn s_prime_refuses_a_center_with_even_parity_or_outside_the_rectangle() {
    let outside = with_things(GALLERY, &[[-540, -200, 0, HEALTH_BONUS, 7]]);
    assert_eq!(barrels(&outside), Err(ARITHMETIC));
    let beyond = with_things(GALLERY, &[[-570, -64, 0, HEALTH_BONUS, 7]]);
    assert_eq!(barrels(&beyond), Err(ARITHMETIC));
    // Inside the alcove (odd parity) the same bonus spawns.
    let alcove = with_things(GALLERY, &[[-540, -64, 0, HEALTH_BONUS, 7]]);
    assert_eq!(barrels(&alcove).unwrap().len(), 1);
    let mut s =
        Simulation::original_player_barrels_map(room(GALLERY).map(), replay_options()).unwrap();
    s.spawn_map_things().unwrap();
    let before = s.snapshot();
    assert_eq!(
        s.spawn_dropped_pickup(PickupKind::Clip, [-540, -200].map(Fixed::from_map_units))
            .map(|_| ()),
        Err(ARITHMETIC)
    );
    assert_eq!(s.snapshot(), before);
    // A dropped clip overlapping the wall is a static: admitted under wire 21.
    s.spawn_dropped_pickup(PickupKind::Clip, [-508, -200].map(Fixed::from_map_units))
        .unwrap();
}

/// The open-box test is strict and the margin exact to the raw unit: on the census
/// room's west wall (`x = −512`, full height) a radius-20 mover with the wall on its
/// box edge is admitted (the open box is clear), the margin 11 is admitted, one raw
/// unit (1/65536 map unit) less is refused, and a static needs only its center.
#[test]
fn s_prime_margin_is_exact_to_the_raw_unit() {
    let p = |x: i16, y: i16| [x, y].map(Fixed::from_map_units);
    let r20 = Fixed::from_map_units(20);
    let boundary = Boundary::derive(&room(12).map()).unwrap();
    assert!(boundary.spawn_admitted_relaxed(p(0, 0), r20, true));
    assert!(boundary.spawn_admitted_relaxed(p(-492, 0), r20, true)); // edge contact
    assert!(boundary.spawn_admitted_relaxed(p(-501, 0), r20, true)); // d 11
    assert!(!boundary.spawn_admitted_relaxed(p(-502, 0), r20, true)); // d 10
    assert!(boundary.spawn_admitted_relaxed(p(-502, 0), r20, false)); // static
    // One raw unit short of the margin is refused; the margin itself admitted.
    let raw = |x: i32| [Fixed::from_bits(x), Fixed::ZERO];
    let margin = (-512 + 11) << 16;
    assert!(boundary.spawn_admitted_relaxed(raw(margin), r20, true));
    assert!(!boundary.spawn_admitted_relaxed(raw(margin - 1), r20, true));
    // Outside the walls: refused for every kind.
    assert!(!boundary.spawn_admitted_relaxed(p(-520, 0), r20, false));
}

/// Every THING angle is admitted under wire 21 (decision F8) and converted as
/// p_mobj.c:794 does: `ANG45 × (angle / 45)` modulo 2³², i.e. `(q mod 8)·ANG45` for
/// the truncated quotient `q`. Under wire 20 an angle of 180 (octant 4) is still
/// `MapSpawnError::Angle` with the state unchanged.
#[test]
fn every_thing_angle_spawns_under_wire_21_with_the_source_angle_word() {
    let angles: [i16; 17] = [
        0, 45, 90, 135, 180, 225, 270, 315, 30, 100, 359, -1, -45, -100, 400, 32767, -32768,
    ];
    let things: Vec<[i16; 5]> = angles
        .iter()
        .enumerate()
        .map(|(i, a)| [-400 + 40 * i as i16, 0, *a, HEALTH_BONUS, 7])
        .collect();
    let lumps = with_things(12, &things);
    let mut s = Simulation::original_player_barrels_map(lumps.map(), replay_options()).unwrap();
    let spawned = s.spawn_map_things().unwrap().spawned;
    assert_eq!(spawned.len(), angles.len());
    for (id, angle) in spawned.iter().zip(angles) {
        let q = i32::from(angle) / 45;
        let expected = (q.rem_euclid(8) as u32) * 0x2000_0000;
        assert_eq!(s.actor(*id).unwrap().angle(), expected, "angle {angle}");
    }
    assert_eq!(
        e1m1(&with_things(12, &[[0, 0, 180, HEALTH_BONUS, 7]])),
        Err(SimError::MapSpawn(MapSpawnError::Angle(180)))
    );
    assert_eq!(
        e1m1(&with_things(12, &[[0, 0, -225, HEALTH_BONUS, 7]])),
        Err(SimError::MapSpawn(MapSpawnError::Angle(-225)))
    );
    assert_eq!(
        e1m1(&with_things(12, &[[0, 0, 135, HEALTH_BONUS, 7]]))
            .unwrap()
            .len(),
        1
    );
}

/// The pickup bound is a policy value (decision F2): 64 placed pickups spawn under
/// wire 21 and a 65th is `MapSpawnError::WorkLimit`; wire 20 keeps 16 and refuses
/// the 17th, nothing spawned.
#[test]
fn sixty_four_pickups_spawn_under_wire_21_and_sixteen_under_wire_20() {
    let grid = |n: usize| -> Vec<[i16; 5]> {
        (0..n)
            .map(|i| {
                let (col, row) = ((i % 10) as i16, (i / 10) as i16);
                [-400 + 64 * col, -320 + 64 * row, 0, HEALTH_BONUS, 7]
            })
            .collect()
    };
    let limit = Err(SimError::MapSpawn(MapSpawnError::WorkLimit));
    assert_eq!(barrels(&with_things(12, &grid(64))).unwrap().len(), 64);
    assert_eq!(barrels(&with_things(12, &grid(65))), limit);
    assert_eq!(e1m1(&with_things(12, &grid(16))).unwrap().len(), 16);
    assert_eq!(e1m1(&with_things(12, &grid(17))), limit);
    assert_eq!(world_pickups::max_pickups(BARRELS), 64);
    assert_eq!(world_pickups::max_pickups(E1M1), MAX_WORLD_PICKUPS);
    assert_eq!(world_pickups::max_bonus(BARRELS), 384);
    assert_eq!(world_pickups::max_bonus(E1M1), 96);
    // A 64-pickup world advances and its capture is read by the DSIM37 reader.
    let mut s =
        Simulation::original_player_barrels_map(with_things(12, &grid(64)).map(), replay_options())
            .unwrap();
    s.spawn_map_things().unwrap();
    s.spawn_player_body(PlayerSlot::One, [-448, 352].map(Fixed::from_map_units))
        .unwrap();
    // The census scenario's weapons (`spawn_filter_medium`).
    let c = e1m1_replay::cases()
        .into_iter()
        .find(|c| c.label == "spawn_filter_medium")
        .unwrap();
    s.initialize_player_weapons(
        PlayerSlot::One,
        WeaponInventory {
            ready: c.ready,
            owned: c.owned,
            ammo: [c.clips, 0, 0, 0],
        },
    )
    .unwrap();
    s.spawn_level_specials().unwrap();
    s.advance([TicCommand::default(); 4]).unwrap();
    let limits = DiagnosticLimits::default();
    let bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
    let view = Diagnostic::decode_barrels(&bytes, limits).unwrap();
    assert_eq!(
        view.field("level.total_items"),
        Some(64i32.to_le_bytes().as_slice())
    );
}

/// The §5.3 budget formulas with the values of the barrels policy (M3h9b note).
#[test]
fn barrels_budget_formulas_hold() {
    let (big_k, p, r, d, b) = (
        MAX_PLAYER_MONSTERS,
        world_pickups::BARRELS_MAX_PICKUPS,
        MAX_PLAYER_MONSTERS,
        MAX_DECORATIONS,
        MAX_BARRELS,
    );
    assert_eq!((big_k, p, r, d, b), (8, 64, 8, 32, 16));
    assert_eq!(BARRELS_LINKED_THINGS, 1 + big_k + p + r + d + b);
    assert_eq!(BARRELS_LINKED_THINGS, 129);
    let live = |k: usize| 1 + big_k + p + 26 * k + (big_k - k) + 8 * (big_k - k) + r + d + b;
    let spawns = |k: usize| 39 - 3 * k;
    assert_eq!(live(8), 337);
    assert_eq!(live(0), 201);
    assert_eq!(live(8) + spawns(8), 352);
    assert_eq!(live(0) + spawns(0), 240);
    let slots = BARRELS_ACTOR_SLOTS as usize;
    assert!(live(8) + 39 <= slots);
    for k in 0..=big_k {
        assert!(live(k) <= live(8) && live(k) + spawns(k) <= slots, "k {k}");
    }
    assert_eq!(world_pickups::max_bonus(BARRELS), 6 * p as i32);
}
