//! Private fault injection verifies the whole-pass preflight proof.
use crate::*;
fn setup() -> Simulation {
    let data = include_bytes!("../../../tests/reference/m3d3i_map_1.bin");
    let wad = doom_wad::Wad::parse(data, doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
    let mut s = Simulation::with_demon_map_spawn_registry(
        map,
        64,
        ActorRegistry::demon_stand_run_fixture(),
        ActorOptions::default(),
    )
    .unwrap();
    s.spawn_map_things().unwrap();
    let target = s
        .spawn_type(ActorTypeId(135), [Fixed::ZERO; 2], SpawnHeight::Floor)
        .unwrap();
    for id in [ActorId::from_parts(0, 1), ActorId::from_parts(1, 1)] {
        s.set_actor_target(id, Some(target)).unwrap();
        s.enter_actor_state(id, StateId(5)).unwrap();
        s.actors.get_mut(id).unwrap().life.as_mut().unwrap().tics = 1;
    }
    s
}
fn unchanged_failure(s: &mut Simulation) {
    let before = s.snapshot();
    assert!(s.step([TicCommand::default(); 4]).is_err());
    assert_eq!(s.snapshot(), before);
    assert!(s.demon_melee_attack(ActorId::from_parts(0, 1)).is_err());
    assert_eq!(s.snapshot(), before);
}
#[test]
fn later_body_death_action_rejects_before_earlier_strike_rng() {
    let mut s = setup();
    let r = s.registry.as_ref().unwrap();
    let mut states = r.states().to_vec();
    let death = r.actor_type(ActorTypeId(135)).unwrap().states()[5];
    states[death.0 as usize].action =
        ActorAction::Original(lifecycle::source_action_id("A_VileChase"));
    s.registry = Some(Arc::new(
        ActorRegistry::new(states, r.types().to_vec()).unwrap(),
    ));
    unchanged_failure(&mut s);
}
#[test]
fn later_bound_body_or_owner_rejects_before_any_thinker() {
    for n in [1, 2] {
        let mut s = setup();
        s.bindings[0] = Some(ActorId::from_parts(n, 1));
        unchanged_failure(&mut s);
    }
}
#[test]
fn later_invalid_state_counter_and_momentum_reject_whole_pass() {
    for kind in 0..4 {
        let mut s = setup();
        let a = s.actors.get_mut(ActorId::from_parts(1, 1)).unwrap();
        match kind {
            0 => a.life.as_mut().unwrap().state = Some(StateId(496)),
            1 => a.life.as_mut().unwrap().reaction_time = i32::MIN + 1,
            2 => a.life.as_mut().unwrap().target = Some(ActorId::from_parts(2, 2)),
            _ => {
                s.actors
                    .get_mut(ActorId::from_parts(2, 1))
                    .unwrap()
                    .spec
                    .momentum[0] = Fixed::from_map_units(65)
            }
        }
        unchanged_failure(&mut s);
    }
}
#[test]
fn native_after_target_suffix_and_removable_target_reject() {
    let mut s = setup();
    let registry = s.registry.as_deref().unwrap();
    lifecycle::LifeHost {
        player: None,
        statistics: None,
        perception: None,
        access: &mut s.actors,
        registry,
        world: &s.world,
        sectors: &s.sectors,
        options: s.actor_options,
        rng: &mut s.rng,
        spatial: &mut s.spatial,
        bindings: &mut s.bindings,
        events: &mut s.pending,
    }
    .spawn(ActorTypeId(136), [Fixed::ZERO; 2], SpawnHeight::Floor);
    unchanged_failure(&mut s);
    let mut s = setup();
    let effect = s
        .spawn_type(ActorTypeId(37), [Fixed::ZERO; 2], SpawnHeight::Floor)
        .unwrap();
    s.actors
        .get_mut(ActorId::from_parts(1, 1))
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .target = Some(effect);
    unchanged_failure(&mut s);
}
#[test]
fn pending_target_rejects_before_any_owner_movement_or_sound() {
    let mut s = setup();
    s.actors.mark_remove(ActorId::from_parts(2, 1)).unwrap();
    unchanged_failure(&mut s);
}
