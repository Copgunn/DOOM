//! Rust admission/lifetime policy and stepping equivalence, separate from C evidence.
#[path = "../../../tests/support/m3g5.rs"]
mod source;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
const ZERO: [TicCommand; 4] = [TicCommand {
    forward: 0,
    side: 0,
    angle_turn: 0,
    buttons: 0,
}; 4];
fn setup(index: usize) -> (Simulation, source::Actors) {
    source::setup(&source::cases()[index])
}
fn reject(s: &mut Simulation, op: impl FnOnce(&mut Simulation) -> Result<(), SimError>) {
    let before = s.snapshot();
    let roster = s.original_player_roster();
    assert!(op(s).is_err());
    assert_eq!(s.snapshot(), before);
    assert_eq!(s.original_player_roster(), roster);
}
#[test]
fn registry_separates_fixed_roster_capability_and_nonmelee_execution() {
    let r =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonAcquisitionV1).unwrap();
    assert_eq!(
        OriginalExecutionProfile::PlayerDemonAcquisitionV1.wire_id(),
        5
    );
    for ty in OriginalActorType::all() {
        assert_eq!(
            r.admit_definition(DefinitionIdentity::Original(ty)).is_ok(),
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
        r.admit_definition(DefinitionIdentity::Context(ContextRole::UnboundBody))
            .is_err()
    );
    for state in [
        OriginalState::S_SARG_STND,
        OriginalState::S_SARG_RUN1,
        OriginalState::S_SARG_RUN8,
    ] {
        assert!(
            r.admit_state(
                DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT),
                state
            )
            .is_ok()
        );
    }
    for state in [
        OriginalState::S_SARG_ATK1,
        OriginalState::S_SARG_ATK2,
        OriginalState::S_SARG_ATK3,
        OriginalState::S_SARG_RAISE1,
    ] {
        assert!(
            r.admit_state(
                DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT),
                state
            )
            .is_err()
        );
    }
}
#[test]
fn missing_body_and_empty_roster_configuration_reject_atomically() {
    let c = &source::cases()[1];
    let mut s = Simulation::original_player_demon_map(
        source::case_map(c.map),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    let d = s
        .spawn_original_actor(OriginalActorType::MT_SERGEANT)
        .unwrap()
        .spawned[0];
    let p = s.original_player_roster().unwrap();
    assert!(p[0].active);
    assert!(p[0].actor.is_none());
    assert!(p[1..].iter().all(|p| !p.active));
    reject(&mut s, |s| s.advance(ZERO));
    reject(&mut s, |s| s.look_for_players(d, false).map(|_| ()));
    reject(&mut s, |s| s.look(d).map(|_| ()));
    reject(&mut s, |s| {
        s.set_perception_player(0, PerceptionPlayer::default())
    });
    reject(&mut s, |s| s.bind_original_player_body(PlayerSlot::One, d));
    let body = s
        .spawn_player_body(PlayerSlot::One, [Fixed::from_map_units(256), Fixed::ZERO])
        .unwrap();
    assert_eq!(s.original_player_roster().unwrap()[0].actor, Some(body));
    s.advance(ZERO).unwrap();
}
#[test]
fn canonical_player_slot_is_independent_of_actor_slot_and_target_is_real() {
    let (mut s, a) = setup(1);
    assert_eq!(a.body.slot(), 1);
    assert_eq!(PlayerSlot::One.ordinal(), 0);
    for _ in 0..10 {
        s.advance(ZERO).unwrap();
    }
    let d = a.origin.unwrap();
    assert_eq!(s.actor(d).unwrap().life().unwrap().target, Some(a.body));
    assert_eq!(
        s.original_actor_identity(a.body).unwrap().0,
        DefinitionIdentity::Original(OriginalActorType::MT_PLAYER)
    );
    assert_eq!(
        s.original_actor_identity(d).unwrap().0,
        DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT)
    );
    assert_ne!(
        s.actor(d).unwrap().position()[0],
        Fixed::from_map_units(-512)
    );
}
#[test]
fn source_stop_slot_and_repeated_scan_do_not_consume_rng() {
    let (mut s, a) = setup(2);
    let d = a.origin.unwrap();
    let rng = s.rng();
    assert_eq!(s.actor(d).unwrap().lastlook(), 1);
    assert!(!s.look_for_players(d, false).unwrap());
    assert_eq!(s.actor(d).unwrap().lastlook(), 0);
    assert!(s.actor(d).unwrap().life().unwrap().target.is_none());
    assert!(s.look_for_players(d, false).unwrap());
    assert_eq!(s.actor(d).unwrap().life().unwrap().target, Some(a.body));
    assert_eq!(s.rng(), rng);
    for _ in 0..4 {
        assert!(s.look_for_players(d, false).unwrap());
        assert_eq!(s.rng(), rng);
    }
}
#[test]
fn rear_distance_visibility_and_actual_wall_quirk_remain_distinct() {
    for (index, found) in [
        (5, false),
        (6, true),
        (7, true),
        (8, false),
        (9, false),
        (10, false),
        (11, true),
    ] {
        let (mut s, a) = setup(index);
        assert_eq!(
            s.look_for_players(a.origin.unwrap(), false).unwrap(),
            found,
            "case{index}"
        );
    }
}
#[test]
fn fixed_roster_rejects_external_copies_wrong_roles_and_body_retirement() {
    let (mut s, a) = setup(1);
    let d = a.origin.unwrap();
    reject(&mut s, |s| s.mark_remove(a.body));
    reject(&mut s, |s| {
        s.bind_original_player_body(PlayerSlot::One, a.body)
    });
    reject(&mut s, |s| {
        s.bind_original_player_body(
            PlayerSlot::One,
            ActorId::from_parts(a.body.slot(), a.body.generation() + 1),
        )
    });
    reject(&mut s, |s| s.set_actor_target(d, Some(d)));
    reject(&mut s, |s| s.look_for_players(a.body, true).map(|_| ()));
    for slot in 0..5 {
        reject(&mut s, |s| {
            s.set_perception_player(
                slot,
                PerceptionPlayer {
                    active: true,
                    health: 999,
                    actor: Some(a.body),
                },
            )
        });
    }
    for slot in 1..4 {
        let mut c = ZERO;
        c[slot].forward = 1;
        reject(&mut s, |s| s.advance(c));
        assert!(PlayerSlot::try_from(slot as u8).is_err());
    }
}
#[test]
fn active_dead_player_is_ineligible_and_old_target_is_retained() {
    let c = &source::cases()[19];
    let (mut s, a) = source::setup(c);
    for op in 1..=c.tics {
        source::operations(&mut s, a, c, op);
        s.advance(source::commands(c, op)).unwrap();
    }
    let p = s.original_player(PlayerSlot::One).unwrap();
    assert_eq!(p.state, PlayerBodyState::Dead);
    assert_eq!(p.mo, Some(a.body));
    let roster = s.original_player_roster().unwrap();
    assert!(roster[0].active);
    assert_eq!(roster[0].health, 0);
    assert_eq!(roster[0].actor, Some(a.body));
    assert_eq!(
        s.actor(a.origin.unwrap()).unwrap().life().unwrap().target,
        Some(a.body)
    );
    assert!(!s.look_for_players(a.origin.unwrap(), true).unwrap());
    let before_player = s.original_player(PlayerSlot::One).unwrap();
    let before_actor = *s.actor(a.body).unwrap();
    let rng = s.rng();
    s.damage_actor(DamageRequest {
        target: a.body,
        inflictor: None,
        source: None,
        amount: 100,
    })
    .unwrap();
    assert_eq!(s.original_player(PlayerSlot::One).unwrap(), before_player);
    assert_eq!(*s.actor(a.body).unwrap(), before_actor);
    assert_eq!(s.rng(), rng);
    let mut c = ZERO;
    c[0].forward = 1;
    reject(&mut s, |s| s.advance(c));
    reject(&mut s, |s| s.mark_remove(a.body));
}
#[test]
fn retired_attacker_generation_never_redirects_to_reused_effect() {
    let c = &source::cases()[23];
    let (mut s, a) = source::setup(c);
    let old = a.origin.unwrap();
    for op in 1..=32 {
        source::operations(&mut s, a, c, op);
        s.advance(source::commands(c, op)).unwrap();
    }
    let p = s.original_player(PlayerSlot::One).unwrap();
    assert_eq!(p.attacker, Some(old));
    assert!(s.actor(old).is_none());
    let replacement = ActorId::from_parts(old.slot(), old.generation() + 1);
    assert_eq!(
        s.actor(replacement).unwrap().life().unwrap().actor_type,
        ActorTypeId(OriginalActorType::MT_PUFF.ordinal())
    );
    assert_ne!(p.attacker, Some(replacement));
    reject(&mut s, |s| {
        s.bind_original_player_body(PlayerSlot::One, replacement)
    });
    reject(&mut s, |s| {
        s.damage_actor(DamageRequest {
            target: a.body,
            source: Some(old),
            inflictor: None,
            amount: 1,
        })
        .map(|_| ())
    });
}
#[test]
fn real_command_stage_precedes_demon_thinking_and_accumulates_with_knockback() {
    let c = &source::cases()[16];
    let (mut s, a) = source::setup(c);
    let before = s.actor(a.body).unwrap().momentum();
    assert_ne!(before, [Fixed::ZERO; 2]);
    s.advance(source::commands(c, 1)).unwrap();
    assert_ne!(s.actor(a.body).unwrap().momentum(), [Fixed::ZERO; 2]);
    let command = s
        .events()
        .iter()
        .position(|e| matches!(e, Event::Player { .. }))
        .unwrap();
    let thinker = s
        .events()
        .iter()
        .position(|e| matches!(e, Event::Think(_)))
        .unwrap();
    assert!(command < thinker);
}
#[test]
fn melee_state_and_late_autonomous_melee_reject_without_partial_command_commit() {
    let (mut s, a) = setup(6);
    let d = a.origin.unwrap();
    for state in [
        OriginalState::S_SARG_ATK1,
        OriginalState::S_SARG_ATK2,
        OriginalState::S_SARG_ATK3,
    ] {
        let before = s.snapshot();
        assert!(s.enter_original_state(d, state).is_err());
        assert_eq!(s.snapshot(), before);
    }
    reject(&mut s, |s| s.demon_melee_attack(d));
    // Controlled direction setup avoids the source initial turnaround exclusion;
    // this is a Rust rollback test, not a new C gameplay observation.
    let mut initial = PursuitSetup::from_actor(s.actor(d).unwrap()).unwrap();
    initial.movedir = 8;
    initial.movecount = 0;
    s.configure_pursuit_actor(d, initial).unwrap();
    let mut command = ZERO;
    command[0].angle_turn = 100;
    let mut rejected = false;
    for _ in 0..256 {
        let before = s.snapshot();
        if let Err(error) = s.advance(command) {
            assert_eq!(error, PlayerBodyError::Unsupported.into());
            assert_eq!(s.snapshot(), before);
            reject(&mut s, |s| s.step(command).map(|_| ()));
            rejected = true;
            break;
        }
    }
    assert!(
        rejected,
        "the bounded chase must eventually reach the excluded melee branch"
    );
}
#[test]
fn all_source_sequences_replay_with_complete_advance_step_equality() {
    for c in source::cases() {
        let (mut a, actors) = source::setup(&c);
        let mut b = a.clone();
        for op in 1..=c.tics {
            source::operations(&mut a, actors, &c, op);
            source::operations(&mut b, actors, &c, op);
            let command = source::commands(&c, op);
            a.advance(command).unwrap();
            assert_eq!(a.snapshot(), b.step(command).unwrap(), "{} op{op}", c.label);
        }
        a.set_paused(true);
        b.set_paused(true);
        let mut bad = ZERO;
        bad[3].buttons = 255;
        a.advance(bad).unwrap();
        assert_eq!(a.snapshot(), b.step(bad).unwrap());
        a.set_paused(false);
        b.set_paused(false);
        let before = a.snapshot();
        assert_eq!(a.advance(bad), b.step(bad).map(|_| ()));
        assert_eq!(a.snapshot(), before);
        assert_eq!(a.snapshot(), b.snapshot());
    }
}
#[test]
fn provenance_rejects_foreign_body_without_changes() {
    let (s, a) = setup(1);
    let mut left = GameInstance::new(s.clone());
    let mut right = GameInstance::new(s);
    let exported = left
        .with_execution_context(|ctx| {
            let body = ctx.admit_local(
                a.body,
                DefinitionIdentity::Original(OriginalActorType::MT_PLAYER),
            )?;
            Ok(ctx.export_actor(body).unwrap())
        })
        .unwrap();
    let before = right.simulation().snapshot();
    right
        .with_execution_context(|ctx| {
            assert!(
                ctx.import_actor(
                    &exported,
                    DefinitionIdentity::Original(OriginalActorType::MT_PLAYER)
                )
                .is_err()
            );
            Ok(())
        })
        .unwrap();
    assert_eq!(right.simulation().snapshot(), before);
}
#[test]
fn new_diagnostics_validate_policy_context_and_historical_versions_are_unchanged() {
    let (s, _) = setup(1);
    let bytes = s
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    let d = Diagnostic::decode_player(&bytes, DiagnosticLimits::default()).unwrap();
    assert_eq!(d.version(), 22);
    assert_eq!(d.field("context.schema"), Some(PLAYER_DEMON_CONTEXT_SCHEMA));
    assert!(Diagnostic::decode(&bytes, DiagnosticLimits::default()).is_err());
    let mut bad = bytes.clone();
    let at = bad
        .windows(PLAYER_DEMON_CONTEXT_SCHEMA.len())
        .position(|v| v == PLAYER_DEMON_CONTEXT_SCHEMA)
        .unwrap();
    bad[at] ^= 1;
    assert!(Diagnostic::decode_player(&bad, DiagnosticLimits::default()).is_err());
    for bytes in [
        include_bytes!("../../../tests/reference/m3g4_state_1_1.bin").as_slice(),
        include_bytes!("../../../tests/reference/m3g3_state_10_1.bin").as_slice(),
    ] {
        assert!(Diagnostic::decode_player(bytes, DiagnosticLimits::default()).is_ok());
    }
}

#[test]
fn new_policy_preserves_one_demon_limit_and_excluded_sound_and_weapon_services() {
    let bytes = include_bytes!("../../../tests/reference/m3d3i_map_1.bin");
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
    let mut s =
        Simulation::original_player_demon_map(map, 64, MapContextOptions::default()).unwrap();
    let before = s.snapshot();
    assert_eq!(
        s.spawn_original_actor(OriginalActorType::MT_SERGEANT),
        Err(OriginalRegistryError::Runtime(
            MapSpawnError::WorkLimit.into()
        ))
    );
    assert_eq!(s.snapshot(), before);
    let (mut s, a) = setup(1);
    let d = a.origin.unwrap();
    reject(&mut s, |s| s.noise_alert(a.body, d));
    let before = s.snapshot();
    assert!(
        s.aim_line_attack(a.body, 0, Fixed::from_map_units(128))
            .is_err()
    );
    assert_eq!(s.snapshot(), before);
    let r =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonAcquisitionV1).unwrap();
    let mut options = ActorOptions::default();
    options.skill = Skill::Easy;
    assert!(
        Simulation::with_original_registry(
            source::map(),
            64,
            r,
            options,
            MapContextOptions::default()
        )
        .is_err()
    );
}
