//! Rust-specific admission and atomicity policies, separate from C equivalence.
#[path = "../../../tests/support/m3d3c.rs"]
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
#[test]
fn capacity_is_checked_before_direct_action_entry_and_delayed_action() {
    for entry in [false, true] {
        let mut s = setup("pos_hit");
        fill(&mut s);
        if entry {
            reject(&mut s, |s| s.enter_actor_state(id(0), StateId(5)));
        } else {
            reject(&mut s, |s| s.pos_attack(id(0)));
        }
    }
    let mut s = setup("pos_hit");
    s.enter_actor_state(id(0), StateId(4)).unwrap();
    for _ in 0..9 {
        s.step([TicCommand::default(); 4]).unwrap();
    }
    assert_eq!(s.actor(id(0)).unwrap().life().unwrap().tics, 1);
    fill(&mut s);
    reject(&mut s, |s| s.step([TicCommand::default(); 4]));
}
#[test]
fn roles_states_flags_player_damage_and_second_owner_reject_atomically() {
    let mut s = setup("pos_hit");
    for ty in [0, 1, 2, 34, 136] {
        reject(&mut s, |s| {
            s.spawn_type(ActorTypeId(ty), [Fixed::ZERO; 2], SpawnHeight::Floor)
        });
    }
    for state in [0, 8, 97, 174, 255] {
        reject(&mut s, |s| s.enter_actor_state(id(0), StateId(state)));
    }
    for state in [1, 2, 3, 4, 5, 6, 7] {
        reject(&mut s, |s| s.enter_actor_state(id(1), StateId(state)));
    }
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
    reject(&mut s, |s| s.set_momentum(id(0), [Fixed::ONE, Fixed::ZERO]));
    reject(&mut s, |s| s.set_vertical_momentum(id(0), Fixed::ONE));
    let mut p = PursuitSetup::from_actor(s.actor(id(0)).unwrap()).unwrap();
    p.flags |= 4;
    reject(&mut s, |s| s.configure_pursuit_actor(id(0), p));
    let c = fixtures::cases()
        .into_iter()
        .find(|c| c.label == "missile_type19_distance196")
        .unwrap();
    let mut decision_only = fixtures::setup(&c);
    reject(&mut decision_only, |s| s.pos_attack(id(0)));
    reject(&mut decision_only, |s| {
        s.enter_actor_state(id(0), StateId(5))
    });
}
#[test]
fn unsupported_reachable_actions_and_modified_impacts_reject_construction() {
    let c = fixtures::cases()
        .into_iter()
        .find(|c| c.label == "pos_hit")
        .unwrap();
    let original = fixtures::registry(&c);
    for index in [2, 3, 4, 5, 6, 7, 149, 90, 93] {
        let mut states = original.states().to_vec();
        states[index].action = ActorAction::Original(ActionId(52));
        let registry = ActorRegistry::new(states, original.types().to_vec()).unwrap();
        assert!(
            Simulation::with_monster_attack_registry(
                fixtures::map(0),
                64,
                registry,
                ActorOptions::default()
            )
            .is_err()
        );
    }
    let vanilla = ActorRegistry::vanilla();
    assert!(!vanilla.action_supported(ActionId(20)));
    assert!(!vanilla.action_supported(ActionId(51)));
    let mut s = Simulation::with_monster_attack_registry(
        fixtures::map(0),
        64,
        vanilla,
        ActorOptions::default(),
    )
    .unwrap();
    reject(&mut s, |s| {
        s.spawn_type(ActorTypeId(136), [Fixed::ZERO; 2], SpawnHeight::Floor)
    });
}
#[test]
fn pending_stale_and_reused_targets_never_alias() {
    let mut s = setup("pos_hit");
    s.set_perception_player(
        0,
        PerceptionPlayer {
            active: false,
            health: 100,
            actor: Some(id(1)),
        },
    )
    .unwrap();
    s.mark_remove(id(1)).unwrap();
    reject(&mut s, |s| s.pos_attack(id(0)));
    reject(&mut s, |s| s.face_target(id(0)));
    reject(&mut s, |s| s.check_missile_range(id(0)));
    reject(&mut s, |s| s.step([TicCommand::default(); 4]));
    s.set_actor_target(id(0), None).unwrap();
    s.step([TicCommand::default(); 4]).unwrap();
    let next = s
        .spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(96), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(next.slot(), 1);
    assert_eq!(next.generation(), 2);
    assert_eq!(s.perception_players()[0].actor, Some(id(1)));
    reject(&mut s, |s| s.set_actor_target(id(0), Some(id(1))));
    reject(&mut s, |s| {
        s.set_perception_player(
            0,
            PerceptionPlayer {
                active: true,
                health: 100,
                actor: Some(id(1)),
            },
        )
    });
    s.set_actor_target(id(0), Some(next)).unwrap();
    s.pos_attack(id(0)).unwrap();
}
#[test]
fn removable_effect_references_and_unbounded_setup_are_atomic() {
    let mut s = setup("pos_hit");
    s.pos_attack(id(0)).unwrap();
    let effect = s.actor_order().last().unwrap();
    reject(&mut s, |s| s.set_actor_target(id(0), Some(effect)));
    reject(&mut s, |s| {
        s.set_perception_player(
            0,
            PerceptionPlayer {
                active: true,
                health: 100,
                actor: Some(effect),
            },
        )
    });
    reject(&mut s, |s| s.noise_alert(effect, id(0)));
    for field in 0..4 {
        let mut p = PursuitSetup::from_actor(s.actor(id(0)).unwrap()).unwrap();
        match field {
            0 => p.movecount = i32::MIN,
            1 => p.reaction_time = i32::MIN,
            2 => p.reaction_time = i32::MIN + 1,
            _ => p.threshold = i32::MIN,
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
    reject(&mut s, |s| {
        s.set_vertical_momentum(id(1), Fixed::from_bits(i32::MAX))
    });
    reject(&mut s, |s| {
        s.set_momentum(id(1), [Fixed::from_bits(i32::MAX), Fixed::ZERO])
    });
}
#[test]
fn null_actions_and_pure_melee_preserve_source_short_circuits() {
    let mut s = setup("pos_null");
    let before = s.snapshot();
    let d = s.check_melee_range(id(0)).unwrap();
    assert!(!d.accepted);
    assert_eq!(d.reason, 1);
    assert_eq!(before, s.snapshot());
    let rng = s.rng();
    let angle = s.actor(id(0)).unwrap().angle();
    s.face_target(id(0)).unwrap();
    s.pos_attack(id(0)).unwrap();
    assert_eq!(s.rng(), rng);
    assert_eq!(s.actor(id(0)).unwrap().angle(), angle);
    reject(&mut s, |s| s.check_missile_range(id(0)));
}
#[test]
fn pause_instances_and_presentation_reads_cannot_change_attack_results() {
    let mut a = setup("thinker_attack_cycles");
    let mut b = setup("thinker_attack_cycles");
    a.set_paused(true);
    let paused = a.snapshot();
    for _ in 0..8 {
        assert_eq!(a.step([TicCommand::default(); 4]).unwrap(), paused);
    }
    a.set_paused(false);
    for _ in 0..110 {
        let before = a.snapshot();
        let _ = a.actor(id(0));
        let _ = a.pending_events();
        let _ = a.events();
        let _ = a.snapshot().canonical_bytes();
        assert_eq!(a.snapshot(), before);
        assert_eq!(
            a.step([TicCommand::default(); 4]).unwrap(),
            b.step([TicCommand::default(); 4]).unwrap()
        );
    }
}
