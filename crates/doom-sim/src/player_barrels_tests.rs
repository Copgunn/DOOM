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
