//! Rust safety/admission and full-state stepping equivalence, not a C oracle.
#[path = "../../../tests/support/m3g4.rs"]
mod source;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
const ZERO: [TicCommand; 4] = [TicCommand {
    forward: 0,
    side: 0,
    angle_turn: 0,
    buttons: 0,
}; 4];
fn commands(forward: i8, side: i8, angle_turn: i16) -> [TicCommand; 4] {
    let mut c = ZERO;
    c[0] = TicCommand {
        forward,
        side,
        angle_turn,
        buttons: 0,
    };
    c
}
fn fresh() -> (Simulation, ActorId) {
    let (s, a) = source::setup(&source::cases()[0]);
    (s, a.body)
}
fn reject(s: &mut Simulation, op: impl FnOnce(&mut Simulation) -> Result<(), SimError>) {
    let before = s.snapshot();
    assert!(op(s).is_err());
    assert_eq!(s.snapshot(), before);
}
#[test]
fn capability_is_separate_and_does_not_admit_synthetic_bodies_or_other_species() {
    let registry =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerCommandMovementV1).unwrap();
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
    for policy in [
        OriginalExecutionProfile::PlayerBodyLifecycleV1,
        OriginalExecutionProfile::PlayerBodyKnockbackV1,
    ] {
        let registry = OriginalExecutionRegistry::new(policy).unwrap();
        let mut s = Simulation::with_original_registry(
            source::map(),
            64,
            registry,
            ActorOptions::default(),
            MapContextOptions::default(),
        )
        .unwrap();
        s.spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
            .unwrap();
        reject(&mut s, |s| s.advance(commands(1, 0, 0)));
        reject(&mut s, |s| {
            s.set_initial_player_reaction_time(PlayerSlot::One, 1)
        });
    }
}
#[test]
fn command_ranges_buttons_and_other_slots_reject_without_consuming_delay_or_rng() {
    let (mut s, _) = fresh();
    s.set_initial_player_reaction_time(PlayerSlot::One, 3)
        .unwrap();
    for n in [i8::MIN, -51, 51, i8::MAX] {
        reject(&mut s, |s| s.advance(commands(n, 0, 0)));
        reject(&mut s, |s| s.advance(commands(0, n, 0)));
    }
    for buttons in [1, 2, 4, 8, 128, 255] {
        let mut c = ZERO;
        c[0].buttons = buttons;
        reject(&mut s, |s| s.advance(c));
    }
    for slot in 1..4 {
        let mut c = ZERO;
        c[slot].angle_turn = 1;
        reject(&mut s, |s| s.advance(c));
    }
    for n in [1, 2, 3, 255] {
        assert!(PlayerSlot::try_from(n).is_err());
    }
    assert_eq!(
        s.original_player(PlayerSlot::One)
            .unwrap()
            .mo
            .and_then(|id| s.actor(id))
            .unwrap()
            .life()
            .unwrap()
            .reaction_time,
        3
    );
    s.advance(commands(50, -50, i16::MIN)).unwrap();
}
#[test]
fn reaction_gate_suppresses_the_entire_move_for_one_tic_and_setup_is_initial_only() {
    let (mut s, id) = fresh();
    let rng = s.rng();
    s.set_initial_player_reaction_time(PlayerSlot::One, 1)
        .unwrap();
    assert_eq!(s.rng(), rng);
    let c = commands(25, 10, -1234);
    s.advance(c).unwrap();
    let a = s.actor(id).unwrap();
    assert_eq!(a.angle(), 0);
    assert_eq!(a.momentum(), [Fixed::ZERO; 2]);
    assert_eq!(a.life().unwrap().reaction_time, 0);
    assert!(!s.events().iter().any(|e| matches!(e, Event::Player { .. })));
    s.advance(c).unwrap();
    assert_ne!(s.actor(id).unwrap().angle(), 0);
    assert!(
        s.events()
            .iter()
            .any(|e| matches!(e,Event::Player{actor,..} if *actor==id))
    );
    reject(&mut s, |s| {
        s.set_initial_player_reaction_time(PlayerSlot::One, 1)
    });
}
#[test]
fn body_binding_rejects_wrong_roles_invalid_generations_and_duplicate_ownership() {
    let (mut s, id) = fresh();
    for candidate in [
        id,
        ActorId::from_parts(id.slot(), 0),
        ActorId::from_parts(id.slot(), id.generation() + 1),
        ActorId::from_parts(u32::MAX, 1),
    ] {
        reject(&mut s, |s| {
            s.bind_original_player_body(PlayerSlot::One, candidate)
        });
    }
    reject(&mut s, |s| s.bind_player(0, id));
    reject(&mut s, |s| s.set_momentum(id, [Fixed::ONE; 2]));
    reject(&mut s, |s| s.set_vertical_momentum(id, Fixed::ONE));
    reject(&mut s, |s| {
        s.enter_actor_state(id, StateId(OriginalState::S_PLAY_RUN1.ordinal()))
    });
    let mut unbound =
        Simulation::original_player_command_map(source::map(), 64, MapContextOptions::default())
            .unwrap();
    let puff = unbound
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    reject(&mut unbound, |s| {
        s.bind_original_player_body(PlayerSlot::One, puff)
    });
    reject(&mut unbound, |s| s.advance(commands(1, 0, 0)));
    unbound.advance(ZERO).unwrap();
}
#[test]
fn pending_retired_and_reused_body_never_redirect_commands() {
    let (mut s, id) = fresh();
    s.mark_remove(id).unwrap();
    reject(&mut s, |s| s.advance(commands(0, 0, 1)));
    reject(&mut s, |s| {
        s.set_initial_player_reaction_time(PlayerSlot::One, 1)
    });
    s.advance(ZERO).unwrap();
    assert_eq!(s.actor_availability(id), ActorAvailability::Retired);
    let puff = s
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(puff.slot(), id.slot());
    assert_ne!(puff.generation(), id.generation());
    reject(&mut s, |s| s.advance(commands(1, 0, 0)));
    s.advance(ZERO).unwrap();
    assert!(!s.events().iter().any(|e| matches!(e, Event::Player { .. })));
}
#[test]
fn dead_body_accepts_only_inactive_commands_and_remains_a_corpse() {
    let (mut s, id) = fresh();
    s.damage_actor(DamageRequest {
        target: id,
        inflictor: None,
        source: None,
        amount: 100,
    })
    .unwrap();
    reject(&mut s, |s| s.advance(commands(1, 0, 0)));
    reject(&mut s, |s| {
        s.set_initial_player_reaction_time(PlayerSlot::One, 1)
    });
    for _ in 0..100 {
        s.advance(ZERO).unwrap();
    }
    assert_eq!(
        s.original_player(PlayerSlot::One).unwrap().state,
        PlayerBodyState::Dead
    );
    assert!(s.actor(id).is_some());
    assert_eq!(s.original_player(PlayerSlot::One).unwrap().mo, Some(id));
    assert!(!s.events().iter().any(|e| matches!(e, Event::Player { .. })));
}
#[test]
fn a_late_travel_postcondition_rejection_rolls_back_run_entry_events_and_all_state() {
    let mut s =
        Simulation::original_player_command_map(source::map(), 64, MapContextOptions::default())
            .unwrap();
    let id = s
        .spawn_player_body(PlayerSlot::One, [Fixed::from_map_units(993), Fixed::ZERO])
        .unwrap();
    reject(&mut s, |s| s.advance(commands(1, 0, 0)));
    assert_eq!(
        s.actor(id).unwrap().life().unwrap().state,
        Some(StateId(OriginalState::S_PLAY.ordinal()))
    );
    s.advance(ZERO).unwrap();
}
#[test]
fn every_source_sequence_matches_step_and_advance_then_has_a_safe_zero_command_continuation() {
    for c in source::cases() {
        let (mut a, ids) = source::setup(&c);
        let mut b = a.clone();
        for op in 1..=c.tics {
            source::operations(&mut a, ids, &c, op);
            source::operations(&mut b, ids, &c, op);
            a.advance(source::commands(&c, op)).unwrap();
            assert_eq!(a.snapshot(), b.step(source::commands(&c, op)).unwrap());
        }
        for _ in 0..100 {
            a.advance(ZERO).unwrap();
            assert_eq!(a.snapshot(), b.step(ZERO).unwrap());
        }
    }
}
#[test]
fn paused_and_rejected_step_advance_keep_existing_error_precedence_and_state() {
    let (mut a, _) = fresh();
    let mut b = a.clone();
    let mut bad = ZERO;
    bad[0].buttons = 255;
    assert_eq!(a.advance(bad), b.step(bad).map(|_| ()));
    assert_eq!(a.snapshot(), b.snapshot());
    a.set_paused(true);
    b.set_paused(true);
    a.advance(bad).unwrap();
    assert_eq!(a.snapshot(), b.step(bad).unwrap());
}
#[test]
fn source_command_stage_precedes_all_thinkers_even_when_the_player_was_spawned_second() {
    let c = &source::cases()[16];
    let (mut s, a) = source::setup(c);
    assert_ne!(a.body.slot(), 0);
    s.advance(source::commands(c, 1)).unwrap();
    let events = s.events();
    let applied = events
        .iter()
        .position(|e| matches!(e, Event::Player { .. }))
        .unwrap();
    let first = events
        .iter()
        .position(|e| matches!(e, Event::Think(_)))
        .unwrap();
    assert!(applied < first);
    assert!(
        events
            .iter()
            .any(|e| matches!(e,Event::Think(id) if Some(*id)==a.origin))
    );
}

