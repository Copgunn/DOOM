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

#[test]
fn isolated_original_c_filter_components_and_explicit_rust_exclusions() {
    let mut compared = 0;
    let mut policies = 0;
    for row in include_str!("../../../tests/reference/m3d3i_components.csv")
        .lines()
        .skip(1)
    {
        let v = row
            .split(',')
            .map(|s| s.parse::<i32>().unwrap())
            .collect::<Vec<_>>();
        let skill = [
            Skill::Baby,
            Skill::Easy,
            Skill::Medium,
            Skill::Hard,
            Skill::Nightmare,
        ][v[0] as usize];
        let mode = [
            ActorGameMode::SinglePlayer,
            ActorGameMode::Cooperative,
            ActorGameMode::Deathmatch,
        ][v[3] as usize];
        let things = [[-32, 0, -91, v[2] as i16, v[1] as i16]];
        let result = Simulation::with_demon_map_spawn_registry(
            maps::map(&things),
            64,
            ActorRegistry::demon_stand_run_fixture(),
            ActorOptions {
                skill,
                mode,
                ..ActorOptions::default()
            },
        );
        if skill == Skill::Nightmare || mode != ActorGameMode::SinglePlayer {
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
        let report = report.unwrap();
        assert_eq!(report.spawned.len(), v[6] as usize);
        assert_eq!(s.rng().indices()[0], v[7] as u8);
        if let Some(id) = report.spawned.first() {
            let a = s.actor(*id).unwrap();
            let l = a.life().unwrap();
            assert_eq!(l.reaction_time, v[9]);
            assert_eq!(l.state, Some(StateId(1)));
            assert_eq!(v[10], 475);
            assert_eq!(l.flags, v[11] as u32 & !4194304);
        }
        compared += 1;
    }
    println!(
        "Isolated C source filter/initial fields {compared}; Rust-only unsupported/fatal admission policies {policies}"
    );
    assert_eq!(compared + policies, 123);
    assert!(compared > 40);
}

#[test]
fn nomonsters_skips_recognized_other_monsters_but_not_unknown_types() {
    let options = ActorOptions {
        no_monsters: true,
        ..ActorOptions::default()
    };
    let mut s = empty(&[[0, 0, 0, 3001, 7], [0, 0, 0, 3006, 7]], 64, options);
    let report = s.spawn_map_things().unwrap();
    assert_eq!(report.filtered, 2);
    assert!(report.spawned.is_empty());
    let mut s = empty(&[[0, 0, 0, 32000, 7]], 64, options);
    let before = s.snapshot();
    assert!(s.spawn_map_things().is_err());
    assert_eq!(s.snapshot(), before);
}
#[test]
fn parsed_signed_coordinates_angles_and_all_option_bits_are_retained() {
    let mut s = empty(
        &[[-128, -64, -32768, 3002, -32753]],
        64,
        ActorOptions::default(),
    );
    let report = s.spawn_map_things().unwrap();
    let a = s.actor(report.spawned[0]).unwrap();
    let point = a.life().unwrap().spawnpoint.unwrap();
    assert_eq!(point.options, 0x800f);
    assert_eq!(point.angle, -32768);
    assert_eq!(
        a.position(),
        [Fixed::from_map_units(-128), Fixed::from_map_units(-64)]
    );
    assert_eq!(
        a.angle(),
        0u32.wrapping_sub(728u32.wrapping_mul(0x20000000))
    );
    assert_ne!(a.life().unwrap().flags & 32, 0);
}
#[test]
fn pause_presentation_and_independent_instances_preserve_map_gameplay() {
    let things = [[-80, -80, 0, 3002, 7], [80, -80, 180, 3002, 7]];
    let mut a = empty(&things, 64, ActorOptions::default());
    let mut b = empty(&things, 64, ActorOptions::default());
    assert_eq!(a.spawn_map_things(), b.spawn_map_things());
    for s in [&mut a, &mut b] {
        let target = body(s);
        s.set_perception_player(
            0,
            PerceptionPlayer {
                active: true,
                health: 100,
                actor: Some(target),
            },
        )
        .unwrap();
    }
    a.set_paused(true);
    let before = a.snapshot();
    a.step([TicCommand::default(); 4]).unwrap();
    assert_eq!(a.snapshot(), before);
    a.set_paused(false);
    for _ in 0..80 {
        let before = a.snapshot();
        let _ = a.actor_order().collect::<Vec<_>>();
        let _ = a.events();
        let _ = a.pending_events();
        let _ = a.check_sight(id(0), id(1));
        assert_eq!(a.snapshot(), before);
        a.step([TicCommand::default(); 4]).unwrap();
        b.step([TicCommand::default(); 4]).unwrap();
        assert_eq!(a.snapshot(), b.snapshot());
    }
}
#[test]
fn shared_damage_and_hitscan_admit_map_demons_and_reserve_real_effects() {
    let mut s = empty(
        &[[-64, 0, 0, 3002, 7], [32, 0, 180, 3002, 7]],
        64,
        ActorOptions::default(),
    );
    s.spawn_map_things().unwrap();
    let shooter = s
        .spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(96), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    let result = s
        .line_attack(
            shooter,
            LineAttack {
                angle: 0x80000000,
                range: Fixed::from_map_units(512),
                slope: Fixed::ZERO,
                damage: 150,
            },
        )
        .unwrap();
    assert_eq!(result.hit, Some(TraceObject::Actor(id(1))));
    assert!(result.effect.is_some());
    assert!(result.damage.unwrap().killed);
    assert!(s.actor(id(1)).unwrap().life().unwrap().health <= 0);
    for _ in 0..40 {
        s.step([TicCommand::default(); 4]).unwrap();
    }
    assert_eq!(
        s.actor(id(1)).unwrap().life().unwrap().state,
        Some(StateId(14))
    );
}
