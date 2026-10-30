//! Private corruption and transaction guards are Rust policy, not C pointer behavior.
use super::*;
use crate::identity::OriginalState;
struct Actors {
    body: ActorId,
    origin: Option<ActorId>,
}
fn fresh(index: usize) -> (Simulation, Actors) {
    let bytes = if index == 0 {
        include_bytes!("../../../tests/reference/m3g5_map_0.bin").as_slice()
    } else {
        include_bytes!("../../../tests/reference/m3g5_map_1.bin").as_slice()
    };
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    let map = Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
    let mut s =
        Simulation::original_player_demon_map(map, 64, MapContextOptions::default()).unwrap();
    let origin = s
        .spawn_original_actor(OriginalActorType::MT_SERGEANT)
        .unwrap()
        .spawned
        .first()
        .copied();
    let body = s
        .spawn_player_body(PlayerSlot::One, [Fixed::from_map_units(256), Fixed::ZERO])
        .unwrap();
    (s, Actors { body, origin })
}
#[test]
fn invalid_current_body_health_liveness_and_generation_reject_without_mutation() {
    for kind in 0..9 {
        let (mut s, a) = fresh(1);
        match kind {
            0 => s.original_player.as_mut().unwrap().mo = None,
            1 => s.original_player.as_mut().unwrap().mo = Some(ActorId::from_parts(u32::MAX, 1)),
            2 => {
                s.original_player.as_mut().unwrap().mo =
                    Some(ActorId::from_parts(a.body.slot(), a.body.generation() + 1))
            }
            3 => s.original_player.as_mut().unwrap().health += 1,
            4 => {
                s.actors
                    .get_mut(a.body)
                    .unwrap()
                    .life
                    .as_mut()
                    .unwrap()
                    .flags &= !original_player::MF_SHOOTABLE
            }
            5 => {
                s.actors.mark_remove(a.body).unwrap();
            }
            6 => s.sound_targets[0] = Some(a.body),
            7 => {
                s.actors
                    .get_mut(a.body)
                    .unwrap()
                    .life
                    .as_mut()
                    .unwrap()
                    .state = Some(StateId(OriginalState::S_SARG_RUN1.ordinal()))
            }
            _ => s.original_player.as_mut().unwrap().mo = a.origin,
        }
        let before = s.snapshot();
        let roster = s.original_player_roster();
        assert!(s.advance([TicCommand::default(); 4]).is_err(), "kind{kind}");
        assert_eq!(s.snapshot(), before);
        assert_eq!(s.original_player_roster(), roster);
    }
}
#[test]
fn retired_and_reused_body_storage_cannot_satisfy_fixed_roster() {
    let (mut s, a) = fresh(0);
    s.actors.mark_remove(a.body).unwrap();
    s.spatial.unlink(a.body);
    s.actors.run(64, |_, _| {}).unwrap();
    assert!(s.actor(a.body).is_none());
    let replacement = s
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(replacement.slot(), a.body.slot());
    assert_ne!(replacement.generation(), a.body.generation());
    assert_eq!(s.original_player_roster().unwrap()[0].actor, Some(a.body));
    let before = s.snapshot();
    assert!(s.advance([TicCommand::default(); 4]).is_err());
    assert_eq!(s.snapshot(), before);
}
#[test]
fn derived_scan_uses_current_transaction_player_instead_of_copied_configuration() {
    let (mut s, a) = fresh(1);
    let d = a.origin.unwrap();
    let before = s.snapshot();
    let result = s.checked_demon_operation(|host| {
        host.player.as_deref_mut().unwrap().health = 0;
        let scan = scan_current(host, d, false).unwrap();
        assert!(scan.target.is_none());
        assert!(scan.events.iter().any(|e| matches!(
            e,
            Event::Perception(PerceptionEvent::Health { health: 0, .. })
        )));
        host.rng.p_random();
        host.target(d, Some(a.body));
        host.fail_sight(PerceptionError::WorkLimit.into());
    });
    assert_eq!(result, Err(PerceptionError::WorkLimit.into()));
    assert_eq!(s.snapshot(), before);
}
#[test]
fn production_advance_and_atomic_rejections_capture_no_implicit_snapshot() {
    let (mut s, a) = fresh(1);
    let count = crate::snapshot::capture_count();
    let mut commands = [TicCommand::default(); 4];
    commands[0].forward = 1;
    s.advance(commands).unwrap();
    assert_eq!(crate::snapshot::capture_count(), count);
    s.step(commands).unwrap();
    assert_eq!(crate::snapshot::capture_count(), count + 1);
    commands[3].buttons = 1;
    assert!(s.advance(commands).is_err());
    assert_eq!(crate::snapshot::capture_count(), count + 1);
    assert!(s.mark_remove(a.body).is_err());
    s.set_paused(true);
    s.advance(commands).unwrap();
    assert_eq!(crate::snapshot::capture_count(), count + 1);
}
#[test]
fn scanner_rejects_invalid_observers_and_lastlook_without_indexing_panics() {
    let (s, a) = fresh(1);
    let roster = s.original_player_roster().unwrap();
    let mut actor = *s.actor(a.origin.unwrap()).unwrap();
    actor.lastlook = 4;
    assert!(
        perception::scan(
            &s.world,
            &s.sectors,
            &roster,
            &|_| Some(actor),
            a.origin.unwrap(),
            false,
            None
        )
        .is_err()
    );
    assert!(
        perception::scan(
            &s.world,
            &s.sectors,
            &roster,
            &|_| None,
            a.origin.unwrap(),
            false,
            None
        )
        .is_err()
    );
}
