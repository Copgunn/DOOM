//! Rust safety policies for the complete burst, distinct from original-C behavior.
#[path = "../../../tests/support/m3d3d.rs"]
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
fn zero_one_two_free_slots_reject_before_sound_rng_or_any_pellet() {
    for count in [62, 63, 64] {
        let mut s = setup("ordinary");
        fill(&mut s, count);
        reject(&mut s, |s| s.shotgun_attack(id(0)));
        reject(&mut s, |s| s.enter_actor_state(id(0), StateId(5)));
    }
    let mut s = setup("ordinary");
    fill(&mut s, 61);
    let before = s.actor_order().count();
    s.shotgun_attack(id(0)).unwrap();
    assert_eq!(s.actor_order().count(), before + 3);
}
#[test]
fn delayed_burst_capacity_is_preflighted_before_tic_mutation() {
    let mut s = setup("ordinary");
    s.enter_actor_state(id(0), StateId(4)).unwrap();
    for _ in 0..9 {
        s.step([TicCommand::default(); 4]).unwrap();
    }
    assert_eq!(s.actor(id(0)).unwrap().life().unwrap().tics, 1);
    fill(&mut s, 62);
    reject(&mut s, |s| s.step([TicCommand::default(); 4]));
    // Pending effects are still occupied when the owner acts first; no early retirement credit.
    let extra = s.actor_order().last().unwrap();
    s.mark_remove(extra).unwrap();
    reject(&mut s, |s| s.step([TicCommand::default(); 4]));
}
#[test]
fn conservative_miss_capacity_and_null_exception_are_explicit() {
    let mut miss = setup("all_miss_open");
    fill(&mut miss, 62);
    reject(&mut miss, |s| s.shotgun_attack(id(0)));
    let mut null = setup("null");
    fill(&mut null, 64);
    let rng = null.rng();
    null.shotgun_attack(id(0)).unwrap();
    assert_eq!(null.rng(), rng);
    assert!(!null.pending_events().iter().any(|e| matches!(
        e,
        Event::Shotgun(_) | Event::Combat(_) | Event::Monster(MonsterEvent::Aim { .. })
    )));
}
#[test]
fn roles_players_pos_action_and_unsupported_source_states_reject() {
    let mut s = setup("ordinary");
    for ty in [0, 1, 2, 3, 19, 21, 34, 136] {
        reject(&mut s, |s| {
            s.spawn_type(ActorTypeId(ty), [Fixed::ZERO; 2], SpawnHeight::Floor)
        });
    }
    for state in [0, 3, 7, 174, 217, 218, 219] {
        reject(&mut s, |s| s.enter_actor_state(id(0), StateId(state)));
    }
    for state in [1, 2, 4, 5, 6] {
        reject(&mut s, |s| s.enter_actor_state(id(1), StateId(state)));
    }
    reject(&mut s, |s| s.shotgun_attack(id(1)));
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
fn original_state_timing_fullbright_and_narrow_action_admission() {
    let mut s = setup("ordinary");
    s.chase(id(0)).unwrap();
    for _ in 0..9 {
        s.step([TicCommand::default(); 4]).unwrap();
        assert_eq!(
            s.actor(id(0)).unwrap().life().unwrap().state,
            Some(StateId(4))
        );
    }
    s.step([TicCommand::default(); 4]).unwrap();
    let life = s.actor(id(0)).unwrap().life().unwrap();
    assert_eq!(
        (life.state, life.tics, life.sprite, life.frame),
        (Some(StateId(5)), 10, 30, 32773)
    );
    for _ in 0..10 {
        s.step([TicCommand::default(); 4]).unwrap();
    }
    let life = s.actor(id(0)).unwrap().life().unwrap();
    assert_eq!(
        (life.state, life.tics, life.frame),
        (Some(StateId(6)), 10, 4)
    );
    for _ in 0..10 {
        s.step([TicCommand::default(); 4]).unwrap();
    }
    assert_eq!(
        s.actor(id(0)).unwrap().life().unwrap().state,
        Some(StateId(2))
    );
    let c = fixtures::cases()
        .into_iter()
        .find(|c| c.label == "ordinary")
        .unwrap();
    let r = fixtures::registry(&c);
    assert!(!r.action_supported(ActionId(55)));
    for (index, tics, frame, action) in [
        (5, 8, 32773, 55),
        (5, 10, 5, 55),
        (5, 10, 32773, 51),
        (4, 0, 4, 20),
    ] {
        let mut states = r.states().to_vec();
        states[index].tics = tics;
        states[index].frame = frame;
        states[index].action = ActorAction::Original(ActionId(action));
        let registry = ActorRegistry::new(states, r.types().to_vec()).unwrap();
        assert!(
            Simulation::with_shotgunner_registry(
                fixtures::map(0),
                64,
                registry,
                ActorOptions::default()
            )
            .is_err()
        );
    }
}
#[test]
fn pending_stale_and_effect_references_reject_complete_bursts() {
    let mut s = setup("ordinary");
    s.mark_remove(id(1)).unwrap();
    reject(&mut s, |s| s.shotgun_attack(id(0)));
    reject(&mut s, |s| s.enter_actor_state(id(0), StateId(5)));
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
    reject(&mut s, |s| s.shotgun_attack(ActorId::from_parts(0, 2)));
    s.set_actor_target(id(0), Some(fresh)).unwrap();
    s.shotgun_attack(id(0)).unwrap();
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
            Simulation::with_shotgunner_registry(
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
fn pellet_health_is_sequential_and_later_shots_skip_a_dead_body() {
    let mut s = setup("multiple_targets_hp6_y0");
    s.shotgun_attack(id(0)).unwrap();
    let damage = s
        .pending_events()
        .iter()
        .filter_map(|e| {
            if let Event::Combat(CombatEvent::Health { target, values }) = e {
                Some((*target, *values))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert!(damage.len() >= 2);
    let first = damage[0];
    assert_eq!(first.0, id(1));
    for pair in damage.windows(2) {
        if pair[0].0 == pair[1].0 {
            assert_eq!(pair[0].1[1], pair[1].1[0]);
        }
    }
    let first_body = s.actor(id(1)).unwrap().life().unwrap();
    assert!(first_body.health <= 0);
    assert_eq!(first_body.flags & 4, 0);
    assert!(damage.iter().any(|(target, _)| *target == id(2)));
    let done = s
        .pending_events()
        .iter()
        .filter(|e| matches!(e, Event::Shotgun(ShotgunEvent::PelletDone { .. })))
        .count();
    assert_eq!(done, 3);
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
    for _ in 0..130 {
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
