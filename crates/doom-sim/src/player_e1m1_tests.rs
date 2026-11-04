//! M3h9 E1M1 policy: its identity, the superset relation to the movers policy and
//! the DSIM36 version plumbing. Expected behavior comes from the original source
//! reading in M3H9_REFERENCE_INVESTIGATION.md and the committed C evidence, never
//! from Rust output.
use super::*;
use crate::diagonal_room::Lumps;
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
