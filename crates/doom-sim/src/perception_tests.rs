//! Perception safety/consistency policies; distinct from original-C equivalence.
use super::*;
#[path = "../../../tests/support/m3d3a.rs"]
mod fixtures;
fn scene(label: &str) -> Simulation {
    let c = fixtures::cases()
        .into_iter()
        .find(|c| c.label == label)
        .unwrap();
    fixtures::setup(&c)
}
fn id(slot: u32) -> ActorId {
    ActorId::from_parts(slot, 1)
}
fn unchanged<T: std::fmt::Debug>(
    sim: &mut Simulation,
    f: impl FnOnce(&mut Simulation) -> Result<T, SimError>,
) {
    let before = sim.snapshot();
    assert!(f(sim).is_err());
    assert_eq!(sim.snapshot(), before);
}
#[test]
fn invalid_profile_handles_roster_and_setup_are_atomic() {
    let mut s = scene("look_visual");
    unchanged(&mut s, |s| s.check_sight(id(999), id(1)));
    unchanged(&mut s, |s| s.look(id(999)));
    unchanged(&mut s, |s| {
        s.look_for_players(ActorId::from_parts(1, 2), false)
    });
    unchanged(&mut s, |s| {
        s.set_perception_player(4, PerceptionPlayer::default())
    });
    unchanged(&mut s, |s| {
        s.set_perception_player(
            0,
            PerceptionPlayer {
                active: true,
                health: 100,
                actor: None,
            },
        )
    });
    unchanged(&mut s, |s| {
        s.set_perception_player(
            0,
            PerceptionPlayer {
                active: true,
                health: 100,
                actor: Some(id(999)),
            },
        )
    });
    unchanged(&mut s, |s| s.configure_perception_observer(id(999), 1, 2));
    unchanged(&mut s, |s| s.noise_alert(id(1), id(999)));
    unchanged(&mut s, |s| s.bind_player(0, id(1)));
    unchanged(&mut s, |s| {
        s.spawn_type(
            ActorTypeId(0),
            [Fixed::from_map_units(1025), Fixed::ZERO],
            SpawnHeight::Floor,
        )
    });
    unchanged(&mut s, |s| {
        s.spawn_type(
            ActorTypeId(0),
            [Fixed::ZERO; 2],
            SpawnHeight::Absolute(Fixed::from_map_units(1025)),
        )
    });
    unchanged(&mut s, |s| {
        s.set_vertical_momentum(id(0), Fixed::from_map_units(65))
    });
    unchanged(&mut s, |s| {
        s.set_momentum(id(0), [Fixed::from_bits(i32::MAX), Fixed::ZERO])
    });
    let mut old = Simulation::actor_states(fixtures::map(0)).unwrap();
    unchanged(&mut old, |s| s.look(id(0)));
    unchanged(&mut old, |s| s.look_for_players(id(0), false));
    unchanged(&mut old, |s| {
        s.set_perception_player(0, PerceptionPlayer::default())
    });
}
#[test]
fn empty_roster_terminates_and_preserves_lastlook_and_old_target() {
    let mut s = scene("look_visual");
    for slot in 0..4 {
        s.set_perception_player(slot, PerceptionPlayer::default())
            .unwrap();
    }
    s.set_actor_target(id(0), Some(id(2))).unwrap();
    let last = s.actor(id(0)).unwrap().lastlook();
    let rng = s.rng();
    assert!(!s.look_for_players(id(0), false).unwrap());
    assert_eq!(s.actor(id(0)).unwrap().lastlook(), last);
    assert_eq!(s.actor(id(0)).unwrap().life().unwrap().target, Some(id(2)));
    assert_eq!(s.rng(), rng);
    assert!(
        s.pending_events()
            .iter()
            .any(|e| matches!(e, Event::Perception(PerceptionEvent::EmptyRoster(_))))
    );
    assert!(!s.look(id(0)).unwrap());
    assert_eq!(s.actor(id(0)).unwrap().life().unwrap().threshold, 0);
    assert_eq!(s.actor(id(0)).unwrap().lastlook(), last);
    assert_eq!(
        s.actor(id(0)).unwrap().life().unwrap().state,
        Some(StateId(1))
    );
    assert_eq!(s.rng(), rng);
}
#[test]
fn ambush_failed_empty_fallback_retains_assigned_sound_target() {
    let mut s = scene("look_ambush_blocked");
    for slot in 0..4 {
        s.set_perception_player(slot, PerceptionPlayer::default())
            .unwrap();
    }
    s.noise_alert(id(1), id(1)).unwrap();
    let rng = s.rng();
    assert!(!s.look(id(0)).unwrap());
    assert_eq!(s.actor(id(0)).unwrap().life().unwrap().target, Some(id(1)));
    assert_eq!(
        s.actor(id(0)).unwrap().life().unwrap().state,
        Some(StateId(1))
    );
    assert_eq!(s.rng(), rng);
}
#[test]
fn pending_and_reused_roster_handles_never_alias() {
    let mut s = scene("look_visual");
    s.mark_remove(id(1)).unwrap();
    unchanged(&mut s, |s| s.look(id(0)));
    unchanged(&mut s, |s| s.look_for_players(id(0), false));
    unchanged(&mut s, |s| s.step([TicCommand::default(); 4]));
    // A pure query depends only on its valid actors, not a broken roster.
    let before = s.snapshot();
    s.check_sight(id(0), id(2)).unwrap();
    assert_eq!(s.snapshot(), before);
    s.set_perception_player(0, PerceptionPlayer::default())
        .unwrap();
    s.step([TicCommand::default(); 4]).unwrap();
    let replacement = s
        .spawn_type(
            ActorTypeId(0),
            [Fixed::from_map_units(64), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(replacement, ActorId::from_parts(1, 2));
    assert_eq!(s.perception_players()[0].actor, None);
    assert!(s.actor(id(1)).is_none());
    unchanged(&mut s, |s| {
        s.set_perception_player(
            0,
            PerceptionPlayer {
                active: true,
                health: 100,
                actor: Some(id(1)),
            },
        )
    });
    unchanged(&mut s, |s| s.check_sight(id(0), id(1)));
    s.set_perception_player(
        0,
        PerceptionPlayer {
            active: true,
            health: 100,
            actor: Some(replacement),
        },
    )
    .unwrap();
    assert!(s.look(id(0)).unwrap());
    assert_eq!(
        s.actor(id(0)).unwrap().life().unwrap().target,
        Some(replacement)
    );
    let mut inactive = scene("look_visual");
    inactive.mark_remove(id(2)).unwrap();
    inactive.step([TicCommand::default(); 4]).unwrap();
    let replacement = inactive
        .spawn_type(ActorTypeId(0), [Fixed::ZERO; 2], SpawnHeight::Floor)
        .unwrap();
    assert_eq!(replacement, ActorId::from_parts(2, 2));
    assert_eq!(inactive.perception_players()[1].actor, Some(id(2)));
    unchanged(&mut inactive, |s| {
        s.set_perception_player(
            1,
            PerceptionPlayer {
                active: true,
                health: 100,
                actor: Some(id(2)),
            },
        )
    });
}
#[test]
fn stale_sound_rejects_before_threshold_rng_or_target_changes() {
    let mut s = scene("look_visual");
    s.noise_alert(id(1), id(1)).unwrap();
    s.mark_remove(id(1)).unwrap();
    s.set_perception_player(0, PerceptionPlayer::default())
        .unwrap();
    unchanged(&mut s, |s| s.look(id(0)));
    unchanged(&mut s, |s| s.step([TicCommand::default(); 4]));
    assert_eq!(s.actor(id(0)).unwrap().life().unwrap().threshold, 100);
    // Remove the observer too to retire both safely; retain the old cached identity.
    s.mark_remove(id(0)).unwrap();
    s.step([TicCommand::default(); 4]).unwrap();
    let caster = s
        .spawn_type(
            ActorTypeId(136),
            [Fixed::from_map_units(-64), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    let replacement = s
        .spawn_type(
            ActorTypeId(0),
            [Fixed::from_map_units(64), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(caster, ActorId::from_parts(0, 2));
    assert_eq!(replacement, ActorId::from_parts(1, 2));
    assert_eq!(s.sound_targets[0], Some(id(1)));
    unchanged(&mut s, |s| s.look(caster));
    s.noise_alert(replacement, caster).unwrap();
    assert_eq!(s.sound_targets[0], Some(replacement));
    assert!(s.look(caster).unwrap());
    assert_eq!(
        s.actor(caster).unwrap().life().unwrap().target,
        Some(replacement)
    );
}
#[test]
fn chase_recursive_handoff_and_unsupported_types_reject_before_execution() {
    let mut s = scene("look_visual");
    unchanged(&mut s, |s| s.enter_actor_state(id(0), StateId(176)));
    unchanged(&mut s, |s| {
        s.spawn_type(ActorTypeId(1), [Fixed::ZERO; 2], SpawnHeight::Floor)
    });
    let c = fixtures::cases()
        .into_iter()
        .find(|c| c.label == "look_visual")
        .unwrap();
    let r = fixtures::registry(&c);
    for (action, tics, next) in [
        (ActorAction::Original(ActionId(15)), 4, StateId(2)),
        (ActorAction::Original(ActionId(43)), 0, StateId(2)),
    ] {
        let mut states = r.states().to_vec();
        states[2].action = action;
        states[2].tics = tics;
        states[2].next = next;
        let r = ActorRegistry::new(states, r.types().to_vec()).unwrap();
        assert!(
            Simulation::with_perception_registry(fixtures::map(0), 64, r, ActorOptions::default())
                .is_err()
        );
    }
    let mut old = Simulation::actor_states(fixtures::map(0)).unwrap();
    let pawn = old
        .spawn_type(ActorTypeId(0), [Fixed::ZERO; 2], SpawnHeight::Floor)
        .unwrap();
    unchanged(&mut old, |s| s.enter_actor_state(pawn, StateId(174)));
}
#[test]
fn active_reference_states_cannot_remove_themselves_during_a_look_tic() {
    let mut s = scene("look_visual");
    unchanged(&mut s, |s| s.enter_actor_state(id(1), StateId(0)));
    unchanged(&mut s, |s| s.enter_actor_state(id(1), StateId(93)));
    let puff = s
        .spawn_type(ActorTypeId(37), [Fixed::ZERO; 2], SpawnHeight::Floor)
        .unwrap();
    unchanged(&mut s, |s| {
        s.set_perception_player(
            0,
            PerceptionPlayer {
                active: true,
                health: 100,
                actor: Some(puff),
            },
        )
    });
    // Damage/weapon/projectile profiles remain separate.
    unchanged(&mut s, |s| {
        s.damage_actor(DamageRequest {
            target: id(1),
            inflictor: Some(id(0)),
            source: Some(id(0)),
            amount: 1,
        })
    });
    unchanged(&mut s, |s| {
        s.launch_projectile(ProjectileRequest {
            source: id(0),
            actor_type: ActorTypeId(34),
            aim: ProjectileAim::Actor(id(1)),
        })
    });
}
#[test]
fn health_is_a_roster_field_and_first_divergence_names_it() {
    let mut s = scene("scan_front");
    let before = s.snapshot();
    let mut p = s.perception_players()[0];
    p.health = i32::MIN;
    s.set_perception_player(0, p).unwrap();
    assert_eq!(
        before.first_difference(&s.snapshot()).unwrap().field,
        "perception.player.0.health"
    );
    assert_eq!(s.actor(id(1)).unwrap().life().unwrap().health, 100);
    let rng = s.rng();
    assert!(!s.look_for_players(id(0), false).unwrap());
    assert_eq!(s.rng(), rng);
    p.health = 1;
    s.set_perception_player(0, p).unwrap();
    assert!(s.look_for_players(id(0), false).unwrap());
}
#[test]
fn pause_independent_instances_and_presentation_queries_are_deterministic() {
    let mut a = scene("thinker_wait_then_wake");
    let mut b = scene("thinker_wait_then_wake");
    for _ in 0..12 {
        let before = b.snapshot();
        for _ in 0..3 {
            b.check_sight(id(0), id(1)).unwrap();
            let _ = b.snapshot().canonical_bytes();
            let _ = b.events();
            let _ = b.perception_players();
        }
        assert_eq!(b.snapshot(), before);
        assert_eq!(
            a.step([TicCommand::default(); 4]).unwrap(),
            b.step([TicCommand::default(); 4]).unwrap()
        );
    }
    b.set_paused(true);
    let before = b.snapshot();
    assert_eq!(
        b.step(
            [TicCommand {
                buttons: 255,
                ..Default::default()
            }; 4]
        )
        .unwrap(),
        before
    );
    assert_eq!(
        a.snapshot().first_difference(&before).unwrap().field,
        "paused"
    );
    assert_eq!(a.snapshot().version(), 8);
}
#[test]
fn capacity_and_projected_arithmetic_step_limits_are_atomic() {
    let mut s = scene("look_visual");
    s.actors.max_slots = 5;
    unchanged(&mut s, |s| {
        s.spawn_type(ActorTypeId(0), [Fixed::ZERO; 2], SpawnHeight::Floor)
    });
    let mut t = Simulation::monster_perception(fixtures::map(0)).unwrap();
    let actor = t
        .spawn_type(
            ActorTypeId(0),
            [Fixed::from_map_units(1000), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    t.set_momentum(actor, [Fixed::ONE, Fixed::ZERO]).unwrap();
    unchanged(&mut t, |s| s.step([TicCommand::default(); 4]));
    let mut t = Simulation::monster_perception(fixtures::map(0)).unwrap();
    let actor = t
        .spawn_type(
            ActorTypeId(0),
            [Fixed::ZERO; 2],
            SpawnHeight::Absolute(Fixed::from_map_units(1000)),
        )
        .unwrap();
    t.set_vertical_momentum(actor, Fixed::from_map_units(64))
        .unwrap();
    unchanged(&mut t, |s| s.step([TicCommand::default(); 4]));
}
fn replace_lump(data: &[u8], wanted: &[u8], replacement: &[u8]) -> Vec<u8> {
    let count = i32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    let offset = i32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    let mut out = b"PWAD".to_vec();
    out.extend((count as i32).to_le_bytes());
    out.extend(0_i32.to_le_bytes());
    let mut directory = Vec::new();
    for i in 0..count {
        let entry = &data[offset + 16 * i..offset + 16 * (i + 1)];
        let start = i32::from_le_bytes(entry[..4].try_into().unwrap()) as usize;
        let size = i32::from_le_bytes(entry[4..8].try_into().unwrap()) as usize;
        let name = &entry[8..16];
        let payload = if name
            .iter()
            .copied()
            .take_while(|c| *c != 0)
            .collect::<Vec<_>>()
            == wanted
        {
            replacement
        } else {
            &data[start..start + size]
        };
        directory.extend((out.len() as i32).to_le_bytes());
        directory.extend((payload.len() as i32).to_le_bytes());
        directory.extend(name);
        out.extend(payload);
    }
    let offset = out.len() as i32;
    out[8..12].copy_from_slice(&offset.to_le_bytes());
    out.extend(directory);
    out
}
fn parse(data: &[u8]) -> Result<doom_map::Map, doom_map::MapError> {
    let wad = doom_wad::Wad::parse(data, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default())
}
fn nodes(count: usize, cycle: bool) -> Vec<u8> {
    let mut out = Vec::new();
    for i in 0..count {
        for v in [0_i16, 0, 0, 128, 64, -64, -128, 128, 64, -64, -128, 128] {
            out.extend(v.to_le_bytes());
        }
        let child = if cycle {
            0
        } else if i == 0 {
            0x8000
        } else {
            (i - 1) as u16
        };
        out.extend(child.to_le_bytes());
        out.extend(child.to_le_bytes());
    }
    out
}
#[test]
fn malformed_cycles_and_expanded_bsp_work_reject_without_unbounded_recursion() {
    let bytes = include_bytes!("../../../tests/reference/m3d3a_map_0.bin");
    let cyclic = replace_lump(bytes, b"NODES", &nodes(1, true));
    assert!(parse(&cyclic).is_err());
    let expanded = replace_lump(bytes, b"NODES", &nodes(14, false));
    let map = parse(&expanded).unwrap();
    assert_eq!(
        Simulation::monster_perception(map).unwrap_err(),
        SimError::Perception(PerceptionError::WorkLimit)
    );
    let mut vertices = Vec::new();
    for (x, y) in [(600_i16, -64_i16), (-128, 64), (128, -64), (128, 64)] {
        vertices.extend(x.to_le_bytes());
        vertices.extend(y.to_le_bytes());
    }
    let wide = replace_lump(bytes, b"VERTEXES", &vertices);
    let map = parse(&wide).unwrap();
    assert_eq!(
        Simulation::monster_perception(map).unwrap_err(),
        SimError::Perception(PerceptionError::Arithmetic)
    );
}
#[test]
fn crossing_into_a_stale_sound_sector_rejects_before_any_physical_movement() {
    let mut c = fixtures::cases()
        .into_iter()
        .find(|c| c.label == "noise_two_soundblocks")
        .unwrap();
    c.values[2] = -8 * 65536;
    c.values[15] = 1;
    let mut s = fixtures::setup(&c);
    s.noise_alert(id(1), id(1)).unwrap();
    assert_eq!(s.sound_targets[0], None);
    assert_eq!(s.sound_targets[1], Some(id(1)));
    s.set_perception_player(0, PerceptionPlayer::default())
        .unwrap();
    s.mark_remove(id(1)).unwrap();
    s.set_momentum(id(0), [Fixed::from_map_units(20), Fixed::ZERO])
        .unwrap();
    unchanged(&mut s, |s| s.step([TicCommand::default(); 4]));
    assert_eq!(
        s.actor(id(0)).unwrap().position()[0],
        Fixed::from_map_units(-8)
    );
}
