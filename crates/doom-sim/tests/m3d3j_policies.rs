//! Rust admission/safety policies; no claims of C equivalence for safe rejection.
#[path = "../../../tests/support/m3d3j.rs"]
mod fixtures;
#[path = "../../../tests/support/m3c.rs"]
mod maps;
use doom_sim::*;
use doom_types::{DoomRng, Fixed};
fn id(n: u32) -> ActorId {
    ActorId::from_parts(n, 1)
}
fn empty(things: &[[i16; 5]], limit: u32, options: ActorOptions) -> Simulation {
    Simulation::with_demon_map_context_registry(
        maps::map(things),
        limit,
        ActorRegistry::demon_map_context_fixture(),
        options,
        MapContextOptions::default(),
    )
    .unwrap()
}
#[test]
fn late_invalid_setup_and_capacity_preserve_all_canonical_statistics() {
    for bad in [
        [0, 0, 0, 32000, 7],
        [0, 0, 0, 2011, 7],
        [0, 0, 0, 1, 7],
        [979, 0, 0, 3002, 7],
    ] {
        let mut s = empty(&[[-64, 0, 0, 3002, 7], bad], 64, ActorOptions::default());
        let before = s.snapshot();
        assert!(s.spawn_map_things().is_err());
        assert_eq!(s.snapshot(), before);
        assert_eq!(s.level_statistics(), Some(LevelStatistics::default()));
    }
    let mut s = empty(
        &[[-64, 0, 0, 3002, 7], [64, 0, 0, 3002, 7]],
        1,
        ActorOptions::default(),
    );
    let before = s.snapshot();
    assert!(s.spawn_map_things().is_err());
    assert_eq!(s.snapshot(), before);
    let mut valid = empty(&[[-64, 0, 0, 3002, 7]], 64, ActorOptions::default());
    valid.spawn_map_things().unwrap();
    assert_eq!(valid.level_statistics().unwrap().total_kills, 1);
}
#[test]
fn filters_and_initialized_totals_match_source_components() {
    let mut compared = 0;
    let mut policies = 0;
    for row in include_str!("../../../tests/reference/m3d3j_components.csv")
        .lines()
        .skip(1)
    {
        let v = row
            .split(',')
            .map(|v| v.parse::<i32>().unwrap())
            .collect::<Vec<_>>();
        let options = ActorOptions {
            skill: [
                Skill::Baby,
                Skill::Easy,
                Skill::Medium,
                Skill::Hard,
                Skill::Nightmare,
            ][v[0] as usize],
            mode: [
                ActorGameMode::SinglePlayer,
                ActorGameMode::Cooperative,
                ActorGameMode::Deathmatch,
            ][v[3] as usize],
            ..ActorOptions::default()
        };
        let result = Simulation::with_demon_map_context_registry(
            maps::map(&[[-32, 0, -91, v[2] as i16, v[1] as i16]]),
            64,
            ActorRegistry::demon_map_context_fixture(),
            options,
            MapContextOptions::default(),
        );
        if v[0] == 4 || v[3] != 0 {
            assert!(result.is_err());
            policies += 1;
            continue;
        }
        let mut s = result.unwrap();
        let before = s.snapshot();
        let report = s.spawn_map_things();
        if v[5] != 0 || v[6] != 0 && v[2] != 3002 {
            assert!(report.is_err());
            assert_eq!(s.snapshot(), before);
            policies += 1;
            continue;
        }
        assert_eq!(report.unwrap().spawned.len(), v[6] as usize);
        assert_eq!(s.rng().indices()[0], v[7] as u8);
        assert_eq!(
            s.level_statistics(),
            Some(LevelStatistics {
                total_kills: v[8],
                ..LevelStatistics::default()
            })
        );
        if let Some(a) = s.actor(id(0)) {
            assert_eq!(a.life().unwrap().flags, v[11] as u32);
        }
        compared += 1;
    }
    assert_eq!((compared, policies), (73, 50));
    let mut s = empty(
        &[[0, 0, 0, 3002, 7], [0, 0, 0, 3001, 7]],
        64,
        ActorOptions {
            no_monsters: true,
            ..ActorOptions::default()
        },
    );
    assert_eq!(s.spawn_map_things().unwrap().filtered, 2);
    assert_eq!(s.level_statistics(), Some(LevelStatistics::default()));
    assert_eq!(s.rng(), DoomRng::default());
}
#[test]
fn respawn_full_original_registry_and_real_player_dependencies_reject() {
    assert!(
        Simulation::with_demon_map_context_registry(
            maps::map(&[]),
            64,
            ActorRegistry::demon_map_context_fixture(),
            ActorOptions::default(),
            MapContextOptions {
                respawn_monsters: true,
                ..MapContextOptions::default()
            }
        )
        .is_err()
    );
    for registry in [
        ActorRegistry::vanilla(),
        ActorRegistry::demon_stand_run_fixture(),
    ] {
        assert!(
            Simulation::with_demon_map_context_registry(
                maps::map(&[]),
                64,
                registry,
                ActorOptions::default(),
                MapContextOptions::default()
            )
            .is_err()
        );
    }
    let mut s = empty(&[[0, 0, 0, 3002, 7]], 64, ActorOptions::default());
    s.spawn_map_things().unwrap();
    let before = s.snapshot();
    for ty in [12, 13] {
        assert!(
            s.spawn_type(ActorTypeId(ty), [Fixed::ZERO; 2], SpawnHeight::Floor)
                .is_err()
        );
        assert_eq!(s.snapshot(), before);
    }
    assert!(s.bind_player(0, id(0)).is_err());
    assert_eq!(s.snapshot(), before);
    assert!(s.request_actor_death(id(0)).is_err());
    assert_eq!(s.snapshot(), before);
    assert!(s.enter_actor_state(id(0), StateId(496)).is_err());
    assert_eq!(s.snapshot(), before);
    let mut p = PursuitSetup::from_actor(s.actor(id(0)).unwrap()).unwrap();
    p.flags &= !4194304;
    assert!(s.configure_pursuit_actor(id(0), p).is_err());
    assert_eq!(s.snapshot(), before);
}
#[test]
fn nonlethal_infighting_rejects_but_lethal_and_dead_source_credit_is_bounded() {
    let mut s = empty(
        &[[-64, 0, 0, 3002, 7], [64, 0, 0, 3002, 7]],
        64,
        ActorOptions::default(),
    );
    s.spawn_map_things().unwrap();
    let before = s.snapshot();
    assert!(
        s.damage_actor(DamageRequest {
            target: id(0),
            source: Some(id(1)),
            inflictor: None,
            amount: 1
        })
        .is_err()
    );
    assert_eq!(s.snapshot(), before);
    for (target, source) in [(id(0), id(1)), (id(1), id(0))] {
        s.damage_actor(DamageRequest {
            target,
            source: Some(source),
            inflictor: None,
            amount: 150,
        })
        .unwrap();
    }
    assert_eq!(s.level_statistics().unwrap().single_player_kill_count, 2);
    assert_eq!(s.level_statistics().unwrap().total_kills, 2);
}
#[test]
fn stale_pending_and_reused_sources_reject_without_redirecting_credit() {
    let mut s = empty(&[[0, 0, 0, 3002, 7]], 64, ActorOptions::default());
    s.spawn_map_things().unwrap();
    let old = s
        .spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(128), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    s.mark_remove(old).unwrap();
    let before = s.snapshot();
    assert!(
        s.damage_actor(DamageRequest {
            target: id(0),
            source: Some(old),
            inflictor: None,
            amount: 150
        })
        .is_err()
    );
    assert_eq!(s.snapshot(), before);
    s.step([TicCommand::default(); 4]).unwrap();
    let new = s
        .spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(128), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(old.slot(), new.slot());
    assert_ne!(old.generation(), new.generation());
    let before = s.snapshot();
    for (source, inflictor) in [(Some(old), None), (None, Some(old))] {
        assert!(
            s.damage_actor(DamageRequest {
                target: id(0),
                source,
                inflictor,
                amount: 150
            })
            .is_err()
        );
        assert_eq!(s.snapshot(), before);
    }
}
#[test]
fn actual_restart_matches_original_continuing_rng_episode_and_independent_bytes() {
    let cases = fixtures::cases();
    let before_case = cases.iter().find(|c| c.label == "restart_before").unwrap();
    let after_index = cases
        .iter()
        .position(|c| c.label == "restart_after")
        .unwrap();
    let after_case = &cases[after_index];
    let mut s = fixtures::setup(before_case);
    for op in 1..=before_case.v("tics") as usize {
        fixtures::advance(&mut s, before_case, op);
    }
    assert_eq!(s.rng().indices()[0], after_case.v("seed") as u8);
    assert_eq!(s.level_statistics().unwrap().single_player_kill_count, 1);
    let old = s.snapshot();
    let fresh = s.restart_demon_map_context().unwrap();
    assert_eq!(s.snapshot(), old);
    let mut fresh = fixtures::install_targets(fresh, after_case);
    assert_eq!(fresh.snapshot(), fixtures::setup(after_case).snapshot());
    let mut trace = fixtures::SourceTrace::for_case(after_case);
    trace.check(&fresh, after_index, 0, fresh.pending_events());
    fixtures::assert_golden(&fresh, after_index, 0);
    for op in 1..=after_case.v("tics") as usize {
        let pending = fresh.pending_events().len();
        fixtures::advance(&mut fresh, after_case, op);
        trace.check(&fresh, after_index, op, &fresh.events()[pending..]);
        fixtures::assert_golden(&fresh, after_index, op);
    }
}
#[test]
fn pause_presentation_and_independent_instances_keep_statistics_deterministic() {
    let cases = fixtures::cases();
    let c = cases
        .iter()
        .find(|c| c.label == "multiple_lifecycle")
        .unwrap();
    let mut a = fixtures::setup(c);
    let mut b = fixtures::setup(c);
    a.set_paused(true);
    let before = a.snapshot();
    for _ in 0..5 {
        a.step([TicCommand::default(); 4]).unwrap();
        assert_eq!(a.snapshot(), before);
    }
    a.set_paused(false);
    for op in 1..=c.v("tics") as usize {
        let _ = a.level_statistics();
        let _ = a.actor_order().collect::<Vec<_>>();
        let _ = a.rng();
        let _ = a.snapshot();
        fixtures::advance(&mut a, c, op);
        fixtures::advance(&mut b, c, op);
        assert_eq!(a.snapshot(), b.snapshot());
    }
}
#[test]
fn source_only_player_pointer_and_network_credit_components_are_not_runtime_admission() {
    let mut count = 0;
    for row in include_str!("../../../tests/reference/m3d3j_credit_components.csv")
        .lines()
        .skip(1)
    {
        let v = row
            .split(',')
            .map(|v| v.parse::<i32>().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(v[2], 0, "direct P_SpawnMobj does not add map totals");
        assert_eq!(v[3], i32::from(v[0] == 0 && v[1] != 2));
        assert_eq!(v[4], i32::from(v[1] == 2));
        assert_eq!(v[5], 5243906);
        assert_eq!(v[6], 14 * 65536);
        assert_eq!(v[10], 0);
        count += 1;
    }
    assert_eq!(count, 10);
}

#[test]
fn capacity_failure_matches_independently_specified_complete_v17_bytes() {
    for (map, limit) in [(0, 0), (1, 1)] {
        let mut sim = Simulation::with_demon_map_context_registry(
            fixtures::map(map),
            limit,
            ActorRegistry::demon_map_context_fixture(),
            ActorOptions::default(),
            MapContextOptions::default(),
        )
        .unwrap();
        let expected = if map == 0 {
            include_bytes!("../../../tests/reference/m3d3j_atomic_capacity_0.bin").as_slice()
        } else {
            include_bytes!("../../../tests/reference/m3d3j_atomic_capacity_1.bin").as_slice()
        };
        assert_eq!(sim.snapshot().canonical_bytes(), expected);
        assert!(sim.spawn_map_things().is_err());
        assert_eq!(sim.snapshot().canonical_bytes(), expected);
    }
}

#[test]
fn genuine_setup_fragments_clear_statistics_and_retain_both_rng_streams() {
    let row = include_str!("../../../tests/reference/m3d3j_setup.csv")
        .lines()
        .nth(1)
        .unwrap();
    let values = row
        .split(',')
        .map(|s| s.parse::<i32>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(&values[..8], &[0, 0, 0, 0, 180, 0, 211, 87]);
    assert!(values[8..].iter().all(|v| *v == 0));
    // Full wminfo/player accounting is source-component evidence only. Runtime
    // admits these four counters and existing owned streams, no fake players.
    let mut sim = Simulation::with_demon_map_context_registry(
        fixtures::map(0),
        64,
        ActorRegistry::demon_map_context_fixture(),
        ActorOptions::default(),
        MapContextOptions {
            initial_rng: DoomRng::from_indices(211, 87),
            respawn_monsters: false,
        },
    )
    .unwrap();
    assert_eq!(sim.level_statistics(), Some(LevelStatistics::default()));
    assert_eq!(sim.rng().indices(), [211, 87]);
    sim.spawn_map_things().unwrap();
    assert_eq!(sim.rng().indices(), [213, 87]);
    let fresh = sim.restart_demon_map_context().unwrap();
    assert_eq!(fresh.rng().indices(), [215, 87]);
    assert_eq!(
        fresh.level_statistics().unwrap().single_player_kill_count,
        0
    );
}
