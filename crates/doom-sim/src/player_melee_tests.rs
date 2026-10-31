//! Current-stage planning and complete rollback are Rust policy, not C oracles.
use super::*;
use crate::identity::{OriginalActorType, OriginalState};
#[path = "../../../tests/support/m3g6.rs"]
mod inputs;

fn windup(index: usize) -> (Simulation, inputs::Actors) {
    let c = &inputs::cases()[index];
    let (mut s, a) = inputs::setup(c);
    for op in 1..=27 {
        inputs::operations(&mut s, a, c, op);
        s.advance(inputs::commands(c, op)).unwrap();
    }
    assert_eq!(
        s.actor(a.origin.unwrap()).unwrap().life().unwrap().state,
        Some(StateId(OriginalState::S_SARG_ATK2.ordinal()))
    );
    (s, a)
}

#[test]
fn actual_nonlethal_and_lethal_strikes_roll_back_after_all_damage_effects() {
    for index in [1, 4] {
        let (mut s, a) = windup(index);
        let before = s.snapshot();
        let player = s.original_player;
        let rng = s.rng();
        let error = s.checked_demon_operation(|host| {
            host.enter(
                a.origin.unwrap(),
                StateId(OriginalState::S_SARG_ATK3.ordinal()),
            );
            assert!(host.player.as_deref().unwrap().health < player.unwrap().health);
            assert_ne!(
                host.access.get(a.body).unwrap().momentum(),
                [Fixed::ZERO; 2]
            );
            assert_ne!(*host.rng, rng);
            if index == 4 {
                assert_eq!(host.player.as_deref().unwrap().state, PlayerBodyState::Dead);
                assert!(host.access.get(a.body).unwrap().life().unwrap().health < 0);
            }
            host.fail_sight(PerceptionError::WorkLimit.into());
        });
        assert_eq!(error, Err(PerceptionError::WorkLimit.into()));
        assert_eq!(s.snapshot(), before);
        assert_eq!(s.original_player, player);
    }
}

#[test]
fn late_plan_uses_health_and_rng_after_an_earlier_staged_damage_action() {
    let (mut s, a) = windup(1);
    s.checked_demon_operation(|host| {
        let body = *host.access.get(a.body).unwrap();
        let earlier = crate::combat::DamagePlan::current_melee_body(
            host.registry,
            body,
            DamageRequest {
                target: a.body,
                source: None,
                inflictor: None,
                amount: 96,
            },
        )
        .unwrap();
        host.apply_damage(earlier);
        assert_eq!(host.player.as_deref().unwrap().health, 4);
        host.enter(
            a.origin.unwrap(),
            StateId(OriginalState::S_SARG_ATK3.ordinal()),
        );
        assert_eq!(host.player.as_deref().unwrap().health, 0);
        assert_eq!(host.player.as_deref().unwrap().state, PlayerBodyState::Dead);
        assert!(host.access.get(a.body).unwrap().life().unwrap().health <= 0);
        assert!(
            host.events
                .iter()
                .any(|e| matches!(e, Event::Combat(CombatEvent::Health { values: [4, _], .. })))
        );
    })
    .unwrap();
    assert_eq!(s.original_player(PlayerSlot::One).unwrap().health, 0);
}

#[test]
fn impulse_envelope_failure_discards_earlier_command_state_and_attack_rng() {
    let (mut s, a) = windup(1);
    // Controlled Rust-only boundary setup, not a claim of original traversal to
    // this position. The one-sector BSP location and spatial order are unchanged.
    s.actors.get_mut(a.body).unwrap().spec.position = [Fixed::from_map_units(980), Fixed::ZERO];
    s.actors.get_mut(a.origin.unwrap()).unwrap().spec.position =
        [Fixed::from_map_units(930), Fixed::ZERO];
    assert!(s.check_melee_range(a.origin.unwrap()).unwrap().accepted);
    let before = s.snapshot();
    let mut command = [TicCommand::default(); 4];
    command[0].angle_turn = 1;
    let count = crate::snapshot::capture_count();
    assert_eq!(s.advance(command), Err(PlayerBodyError::Unsupported.into()));
    assert_eq!(crate::snapshot::capture_count(), count);
    assert_eq!(s.snapshot(), before);
    assert_eq!(s.step(command), Err(PlayerBodyError::Unsupported.into()));
    assert_eq!(s.snapshot(), before);
}

#[test]
fn current_role_health_generation_pending_and_unsupported_flags_fail_atomically() {
    // p_mobj.h SHADOW is unadmitted player-power behavior, including at FaceTarget.
    const MF_SHADOW: u32 = 0x40000;
    for kind in 0..7 {
        let (mut s, a) = windup(1);
        match kind {
            0 => {
                s.actors
                    .get_mut(a.origin.unwrap())
                    .unwrap()
                    .life
                    .as_mut()
                    .unwrap()
                    .target = Some(ActorId::from_parts(a.body.slot(), a.body.generation() + 1))
            }
            1 => {
                s.actors
                    .get_mut(a.origin.unwrap())
                    .unwrap()
                    .life
                    .as_mut()
                    .unwrap()
                    .actor_type = ActorTypeId(OriginalActorType::MT_SHOTGUY.ordinal())
            }
            2 => s.original_player.as_mut().unwrap().health += 1,
            3 => {
                s.actors.mark_remove(a.body).unwrap();
            }
            4 => {
                s.actors
                    .get_mut(a.body)
                    .unwrap()
                    .life
                    .as_mut()
                    .unwrap()
                    .flags |= MF_SHADOW
            }
            5 => s.actors.get_mut(a.body).unwrap().spec.momentum[0] = Fixed::from_bits(i32::MAX),
            _ => s.original_player.as_mut().unwrap().mo = None,
        }
        let before = s.snapshot();
        let mut command = [TicCommand::default(); 4];
        command[0].angle_turn = 1;
        assert!(s.advance(command).is_err(), "kind{kind}");
        assert_eq!(s.snapshot(), before, "kind{kind}");
    }
}

#[test]
fn retired_reused_body_cannot_become_the_current_melee_target() {
    let (mut s, a) = windup(1);
    s.actors.mark_remove(a.body).unwrap();
    s.spatial.unlink(a.body);
    s.actors.run(64, |_, _| {}).unwrap();
    let replacement = s
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(replacement.slot(), a.body.slot());
    assert_ne!(replacement.generation(), a.body.generation());
    let before = s.snapshot();
    assert!(s.advance([TicCommand::default(); 4]).is_err());
    assert_eq!(s.snapshot(), before);
}

#[test]
fn production_combat_advance_captures_nothing_including_hit_death_and_corpse() {
    let c = &inputs::cases()[0];
    let (mut s, _) = inputs::setup(c);
    let count = crate::snapshot::capture_count();
    for op in 1..=416 {
        s.advance(inputs::commands(c, op)).unwrap();
        assert_eq!(crate::snapshot::capture_count(), count);
    }
    s.step([TicCommand::default(); 4]).unwrap();
    assert_eq!(crate::snapshot::capture_count(), count + 1);
    s.set_paused(true);
    let invalid = [TicCommand {
        buttons: 255,
        ..Default::default()
    }; 4];
    s.advance(invalid).unwrap();
    assert_eq!(crate::snapshot::capture_count(), count + 1);
    s.step(invalid).unwrap();
    assert_eq!(crate::snapshot::capture_count(), count + 2);
    s.set_paused(false);
    assert!(s.step(invalid).is_err());
    assert_eq!(crate::snapshot::capture_count(), count + 2);
}