#[test]
fn qualified_player_binding_rejects_other_instances_and_wrong_definitions() {
    let (left, raw) = fresh();
    let mut a = GameInstance::new(left.clone());
    let mut b = GameInstance::new(left);
    let reference = a
        .with_execution_context(|ctx| {
            let body = ctx.admit_local(
                raw,
                DefinitionIdentity::Original(OriginalActorType::MT_PLAYER),
            )?;
            Ok(ctx.export_actor(body).unwrap())
        })
        .unwrap();
    let before = b.simulation().snapshot();
    let err = b.with_execution_context(|ctx| {
        assert!(matches!(
            ctx.import_actor(
                &reference,
                DefinitionIdentity::Original(OriginalActorType::MT_PLAYER)
            ),
            Err(ProvenanceError::InstanceMismatch)
        ));
        Ok(())
    });
    assert!(err.is_ok());
    assert_eq!(before, b.simulation().snapshot());
    a.with_execution_context(|ctx| {
        assert!(
            ctx.import_actor(
                &reference,
                DefinitionIdentity::Context(ContextRole::UnboundBody)
            )
            .is_err()
        );
        Ok(())
    })
    .unwrap();
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
        ("profile", 3_u32.to_le_bytes().to_vec()),
        ("context.schema", PLAYER_KNOCKBACK_CONTEXT_SCHEMA.to_vec()),
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
            Simulation::original_player_command_map(map, 64, MapContextOptions::default()).is_err()
        );
    }
    for skill in [Skill::Baby, Skill::Easy, Skill::Hard, Skill::Nightmare] {
        let registry =
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerCommandMovementV1)
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
