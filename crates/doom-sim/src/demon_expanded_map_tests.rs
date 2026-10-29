//! Rust admission/atomicity policies; these are not fabricated original-C results.
use super::*;
#[path = "../../../tests/support/m3f2.rs"]
mod extent;
#[path = "../../../tests/support/m3f3.rs"]
mod fixtures;
fn map(label: &str) -> Map {
    let c = fixtures::cases()
        .into_iter()
        .find(|c| c.label == label)
        .unwrap();
    fixtures::map(c.v("map") as usize)
}
fn setup(label: &str) -> Simulation {
    let c = fixtures::cases()
        .into_iter()
        .find(|c| c.label == label)
        .unwrap();
    fixtures::setup(&c)
}
fn create(map: Map, limit: u32) -> Result<Simulation, SimError> {
    Simulation::with_demon_expanded_map_registry(
        map,
        limit,
        ActorRegistry::demon_map_context_fixture(),
        ActorOptions::default(),
        MapContextOptions::default(),
    )
}
fn atomic<T: std::fmt::Debug>(
    s: &mut Simulation,
    f: impl FnOnce(&mut Simulation) -> Result<T, SimError>,
) {
    let before = s.snapshot();
    assert!(f(s).is_err());
    assert_eq!(s.snapshot(), before);
}
fn changed_thing(angle: i16, filtered: bool) -> Map {
    let c = fixtures::cases()
        .into_iter()
        .find(|c| c.label == "multiple_lifecycle")
        .unwrap();
    let mut data = fixtures::map_bytes(c.v("map") as usize).to_vec();
    let directory = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    let count = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    let entry = (0..count)
        .map(|i| directory + 16 * i)
        .find(|&i| &data[i + 8..i + 16] == b"THINGS\0\0")
        .unwrap();
    let offset = u32::from_le_bytes(data[entry..entry + 4].try_into().unwrap()) as usize;
    data[offset + 14..offset + 16].copy_from_slice(&angle.to_le_bytes());
    if filtered {
        data[offset + 18..offset + 20].copy_from_slice(&0u16.to_le_bytes());
    }
    let wad = doom_wad::Wad::parse(&data, doom_wad::Limits::default()).unwrap();
    Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
#[test]
fn explicit_composition_preserves_both_historical_boundaries() {
    assert!(
        Simulation::with_demon_map_context_registry(
            map("long_lifecycle"),
            64,
            ActorRegistry::demon_map_context_fixture(),
            ActorOptions::default(),
            MapContextOptions::default()
        )
        .is_err()
    );
    let mut geometry = Simulation::map_extent(map("long_lifecycle")).unwrap();
    atomic(&mut geometry, |s| {
        s.spawn_type(ActorTypeId(136), [Fixed::ZERO; 2], SpawnHeight::Floor)
    });
    let mut composed = create(map("long_lifecycle"), 64).unwrap();
    assert_eq!(composed.profile(), Profile::DemonExpandedMap);
    assert_eq!(composed.snapshot().version(), 20);
    assert_eq!(composed.sectors().len(), 256);
    assert_eq!(composed.spawn_map_things().unwrap().spawned.len(), 1);
    atomic(&mut composed, |s| {
        s.spawn_type(ActorTypeId(136), [Fixed::ZERO; 2], SpawnHeight::Floor)
    });
}
#[test]
fn late_thing_angle_overflow_rejects_the_entire_batch() {
    for angle in [-225, 180] {
        let mut s = create(changed_thing(angle, false), 64).unwrap();
        let before = s.snapshot();
        assert_eq!(
            s.spawn_map_things(),
            Err(MapSpawnError::Angle(angle).into())
        );
        assert_eq!(s.snapshot(), before);
        assert_eq!(s.level_statistics().unwrap().total_kills, 0);
    }
    let mut s = create(changed_thing(180, true), 64).unwrap();
    assert_eq!(s.spawn_map_things().unwrap().spawned.len(), 1);
}
#[test]
fn capacity_and_statistics_overflow_reject_before_spawn_or_damage_rng() {
    let mut empty = create(map("multiple_lifecycle"), 1).unwrap();
    atomic(&mut empty, Simulation::spawn_map_things);
    let mut empty = create(map("long_lifecycle"), 64).unwrap();
    empty.statistics.as_mut().unwrap().total_kills = i32::MAX;
    atomic(&mut empty, Simulation::spawn_map_things);
    let mut s = setup("long_lifecycle");
    s.statistics.as_mut().unwrap().single_player_kill_count = i32::MAX;
    atomic(&mut s, |s| {
        s.damage_actor(DamageRequest {
            target: ActorId::from_parts(0, 1),
            source: None,
            inflictor: None,
            amount: 150,
        })
    });
}
fn eligible_two() -> Simulation {
    let mut s = setup("multiple_lifecycle");
    let owner = ActorId::from_parts(0, 1);
    let target = ActorId::from_parts(2, 1);
    s.set_actor_target(owner, Some(target)).unwrap();
    let life = s.actors.get_mut(owner).unwrap().life.as_mut().unwrap();
    life.state = Some(StateId(2));
    life.tics = 1;
    s
}
#[test]
fn invalid_later_owner_does_not_commit_an_earlier_eligible_owner() {
    let mut control = eligible_two();
    let rng = control.rng();
    control.step([TicCommand::default(); 4]).unwrap();
    assert_ne!(control.rng(), rng);
    let mut s = eligible_two();
    s.actors
        .get_mut(ActorId::from_parts(1, 1))
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .state = Some(StateId(999));
    atomic(&mut s, |s| s.step([TicCommand::default(); 4]));
}
#[test]
fn pending_stale_and_reused_targets_never_redirect_references() {
    let mut s = setup("long_lifecycle");
    let body = ActorId::from_parts(1, 1);
    s.set_actor_target(ActorId::from_parts(0, 1), Some(body))
        .unwrap();
    s.mark_remove(body).unwrap();
    atomic(&mut s, |s| s.step([TicCommand::default(); 4]));
    s.actors.run(64, |_, _| {}).unwrap();
    let new = s
        .spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(96), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(new.slot(), body.slot());
    assert_ne!(new.generation(), body.generation());
    assert_eq!(
        s.actor(ActorId::from_parts(0, 1))
            .unwrap()
            .life()
            .unwrap()
            .target,
        Some(body)
    );
    atomic(&mut s, |s| s.step([TicCommand::default(); 4]));
}
#[test]
fn invalid_sector_and_spatial_membership_are_recoverable() {
    let mut s = setup("long_lifecycle");
    s.actors
        .get_mut(ActorId::from_parts(0, 1))
        .unwrap()
        .location = Some(Location {
        subsector: 0,
        sector: 999,
    });
    atomic(&mut s, |s| s.step([TicCommand::default(); 4]));
    let mut s = setup("long_lifecycle");
    s.spatial.order.push(ActorId::from_parts(0, 1));
    atomic(&mut s, |s| s.step([TicCommand::default(); 4]));
}
#[test]
fn out_of_range_target_and_control_reject_without_mutation() {
    let mut s = setup("long_lifecycle");
    let owner = ActorId::from_parts(0, 1);
    atomic(&mut s, |s| {
        s.set_momentum(owner, [Fixed::from_map_units(65), Fixed::ZERO])
    });
    atomic(&mut s, |s| {
        s.check_multisector_position(owner, [Fixed::from_map_units(1025), Fixed::ZERO])
    });
    s.actors
        .get_mut(ActorId::from_parts(1, 1))
        .unwrap()
        .spec
        .position[0] = Fixed::from_map_units(1025);
    atomic(&mut s, |s| s.step([TicCommand::default(); 4]));
}
#[test]
fn owner_suffix_and_unsupported_state_actions_reject() {
    let mut s = setup("long_lifecycle");
    let owner = ActorId::from_parts(0, 1);
    let a = *s.actor(owner).unwrap();
    s.mark_remove(owner).unwrap();
    s.actors.run(64, |_, _| {}).unwrap();
    let id = s.actors.spawn(a).unwrap();
    s.spatial.link(id);
    atomic(&mut s, |s| s.step([TicCommand::default(); 4]));
    let mut s = setup("long_lifecycle");
    atomic(&mut s, |s| s.enter_actor_state(owner, StateId(500)));
    atomic(&mut s, |s| s.pos_attack(owner));
    atomic(&mut s, |s| s.request_actor_death(owner));
    atomic(&mut s, |s| s.bind_player(0, owner));
    atomic(&mut s, |s| {
        s.aim_line_attack(owner, 0, Fixed::from_map_units(64))
    });
}
#[test]
fn registry_modes_and_respawning_are_not_implicitly_admitted() {
    for mode in [ActorGameMode::Cooperative, ActorGameMode::Deathmatch] {
        assert!(
            Simulation::with_demon_expanded_map_registry(
                map("long_lifecycle"),
                64,
                ActorRegistry::demon_map_context_fixture(),
                ActorOptions {
                    mode,
                    ..Default::default()
                },
                MapContextOptions::default()
            )
            .is_err()
        );
    }
    assert!(
        Simulation::with_demon_expanded_map_registry(
            map("long_lifecycle"),
            64,
            ActorRegistry::demon_map_context_fixture(),
            ActorOptions {
                skill: Skill::Nightmare,
                ..Default::default()
            },
            MapContextOptions::default()
        )
        .is_err()
    );
    assert!(
        Simulation::with_demon_expanded_map_registry(
            map("long_lifecycle"),
            64,
            ActorRegistry::demon_map_context_fixture(),
            ActorOptions::default(),
            MapContextOptions {
                respawn_monsters: true,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut registry = ActorRegistry::demon_map_context_fixture();
    registry.types[136].properties[22] = 7;
    assert!(
        Simulation::with_demon_expanded_map_registry(
            map("long_lifecycle"),
            64,
            registry,
            ActorOptions::default(),
            MapContextOptions::default()
        )
        .is_err()
    );
    assert!(create(map("long_lifecycle"), 65).is_err());
}
fn clipping(two: bool) -> Simulation {
    let mut s = create(extent::source_clip_boundary_map(), 64).unwrap();
    let points = extent::source_clip_boundary_positions();
    let mut native = |position| {
        let a = lifecycle::make_actor(
            &s.world,
            &s.sectors,
            s.registry.as_deref().unwrap(),
            s.actor_options,
            ActorTypeId(136),
            position,
            SpawnHeight::Floor,
        );
        let id = s.actors.spawn(a).unwrap();
        s.spatial.link(id);
        id
    };
    if two {
        native([Fixed::from_map_units(-120), Fixed::from_map_units(-200)]);
    }
    let observer = native(points[0]);
    let target = s
        .spawn_type(ActorTypeId(135), points[1], SpawnHeight::Floor)
        .unwrap();
    s.set_perception_player(
        0,
        PerceptionPlayer {
            active: true,
            health: 100,
            actor: Some(target),
        },
    )
    .unwrap();
    s.actors
        .get_mut(observer)
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .tics = 1;
    if two {
        let a = s.actors.get_mut(ActorId::from_parts(0, 1)).unwrap();
        let l = a.life.as_mut().unwrap();
        l.state = Some(StateId(2));
        l.tics = 1;
        l.target = Some(target);
    }
    s
}
#[test]
fn undefined_clipping_is_excluded_from_direct_and_nested_ai_sight() {
    let mut s = clipping(false);
    let owner = ActorId::from_parts(0, 1);
    let target = ActorId::from_parts(1, 1);
    assert_eq!(
        s.check_sight(owner, target),
        Err(PerceptionError::Arithmetic.into())
    );
    atomic(&mut s, |s| s.look(owner));
    atomic(&mut s, |s| s.look_for_players(owner, true));
    atomic(&mut s, |s| s.step([TicCommand::default(); 4]));
}
#[test]
fn later_sight_failure_discards_earlier_movement_rng_events_and_commands() {
    let mut control = clipping(true);
    control
        .actors
        .get_mut(ActorId::from_parts(1, 1))
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .tics = 2;
    let before = control.rng();
    control.step([TicCommand::default(); 4]).unwrap();
    assert_ne!(control.rng(), before);
    let mut s = clipping(true);
    let mut commands = [TicCommand::default(); 4];
    commands[2].angle_turn = 7;
    let before = s.snapshot();
    assert_eq!(s.step(commands), Err(PerceptionError::Arithmetic.into()));
    assert_eq!(s.snapshot(), before);
}

#[test]
fn later_sight_failure_discards_planned_kill_credit_and_corpse_changes() {
    // Injected native actors isolate a Rust atomicity policy, not new C output.
    let killed = ActorId::from_parts(0, 1);
    let observer = ActorId::from_parts(1, 1);
    let request = DamageRequest {
        target: killed,
        source: None,
        inflictor: None,
        amount: 150,
    };
    let mut control = clipping(true);
    control.damage_actor(request).unwrap();
    assert_eq!(
        control.level_statistics().unwrap().single_player_kill_count,
        1
    );
    assert_eq!(
        control.actor(killed).unwrap().height(),
        Fixed::from_map_units(14)
    );
    assert_ne!(
        control.actor(killed).unwrap().life().unwrap().flags & 1048576,
        0
    );

    let mut s = clipping(true);
    let plan = s.plan_damage(request).unwrap();
    let before = s.snapshot();
    let result = s.checked_demon_operation(|host| {
        host.apply_damage(plan);
        assert_eq!(
            host.statistics.as_ref().unwrap().single_player_kill_count,
            1
        );
        assert_eq!(
            host.access.get(killed).unwrap().height(),
            Fixed::from_map_units(14)
        );
        host.invoke_look(observer);
    });
    assert_eq!(result, Err(PerceptionError::Arithmetic.into()));
    assert_eq!(s.snapshot(), before);
}

#[test]
fn direct_damage_retargeting_and_nested_chase_sight_are_atomic() {
    let owner = ActorId::from_parts(0, 1);
    let source = ActorId::from_parts(1, 1);
    let prepare = || {
        let mut s = clipping(false);
        // The next original draw is 220: damage fails the 180 pain threshold.
        s.rng = DoomRng::from_indices(2, 0);
        s.actors
            .get_mut(source)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .flags &= !4;
        s
    };
    let mut control = prepare();
    let result = control
        .damage_actor(DamageRequest {
            target: owner,
            source: None,
            inflictor: None,
            amount: 1,
        })
        .unwrap();
    assert!(!result.pain);
    assert_eq!(control.actor(owner).unwrap().life().unwrap().health, 149);
    assert_eq!(
        control.actor(owner).unwrap().life().unwrap().state,
        Some(StateId(1))
    );

    let mut s = prepare();
    atomic(&mut s, |s| {
        let result = s.damage_actor(DamageRequest {
            target: owner,
            source: Some(source),
            inflictor: None,
            amount: 1,
        });
        assert_eq!(result, Err(PerceptionError::Arithmetic.into()));
        result
    });
}
#[test]
fn expanded_sight_work_is_exercised_and_excess_rejected() {
    let mut s = setup("expanded_sight_dag");
    assert_eq!(s.world.extent_sight_work, Some(16382));
    s.look(ActorId::from_parts(0, 1)).unwrap();
    let mut data = extent::lumps(27);
    let n = data[7].1.len() / 28;
    let mut node = data[7].1[(n - 1) * 28..].to_vec();
    node[24..26].copy_from_slice(&((n - 1) as u16).to_le_bytes());
    node[26..28].copy_from_slice(&((n - 1) as u16).to_le_bytes());
    data[7].1.extend(node);
    assert!(create(extent::decode(&data).unwrap(), 64).is_err());
}
#[test]
fn pause_reads_independent_instances_and_restart_preserve_their_contracts() {
    let mut a = setup("long_lifecycle");
    let mut b = setup("long_lifecycle");
    a.set_paused(true);
    let old = a.snapshot();
    assert_eq!(
        a.step(
            [TicCommand {
                buttons: 255,
                ..Default::default()
            }; 4]
        )
        .unwrap(),
        old
    );
    a.set_paused(false);
    for _ in 0..12 {
        a.check_sight(ActorId::from_parts(0, 1), ActorId::from_parts(1, 1))
            .unwrap();
        assert_eq!(
            a.step([TicCommand::default(); 4]).unwrap(),
            b.step([TicCommand::default(); 4]).unwrap()
        );
    }
    let before = a.snapshot();
    let seed = a.rng();
    let fresh = a.restart_demon_map_context().unwrap();
    assert_eq!(a.snapshot(), before);
    assert_eq!(fresh.profile(), Profile::DemonExpandedMap);
    assert_eq!(fresh.level_statistics().unwrap().total_kills, 1);
    let mut expected = seed;
    expected.p_random();
    expected.p_random();
    assert_eq!(fresh.rng(), expected);
}
#[test]
fn full_actor_capacity_and_nonlethal_native_infighting_reject_atomically() {
    let mut s = setup("multiple_lifecycle");
    atomic(&mut s, |s| {
        s.damage_actor(DamageRequest {
            target: ActorId::from_parts(0, 1),
            source: Some(ActorId::from_parts(1, 1)),
            inflictor: None,
            amount: 1,
        })
    });
    while s.actor_order().count() < 64 {
        s.spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(-200); 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    }
    atomic(&mut s, |s| {
        s.spawn_type(ActorTypeId(135), [Fixed::ZERO; 2], SpawnHeight::Floor)
    });
}
#[test]
fn bounded_action_query_exhaustion_discards_the_entire_operation() {
    let mut s = setup("long_lifecycle");
    let before = s.snapshot();
    let result = s.checked_demon_operation(|h| {
        for _ in 0..65 {
            h.invoke_look(ActorId::from_parts(0, 1));
            if h.sight_failed() {
                break;
            }
        }
    });
    assert_eq!(result, Err(PerceptionError::WorkLimit.into()));
    assert_eq!(s.snapshot(), before);
}
