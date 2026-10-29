//! Original-C metadata and separately labeled Rust identity policy tests.
#[path = "../../../tests/support/m3c.rs"]
mod fixtures;
use doom_sim::{identity::*, *};
use doom_types::Fixed;

#[test]
fn every_identity_and_descriptor_matches_compiled_c() {
    let mut counts = [0; 3];
    let registry = ActorRegistry::vanilla();
    let mut actions = std::collections::BTreeSet::new();
    let mut rows = include_str!("../../../tests/reference/m3f4_identities.csv").lines();
    assert_eq!(rows.next(), Some("DOOM-IDENTITY-v1"));
    assert_eq!(rows.next(), Some("counts,137,967,138"));
    for row in rows {
        let c: Vec<_> = row.split(',').collect();
        let id: u16 = c[1].parse().unwrap();
        match c[0] {
            "type" => {
                assert_eq!(usize::from(id), counts[0]);
                counts[0] += 1;
                let t = OriginalActorType::try_from(id).unwrap();
                assert_eq!(t.name(), c[2]);
                assert_eq!(OriginalActorType::from_name(c[2]), Some(t));
                assert_eq!(
                    t.properties().as_slice(),
                    &c[3..]
                        .iter()
                        .map(|s| s.parse::<i32>().unwrap())
                        .collect::<Vec<_>>()
                );
                let d = t.descriptor();
                let p = t.properties();
                assert_eq!(
                    [
                        d.doomed_number,
                        d.spawn_health,
                        d.reaction_time,
                        d.pain_chance,
                        d.speed,
                        d.radius,
                        d.height,
                        d.mass,
                        d.damage,
                        d.flags as i32
                    ],
                    [
                        p[0], p[2], p[5], p[8], p[15], p[16], p[17], p[18], p[19], p[21]
                    ]
                );
                assert_eq!(d.sounds.map(i32::from), [p[4], p[6], p[9], p[14], p[20]]);
                let s = t.states();
                for (state, field) in [
                    s.spawn,
                    s.see,
                    s.pain,
                    s.melee,
                    s.missile,
                    s.death,
                    s.extreme_death,
                    s.raise,
                ]
                .into_iter()
                .zip([1, 3, 7, 10, 11, 12, 13, 22])
                {
                    assert_eq!(
                        i32::from(state.ordinal()),
                        c[3 + field].parse::<i32>().unwrap()
                    );
                }
                assert_eq!(
                    registry.original_actor_type(t.resolve(&registry).unwrap()),
                    Ok(t)
                );
            }
            "state" => {
                assert_eq!(usize::from(id), counts[1]);
                counts[1] += 1;
                let s = OriginalState::try_from(id).unwrap();
                assert_eq!(s.name(), c[2]);
                assert_eq!(OriginalState::from_name(c[2]), Some(s));
                let d = s.descriptor();
                assert_eq!(i32::from(d.sprite.ordinal()), c[3].parse::<i32>().unwrap());
                assert_eq!(d.frame, c[4].parse::<u32>().unwrap());
                assert_eq!(d.tics, c[5].parse::<i32>().unwrap());
                assert_eq!(d.action.unwrap_or("NULL"), c[6]);
                if let Some(a) = d.action {
                    actions.insert(a);
                }
                assert_eq!(i32::from(d.next.ordinal()), c[7].parse::<i32>().unwrap());
                assert_eq!(d.misc, [c[8].parse().unwrap(), c[9].parse().unwrap()]);
                assert_eq!(
                    registry.original_state(s.resolve(&registry).unwrap()),
                    Ok(s)
                );
            }
            "sprite" => {
                assert_eq!(usize::from(id), counts[2]);
                counts[2] += 1;
                let s = OriginalSprite::try_from(id).unwrap();
                assert_eq!(s.name(), c[2]);
                assert_eq!(OriginalSprite::from_name(c[2]), Some(s));
                assert_eq!(s.resource_name(), c[3]);
            }
            _ => panic!("unknown identity capture row"),
        }
    }
    assert_eq!(counts, [137, 967, 138]);
    assert_eq!(actions.len(), 74);
    assert_eq!(OriginalActorType::MT_SERGEANT.ordinal(), 12);
    assert_eq!(OriginalActorType::MT_SHADOWS.ordinal(), 13);
    assert_ne!(
        OriginalActorType::MT_SERGEANT,
        OriginalActorType::MT_SHADOWS
    );
}

