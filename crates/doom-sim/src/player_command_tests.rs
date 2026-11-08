//! White-box policy failures and structural snapshot-cost checks.
use super::*;
fn fresh() -> (Simulation, ActorId) {
    let bytes = include_bytes!("../../../tests/reference/m3g4_map_0.bin");
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    let map = Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
    let mut s =
        Simulation::original_player_command_map(map, 64, MapContextOptions::default()).unwrap();
    let id = s
        .spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
        .unwrap();
    (s, id)
}
#[test]
fn staged_command_and_reaction_mutations_rollback_with_existing_transaction() {
    for delay in [0, 1] {
        let (mut s, id) = fresh();
        s.set_initial_player_reaction_time(PlayerSlot::One, delay)
            .unwrap();
        let before = s.snapshot();
        let result = s.checked_demon_operation(|host| {
            stage(
                host,
                id,
                TicCommand {
                    forward: 25,
                    angle_turn: -100,
                    ..Default::default()
                },
            )
            .unwrap();
            host.fail_sight(PerceptionError::WorkLimit.into());
        });
        assert_eq!(result, Err(PerceptionError::WorkLimit.into()));
        assert_eq!(s.snapshot(), before);
    }
}
#[test]
fn malformed_runtime_counters_and_excluded_command_rewriting_reject_atomically() {
    for counter in [-1, 256, i32::MIN, i32::MAX] {
        let (mut s, id) = fresh();
        s.actors
            .get_mut(id)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .reaction_time = counter;
        let before = s.snapshot();
        assert!(s.advance([TicCommand::default(); 4]).is_err());
        assert_eq!(s.snapshot(), before);
    }
    let (mut s, id) = fresh();
    s.actors.get_mut(id).unwrap().life.as_mut().unwrap().flags |= MF_JUSTATTACKED;
    let before = s.snapshot();
    assert!(s.advance([TicCommand::default(); 4]).is_err());
    assert_eq!(s.snapshot(), before);
}
#[test]
fn ordinary_commands_and_rejections_do_not_capture_implicit_snapshots() {
    let (mut s, _) = fresh();
    let n = crate::snapshot::capture_count();
    let mut c = [TicCommand::default(); 4];
    c[0].forward = 1;
    s.advance(c).unwrap();
    assert_eq!(crate::snapshot::capture_count(), n);
    s.step(c).unwrap();
    assert_eq!(crate::snapshot::capture_count(), n + 1);
    c[0].buttons = 1;
    assert!(s.advance(c).is_err());
    assert_eq!(crate::snapshot::capture_count(), n + 1);
    s.set_paused(true);
    s.advance(c).unwrap();
    assert_eq!(crate::snapshot::capture_count(), n + 1);
}

/// Refusal witness below the public API: `Simulation::preflight_player_commands`, the
/// bound-body clause (player_commands.rs:150, `type != MT_PLAYER || state is none`).
/// The bound body is spawned as `MT_PLAYER` in its spawn state, and source state entry
/// never leaves a live typed actor without a state (`S_NULL` removes it), so no map or
/// command reaches a stateless bound body; the state is cleared from inside the crate.
#[test]
fn a_bound_body_without_a_state_refuses_the_command_preflight() {
    let (mut s, id) = fresh();
    let commands = [TicCommand::default(); 4];
    assert_eq!(s.preflight_player_commands(&commands), Ok(Some(id)));
    s.actors.get_mut(id).unwrap().life.as_mut().unwrap().state = None;
    assert_eq!(
        s.preflight_player_commands(&commands),
        Err(SimError::PlayerBody(PlayerBodyError::Invariant))
    );
}
