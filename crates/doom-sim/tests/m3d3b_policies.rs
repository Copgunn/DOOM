//! Rust-only safety policies, separate from the original-C equivalence suite.
#[path = "../../../tests/support/m3d3b.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
fn id(slot: u32) -> ActorId {
    ActorId::from_parts(slot, 1)
}
fn fixture(label: &str) -> (Simulation, fixtures::Case) {
    let c = fixtures::cases()
        .into_iter()
        .find(|c| c.label == label)
        .unwrap();
    (fixtures::setup(&c), c)
}
fn reject<T: std::fmt::Debug>(
    s: &mut Simulation,
    f: impl FnOnce(&mut Simulation) -> Result<T, SimError>,
) {
    let before = s.snapshot();
    assert!(f(s).is_err());
    assert_eq!(s.snapshot(), before);
}
fn edit(s: &mut Simulation, actor: ActorId, f: impl FnOnce(&mut PursuitSetup)) {
    let mut p = PursuitSetup::from_actor(s.actor(actor).unwrap()).unwrap();
    f(&mut p);
    s.configure_pursuit_actor(actor, p).unwrap();
}
#[test]
fn invalid_direction_countdowns_flags_and_handles_are_atomic() {
    let (mut s, _) = fixture("chase_count_zero");
    for n in [9, 255] {
        let mut p = PursuitSetup::from_actor(s.actor(id(0)).unwrap()).unwrap();
        p.movedir = n;
        reject(&mut s, |s| s.configure_pursuit_actor(id(0), p));
    }
    for (field, value) in [
        (0, i32::MIN),
        (1, i32::MIN),
        (1, i32::MIN + 1),
        (2, i32::MIN),
    ] {
        let mut p = PursuitSetup::from_actor(s.actor(id(0)).unwrap()).unwrap();
        match field {
            0 => p.movecount = value,
            1 => p.reaction_time = value,
            _ => p.threshold = value,
        };
        reject(&mut s, |s| s.configure_pursuit_actor(id(0), p));
    }
    let mut p = PursuitSetup::from_actor(s.actor(id(0)).unwrap()).unwrap();
    p.flags |= 16384;
    reject(&mut s, |s| s.configure_pursuit_actor(id(0), p));
    for actor in [ActorId::from_parts(0, 2), id(99)] {
        reject(&mut s, |s| s.chase(actor));
        reject(&mut s, |s| s.try_walk(actor));
        reject(&mut s, |s| s.new_chase_direction(actor));
    }
}
#[test]
fn attack_float_speed_and_recursive_fixture_admission_reject() {
    let (_, c) = fixture("spawn_defaults");
    for (property, value) in [(10, 1), (11, 155), (21, 6 | 16384), (15, 0), (15, 17)] {
        let r = fixtures::registry(&c);
        let mut types = r.types().to_vec();
        types[136].properties[property] = value;
        let r = ActorRegistry::new(r.states().to_vec(), types).unwrap();
        assert!(
            Simulation::with_pursuit_registry(fixtures::map(0), 64, r, ActorOptions::default())
                .is_err()
        );
    }
    let r = fixtures::registry(&c);
    let mut states = r.states().to_vec();
    states[2].tics = 0;
    let r = ActorRegistry::new(states, r.types().to_vec()).unwrap();
    assert!(
        Simulation::with_pursuit_registry(fixtures::map(0), 64, r, ActorOptions::default())
            .is_err()
    );
    let mut s = Simulation::monster_pursuit(fixtures::map(0)).unwrap();
    reject(&mut s, |s| {
        s.spawn_type(ActorTypeId(1), [Fixed::ZERO; 2], SpawnHeight::Floor)
    });
}
fn map_special(name: &[u8], offset: usize) -> doom_map::Map {
    let mut b = include_bytes!("../../../tests/reference/m3d3b_map_0.bin").to_vec();
    let count = i32::from_le_bytes(b[4..8].try_into().unwrap()) as usize;
    let directory = i32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
    for i in 0..count {
        let d = directory + i * 16;
        if b[d + 8..d + 8 + name.len()] == *name {
            let start = i32::from_le_bytes(b[d..d + 4].try_into().unwrap()) as usize;
            b[start + offset..start + offset + 2].copy_from_slice(&1_i16.to_le_bytes());
        }
    }
    let w = doom_wad::Wad::parse(&b, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&w, "MAP01", doom_map::MapLimits::default()).unwrap()
}
#[test]
fn line_and_sector_specials_reject_without_executing_door_services() {
    for (name, offset) in [(b"LINEDEFS".as_slice(), 6), (b"SECTORS".as_slice(), 22)] {
        assert!(matches!(
            Simulation::monster_pursuit(map_special(name, offset)),
            Err(SimError::Pursuit(PursuitError::Specials))
        ));
    }
}
#[test]
fn empty_roster_terminates_and_missing_target_needs_new_direction_admission() {
    let (mut s, _) = fixture("chase_missing_reacquires");
    reject(&mut s, |s| s.new_chase_direction(id(0)));
    for i in 0..4 {
        s.set_perception_player(i, PerceptionPlayer::default())
            .unwrap();
    }
    let lastlook = s.actor(id(0)).unwrap().lastlook();
    s.chase(id(0)).unwrap();
    assert_eq!(s.actor(id(0)).unwrap().lastlook(), lastlook);
    assert_eq!(
        s.actor(id(0)).unwrap().life().unwrap().state,
        Some(StateId(1))
    );
    assert_eq!(s.actor(id(0)).unwrap().life().unwrap().target, None);
}
#[test]
fn pending_targets_reject_before_counters_targets_movement_events_and_rng() {
    let (mut s, _) = fixture("chase_count_zero");
    s.mark_remove(id(1)).unwrap();
    s.set_perception_player(0, PerceptionPlayer::default())
        .unwrap();
    reject(&mut s, |s| s.chase(id(0)));
    reject(&mut s, |s| s.look(id(0)));
    reject(&mut s, |s| s.new_chase_direction(id(0)));
    reject(&mut s, |s| s.step([TicCommand::default(); 4]));
    reject(&mut s, |s| s.set_actor_target(id(0), Some(id(1))));
}
#[test]
fn retired_and_reused_slots_preserve_inactive_identity_without_aliasing() {
    let (mut s, _) = fixture("chase_count_zero");
    s.set_perception_player(
        1,
        PerceptionPlayer {
            active: false,
            health: 100,
            actor: Some(id(1)),
        },
    )
    .unwrap();
    s.set_perception_player(0, PerceptionPlayer::default())
        .unwrap();
    s.set_actor_target(id(0), None).unwrap();
    s.mark_remove(id(1)).unwrap();
    s.step([TicCommand::default(); 4]).unwrap();
    let new = s
        .spawn_type(
            ActorTypeId(0),
            [Fixed::from_map_units(96), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(new.slot(), 1);
    assert_ne!(new.generation(), 1);
    assert_eq!(s.perception_players()[1].actor, Some(id(1)));
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
}
#[test]
fn stale_sound_cache_rejects_before_a_chase_can_return_to_look() {
    let (mut s, _) = fixture("chase_count_zero");
    s.enter_actor_state(id(0), StateId(2)).unwrap();
    s.noise_alert(id(1), id(0)).unwrap();
    s.set_perception_player(0, PerceptionPlayer::default())
        .unwrap();
    s.set_actor_target(id(0), None).unwrap();
    s.mark_remove(id(1)).unwrap();
    reject(&mut s, |s| s.chase(id(0)));
    reject(&mut s, |s| s.step([TicCommand::default(); 4]));
}
#[test]
fn immediate_callback_margin_preflights_before_look_or_state_entry() {
    let (_, c) = fixture("spawn_defaults");
    let mut s = Simulation::with_pursuit_registry(
        fixtures::map(0),
        64,
        fixtures::registry(&c),
        ActorOptions::default(),
    )
    .unwrap();
    let a = s
        .spawn_type(
            ActorTypeId(136),
            [Fixed::from_map_units(1009), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    reject(&mut s, |s| s.look(a));
    reject(&mut s, |s| s.enter_actor_state(a, StateId(2)));
    reject(&mut s, |s| s.move_pursuer(a));
    reject(&mut s, |s| s.step([TicCommand::default(); 4]));
}
#[test]
fn nested_target_loss_look_chase_proves_two_reaction_decrements() {
    let (mut s, _) = fixture("chase_missing_reacquires");
    let mut p = s.perception_players()[0];
    p.health = 0;
    s.set_perception_player(0, p).unwrap();
    s.noise_alert(id(1), id(0)).unwrap();
    edit(&mut s, id(0), |p| p.reaction_time = i32::MIN + 2);
    s.chase(id(0)).unwrap();
    assert_eq!(
        s.actor(id(0)).unwrap().life().unwrap().reaction_time,
        i32::MIN
    );
    reject(&mut s, |s| s.chase(id(0)));
    reject(&mut s, |s| s.step([TicCommand::default(); 4]));
}
#[test]
fn capacity_rejects_before_spawn_rng_and_canonical_mutation() {
    let (_, c) = fixture("spawn_defaults");
    let mut s = Simulation::with_pursuit_registry(
        fixtures::map(0),
        1,
        fixtures::registry(&c),
        ActorOptions::default(),
    )
    .unwrap();
    s.spawn_type(ActorTypeId(136), [Fixed::ZERO; 2], SpawnHeight::Floor)
        .unwrap();
    reject(&mut s, |s| {
        s.spawn_type(ActorTypeId(0), [Fixed::ZERO; 2], SpawnHeight::Floor)
    });
    assert!(
        Simulation::with_pursuit_registry(
            fixtures::map(0),
            65,
            fixtures::registry(&c),
            ActorOptions::default()
        )
        .is_err()
    );
}
#[test]
fn pause_independent_instances_and_presentation_reads_preserve_pursuit() {
    let (mut a, c) = fixture("thinker_acquire_and_pursue");
    let mut b = fixtures::setup(&c);
    for op in 1..=12 {
        let before = a.snapshot();
        a.check_sight(id(0), id(1)).unwrap();
        assert_eq!(a.snapshot(), before);
        fixtures::advance(&mut a, &c, op);
        fixtures::advance(&mut b, &c, op);
        assert_eq!(a.snapshot(), b.snapshot());
    }
    a.set_paused(true);
    let before = a.snapshot();
    assert_eq!(a.step([TicCommand::default(); 4]).unwrap(), before);
    assert_eq!(a.snapshot(), before);
}
#[test]
fn pure_sight_and_profile_boundaries_remain_deliberate() {
    let (mut s, _) = fixture("chase_count_zero");
    reject(&mut s, |s| s.chase(id(1)));
    reject(&mut s, |s| s.bind_player(0, id(1)));
    reject(&mut s, |s| s.enter_actor_state(id(1), StateId(2)));
    reject(&mut s, |s| s.enter_actor_state(id(0), StateId(0)));
    s.mark_remove(id(1)).unwrap();
    let before = s.snapshot();
    assert!(s.check_sight(id(0), id(2)).is_ok());
    assert_eq!(s.snapshot(), before);
}
