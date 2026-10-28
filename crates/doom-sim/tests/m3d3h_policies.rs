//! Rust safety and source timing probes; independent C evidence is separate.
#[path = "../../../tests/support/m3d3f.rs"]
mod old;
use doom_sim::*;
use doom_types::Fixed;
fn id(n: u32) -> ActorId {
    ActorId::from_parts(n, 1)
}
fn setup() -> Simulation {
    let mut s = Simulation::with_demon_stand_run_registry(
        old::map(0),
        64,
        ActorRegistry::demon_stand_run_fixture(),
        ActorOptions::default(),
    )
    .unwrap();
    s.spawn_type(
        ActorTypeId(136),
        [Fixed::from_map_units(-64), Fixed::ZERO],
        SpawnHeight::Floor,
    )
    .unwrap();
    s.spawn_type(
        ActorTypeId(135),
        [Fixed::from_map_units(96), Fixed::ZERO],
        SpawnHeight::Floor,
    )
    .unwrap();
    s
}
fn damage(
    s: &mut Simulation,
    amount: i32,
    source: Option<ActorId>,
    inflictor: Option<ActorId>,
) -> DamageResult {
    s.damage_actor(DamageRequest {
        target: id(0),
        source,
        inflictor,
        amount,
    })
    .unwrap()
}
fn tick(s: &mut Simulation) {
    s.step([TicCommand::default(); 4]).unwrap();
}
fn sound(events: &[Event], n: u16) -> bool {
    events
        .iter()
        .any(|e| matches!(e,Event::ActorState(ActorStateEvent::Sound{sound,..}) if *sound==n))
}
#[test]
fn ordinary_death_keeps_solid_until_delayed_fall_and_terminal_persists() {
    let mut s = setup();
    let result = damage(&mut s, 1000, None, None);
    assert!(result.killed && !result.extreme);
    let a = s.actor(id(0)).unwrap();
    let l = a.life().unwrap();
    let initial = l.tics;
    assert!((5..=8).contains(&initial));
    assert_eq!(l.state, Some(StateId(9)));
    assert_eq!(a.height(), Fixed::from_map_units(14));
    assert_ne!(l.flags & 2, 0);
    assert_eq!(l.flags & 4, 0);
    assert_eq!(l.flags & (1048576 | 1024), 1048576 | 1024);
    assert!(!sound(s.pending_events(), 64));
    for t in 1..=initial + 8 + 4 + 4 + 4 + 40 {
        tick(&mut s);
        let l = s.actor(id(0)).unwrap().life().unwrap();
        assert_eq!(sound(s.events(), 64), t == initial);
        assert_eq!(l.flags & 2 != 0, t < initial + 8 + 4);
        if t >= initial + 8 + 4 + 4 + 4 {
            assert_eq!(l.state, Some(StateId(14)));
            assert_eq!(l.tics, -1);
        }
    }
    assert!(s.actor(id(0)).is_some());
    let rng = s.rng();
    let r = damage(&mut s, 1, None, None);
    assert!(r.ignored);
    assert_eq!(s.rng(), rng);
}
#[test]
fn nonlethal_damage_can_immediately_enter_chase_without_pain() {
    let mut found = false;
    for skip in 0..256 {
        let mut s = setup();
        for _ in 0..skip {
            s.gameplay_random();
        }
        let r = damage(&mut s, 1, Some(id(1)), None);
        if !r.pain {
            found = true;
            let l = s.actor(id(0)).unwrap().life().unwrap();
            assert_eq!(l.health, 149);
            assert_ne!(l.state, Some(StateId(1)));
            assert_eq!(l.target, Some(id(1)));
            break;
        }
    }
    assert!(found);
}
#[test]
fn pain_sound_waits_for_second_state_and_then_returns_to_chase() {
    let mut found = false;
    for skip in 0..256 {
        let mut s = setup();
        for _ in 0..skip {
            s.gameplay_random();
        }
        if damage(&mut s, 1, None, None).pain {
            found = true;
            assert_eq!(
                s.actor(id(0)).unwrap().life().unwrap().state,
                Some(StateId(7))
            );
            assert!(!sound(s.pending_events(), 26));
            tick(&mut s);
            assert!(!sound(s.events(), 26));
            tick(&mut s);
            assert!(sound(s.events(), 26));
            assert_eq!(
                s.actor(id(0)).unwrap().life().unwrap().state,
                Some(StateId(8))
            );
            tick(&mut s);
            tick(&mut s);
            assert_ne!(
                s.actor(id(0)).unwrap().life().unwrap().state,
                Some(StateId(8))
            );
            break;
        }
    }
    assert!(found);
}
fn reject<T: std::fmt::Debug>(
    s: &mut Simulation,
    f: impl FnOnce(&mut Simulation) -> Result<T, SimError>,
) {
    let before = s.snapshot();
    assert!(f(s).is_err());
    assert_eq!(s.snapshot(), before);
}
fn shot() -> LineAttack {
    LineAttack {
        angle: 0x80000000,
        range: Fixed::from_map_units(512),
        slope: Fixed::ZERO,
        damage: 1,
    }
}
#[test]
fn capacity_direct_damage_succeeds_and_hitscan_reservation_is_atomic() {
    let mut s = setup();
    while s.actor_order().count() < 64 {
        s.spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(-200), Fixed::from_map_units(-96)],
            SpawnHeight::Floor,
        )
        .unwrap();
    }
    damage(&mut s, 1, None, None);
    reject(&mut s, |s| s.line_attack(id(1), shot()));
    damage(&mut s, 1000, None, None);
    assert_eq!(s.actor_order().count(), 64);
}
#[test]
fn corpse_resurrection_and_unsupported_actor_actions_reject() {
    let mut s = setup();
    damage(&mut s, 150, None, None);
    let mut p = PursuitSetup::from_actor(s.actor(id(0)).unwrap()).unwrap();
    p.health = 150;
    p.flags = 6;
    reject(&mut s, |s| s.configure_pursuit_actor(id(0), p));
    for state in [1, 2, 3, 4, 7, 9, 14, 15, 16, 17, 18, 19, 20, 21, 496] {
        reject(&mut s, |s| s.enter_actor_state(id(0), StateId(state)));
    }
    reject(&mut s, |s| s.request_actor_death(id(0)));
    reject(&mut s, |s| s.chase(id(0)));
    reject(&mut s, |s| s.demon_melee_attack(id(0)));
    for ty in [3, 12, 13, 34, 137] {
        reject(&mut s, |s| {
            s.spawn_type(ActorTypeId(ty), [Fixed::ZERO; 2], SpawnHeight::Floor)
        });
    }
}
#[test]
fn metadata_scoring_extreme_and_raise_dependencies_are_rejected() {
    for field in [13, 21, 22] {
        let r = ActorRegistry::demon_stand_run_fixture();
        let mut types = r.types().to_vec();
        types[136].properties[field] = match field {
            13 => 169,
            21 => 4194310,
            _ => 496,
        };
        let r = ActorRegistry::new(r.states().to_vec(), types).unwrap();
        assert!(
            Simulation::with_demon_stand_run_registry(old::map(0), 64, r, ActorOptions::default())
                .is_err()
        );
    }
    assert!(
        Simulation::with_demon_stand_run_registry(
            old::map(0),
            65,
            ActorRegistry::demon_stand_run_fixture(),
            ActorOptions::default()
        )
        .is_err()
    );
    assert!(
        Simulation::with_demon_stand_run_registry(
            old::map(0),
            64,
            ActorRegistry::demon_stand_run_fixture(),
            ActorOptions {
                skill: Skill::Nightmare,
                ..ActorOptions::default()
            }
        )
        .is_err()
    );
}
#[test]
fn pending_stale_and_reused_damage_sources_never_alias() {
    let mut s = setup();
    s.mark_remove(id(1)).unwrap();
    let req = DamageRequest {
        target: id(0),
        source: Some(id(1)),
        inflictor: None,
        amount: 1,
    };
    reject(&mut s, |s| s.damage_actor(req));
    tick(&mut s);
    let fresh = s
        .spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(96), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(fresh, ActorId::from_parts(1, 2));
    reject(&mut s, |s| s.damage_actor(req));
    let req = DamageRequest {
        inflictor: Some(id(1)),
        source: None,
        ..req
    };
    reject(&mut s, |s| s.damage_actor(req));
}
#[test]
fn players_damage_arithmetic_counters_and_direct_hitscan_roles_reject_atomically() {
    let mut s = setup();
    reject(&mut s, |s| s.bind_player(0, id(0)));
    reject(&mut s, |s| s.line_attack(id(0), shot()));
    for amount in [-1, 1001] {
        reject(&mut s, |s| {
            s.damage_actor(DamageRequest {
                target: id(0),
                source: None,
                inflictor: None,
                amount,
            })
        });
    }
    for field in 0..5 {
        let mut p = PursuitSetup::from_actor(s.actor(id(0)).unwrap()).unwrap();
        match field {
            0 => p.health = i32::MIN,
            1 => p.reaction_time = i32::MIN + 1,
            2 => p.movecount = i32::MIN,
            3 => p.threshold = i32::MIN,
            _ => p.flags |= 4194304,
        };
        reject(&mut s, |s| s.configure_pursuit_actor(id(0), p));
    }
    reject(&mut s, |s| {
        s.spawn_type(
            ActorTypeId(135),
            [Fixed::from_bits(i32::MIN), Fixed::ZERO],
            SpawnHeight::Floor,
        )
    });
}
#[test]
fn all_four_original_first_death_timers_are_observed() {
    let mut times = std::collections::BTreeSet::new();
    for skip in 0..32 {
        let mut s = setup();
        for _ in 0..skip {
            s.gameplay_random();
        }
        damage(&mut s, 150, None, None);
        times.insert(s.actor(id(0)).unwrap().life().unwrap().tics);
    }
    assert_eq!(times, (5..=8).collect());
}
#[test]
fn pause_reads_and_independent_instances_do_not_change_lifecycle() {
    let mut a = setup();
    let mut b = setup();
    damage(&mut a, 150, None, None);
    damage(&mut b, 150, None, None);
    for _ in 0..60 {
        let state = a.snapshot();
        a.set_paused(true);
        let paused = a.snapshot();
        a.step([TicCommand::default(); 4]).unwrap();
        assert_eq!(a.snapshot(), paused);
        a.set_paused(false);
        assert_eq!(a.snapshot(), state);
        let _ = a.actor_order().collect::<Vec<_>>();
        let _ = a.actor(id(0));
        assert_eq!(a.snapshot(), b.snapshot());
        tick(&mut a);
        tick(&mut b);
        assert_eq!(a.snapshot(), b.snapshot());
    }
}
#[test]
fn demon_direct_entry_keeps_sarg_context_and_rejects_other_attacks() {
    let mut s = setup();
    s.set_actor_target(id(0), Some(id(1))).unwrap();
    reject(&mut s, |s| s.pos_attack(id(0)));
    reject(&mut s, |s| s.shotgun_attack(id(0)));
    reject(&mut s, |s| s.chaingunner_attack(id(0)));
    reject(&mut s, |s| s.chaingunner_refire(id(0)));
    reject(&mut s, |s| s.check_missile_range(id(0)));
    s.enter_actor_state(id(0), StateId(6)).unwrap();
    assert_eq!(
        s.actor(id(0)).unwrap().life().unwrap().state,
        Some(StateId(6))
    );
}

