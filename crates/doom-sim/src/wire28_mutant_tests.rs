//! Wire 28: a Rust-only test for two cargo-mutants survivors in doom-sim that
//! doom-game's tests also missed. The test names the mutants it kills; its
//! expected values come from the C text cited there, not from the code under test.
use crate::barrels_replay::room;
use crate::identity::OriginalActorType as T;
use crate::*;

/// Kills `diagnostic.rs:580:35` and `diagnostic.rs:581:35` (`&&` → `||` in
/// `Diagnostic::v40_skill`), which count a DSIM44 skill-2 frame at skill 0 or 1.
///
/// `P_SpawnMapThing` spawns a THING at skill 0 or 1 when its option bit 1 is set,
/// at skill 2 bit 2, at skill 3 bit 4 (p_mobj.c:743-751), and counts each spawned
/// COUNTKILL monster in `totalkills` (p_mobj.c:789-790; MT_TROOP is COUNTKILL,
/// info.c:1416). In the M3h9b hall of `wire28_tests` (map 6), one Imp with bit 1
/// and two with bit 2 only: skills 0 and 1 spawn one, skill 2 two, skill 3 none,
/// and the DSIM44 reader admits each capture with that `level.total_kills`. Under
/// either mutant the skill-2 capture is counted with bit 1 (`K` = 1 against
/// `total_kills` = 2) and refused with `Identity`. Read from the C, not observed:
/// no genuine-engine DSIM44 golden has a skill-2 map whose bit-1 and bit-2 counts
/// differ.
#[test]
fn dsim44_counts_a_skill_2_capture_with_thing_bit_2() {
    let mut l = room(6);
    l.clear_things();
    for thing in [
        [-300, 0, 0, 1, 7],
        [200, 100, 0, 3001, 1],
        [200, -100, 0, 3001, 2],
        [300, 0, 0, 3001, 2],
    ] {
        l.add_thing(thing);
    }
    for (skill, spawned) in [
        (Skill::Baby, 1),
        (Skill::Easy, 1),
        (Skill::Medium, 2),
        (Skill::Hard, 0),
    ] {
        let s = Simulation::original_player_episode1_v6_level(
            l.map(),
            1,
            skill,
            Default::default(),
            LevelStart::NewGame,
        )
        .unwrap();
        let imps = s
            .actor_order()
            .filter_map(|id| s.actor(id).and_then(Actor::life))
            .filter(|life| life.actor_type.0 == T::MT_TROOP.ordinal())
            .count();
        assert_eq!(imps, spawned, "{skill:?}");
        let bytes = s.snapshot().canonical_bytes();
        let got = Diagnostic::decode_episode1_v6(&bytes, DiagnosticLimits::default()).map(|d| {
            let kills = d.field("level.total_kills").unwrap();
            (d.version(), i32::from_le_bytes(kills.try_into().unwrap()))
        });
        assert_eq!(got, Ok((44, spawned as i32)), "{skill:?}");
    }
}
