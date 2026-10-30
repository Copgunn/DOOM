//! Rust admission, ownership and arithmetic policies; separate from C comparisons.
#[path = "../../../tests/support/m3g3.rs"]
mod source;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
fn rejected(sim: &mut Simulation, op: impl FnOnce(&mut Simulation) -> Result<(), SimError>) {
    let before = sim.snapshot();
    assert!(op(sim).is_err());
    assert_eq!(sim.snapshot(), before);
}
fn request(a: source::Actors) -> DamageRequest {
    DamageRequest {
        target: a.body,
        inflictor: Some(a.origin),
        source: Some(a.origin),
        amount: 20,
    }
}
fn fresh() -> (Simulation, source::Actors) {
    source::setup(&source::cases()[1])
}
#[test]
fn explicit_capability_preserves_old_policy_and_rejects_unadmitted_roles() {
    let policy = OriginalExecutionProfile::PlayerBodyKnockbackV1;
    let registry = OriginalExecutionRegistry::new(policy).unwrap();
    for ty in OriginalActorType::all() {
        assert_eq!(
            registry
                .admit_definition(DefinitionIdentity::Original(ty))
                .is_ok(),
            [
                OriginalActorType::MT_PLAYER,
                OriginalActorType::MT_SERGEANT,
                OriginalActorType::MT_PUFF,
                OriginalActorType::MT_BLOOD
            ]
            .contains(&ty)
        );
    }
    assert!(
        registry
            .admit_definition(DefinitionIdentity::Context(ContextRole::UnboundBody))
            .is_err()
    );
    let (sim, _) = fresh();
    assert_eq!(sim.original_execution_profile(), Some(policy));
    assert_eq!(sim.snapshot().version(), 22);
    let mut old =
        Simulation::original_player_map(source::map(), 64, MapContextOptions::default()).unwrap();
    let origin = old
        .spawn_original_actor(OriginalActorType::MT_SERGEANT)
        .unwrap()
        .spawned[0];
    let body = old
        .spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
        .unwrap();
    rejected(&mut old, |s| {
        s.damage_actor(DamageRequest {
            target: body,
            inflictor: Some(origin),
            source: None,
            amount: 1,
        })
        .map(|_| ())
    });
    assert_eq!(
        old.original_execution_profile(),
        Some(OriginalExecutionProfile::PlayerBodyLifecycleV1)
    );
    assert_ne!(
        policy.context_schema(),
        OriginalExecutionProfile::PlayerBodyLifecycleV1.context_schema()
    );
}
fn altered(lump: &str, edit: impl FnOnce(&mut [u8])) -> doom_map::Map {
    let mut bytes = source::MAP_BYTES[0].to_vec();
    let count = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let dir = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let entry = (0..count)
        .map(|i| dir + 16 * i)
        .find(|&at| bytes[at + 8..at + 16].starts_with(lump.as_bytes()))
        .unwrap();
    let start = u32::from_le_bytes(bytes[entry..entry + 4].try_into().unwrap()) as usize;
    let len = u32::from_le_bytes(bytes[entry + 4..entry + 8].try_into().unwrap()) as usize;
    edit(&mut bytes[start..start + len]);
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
#[test]
fn setup_rejects_geometry_and_settings_outside_the_proved_domain() {
    let diagonal = altered("VERTEXES", |b| {
        b[0..2].copy_from_slice(&(-127_i16).to_le_bytes())
    });
    let two_sided = altered("LINEDEFS", |b| {
        b[4..6].copy_from_slice(&5_u16.to_le_bytes());
        b[12..14].copy_from_slice(&0_i16.to_le_bytes());
    });
    let raised = altered("SECTORS", |b| b[0..2].copy_from_slice(&1_i16.to_le_bytes()));
    for map in [diagonal, two_sided, raised] {
        assert!(
            Simulation::original_player_knockback_map(map, 64, MapContextOptions::default())
                .is_err()
        );
    }
    for skill in [Skill::Baby, Skill::Easy, Skill::Hard, Skill::Nightmare] {
        let registry =
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerBodyKnockbackV1)
                .unwrap();
        assert!(
            Simulation::with_original_registry(
                source::map(),
                64,
                registry,
                ActorOptions {
                    skill,
                    ..Default::default()
                },
                MapContextOptions::default()
            )
            .is_err()
        );
    }
}
#[test]
fn malformed_pending_stale_and_wrong_role_damage_references_reject_atomically() {
    let (mut sim, a) = fresh();
    let puff = sim
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    let bad = [
        ActorId::from_parts(u32::MAX, 1),
        ActorId::from_parts(a.origin.slot(), 0),
        ActorId::from_parts(a.origin.slot(), 2),
        puff,
    ];
    for id in bad {
        for request in [
            DamageRequest {
                inflictor: Some(id),
                ..request(a)
            },
            DamageRequest {
                source: Some(id),
                ..request(a)
            },
            DamageRequest {
                target: id,
                ..request(a)
            },
        ] {
            rejected(&mut sim, |s| s.damage_actor(request).map(|_| ()));
        }
    }
    sim.mark_remove(a.origin).unwrap();
    rejected(&mut sim, |s| s.damage_actor(request(a)).map(|_| ()));
    sim.advance(source::COMMANDS).unwrap();
    assert_eq!(sim.actor_availability(a.origin), ActorAvailability::Retired);
    rejected(&mut sim, |s| s.damage_actor(request(a)).map(|_| ()));
    let reused = sim
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(reused.slot(), a.origin.slot());
    assert_ne!(reused.generation(), a.origin.generation());
    rejected(&mut sim, |s| s.damage_actor(request(a)).map(|_| ()));
}
#[test]
fn physical_domain_rejects_large_or_invalid_inputs_before_any_rng_or_event_change() {
    let (mut sim, a) = fresh();
    for amount in [-1, 1001, 1000] {
        rejected(&mut sim, |s| {
            s.damage_actor(DamageRequest {
                amount,
                ..request(a)
            })
            .map(|_| ())
        });
    }
    // Physical damage to a Demon is not admission of another gameplay closure.
    rejected(&mut sim, |s| {
        s.damage_actor(DamageRequest {
            target: a.origin,
            inflictor: Some(a.body),
            source: None,
            amount: 0,
        })
        .map(|_| ())
    });
    rejected(&mut sim, |s| s.set_momentum(a.body, [Fixed::ZERO; 2]));
    rejected(&mut sim, |s| {
        s.set_momentum(a.origin, [Fixed::from_map_units(1), Fixed::ZERO])
    });
    rejected(&mut sim, |s| {
        s.set_vertical_momentum(a.body, Fixed::from_map_units(1))
    });
    rejected(&mut sim, |s| {
        s.enter_actor_state(a.body, StateId(OriginalState::S_PLAY_ATK1.ordinal()))
    });
    rejected(&mut sim, |s| {
        s.advance(
            [TicCommand {
                forward: 1,
                ..Default::default()
            }; 4],
        )
    });
    // Without an inflictor the independently established old damage domain remains.
    sim.damage_actor(DamageRequest {
        inflictor: None,
        amount: 1000,
        ..request(a)
    })
    .unwrap();
    assert_eq!(sim.actor(a.body).unwrap().momentum(), [Fixed::ZERO; 2]);
}
#[test]
fn source_and_inflictor_stay_distinct_and_zero_damage_has_no_thrust_rng_draw() {
    let c = &source::cases()[3];
    let (mut sim, a) = source::setup(c);
    source::hit(&mut sim, a, c, 0).unwrap();
    assert_eq!(
        sim.original_player(PlayerSlot::One).unwrap().attacker,
        a.other
    );
    assert_eq!(sim.actor(a.body).unwrap().momentum(), [Fixed::ZERO; 2]);
    assert!(
        !sim.pending_events()
            .iter()
            .any(|e| matches!(e, Event::Combat(CombatEvent::Random { kind: 0, .. })))
    );
    let before = sim.rng();
    source::hit(&mut sim, a, c, 20).unwrap();
    assert_ne!(sim.rng(), before);
    assert_eq!(
        sim.original_player(PlayerSlot::One).unwrap().attacker,
        a.other
    );
    assert_ne!(sim.actor(a.body).unwrap().momentum(), [Fixed::ZERO; 2]);
    let p = sim.pending_events();
    let knock = p
        .iter()
        .rposition(|e| matches!(e, Event::Combat(CombatEvent::Knockback { .. })))
        .unwrap();
    let player = p
        .iter()
        .rposition(|e| matches!(e, Event::PlayerBody(PlayerBodyEvent::Damaged { .. })))
        .unwrap();
    let health = p
        .iter()
        .rposition(|e| matches!(e, Event::Combat(CombatEvent::Health { .. })))
        .unwrap();
    assert!(knock < player && player < health);
}
#[test]
fn forward_travel_boundary_is_exact_and_accepted_hits_remain_steppable() {
    for which in [1, 2] {
        let c = &source::cases()[which];
        // Source C-observed cardinal impulse; the safety envelope is independently
        // specified policy, not an original-C coordinate-limit claim.
        let g = &source::rows(which, 1)
            .iter()
            .find(|(tag, v)| tag == "G" && v[0] == 4)
            .unwrap()
            .1;
        let m = g[4].abs().max(g[5].abs());
        let travel = (160 * m + 2) / 3;
        let sign = if which == 1 { 1 } else { -1 };
        let edge = sign * (994 * 65536 - travel);
        for extra in [0, 1] {
            let mut sim = Simulation::original_player_knockback_map(
                source::case_map(c.map),
                64,
                MapContextOptions::default(),
            )
            .unwrap();
            let origin = sim
                .spawn_original_actor(OriginalActorType::MT_SERGEANT)
                .unwrap()
                .spawned[0];
            let body = sim
                .spawn_player_body(
                    PlayerSlot::One,
                    [
                        Fixed::from_bits(i32::try_from(edge + sign * extra).unwrap()),
                        Fixed::ZERO,
                    ],
                )
                .unwrap();
            sim.damage_actor(DamageRequest {
                target: origin,
                source: None,
                inflictor: None,
                amount: 150,
            })
            .unwrap();
            let hit = DamageRequest {
                target: body,
                source: Some(origin),
                inflictor: Some(origin),
                amount: 20,
            };
            if extra == 1 {
                rejected(&mut sim, |s| s.damage_actor(hit).map(|_| ()));
            } else {
                sim.damage_actor(hit).unwrap();
                for _ in 0..100 {
                    sim.advance(source::COMMANDS).unwrap();
                    assert!(
                        sim.actor(body)
                            .unwrap()
                            .position()
                            .iter()
                            .all(|p| p.bits().unsigned_abs() <= 994 * 65536)
                    );
                }
                assert_eq!(sim.actor(body).unwrap().momentum(), [Fixed::ZERO; 2]);
            }
        }
    }
    for x in [-994, 994] {
        let mut sim = Simulation::original_player_knockback_map(
            source::map(),
            64,
            MapContextOptions::default(),
        )
        .unwrap();
        let origin = sim
            .spawn_original_actor(OriginalActorType::MT_SERGEANT)
            .unwrap()
            .spawned[0];
        let body = sim
            .spawn_player_body(PlayerSlot::One, [Fixed::from_map_units(x), Fixed::ZERO])
            .unwrap();
        sim.damage_actor(DamageRequest {
            target: origin,
            source: None,
            inflictor: None,
            amount: 150,
        })
        .unwrap();
        sim.advance(source::COMMANDS).unwrap();
        rejected(&mut sim, |s| {
            s.damage_actor(DamageRequest {
                target: body,
                source: None,
                inflictor: Some(origin),
                amount: 1,
            })
            .map(|_| ())
        });
    }
}
#[test]
fn lethal_thrust_moves_an_allocated_corpse_and_repeated_damage_is_ignored() {
    let c = &source::cases()[12];
    let (mut sim, a) = source::setup(c);
    source::hit(&mut sim, a, c, c.amount).unwrap();
    let momentum = sim.actor(a.body).unwrap().momentum();
    assert_ne!(momentum, [Fixed::ZERO; 2]);
    let p = sim.original_player(PlayerSlot::One).unwrap();
    assert_eq!(p.state, PlayerBodyState::Dead);
    assert_eq!(p.health, 0);
    assert_eq!(p.mo, Some(a.body));
    assert_eq!(sim.actor(a.body).unwrap().life().unwrap().health, -119);
    let rng = sim.rng();
    assert!(source::hit(&mut sim, a, c, 1000).unwrap().ignored);
    assert_eq!(sim.rng(), rng);
    assert_eq!(sim.original_player(PlayerSlot::One).unwrap(), p);
    assert_eq!(sim.actor(a.body).unwrap().momentum(), momentum);
    let position = sim.actor(a.body).unwrap().position();
    sim.advance(source::COMMANDS).unwrap();
    assert_ne!(sim.actor(a.body).unwrap().position(), position);
    for _ in 0..99 {
        sim.advance(source::COMMANDS).unwrap();
    }
    assert_eq!(sim.actor(a.body).unwrap().life().unwrap().tics, -1);
    assert_eq!(sim.actor_availability(a.body), ActorAvailability::Live);
    assert_eq!(sim.actor(a.body).unwrap().momentum(), [Fixed::ZERO; 2]);
    assert_eq!(
        sim.original_player(PlayerSlot::One).unwrap().attacker,
        Some(a.origin)
    );
}
#[test]
fn retirement_and_slot_reuse_do_not_redirect_stored_player_references() {
    let (mut sim, a) = fresh();
    sim.damage_actor(request(a)).unwrap();
    sim.mark_remove(a.origin).unwrap();
    sim.advance(source::COMMANDS).unwrap();
    let replacement = sim
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(replacement.slot(), a.origin.slot());
    assert_ne!(replacement.generation(), a.origin.generation());
    assert_eq!(
        sim.original_player(PlayerSlot::One).unwrap().attacker,
        Some(a.origin)
    );
    rejected(&mut sim, |s| s.damage_actor(request(a)).map(|_| ()));
    sim.mark_remove(a.body).unwrap();
    sim.advance(source::COMMANDS).unwrap();
    let next = sim
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(next.slot(), a.body.slot());
    assert_ne!(next.generation(), a.body.generation());
    assert_eq!(
        sim.original_player(PlayerSlot::One).unwrap().mo,
        Some(a.body)
    );
    rejected(&mut sim, |s| {
        s.bind_original_player_body(PlayerSlot::One, next)
    });
}
#[test]
fn scoped_damage_requires_instance_provenance_and_retains_recoverable_atomicity() {
    let (sim, a) = fresh();
    let mut first = GameInstance::new(sim);
    let mut second = GameInstance::new(fresh().0);
    let foreign = first
        .with_execution_context(|ctx| {
            let body = ctx.admit_local(
                a.body,
                DefinitionIdentity::Original(OriginalActorType::MT_PLAYER),
            )?;
            ctx.export_actor(body)
        })
        .unwrap();
    let before = second.simulation().snapshot();
    second
        .with_execution_context(|ctx| {
            assert!(
                ctx.import_actor(
                    &foreign,
                    DefinitionIdentity::Original(OriginalActorType::MT_PLAYER)
                )
                .is_err()
            );
            Ok(())
        })
        .unwrap();
    assert_eq!(second.simulation().snapshot(), before);
    first
        .with_execution_context(|ctx| {
            let body = ctx.admit_local(
                a.body,
                DefinitionIdentity::Original(OriginalActorType::MT_PLAYER),
            )?;
            let origin = ctx.admit_local(
                a.origin,
                DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT),
            )?;
            let before = ctx.snapshot();
            assert!(ctx.damage(body, Some(origin), None, 1000).is_err());
            assert_eq!(ctx.snapshot(), before);
            ctx.damage(body, Some(origin), Some(origin), 20)?;
            ctx.advance(source::COMMANDS)?;
            Ok(())
        })
        .unwrap();
}
#[test]
fn every_new_source_input_replays_identically_through_both_stepping_apis() {
    for c in source::cases() {
        let (mut step, a) = source::setup(&c);
        let mut advance = step.clone();
        for op in 1..=c.tics {
            source::operations(&mut step, a, &c, op);
            source::operations(&mut advance, a, &c, op);
            let expected = step.step(source::COMMANDS).unwrap();
            advance.advance(source::COMMANDS).unwrap();
            assert_eq!(expected, advance.snapshot(), "{} op{op}", c.label);
        }
    }
}
fn field_edit(bytes: &[u8], wanted: &str, value: &[u8]) -> Vec<u8> {
    let mut out = bytes[..16].to_vec();
    let mut at = 16;
    while at < bytes.len() {
        let start = at;
        let n = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        at += 4;
        let path = std::str::from_utf8(&bytes[at..at + n]).unwrap();
        at += n;
        let len = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()) as usize;
        at += 8;
        let v = if path == wanted {
            value
        } else {
            &bytes[at..at + len]
        };
        out.extend_from_slice(&bytes[start..start + 4 + n]);
        out.extend_from_slice(&(v.len() as u64).to_le_bytes());
        out.extend_from_slice(v);
        at += len;
    }
    out
}
#[test]
fn diagnostic_policy_and_context_pairs_are_explicit_without_reinterpreting_old_versions() {
    let (sim, _) = fresh();
    let bytes = sim.snapshot().canonical_bytes();
    assert!(Diagnostic::decode(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_player(&bytes, DiagnosticLimits::default()).is_ok());
    for (field, value) in [
        ("profile", 2_u32.to_le_bytes().to_vec()),
        ("profile", 4_u32.to_le_bytes().to_vec()),
        ("context.schema", PLAYER_BODY_CONTEXT_SCHEMA.to_vec()),
        ("original_player.0.mo", vec![1; 13]),
    ] {
        assert!(
            Diagnostic::decode_player(
                &field_edit(&bytes, field, &value),
                DiagnosticLimits::default()
            )
            .is_err()
        );
    }
    for n in [0, 4, 8, 15, bytes.len() - 1] {
        assert!(Diagnostic::decode_player(&bytes[..n], DiagnosticLimits::default()).is_err());
    }
    assert!(
        Diagnostic::decode_player(
            &bytes,
            DiagnosticLimits {
                bytes: bytes.len() - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        Diagnostic::decode_player(
            include_bytes!("../../../tests/reference/m3g2_state_0_0.bin"),
            DiagnosticLimits::default()
        )
        .is_ok()
    );
}
#[test]
fn physical_hit_requires_an_executable_thinker_closure_before_committing_momentum() {
    let mut sim =
        Simulation::original_player_knockback_map(source::map(), 64, MapContextOptions::default())
            .unwrap();
    let origin = sim
        .spawn_original_actor(OriginalActorType::MT_SERGEANT)
        .unwrap()
        .spawned[0];
    let body = sim
        .spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
        .unwrap();
    let hit = DamageRequest {
        target: body,
        source: Some(origin),
        inflictor: Some(origin),
        amount: 20,
    };
    rejected(&mut sim, |s| s.damage_actor(hit).map(|_| ()));
    sim.damage_actor(DamageRequest {
        target: origin,
        source: None,
        inflictor: None,
        amount: 150,
    })
    .unwrap();
    sim.damage_actor(hit).unwrap();
    sim.advance(source::COMMANDS).unwrap();
}
