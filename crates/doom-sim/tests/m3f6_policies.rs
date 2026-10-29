//! Namespace/storage policy, separate from the compiled-C comparison test.
#[path = "../../../tests/support/m3f5.rs"]
mod fixtures;
use doom_sim::{identity::*, *};
use doom_types::Fixed;

#[test]
fn every_native_candidate_is_checked_without_canonical_reinterpretation() {
    let registry = ActorRegistry::demon_map_context_fixture();
    let binding = DemonExecutionBinding::admit(&registry).unwrap();
    for raw in 0..=u16::MAX {
        let ty = binding.check_native_type(ActorTypeId(raw));
        assert_eq!(ty.is_ok(), raw == 136);
        if let Ok(ty) = ty {
            assert_eq!(ty.runtime(), ActorTypeId(raw));
            assert_eq!(ty.original(), OriginalActorType::MT_SERGEANT);
        }
        let state = binding.check_native_state(StateId(raw));
        assert_eq!(state.is_ok(), (1..=21).contains(&raw));
        if let Ok(state) = state {
            assert_eq!(state.runtime(), StateId(raw));
            assert_eq!(binding.native_state(state.original()), Ok(state));
            assert_ne!(state.original().ordinal(), raw);
        }
    }
    // These numbers are valid original identities with completely different records.
    assert_eq!(
        OriginalActorType::try_from(136u16).unwrap().name(),
        "MT_MISC86"
    );
    assert_eq!(OriginalState::try_from(1u16).unwrap().name(), "S_LIGHTDONE");
    assert!(
        binding
            .native_state(OriginalState::try_from(1u16).unwrap())
            .is_err()
    );
    assert!(binding.native_type(OriginalActorType::MT_MISC86).is_err());
    assert!(binding.check_native_type(ActorTypeId(12)).is_err());
    assert!(registry.original_actor_type(ActorTypeId(136)).is_err());
    assert!(registry.original_state(StateId(1)).is_err());
}

#[test]
fn canonical_values_have_only_the_declared_native_execution_closure() {
    let registry = ActorRegistry::demon_map_context_fixture();
    let b = DemonExecutionBinding::admit(&registry).unwrap();
    for state in OriginalState::all() {
        assert_eq!(
            b.native_state(state).is_ok(),
            (475..=495).contains(&state.ordinal())
        );
    }
    for ty in OriginalActorType::all() {
        assert_eq!(
            b.native_type(ty).is_ok(),
            ty == OriginalActorType::MT_SERGEANT
        );
    }
    assert_eq!(
        std::mem::size_of::<NativeDemonState>(),
        std::mem::size_of::<StateId>()
    );
    assert_eq!(std::mem::size_of::<NativeDemonType>(), 0);
}

#[test]
fn typed_observation_grants_no_pain_death_raise_or_foreign_entry() {
    let case = &fixtures::cases()[0];
    let owner = ActorId::from_parts(0, 1);
    for original in OriginalState::all() {
        if !(475..=501).contains(&original.ordinal()) {
            continue;
        }
        let mut sim = fixtures::setup(case);
        let before = sim.snapshot();
        let result = sim.enter_original_demon_state(owner, original);
        let direct = (475..=487).contains(&original.ordinal());
        assert_eq!(result.is_ok(), direct, "{}", original.name());
        if !direct {
            assert_eq!(sim.snapshot(), before);
        }
    }
    let mut sim = fixtures::setup(case);
    for raw in 475..=501 {
        let before = sim.snapshot();
        assert!(sim.enter_actor_state(owner, StateId(raw)).is_err());
        assert_eq!(sim.snapshot(), before);
    }
    let body = ActorId::from_parts(1, 1);
    assert!(sim.native_demon_identity(body).is_err());
    let before = sim.snapshot();
    assert!(
        sim.spawn_type(ActorTypeId(12), [Fixed::ZERO; 2], SpawnHeight::Floor)
            .is_err()
    );
    assert_eq!(sim.snapshot(), before);
}

#[test]
fn views_are_pure_and_do_not_change_native_storage_or_replay() {
    let case = &fixtures::cases()[0];
    let (mut observed, mut control) = (fixtures::setup(case), fixtures::setup(case));
    let owner = ActorId::from_parts(0, 1);
    for op in 0..=80 {
        let before = observed.snapshot();
        let (ty, state) = observed.native_demon_identity(owner).unwrap();
        let life = observed.actor(owner).unwrap().life().unwrap();
        assert_eq!(ty.runtime(), life.actor_type);
        assert_eq!(state.map(NativeDemonState::runtime), life.state);
        assert_eq!(
            observed.original_demon_identity(owner),
            Ok((ty.original(), state.map(NativeDemonState::original)))
        );
        assert_eq!(before, observed.snapshot());
        assert_eq!(observed.snapshot(), control.snapshot());
        fixtures::assert_golden(&observed, 0, op);
        if op < 80 {
            fixtures::advance(&mut observed, case, op + 1);
            fixtures::advance(&mut control, case, op + 1);
        }
    }
}

#[test]
fn pending_retired_reused_and_wrong_profile_records_keep_their_boundaries() {
    let mut sim = fixtures::setup(&fixtures::cases()[0]);
    let owner = ActorId::from_parts(0, 1);
    sim.mark_remove(owner).unwrap();
    assert!(sim.native_demon_identity(owner).is_ok());
    sim.step([TicCommand::default(); 4]).unwrap();
    assert!(sim.native_demon_identity(owner).is_err());
    let replacement = sim
        .spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(64); 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(replacement.slot(), owner.slot());
    assert_ne!(replacement.generation(), owner.generation());
    for id in [owner, replacement] {
        assert!(sim.native_demon_identity(id).is_err());
    }
    let old = Simulation::actor_states(fixtures::map(0)).unwrap();
    assert_eq!(
        old.native_demon_identity(owner),
        Err(CanonicalDemonError::Profile)
    );
}