#[test]
fn rust_policy_all_u16_values_and_signed_boundaries() {
    for n in 0..=u16::MAX {
        assert_eq!(
            OriginalActorType::try_from(n).is_ok(),
            n < OriginalActorType::COUNT
        );
        assert_eq!(OriginalState::try_from(n).is_ok(), n < OriginalState::COUNT);
        assert_eq!(
            OriginalSprite::try_from(n).is_ok(),
            n < OriginalSprite::COUNT
        );
    }
    for n in [i32::MIN, -65536, -1, 65536, i32::MAX] {
        assert_eq!(
            OriginalActorType::try_from(n),
            Err(IdentityError::OutOfRange)
        );
        assert_eq!(OriginalState::try_from(n), Err(IdentityError::OutOfRange));
        assert_eq!(OriginalSprite::try_from(n), Err(IdentityError::OutOfRange));
    }
    assert_eq!(OriginalActorType::all().count(), 137);
    assert_eq!(OriginalState::all().count(), 967);
    assert_eq!(OriginalSprite::all().count(), 138);
    assert_eq!(OriginalActorType::from_name("MT_DEMON"), None);
    assert_eq!(OriginalState::from_name("s_null"), None);
    assert_eq!(OriginalSprite::from_name("SARG"), None);
    assert_eq!(OriginalState::S_NULL.ordinal(), 0);
    assert_eq!(OriginalState::S_NULL.descriptor().tics, -1);
    assert_eq!(
        OriginalState::S_SARG_DIE6.descriptor().next,
        OriginalState::S_NULL
    );
    assert_eq!(OriginalState::S_SARG_DIE6.descriptor().tics, -1);
}

#[test]
fn rust_policy_fixture_aliases_are_not_original_identities() {
    let native = ActorRegistry::demon_map_context_fixture();
    assert_eq!(
        native.original_actor_type(ActorTypeId(136)),
        Err(IdentityError::RegistryMismatch)
    );
    assert_eq!(
        native.original_actor_type(ActorTypeId(u16::MAX)),
        Err(IdentityError::OutOfRange)
    );
    for id in [1, 2, 3, 15, 21] {
        assert_eq!(
            native.original_state(StateId(id)),
            Err(IdentityError::RegistryMismatch)
        );
    }
    // Recognition of untouched metadata does not authorize its gameplay.
    let registry = ActorRegistry::vanilla();
    assert!(OriginalActorType::MT_SERGEANT.resolve(&registry).is_ok());
    let mut sim =
        Simulation::with_actor_registry(fixtures::map(&[]), 8, registry, ActorOptions::default())
            .unwrap();
    let before = sim.snapshot();
    assert!(
        sim.spawn_type(
            OriginalActorType::MT_SERGEANT
                .resolve(sim.actor_registry().unwrap())
                .unwrap(),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor
        )
        .is_err()
    );
    assert_eq!(sim.snapshot(), before);
}

