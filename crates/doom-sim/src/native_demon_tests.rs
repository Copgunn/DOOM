//! Private corruption tests of the representation guard, not C observations.
use super::*;
mod maps {
    include!("../../../tests/reference/m3f5_maps.rs");
}
fn setup() -> Simulation {
    let wad = doom_wad::Wad::parse(maps::MAP_BYTES[0], doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
    let mut sim = Simulation::canonical_demon_map(
        map,
        64,
        ActorOptions::default(),
        MapContextOptions::default(),
    )
    .unwrap();
    sim.spawn_original_demons(OriginalActorType::MT_SERGEANT)
        .unwrap();
    sim
}

#[test]
fn canonical_numbers_in_native_storage_reject_without_mutation() {
    let owner = ActorId::from_parts(0, 1);
    for raw in [0, 22, 475, 487, 495, 496, u16::MAX] {
        let mut sim = setup();
        sim.actors
            .get_mut(owner)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .state = Some(StateId(raw));
        let before = sim.snapshot();
        assert!(sim.native_demon_identity(owner).is_err());
        assert!(sim.step([TicCommand::default(); 4]).is_err());
        assert_eq!(sim.snapshot(), before);
    }
    let mut sim = setup();
    sim.actors
        .get_mut(owner)
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .actor_type = ActorTypeId(12);
    let before = sim.snapshot();
    assert!(sim.native_demon_identity(owner).is_err());
    assert!(sim.step([TicCommand::default(); 4]).is_err());
    assert_eq!(sim.snapshot(), before);
}

#[test]
fn named_corpse_and_entry_classifiers_cover_exact_historical_slots() {
    for raw in 0..=u16::MAX {
        if let Some(state) = NativeDemonState::checked(StateId(raw)) {
            assert_eq!(
                state.allows_direct_entry(),
                [1, 2, 3, 4, 5, 6, 15, 16, 17, 18, 19, 20, 21].contains(&raw)
            );
            assert_eq!(state.is_death(), (9..=14).contains(&raw));
            assert_eq!(state.before_fall(), (9..=11).contains(&raw));
        }
    }
}