#[test]
fn altered_new_stand_run_closure_and_old_registry_reject() {
    assert!(
        Simulation::with_demon_stand_run_registry(
            old::map(0),
            64,
            ActorRegistry::demon_lifecycle_fixture(),
            ActorOptions::default()
        )
        .is_err()
    );
    for id in [1, 3, 2, 15, 16, 17, 18, 19, 20, 21] {
        for change in 0..3 {
            let registry = ActorRegistry::demon_stand_run_fixture();
            let mut states = registry.states().to_vec();
            match change {
                0 => states[id].tics = 1,
                1 => states[id].next = StateId(0),
                _ => states[id].action = ActorAction::None,
            };
            let registry = ActorRegistry::new(states, registry.types().to_vec()).unwrap();
            assert!(
                Simulation::with_demon_stand_run_registry(
                    old::map(0),
                    64,
                    registry,
                    ActorOptions::default()
                )
                .is_err()
            );
        }
    }
}
#[test]
fn added_live_entries_are_separate_from_old_profile_and_death_entry() {
    let mut previous = Simulation::with_demon_lifecycle_registry(
        old::map(0),
        64,
        ActorRegistry::demon_lifecycle_fixture(),
        ActorOptions::default(),
    )
    .unwrap();
    previous
        .spawn_type(
            ActorTypeId(136),
            [Fixed::from_map_units(-64), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    for state in [3, 15, 16, 17, 18, 19, 20, 21] {
        reject(&mut previous, |s| {
            s.enter_actor_state(id(0), StateId(state))
        });
        let mut current = setup();
        current.enter_actor_state(id(0), StateId(state)).unwrap();
        assert!(current.pending_events().iter().any(|e|matches!(e,Event::ActorState(ActorStateEvent::Entered{state:StateId(n),..}) if *n==state)));
    }
    let mut current = setup();
    for state in [0, 7, 8, 9, 10, 11, 12, 13, 14, 496] {
        reject(&mut current, |s| s.enter_actor_state(id(0), StateId(state)));
    }
}
#[path = "../../../tests/support/m3d3h.rs"]
mod live_fixture;
#[test]
fn pause_presentation_and_independent_instances_preserve_live_stand_run_lifecycle() {
    let c = live_fixture::cases()
        .into_iter()
        .find(|c| c.label == "stand_run_lifecycle")
        .unwrap();
    let mut a = live_fixture::setup(&c);
    let mut b = live_fixture::setup(&c);
    for op in 1..=c.v("tics") as usize {
        let before = a.snapshot();
        a.set_paused(true);
        let paused = a.snapshot();
        a.step([TicCommand::default(); 4]).unwrap();
        assert_eq!(a.snapshot(), paused);
        a.set_paused(false);
        assert_eq!(a.snapshot(), before);
        let _ = a.actor_order().collect::<Vec<_>>();
        let _ = a.events();
        let _ = a.snapshot().fields().count();
        live_fixture::advance(&mut a, &c, op);
        live_fixture::advance(&mut b, &c, op);
        assert_eq!(a.snapshot(), b.snapshot());
    }
}
