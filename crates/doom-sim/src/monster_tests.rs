//! M3h1 former humans, mixed rosters, infighting and drops against compiled
//! original-C rows. Expected values never come from Rust output.
use super::*;
use crate::identity::OriginalActorType;

#[test]
fn monster_source_case_00_possessed_alone() {
    monster_evidence::verify_case(0);
}
#[test]
fn monster_source_case_01_shotguy_alone() {
    monster_evidence::verify_case(1);
}
#[test]
fn monster_source_case_02_chainguy_alone() {
    monster_evidence::verify_case(2);
}
#[test]
fn monster_source_case_03_possessed_killed_drop() {
    monster_evidence::verify_case(3);
}
#[test]
fn monster_source_case_04_shotguy_killed_drop() {
    monster_evidence::verify_case(4);
}
#[test]
fn monster_source_case_05_chainguy_killed_drop() {
    monster_evidence::verify_case(5);
}
#[test]
fn monster_source_case_06_mixed_group() {
    monster_evidence::verify_case(6);
}
#[test]
fn monster_source_case_07_infighting_shotguy_demon() {
    monster_evidence::verify_case(7);
}
#[test]
fn monster_source_case_08_infighting_chainguy_possessed() {
    monster_evidence::verify_case(8);
}
#[test]
fn monster_source_case_09_infighting_possessed_shotguy() {
    monster_evidence::verify_case(9);
}
#[test]
fn monster_source_case_10_player_death_mixed() {
    monster_evidence::verify_case(10);
}
#[test]
fn monster_source_case_11_roster_eight() {
    monster_evidence::verify_case(11);
}
#[test]
fn monster_source_case_12_pursuit_collision() {
    monster_evidence::verify_case(12);
}
#[test]
fn monster_source_case_13_drops_collected() {
    monster_evidence::verify_case(13);
}

