//! Rust safety policies for the CPOS closure, distinct from original-C behavior.
#[path = "../../../tests/support/m3d3e.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
fn id(slot: u32) -> ActorId {
    ActorId::from_parts(slot, 1)
}
fn setup(label: &str) -> Simulation {
    fixtures::setup(
        &fixtures::cases()
            .into_iter()
            .find(|c| c.label == label)
            .unwrap(),
    )
}
fn reject<T: std::fmt::Debug>(
    s: &mut Simulation,
    f: impl FnOnce(&mut Simulation) -> Result<T, SimError>,
) {
    let before = s.snapshot();
    assert!(f(s).is_err());
    assert_eq!(s.snapshot(), before);
}
fn fill(s: &mut Simulation, count: usize) {
    while s.actor_order().count() < count {
        s.spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(-200), Fixed::from_map_units(-96)],
            SpawnHeight::Floor,
        )
        .unwrap();
    }
}
#[test]
fn single_impact_capacity_and_delayed_shots_reject_atomically() {
    let mut s = setup("ordinary");
    fill(&mut s, 64);
    reject(&mut s, |s| s.chaingunner_attack(id(0)));
    for state in [5, 6] {
        reject(&mut s, |s| s.enter_actor_state(id(0), StateId(state)));
    }
    s.mark_remove(id(63)).unwrap();
    reject(&mut s, |s| s.chaingunner_attack(id(0)));
    let mut s = setup("ordinary");
    fill(&mut s, 63);
    s.chaingunner_attack(id(0)).unwrap();
    assert_eq!(s.actor_order().count(), 64);
    let mut s = setup("ordinary");
    s.enter_actor_state(id(0), StateId(4)).unwrap();
    for _ in 0..9 {
        s.step([TicCommand::default(); 4]).unwrap();
    }
    fill(&mut s, 64);
    reject(&mut s, |s| s.step([TicCommand::default(); 4]));
}
#[test]
fn misses_null_and_nonallocating_refire_work_at_capacity() {
    let mut s = setup("all_miss_open");
    fill(&mut s, 64);
    s.chaingunner_attack(id(0)).unwrap();
    assert_eq!(s.actor_order().count(), 64);
    let mut s = setup("null");
    fill(&mut s, 64);
    let rng = s.rng();
    let start = s.pending_events().len();
    s.chaingunner_attack(id(0)).unwrap();
    assert_eq!(rng, s.rng());
    assert_eq!(s.pending_events().len(), start + 1);
    s.chaingunner_refire(id(0)).unwrap();
    assert_eq!(s.actor_order().count(), 64);
    let mut s = setup("refire_wall_occluded");
    fill(&mut s, 64);
    s.chaingunner_refire(id(0)).unwrap();
    assert_eq!(s.actor_order().count(), 64);
}
#[test]
fn cpos_timing_keeps_two_separately_aimed_shots_and_one_tic_refire() {
    let mut s = setup("ordinary");
    s.chase(id(0)).unwrap();
    for _ in 0..9 {
        s.step([TicCommand::default(); 4]).unwrap();
        assert_eq!(
            s.actor(id(0)).unwrap().life().unwrap().state,
            Some(StateId(4))
        );
    }
    for (ticks, state, timer, frame) in [
        (1, 5, 4, 32773),
        (4, 6, 4, 32772),
        (4, 7, 1, 5),
        (1, 5, 4, 32773),
    ] {
        for _ in 0..ticks {
            s.step([TicCommand::default(); 4]).unwrap();
        }
        let l = s.actor(id(0)).unwrap().life().unwrap();
        assert_eq!(
            (l.state, l.tics, l.sprite, l.frame),
            (Some(StateId(state)), timer, 38, frame)
        );
    }
    let c = fixtures::cases()
        .into_iter()
        .find(|c| c.label == "ordinary")
        .unwrap();
    let r = fixtures::registry(&c);
    assert!(!r.action_supported(ActionId(13)));
    assert!(!r.action_supported(ActionId(14)));
    for (index, tics, frame, action, next) in [
        (5, 10, 32773, 13, 6),
        (6, 4, 4, 13, 7),
        (7, 1, 5, 14, 2),
        (7, 0, 5, 14, 5),
        (5, 4, 32773, 55, 6),
        (7, 1, 5, 56, 5),
    ] {
        let mut states = r.states().to_vec();
        states[index].tics = tics;
        states[index].frame = frame;
        states[index].action = ActorAction::Original(ActionId(action));
        states[index].next = StateId(next);
        let reg = ActorRegistry::new(states, r.types().to_vec()).unwrap();
        assert!(
            Simulation::with_chaingunner_registry(
                fixtures::map(0),
                64,
                reg,
                ActorOptions::default()
            )
            .is_err()
        );
    }
}
#[test]
fn original_lut_threshold_and_health_short_circuit_ignore_roster_health() {
    for (label, roll, wanted, shadow) in [
        (
            "refire_r39_shadow0_hp0",
            39,
            RefireDecision::EarlyContinue,
            false,
        ),
        (
            "refire_r41_shadow0_hp0",
            41,
            RefireDecision::DeadTarget,
            false,
        ),
        (
            "refire_r39_shadow1_hp0",
            39,
            RefireDecision::EarlyContinue,
            true,
        ),
        (
            "refire_body_alive_roster_dead",
            41,
            RefireDecision::VisibleContinue,
            false,
        ),
    ] {
        let mut s = setup(label);
        let start = s.pending_events().len();
        s.chaingunner_refire(id(0)).unwrap();
        let es = &s.pending_events()[start..];
        let end = es
            .iter()
            .position(|e| matches!(e, Event::Refire(_)))
            .unwrap();
        assert!(matches!(&es[end],Event::Refire(e) if e.decision==wanted));
        let draws = es[..end]
            .iter()
            .filter_map(|e| match e {
                Event::Monster(MonsterEvent::Random { kind, value, .. }) => Some((*kind, *value)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(draws.last(), Some(&(6, roll)));
        assert_eq!(draws.len(), if shadow { 3 } else { 1 });
        let sights = es[..end]
            .iter()
            .filter(|e| matches!(e, Event::Perception(PerceptionEvent::Sight { .. })))
            .count();
        assert_eq!(
            sights,
            usize::from(wanted == RefireDecision::VisibleContinue)
        );
    }
}
#[test]
fn refire_exit_runs_nested_look_chase_movement_with_safe_counter_bounds() {
    let mut s = setup("refire_dead_sound_reacquire");
    s.noise_alert(id(2), id(0)).unwrap();
    let mut p = PursuitSetup::from_actor(s.actor(id(0)).unwrap()).unwrap();
    p.reaction_time = i32::MIN + 2;
    s.configure_pursuit_actor(id(0), p).unwrap();
    let position = s.actor(id(0)).unwrap().position();
    let start = s.pending_events().len();
    s.chaingunner_refire(id(0)).unwrap();
    let es = &s.pending_events()[start..];
    let decision = es
        .iter()
        .position(|e| {
            matches!(
                e,
                Event::Refire(RefireEvent {
                    decision: RefireDecision::DeadTarget,
                    ..
                })
            )
        })
        .unwrap();
    let chase_calls = es[decision + 1..]
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::ActorState(ActorStateEvent::Action {
                    action: ActorAction::Original(ActionId(15)),
                    ..
                })
            )
        })
        .count();
    assert_eq!(chase_calls, 2);
    assert_ne!(position, s.actor(id(0)).unwrap().position());
    assert_eq!(
        s.actor(id(0)).unwrap().life().unwrap().reaction_time,
        i32::MIN
    );
    assert_eq!(s.actor(id(0)).unwrap().life().unwrap().target, Some(id(2)));
}
#[test]
fn successive_shots_aim_at_current_moving_target_state() {
    let mut s = setup("thinker_moving_target");
    let face = |es: &[Event]| {
        es.iter()
            .find_map(|e| match e {
                Event::Monster(MonsterEvent::Face { angles, .. }) => Some(angles[1]),
                _ => None,
            })
            .unwrap()
    };
    let first = face(s.pending_events());
    let position = s.actor(id(1)).unwrap().position();
    for _ in 0..4 {
        s.step([TicCommand::default(); 4]).unwrap();
    }
    assert_ne!(position, s.actor(id(1)).unwrap().position());
    assert_ne!(first, face(s.events()));
    assert_eq!(
        s.events()
            .iter()
            .filter(|e| matches!(e, Event::Monster(MonsterEvent::Aim { .. })))
            .count(),
        1
    );
}
#[test]
fn roles_players_pos_action_and_unsupported_source_states_reject() {
    let mut s = setup("ordinary");
    for ty in [0, 1, 2, 3, 10, 19, 21, 34, 136] {
        reject(&mut s, |s| {
            s.spawn_type(ActorTypeId(ty), [Fixed::ZERO; 2], SpawnHeight::Floor)
        });
    }
    for state in [0, 3, 174, 416, 417, 418, 419] {
        reject(&mut s, |s| s.enter_actor_state(id(0), StateId(state)));
    }
    for state in [1, 2, 4, 5, 6, 7] {
        reject(&mut s, |s| s.enter_actor_state(id(1), StateId(state)));
    }
    reject(&mut s, |s| s.chaingunner_attack(id(1)));
    reject(&mut s, |s| s.chaingunner_refire(id(1)));
    reject(&mut s, |s| s.shotgun_attack(id(0)));
    reject(&mut s, |s| s.pos_attack(id(0)));
    reject(&mut s, |s| s.bind_player(0, id(1)));
    reject(&mut s, |s| {
        s.damage_actor(DamageRequest {
            target: id(1),
            inflictor: Some(id(0)),
            source: Some(id(0)),
            amount: 3,
        })
    });
    reject(&mut s, |s| {
        s.line_attack(
            id(0),
            LineAttack {
                angle: 0,
                range: Fixed::from_map_units(2048),
                slope: Fixed::ZERO,
                damage: 3,
            },
        )
    });
    reject(&mut s, |s| {
        s.aim_line_attack(id(0), 0, Fixed::from_map_units(2048))
    });
}
#[test]
fn pending_stale_and_effect_references_reject_complete_bursts() {
    let mut s = setup("ordinary");
    s.mark_remove(id(1)).unwrap();
    reject(&mut s, |s| s.chaingunner_attack(id(0)));
    reject(&mut s, |s| s.chaingunner_refire(id(0)));
    reject(&mut s, |s| s.enter_actor_state(id(0), StateId(7)));
    reject(&mut s, |s| s.step([TicCommand::default(); 4]));
    s.set_actor_target(id(0), None).unwrap();
    s.set_perception_player(0, PerceptionPlayer::default())
        .unwrap();
    s.step([TicCommand::default(); 4]).unwrap();
    let fresh = s
        .spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(96), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!((fresh.slot(), fresh.generation()), (1, 2));
    reject(&mut s, |s| s.set_actor_target(id(0), Some(id(1))));
    reject(&mut s, |s| s.chaingunner_attack(ActorId::from_parts(0, 2)));
    s.set_actor_target(id(0), Some(fresh)).unwrap();
    s.chaingunner_attack(id(0)).unwrap();
    let effect = s.actor_order().last().unwrap();
    reject(&mut s, |s| s.set_actor_target(id(0), Some(effect)));
    reject(&mut s, |s| s.noise_alert(effect, id(0)));
}
#[test]
fn unsupported_damage_metadata_flags_and_arithmetic_reject_before_mutation() {
    let c = fixtures::cases()
        .into_iter()
        .find(|c| c.label == "ordinary")
        .unwrap();
    let r = fixtures::registry(&c);
    for (key, value) in [(18, 10), (21, 6 | 4194304), (7, 217), (12, 217), (17, 0)] {
        let mut types = r.types().to_vec();
        types[135].properties[key] = value;
        let registry = ActorRegistry::new(r.states().to_vec(), types).unwrap();
        assert!(
            Simulation::with_chaingunner_registry(
                fixtures::map(0),
                64,
                registry,
                ActorOptions::default()
            )
            .is_err()
        );
    }
    let mut s = setup("ordinary");
    for field in 0..5 {
        let mut p = PursuitSetup::from_actor(s.actor(id(0)).unwrap()).unwrap();
        match field {
            0 => p.flags |= 4,
            1 => p.reaction_time = i32::MIN,
            2 => p.reaction_time = i32::MIN + 1,
            3 => p.movecount = i32::MIN,
            _ => p.threshold = i32::MIN,
        };
        reject(&mut s, |s| s.configure_pursuit_actor(id(0), p));
    }
    reject(&mut s, |s| s.set_momentum(id(0), [Fixed::ONE, Fixed::ZERO]));
    reject(&mut s, |s| s.set_vertical_momentum(id(0), Fixed::ONE));
    reject(&mut s, |s| {
        s.spawn_type(
            ActorTypeId(135),
            [Fixed::from_bits(i32::MIN), Fixed::ZERO],
            SpawnHeight::Floor,
        )
    });
    reject(&mut s, |s| {
        s.set_momentum(id(1), [Fixed::from_bits(i32::MAX), Fixed::ZERO])
    });
}
#[test]
fn pause_instances_reads_and_named_field_diagnostics_remain_independent() {
    let mut a = setup("thinker_cycles");
    let mut b = setup("thinker_cycles");
    a.set_paused(true);
    let before = a.snapshot();
    for _ in 0..3 {
        assert_eq!(a.step([TicCommand::default(); 4]).unwrap(), before);
    }
    a.set_paused(false);
    for _ in 0..150 {
        let before = a.snapshot();
        let _ = a.snapshot().canonical_bytes();
        let _ = a.events();
        let _ = a.actor(id(0));
        assert_eq!(a.snapshot(), before);
        assert_eq!(
            a.step([TicCommand::default(); 4]).unwrap(),
            b.step([TicCommand::default(); 4]).unwrap()
        );
    }
    let before = a.snapshot();
    let mut p = PursuitSetup::from_actor(a.actor(id(0)).unwrap()).unwrap();
    p.angle = p.angle.wrapping_add(1);
    a.configure_pursuit_actor(id(0), p).unwrap();
    assert_eq!(
        before.first_difference(&a.snapshot()).unwrap().field,
        "actor.0.angle"
    );
}
