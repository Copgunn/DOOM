//! Rust policy/atomicity evidence; source equivalence is in m3d3f_reference.
#[path = "../../../tests/support/m3d3f.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
fn id(n: u32) -> ActorId {
    ActorId::from_parts(n, 1)
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
fn fill(s: &mut Simulation) {
    while s.actor_order().count() < 64 {
        s.spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(-200), Fixed::from_map_units(-96)],
            SpawnHeight::Floor,
        )
        .unwrap();
    }
}
fn no_impacts(es: &[Event]) {
    assert!(!es.iter().any(|e| matches!(
        e,
        Event::Spawned(_)
            | Event::Combat(
                CombatEvent::Impact { .. } | CombatEvent::Attack { .. } | CombatEvent::Trace { .. }
            )
            | Event::Monster(MonsterEvent::Aim { .. })
    )));
}
#[test]
fn actual_zero_allocation_allows_hit_miss_null_and_delayed_strike_at_capacity() {
    for label in [
        "ordinary",
        "shadow",
        "outside",
        "wall",
        "null",
        "dead_shootable",
        "unshootable",
    ] {
        let mut s = setup(label);
        fill(&mut s);
        let start = s.pending_events().len();
        s.demon_melee_attack(id(0)).unwrap();
        no_impacts(&s.pending_events()[start..]);
        assert_eq!(s.actor_order().count(), 64);
    }
    let mut s = setup("ordinary");
    s.enter_actor_state(id(0), StateId(5)).unwrap();
    for _ in 0..7 {
        s.step([TicCommand::default(); 4]).unwrap();
    }
    fill(&mut s);
    let start = s.pending_events().len();
    s.step([TicCommand::default(); 4]).unwrap();
    no_impacts(&s.events()[start..]);
    assert_eq!(s.actor_order().count(), 64);
    assert_eq!(
        s.actor(id(0)).unwrap().life().unwrap().state,
        Some(StateId(6))
    );
}
#[test]
fn exact_sarg_frames_three_shadow_faces_and_selection_only_sound() {
    let mut s = setup("shadow");
    let start = s.pending_events().len();
    s.chase(id(0)).unwrap();
    let mut all = s.pending_events()[start..].to_vec();
    let sound = all
        .iter()
        .position(|e| {
            matches!(
                e,
                Event::ActorState(ActorStateEvent::Sound { sound: 52, .. })
            )
        })
        .unwrap();
    let entered = all
        .iter()
        .position(|e| {
            matches!(
                e,
                Event::ActorState(ActorStateEvent::Entered {
                    state: StateId(4),
                    ..
                })
            )
        })
        .unwrap();
    assert!(sound < entered);
    for tic in 0..24 {
        let l = s.actor(id(0)).unwrap().life().unwrap();
        let state = 4 + tic / 8;
        assert_eq!(
            (l.state, l.tics, l.sprite, l.frame),
            (
                Some(StateId(state)),
                8 - i32::from(tic % 8),
                39,
                u32::from(state)
            )
        );
        if tic < 23 {
            let pending = s.pending_events().len();
            s.step([TicCommand::default(); 4]).unwrap();
            all.extend_from_slice(&s.events()[pending..]);
        }
    }
    assert_eq!(
        all.iter()
            .filter(|e| matches!(e, Event::Monster(MonsterEvent::Face { .. })))
            .count(),
        3
    );
    assert_eq!(
        all.iter()
            .filter(|e| matches!(e, Event::Monster(MonsterEvent::Random { kind: 0 | 1, .. })))
            .count(),
        6
    );
    assert_eq!(
        all.iter()
            .filter(|e| matches!(e, Event::Monster(MonsterEvent::Random { kind: 4, .. })))
            .count(),
        1
    );
    assert_eq!(
        all.iter()
            .filter(|e| matches!(
                e,
                Event::ActorState(ActorStateEvent::Sound { sound: 52, .. })
            ))
            .count(),
        1
    );
    no_impacts(&all);
    s.step([TicCommand::default(); 4]).unwrap();
    assert!(s.events().iter().any(|e| matches!(
        e,
        Event::ActorState(ActorStateEvent::Entered {
            state: StateId(2),
            ..
        })
    )));
}
#[test]
fn current_strike_rechecks_movement_sight_and_dead_or_null_targets() {
    for label in [
        "moving_miss",
        "lost_sight",
        "target_dies_during_animation",
        "target_lost",
    ] {
        let c = fixtures::cases()
            .into_iter()
            .find(|c| c.label == label)
            .unwrap();
        let mut s = fixtures::setup(&c);
        for op in 1..=16 {
            fixtures::advance(&mut s, &c, op);
            fixtures::assert_golden(
                &s,
                fixtures::cases()
                    .iter()
                    .position(|x| x.label == label)
                    .unwrap(),
                op,
            );
        }
        let es = s.events();
        match label {
            "moving_miss" | "lost_sight" => {
                assert!(
                    es.iter()
                        .any(|e| matches!(e, Event::Monster(MonsterEvent::Face { .. })))
                );
                assert!(es.iter().any(|e|matches!(e,Event::Monster(MonsterEvent::Range{decision,..}) if !decision.accepted)));
                assert!(!es.iter().any(|e| matches!(
                    e,
                    Event::Monster(MonsterEvent::Random { kind: 4, .. })
                        | Event::Combat(CombatEvent::Damage(_))
                )));
            }
            "target_lost" => {
                assert!(!es.iter().any(|e| matches!(
                    e,
                    Event::Monster(
                        MonsterEvent::Face { .. }
                            | MonsterEvent::Range { .. }
                            | MonsterEvent::Random { .. }
                    )
                )));
            }
            _ => {
                assert!(s.actor(id(1)).unwrap().life().unwrap().health <= 0);
                assert!(s.perception_players()[0].health > 0);
                assert!(
                    es.iter()
                        .any(|e| matches!(e, Event::Combat(CombatEvent::Ignored(_))))
                );
                assert!(
                    es.iter()
                        .any(|e| matches!(e, Event::Monster(MonsterEvent::Random { kind: 4, .. })))
                );
            }
        }
    }
}
#[test]
fn original_lut_cases_cover_all_damage_values_and_shared_target_consequences() {
    for amount in (4..=40).step_by(4) {
        for shadow in 0..2 {
            let mut s = setup(&format!("damage{amount}_shadow{shadow}"));
            let start = s.pending_events().len();
            s.demon_melee_attack(id(0)).unwrap();
            let es = &s.pending_events()[start..];
            assert!(es.iter().any(|e|matches!(e,Event::Combat(CombatEvent::Damage(r)) if r.amount==amount&&r.target==id(1)&&r.inflictor==Some(id(0))&&r.source==Some(id(0)))));
            assert_eq!(
                s.actor(id(1)).unwrap().life().unwrap().health,
                1000 - amount
            );
            assert_ne!(s.actor(id(1)).unwrap().momentum(), [Fixed::ZERO; 2]);
            let l = s.actor(id(1)).unwrap().life().unwrap();
            assert_eq!(
                (l.target, l.threshold, l.reaction_time),
                (Some(id(0)), 100, 0)
            );
            no_impacts(es);
        }
    }
    for (label, kind) in [("pain_always", 0), ("kill_first", 1), ("extreme", 2)] {
        let mut s = setup(label);
        s.demon_melee_attack(id(0)).unwrap();
        assert!(s.pending_events().iter().any(|e| matches!(
            (kind, e),
            (0, Event::Combat(CombatEvent::Pain { .. }))
                | (1, Event::Combat(CombatEvent::Kill { extreme: false, .. }))
                | (2, Event::Combat(CombatEvent::Kill { extreme: true, .. }))
        )));
    }
}
#[test]
fn original_monsters_other_attacks_players_and_role_crossovers_reject() {
    let mut s = setup("ordinary");
    for ty in [0, 1, 2, 3, 10, 12, 13, 19, 21, 34, 37, 38, 136] {
        reject(&mut s, |s| {
            s.spawn_type(ActorTypeId(ty), [Fixed::ZERO; 2], SpawnHeight::Floor)
        });
    }
    for state in [0, 3, 7, 174, 485, 486, 487] {
        reject(&mut s, |s| s.enter_actor_state(id(0), StateId(state)));
    }
    for state in [1, 2, 4, 5, 6] {
        reject(&mut s, |s| s.enter_actor_state(id(1), StateId(state)));
    }
    reject(&mut s, |s| s.demon_melee_attack(id(1)));
    reject(&mut s, |s| s.pos_attack(id(0)));
    reject(&mut s, |s| s.shotgun_attack(id(0)));
    reject(&mut s, |s| s.chaingunner_attack(id(0)));
    reject(&mut s, |s| s.chaingunner_refire(id(0)));
    reject(&mut s, |s| s.check_missile_range(id(0)));
    reject(&mut s, |s| s.bind_player(0, id(1)));
    reject(&mut s, |s| {
        s.damage_actor(DamageRequest {
            target: id(1),
            inflictor: Some(id(0)),
            source: Some(id(0)),
            amount: 4,
        })
    });
    reject(&mut s, |s| {
        s.aim_line_attack(id(0), 0, Fixed::from_map_units(2048))
    });
    reject(&mut s, |s| {
        s.line_attack(
            id(0),
            LineAttack {
                angle: 0,
                range: Fixed::from_map_units(2048),
                slope: Fixed::ZERO,
                damage: 4,
            },
        )
    });
    let mut older = Simulation::new(fixtures::map(0)).unwrap();
    reject(&mut older, |s| s.demon_melee_attack(id(0)));
}
#[test]
fn pending_retirement_stale_and_reused_generations_never_alias() {
    let mut s = setup("ordinary");
    s.mark_remove(id(1)).unwrap();
    reject(&mut s, |s| s.demon_melee_attack(id(0)));
    reject(&mut s, |s| s.enter_actor_state(id(0), StateId(6)));
    reject(&mut s, |s| s.step([TicCommand::default(); 4]));
    s.set_actor_target(id(0), None).unwrap();
    s.set_perception_player(0, PerceptionPlayer::default())
        .unwrap();
    s.step([TicCommand::default(); 4]).unwrap();
    let fresh = s
        .spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(16), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(fresh, ActorId::from_parts(1, 2));
    reject(&mut s, |s| s.set_actor_target(id(0), Some(id(1))));
    reject(&mut s, |s| s.demon_melee_attack(id(1)));
    reject(&mut s, |s| s.demon_melee_attack(ActorId::from_parts(0, 2)));
    s.set_actor_target(id(0), Some(fresh)).unwrap();
    s.demon_melee_attack(id(0)).unwrap();
}
#[test]
fn strict_metadata_state_actions_and_normal_timing_reject_unproved_closures() {
    let c = fixtures::cases()
        .into_iter()
        .find(|c| c.label == "ordinary")
        .unwrap();
    let r = fixtures::registry(&c);
    assert!(!r.action_supported(ActionId(56)));
    assert_eq!(r.find_type("MT_SERGEANT"), Some(ActorTypeId(12)));
    assert_eq!(r.find_type("MT_SHADOWS"), Some(ActorTypeId(13)));
    for (index, tics, frame, action, next) in [
        (4, 4, 4, 20, 5),
        (5, 8, 32773, 20, 6),
        (6, 8, 6, 51, 2),
        (6, 8, 6, 13, 2),
        (6, 8, 6, 55, 2),
        (6, 0, 6, 56, 2),
        (6, 8, 6, 56, 485),
    ] {
        let mut states = r.states().to_vec();
        states[index].tics = tics;
        states[index].frame = frame;
        states[index].action = ActorAction::Original(ActionId(action));
        states[index].next = StateId(next);
        let reg = ActorRegistry::new(states, r.types().to_vec()).unwrap();
        assert!(
            Simulation::with_demon_melee_registry(
                fixtures::map(0),
                64,
                reg,
                ActorOptions::default()
            )
            .is_err()
        );
    }
    for (ty, key, value) in [
        (135, 18, 10),
        (135, 21, 6 | 4194304),
        (135, 12, 217),
        (135, 17, 0),
        (136, 2, 100),
        (136, 18, 100),
        (136, 6, 0),
        (136, 11, 4),
    ] {
        let mut types = r.types().to_vec();
        types[ty].properties[key] = value;
        let reg = ActorRegistry::new(r.states().to_vec(), types).unwrap();
        assert!(
            Simulation::with_demon_melee_registry(
                fixtures::map(0),
                64,
                reg,
                ActorOptions::default()
            )
            .is_err()
        );
    }
    assert!(
        Simulation::with_demon_melee_registry(
            fixtures::map(0),
            64,
            r.clone(),
            ActorOptions {
                skill: Skill::Nightmare,
                ..ActorOptions::default()
            }
        )
        .is_err()
    );
    assert!(
        Simulation::with_demon_melee_registry(fixtures::map(0), 65, r, ActorOptions::default())
            .is_err()
    );
}
#[test]
fn arithmetic_momentum_and_work_rejections_preserve_full_canonical_state() {
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
    reject(&mut s, |s| {
        s.set_vertical_momentum(id(1), Fixed::from_bits(i32::MAX))
    });
}
#[test]
fn pause_instances_and_presentation_reads_are_independent() {
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