fn map_with_things(things: &[[i16; 5]]) -> doom_map::Map {
    // Test-input assembly only: case-0 geometry with replaced THINGS.
    let bytes = monster_evidence::MAP_BYTES[0];
    let u32at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    let count = u32at(4);
    let directory = u32at(8);
    let mut output = vec![0; 12];
    output[..4].copy_from_slice(b"PWAD");
    let mut entries = Vec::new();
    for i in 0..count {
        let at = directory + i * 16;
        let name = &bytes[at + 8..at + 16];
        let offset = output.len();
        if name == b"THINGS\0\0" {
            for t in things {
                for v in t {
                    output.extend(v.to_le_bytes());
                }
            }
        } else {
            output.extend_from_slice(&bytes[u32at(at)..u32at(at) + u32at(at + 4)]);
        }
        entries.extend((offset as u32).to_le_bytes());
        entries.extend(((output.len() - offset) as u32).to_le_bytes());
        entries.extend_from_slice(name);
    }
    output[4..8].copy_from_slice(&(count as u32).to_le_bytes());
    let directory = output.len() as u32;
    output[8..12].copy_from_slice(&directory.to_le_bytes());
    output.extend(entries);
    let wad = doom_wad::Wad::parse(&output, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}

/// Advance a case until the first error; the failing call must leave the
/// complete canonical state, RNG and both event queues unchanged.
fn first_error_preserves_state(which: usize, limit: u32) -> (usize, SimError) {
    let c = &monster_evidence::cases()[which];
    let (mut s, _) = monster_evidence::setup_with_limit(c, limit);
    for op in 1..=c.tics {
        let before = s.clone();
        match s.advance(monster_evidence::commands(c, op)) {
            Ok(()) => {}
            Err(error) => {
                assert_eq!(s.snapshot(), before.snapshot(), "op{op} snapshot");
                assert_eq!(s.rng(), before.rng(), "op{op} rng");
                assert_eq!(s.events(), before.events(), "op{op} events");
                assert_eq!(s.pending_events(), before.pending_events());
                return (op, error);
            }
        }
    }
    panic!("{} never exhausted capacity", c.label);
}

#[test]
fn monster_hitscan_capacity_error_rolls_back_whole_tic() {
    // possessed_alone: two live actors and no free slot for the first impact.
    let (op, error) = first_error_preserves_state(0, 2);
    assert_eq!(error, SimError::Store(StoreError::Capacity));
    // The source shot happens at op44 in the committed C trace.
    assert_eq!(op, 44);
}

#[test]
fn monster_kill_drop_demand_rolls_back_whole_tic() {
    // possessed_killed_drop: one spare slot holds impacts until the lethal
    // bullet needs blood plus the P_KillMobj drop.
    let (op, error) = first_error_preserves_state(3, 3);
    assert_eq!(error, SimError::Store(StoreError::Capacity));
    assert!(op >= 15, "player fire starts at op15, got op{op}");
}

#[test]
fn monster_roster_cap_rejects_ninth_monster_atomically() {
    let things: Vec<[i16; 5]> = (0..9)
        .map(|i| {
            [
                -300 + 30 * (i % 3),
                -60 + 60 * (i / 3),
                0,
                [3004, 9, 65][i as usize % 3],
                7,
            ]
        })
        .collect();
    let mut s = Simulation::original_player_monsters_map(
        map_with_things(&things),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    let before = s.clone();
    assert_eq!(
        s.spawn_map_things().unwrap_err(),
        SimError::MapSpawn(MapSpawnError::WorkLimit)
    );
    assert_eq!(s.snapshot(), before.snapshot());
    assert_eq!(s.rng(), before.rng());
    let mut ok = Simulation::original_player_monsters_map(
        map_with_things(&things[..MAX_PLAYER_MONSTERS]),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    assert_eq!(
        ok.spawn_map_things().unwrap().spawned.len(),
        MAX_PLAYER_MONSTERS
    );
    assert_eq!(ok.level_statistics().unwrap().total_kills, 8);
}

#[test]
fn historical_policies_reject_former_humans() {
    for doomed in [3004, 9, 65] {
        let map = || map_with_things(&[[-300, 0, 0, doomed, 7]]);
        let mut bonuses =
            Simulation::original_player_bonuses_map(map(), 64, MapContextOptions::default())
                .unwrap();
        assert_eq!(
            bonuses.spawn_map_things().unwrap_err(),
            SimError::MapSpawn(MapSpawnError::UnsupportedType(doomed))
        );
        for profile in [
            OriginalExecutionProfile::PlayerDemonArmorV1,
            OriginalExecutionProfile::PlayerDemonBonusesV1,
            OriginalExecutionProfile::DemonNormalV1,
        ] {
            let registry = OriginalExecutionRegistry::new(profile).unwrap();
            for ty in [
                OriginalActorType::MT_POSSESSED,
                OriginalActorType::MT_SHOTGUY,
                OriginalActorType::MT_CHAINGUY,
            ] {
                assert_eq!(
                    registry.admit_definition(DefinitionIdentity::Original(ty)),
                    Err(OriginalRegistryError::Unsupported(
                        DefinitionIdentity::Original(ty)
                    ))
                );
            }
        }
    }
}

#[test]
fn monster_policy_rejects_raise_and_unadmitted_types() {
    let registry =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerMonstersV1).unwrap();
    for ty in [
        OriginalActorType::MT_POSSESSED,
        OriginalActorType::MT_SHOTGUY,
        OriginalActorType::MT_CHAINGUY,
    ] {
        let id = DefinitionIdentity::Original(ty);
        registry.admit_definition(id).unwrap();
        let roots = ty.states();
        for state in [
            roots.spawn,
            roots.see,
            roots.pain,
            roots.missile,
            roots.death,
            roots.extreme_death,
        ] {
            registry.admit_state(id, state).unwrap();
        }
        assert_eq!(
            registry.admit_state(id, roots.raise),
            Err(OriginalRegistryError::State(roots.raise.ordinal()))
        );
    }
    for ty in [
        OriginalActorType::MT_VILE,
        OriginalActorType::MT_TROOP,
        OriginalActorType::MT_WOLFSS,
    ] {
        assert!(
            registry
                .admit_definition(DefinitionIdentity::Original(ty))
                .is_err()
        );
    }
    let mut s = Simulation::original_player_monsters_map(
        map_with_things(&[[-300, 0, 0, 3001, 7]]),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    assert_eq!(
        s.spawn_map_things().unwrap_err(),
        SimError::MapSpawn(MapSpawnError::UnsupportedType(3001))
    );
}

#[test]
fn monster_damage_rejects_stale_and_foreign_handles() {
    let c = &monster_evidence::cases()[3];
    let (mut s, a) = monster_evidence::setup(c);
    let (other, _) = monster_evidence::setup(&monster_evidence::cases()[0]);
    let monster = s.actors.order().next().unwrap();
    let stale = ActorId::from_parts(monster.slot(), monster.generation() + 1);
    let before = s.clone();
    for target in [stale, ActorId::from_parts(40, 1)] {
        assert!(
            s.damage_actor(DamageRequest {
                target,
                inflictor: Some(a.body),
                source: Some(a.body),
                amount: 10,
            })
            .is_err()
        );
        assert_eq!(s.snapshot(), before.snapshot());
    }
    // A handle exported from one game instance never imports into another,
    // even for an identical-looking former-human slot and generation.
    let mut left = GameInstance::new(before);
    let mut right = GameInstance::new(other);
    let id = DefinitionIdentity::Original(OriginalActorType::MT_POSSESSED);
    let reference = left
        .with_execution_context(|ctx| {
            let actor = ctx.admit_local(monster, id)?;
            ctx.export_actor(actor)
        })
        .unwrap();
    let snapshot = right.simulation().snapshot();
    assert!(
        right
            .with_execution_context(|ctx| {
                ctx.import_actor(&reference, id)?;
                Ok(())
            })
            .is_err()
    );
    assert_eq!(right.simulation().snapshot(), snapshot);
}

#[test]
fn dsim28_reader_is_explicit_and_rejects_malformed_frames() {
    let c = &monster_evidence::cases()[11];
    let (mut s, _) = monster_evidence::setup(c);
    for op in 1..=40 {
        s.advance(monster_evidence::commands(c, op)).unwrap();
    }
    let bytes = s
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    let limits = DiagnosticLimits::default();
    let view = Diagnostic::decode_monsters(&bytes, limits).unwrap();
    assert_eq!(view.version(), 28);
    assert_eq!(view.encode(limits).unwrap(), bytes);
    assert!(Diagnostic::decode_bonuses(&bytes, limits).is_err());
    assert!(Diagnostic::decode_armor(&bytes, limits).is_err());
    assert!(Diagnostic::decode(&bytes, limits).is_err());
    // Patch one named field in place, keeping the framing intact.
    let patch = |path: &str, value: &[u8]| {
        let mut out = bytes.clone();
        let needle = [&(path.len() as u32).to_le_bytes()[..], path.as_bytes()].concat();
        let at = out.windows(needle.len()).position(|w| w == needle).unwrap() + needle.len();
        let len = u64::from_le_bytes(out[at..at + 8].try_into().unwrap()) as usize;
        assert_eq!(len, value.len());
        out[at + 8..at + 8 + len].copy_from_slice(value);
        out
    };
    for (path, value) in [
        ("level.total_kills", 9_i32.to_le_bytes().to_vec()),
        ("level.total_kills", 7_i32.to_le_bytes().to_vec()),
        (
            "level.single_player_kill_count",
            9_i32.to_le_bytes().to_vec(),
        ),
        ("profile", 11_u32.to_le_bytes().to_vec()),
    ] {
        assert!(
            Diagnostic::decode_monsters(&patch(path, &value), limits).is_err(),
            "{path}"
        );
    }
    // A former human in a v27 frame is not an admitted bonus-world identity.
    let mut v27 = bytes.clone();
    v27[4..8].copy_from_slice(&27_u32.to_le_bytes());
    assert!(Diagnostic::decode_bonuses(&v27, limits).is_err());
    // Bonus-world bytes still decode only through their own reader.
    let bonus_case = &bonus_evidence::cases()[0];
    let (bonus, _) = bonus_evidence::setup(bonus_case);
    let old = bonus.snapshot().diagnostic_bytes(limits).unwrap();
    assert!(Diagnostic::decode_bonuses(&old, limits).is_ok());
    assert!(Diagnostic::decode_monsters(&old, limits).is_err());
}

#[test]
fn monster_step_advance_and_debug_release_digest_is_stable() {
    // Every source case already checks step==advance per tic; also pin the
    // aggregate of final digests across all cases for determinism.
    let mut digest = 0;
    for (which, c) in monster_evidence::cases().iter().enumerate() {
        let (mut s, _) = monster_evidence::setup(c);
        for op in 1..=c.tics {
            s.advance(monster_evidence::commands(c, op)).unwrap();
        }
        digest ^= s.snapshot().digest().rotate_left(which as u32);
    }
    let again = monster_evidence::cases()
        .iter()
        .enumerate()
        .fold(0, |acc, (which, c)| {
            let (mut s, _) = monster_evidence::setup(c);
            for op in 1..=c.tics {
                s.advance(monster_evidence::commands(c, op)).unwrap();
            }
            acc ^ s.snapshot().digest().rotate_left(which as u32)
        });
    assert_eq!(digest, again);
}

#[test]
fn former_human_direct_entry_rejects_death_and_pain_states() {
    // Review regression: live former humans, like the Demon, may only be
    // entered directly into spawn/see states; death needs P_KillMobj.
    let c = &monster_evidence::cases()[0];
    let (mut s, _) = monster_evidence::setup(c);
    let possessed = s.actors.order().next().unwrap();
    let roots = OriginalActorType::MT_POSSESSED.states();
    for state in [roots.death, roots.extreme_death, roots.pain, roots.missile] {
        let before = s.clone();
        assert!(
            s.enter_original_state(possessed, state).is_err(),
            "{state:?}"
        );
        assert_eq!(s.snapshot(), before.snapshot());
    }
    s.enter_original_state(possessed, roots.see).unwrap();
}
