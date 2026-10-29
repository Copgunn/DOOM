//! Rust-only bounded admission and atomicity; original-C evidence is separate.
#[path = "../../../tests/support/m3c.rs"]
mod maps;
use doom_sim::*;
use doom_types::{DoomRng, Fixed};
fn id(n: u32) -> ActorId {
    ActorId::from_parts(n, 1)
}
fn empty(things: &[[i16; 5]], limit: u32, options: ActorOptions) -> Simulation {
    Simulation::with_demon_map_spawn_registry(
        maps::map(things),
        limit,
        ActorRegistry::demon_stand_run_fixture(),
        options,
    )
    .unwrap()
}
fn body(s: &mut Simulation) -> ActorId {
    s.spawn_type(ActorTypeId(135), [Fixed::ZERO; 2], SpawnHeight::Floor)
        .unwrap()
}
#[test]
fn batch_is_atomic_for_late_unknown_unsupported_starts_and_bounds() {
    for bad in [
        [0, 0, 0, 32000, 7],
        [0, 0, 0, 2011, 7],
        [0, 0, 0, 1, 7],
        [0, 0, 0, 11, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, -1, 7],
        [979, 0, 0, 3002, 7],
    ] {
        let mut s = empty(&[[-64, 0, 0, 3002, 7], bad], 64, ActorOptions::default());
        let before = s.snapshot();
        assert!(s.spawn_map_things().is_err());
        assert_eq!(s.snapshot(), before);
    }
}
#[test]
fn capacity_and_native_count_fail_before_any_spawn_or_rng() {
    let things = [
        [-96, -96, 0, 3002, 7],
        [96, -96, 90, 3002, 7],
        [-96, 96, 180, 3002, 7],
        [96, 96, 270, 3002, 7],
    ];
    let mut s = empty(&things, 3, ActorOptions::default());
    let before = s.snapshot();
    assert!(s.spawn_map_things().is_err());
    assert_eq!(s.snapshot(), before);
    let mut five = things.to_vec();
    five.push([0, 0, 0, 3002, 7]);
    let mut s = empty(&five, 64, ActorOptions::default());
    let before = s.snapshot();
    assert_eq!(s.spawn_map_things(), Err(MapSpawnError::WorkLimit.into()));
    assert_eq!(s.snapshot(), before);
}
#[test]
fn source_skill_groups_and_single_player_filter_use_original_metadata() {
    for (skill, expected) in [
        (Skill::Baby, 2),
        (Skill::Easy, 2),
        (Skill::Medium, 2),
        (Skill::Hard, 2),
    ] {
        let mut s = empty(
            &[
                [-64, 0, 0, 3002, 1],
                [0, 0, 0, 3002, 2],
                [64, 0, 0, 3002, 4],
                [0, 64, 0, 3002, 7],
                [0, 0, 0, 3002, 23],
                [0, 0, 0, 32000, 0],
                [0, 0, 0, 2011, 16],
            ],
            64,
            ActorOptions {
                skill,
                ..ActorOptions::default()
            },
        );
        let report = s.spawn_map_things().unwrap();
        assert_eq!(report.spawned.len(), expected);
        assert_eq!(report.filtered, 5);
        assert_eq!(s.rng().indices()[0], 4);
    }
    let mut s = empty(
        &[[-64, 0, 0, 3002, 7]],
        64,
        ActorOptions {
            no_monsters: true,
            ..ActorOptions::default()
        },
    );
    let report = s.spawn_map_things().unwrap();
    assert_eq!(report.filtered, 1);
    assert!(report.spawned.is_empty());
    assert_eq!(s.rng(), DoomRng::default());
}
#[test]
fn unsupported_modes_nightmare_and_registry_reject() {
    for mode in [ActorGameMode::Cooperative, ActorGameMode::Deathmatch] {
        assert!(
            Simulation::with_demon_map_spawn_registry(
                maps::map(&[]),
                64,
                ActorRegistry::demon_stand_run_fixture(),
                ActorOptions {
                    mode,
                    ..ActorOptions::default()
                }
            )
            .is_err()
        );
    }
    assert!(
        Simulation::with_demon_map_spawn_registry(
            maps::map(&[]),
            64,
            ActorRegistry::demon_stand_run_fixture(),
            ActorOptions {
                skill: Skill::Nightmare,
                ..ActorOptions::default()
            }
        )
        .is_err()
    );
    assert!(
        Simulation::with_demon_map_spawn_registry(
            maps::map(&[]),
            64,
            ActorRegistry::vanilla(),
            ActorOptions::default()
        )
        .is_err()
    );
    assert!(
        Simulation::with_demon_map_spawn_registry(
            maps::map(&vec![[0, 0, 0, 3002, 0]; 257]),
            64,
            ActorRegistry::demon_stand_run_fixture(),
            ActorOptions::default()
        )
        .is_err()
    );
}
#[test]
fn raw_spawn_draws_twice_per_demon_and_does_not_execute_look() {
    let things = [[-64, -32, -1, 3002, 15], [64, 32, -45, 3002, 7]];
    let mut s = empty(&things, 64, ActorOptions::default());
    let report = s.spawn_map_things().unwrap();
    assert_eq!(report.spawned, vec![id(0), id(1)]);
    let mut rng = DoomRng::default();
    for (index, thing) in things.iter().enumerate() {
        let lastlook = rng.p_random() % 4;
        let timer = 1 + i32::from(rng.p_random()) % 10;
        let a = s.actor(id(index as u32)).unwrap();
        let l = a.life().unwrap();
        assert_eq!(a.lastlook(), lastlook);
        assert_eq!(l.tics, timer);
        assert_eq!(l.state, Some(StateId(1)));
        assert_eq!(l.spawnpoint.unwrap().angle, thing[2]);
        assert_eq!(l.health, 150);
        assert_eq!(l.reaction_time, 8);
        assert_eq!(l.target, None);
        assert_eq!(a.z(), a.constraints()[0]);
    }
    assert_eq!(s.rng(), rng);
    assert_eq!(s.actor(id(0)).unwrap().angle(), 0);
    assert_eq!(s.actor(id(1)).unwrap().angle(), 0xe0000000);
    assert_eq!(s.actor(id(0)).unwrap().life().unwrap().flags, 38);
    assert!(
        !s.pending_events()
            .iter()
            .any(|e| matches!(e, Event::ActorState(ActorStateEvent::Action { .. })))
    );
    assert_eq!(s.actor_order().collect::<Vec<_>>(), report.spawned);
}
#[test]
fn batch_requires_fresh_store_and_native_direct_spawn_rejects() {
    let mut s = empty(&[[-64, 0, 0, 3002, 7]], 64, ActorOptions::default());
    let before = s.snapshot();
    assert_eq!(
        s.spawn_type(ActorTypeId(136), [Fixed::ZERO; 2], SpawnHeight::Floor),
        Err(MapSpawnError::DirectSpawn.into())
    );
    assert_eq!(s.snapshot(), before);
    s.spawn_map_things().unwrap();
    let before = s.snapshot();
    assert_eq!(s.spawn_map_things(), Err(MapSpawnError::Setup.into()));
    assert_eq!(s.snapshot(), before);
    s.mark_remove(id(0)).unwrap();
    s.step([TicCommand::default(); 4]).unwrap();
    let before = s.snapshot();
    assert_eq!(s.spawn_map_things(), Err(MapSpawnError::Setup.into()));
    assert_eq!(s.snapshot(), before);
}
#[test]
fn simultaneous_strikes_use_sequential_rng_and_current_shared_body_health() {
    let mut s = empty(
        &[[-32, -32, 0, 3002, 7], [32, 32, 180, 3002, 7]],
        64,
        ActorOptions::default(),
    );
    s.spawn_map_things().unwrap();
    let target = body(&mut s);
    let mut p = PursuitSetup::from_actor(s.actor(target).unwrap()).unwrap();
    p.health = 1;
    s.configure_pursuit_actor(target, p).unwrap();
    for owner in [id(0), id(1)] {
        s.set_actor_target(owner, Some(target)).unwrap();
        s.enter_actor_state(owner, StateId(5)).unwrap();
    }
    for _ in 0..8 {
        s.step([TicCommand::default(); 4]).unwrap();
    }
    let damage = s
        .events()
        .iter()
        .filter(|e| matches!(e, Event::Combat(CombatEvent::Damage(_))))
        .count();
    assert_eq!(damage, 2);
    assert!(
        s.events()
            .contains(&Event::Combat(CombatEvent::Ignored(target)))
    );
    assert!(s.actor(target).unwrap().life().unwrap().health < 0);
    assert_eq!(
        s.events()
            .iter()
            .filter(|e| matches!(e, Event::Monster(MonsterEvent::Random { kind: 4, .. })))
            .count(),
        2
    );
}
#[test]
fn live_native_targets_roster_sound_and_cross_native_damage_reject_atomically() {
    let mut s = empty(
        &[[-64, 0, 0, 3002, 7], [64, 0, 0, 3002, 7]],
        64,
        ActorOptions::default(),
    );
    s.spawn_map_things().unwrap();
    let before = s.snapshot();
    assert!(s.set_actor_target(id(0), Some(id(1))).is_err());
    assert_eq!(s.snapshot(), before);
    assert!(
        s.set_perception_player(
            0,
            PerceptionPlayer {
                active: true,
                health: 100,
                actor: Some(id(1))
            }
        )
        .is_err()
    );
    assert_eq!(s.snapshot(), before);
    assert!(s.noise_alert(id(1), id(0)).is_err());
    assert_eq!(s.snapshot(), before);
    assert_eq!(
        s.damage_actor(DamageRequest {
            target: id(0),
            source: Some(id(1)),
            inflictor: Some(id(1)),
            amount: 1
        }),
        Err(MapSpawnError::Infighting.into())
    );
    assert_eq!(s.snapshot(), before);
    assert!(s.bind_player(0, id(0)).is_err());
    assert_eq!(s.snapshot(), before);
    assert!(s.enter_actor_state(id(0), StateId(496)).is_err());
    assert_eq!(s.snapshot(), before);
}