#[test]
fn rust_policy_every_actor_property_and_state_field_is_checked() {
    let source = ActorRegistry::vanilla();
    let ty = OriginalActorType::MT_SERGEANT;
    for field in 0..23 {
        let mut types = source.types().to_vec();
        // Keep state/sound references structurally valid so mismatch is tested separately.
        types[usize::from(ty.ordinal())].properties[field] ^= 1;
        let changed = ActorRegistry::new(source.states().to_vec(), types).unwrap();
        assert_eq!(
            ty.resolve(&changed),
            Err(IdentityError::RegistryMismatch),
            "property {field}"
        );
    }
    let state = OriginalState::S_SARG_RUN1;
    for field in 0..8 {
        let mut states = source.states().to_vec();
        let s = &mut states[usize::from(state.ordinal())];
        match field {
            0 => s.name.push('_'),
            1 => s.sprite ^= 1,
            2 => s.frame ^= 1,
            3 => s.tics += 1,
            4 => s.action = ActorAction::None,
            5 => s.next = StateId(0),
            6 => s.misc[0] = 1,
            7 => s.misc[1] = 1,
            _ => unreachable!(),
        }
        let changed = ActorRegistry::new(states, source.types().to_vec()).unwrap();
        assert_eq!(
            state.resolve(&changed),
            Err(IdentityError::RegistryMismatch),
            "state field {field}"
        );
    }
    let mut types = source.types().to_vec();
    types[12].name = "renamed".into();
    let renamed = ActorRegistry::new(source.states().to_vec(), types).unwrap();
    assert_eq!(ty.resolve(&renamed), Err(IdentityError::RegistryMismatch));
}

#[test]
fn existing_state_chronology_rng_and_bytes_survive_identity_queries() {
    // Independent original-C DSIM-v4 observations: queries cannot supply expected bytes.
    let mut queried = fixtures::setup(0);
    let mut control = fixtures::setup(0);
    for tic in 1..=80 {
        let before = queried.snapshot();
        let registry = queried.actor_registry().unwrap();
        for id in OriginalActorType::all() {
            assert!(id.resolve(registry).is_ok());
        }
        for id in OriginalState::all() {
            assert!(id.resolve(registry).is_ok());
        }
        assert_eq!(queried.snapshot(), before);
        let got = queried.step([TicCommand::default(); 4]).unwrap();
        assert_eq!(got, control.step([TicCommand::default(); 4]).unwrap());
        let row = include_str!("../../../tests/reference/m3c_snapshots.csv")
            .lines()
            .skip(1)
            .find(|row| {
                let c: Vec<_> = row.split(',').collect();
                c[0] == "0" && c[1].parse::<usize>().unwrap() == tic
            })
            .unwrap();
        assert_eq!(
            format!("{:016x}", got.digest()),
            row.split(',').nth(3).unwrap()
        );
    }
}

#[test]
fn canonical_resolution_drives_existing_typed_owner_services() {
    let mut canonical = Simulation::actor_states(fixtures::map(&[])).unwrap();
    let mut legacy = Simulation::actor_states(fixtures::map(&[])).unwrap();
    let fog = OriginalActorType::MT_TFOG;
    let slot = fog.resolve(canonical.actor_registry().unwrap()).unwrap();
    let a = canonical
        .spawn_type(slot, [Fixed::ZERO; 2], SpawnHeight::Floor)
        .unwrap();
    let b = legacy
        .spawn_type(
            ActorTypeId(fog.ordinal()),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(a, b);
    let life = canonical.actor(a).unwrap().life().unwrap();
    let registry = canonical.actor_registry().unwrap();
    assert_eq!(registry.original_actor_type(life.actor_type), Ok(fog));
    assert_eq!(
        registry.original_state(life.state.unwrap()),
        Ok(fog.states().spawn)
    );
    assert_eq!(life.tics, fog.states().spawn.descriptor().tics);
    assert_eq!(canonical.snapshot(), legacy.snapshot());
    let state = OriginalState::S_TFOG2.resolve(registry).unwrap();
    canonical.enter_actor_state(a, state).unwrap();
    legacy
        .enter_actor_state(b, StateId(OriginalState::S_TFOG2.ordinal()))
        .unwrap();
    for _ in 0..80 {
        assert_eq!(
            canonical.step([TicCommand::default(); 4]).unwrap(),
            legacy.step([TicCommand::default(); 4]).unwrap()
        );
    }
    assert!(canonical.actor(a).is_none());
    // Exact name lookup must not recover a moved/reordered record.
    let original = ActorRegistry::vanilla();
    let mut types = original.types().to_vec();
    types.swap(12, 13);
    let moved = ActorRegistry::new(original.states().to_vec(), types).unwrap();
    assert_eq!(
        OriginalActorType::MT_SERGEANT.resolve(&moved),
        Err(IdentityError::RegistryMismatch)
    );
}
