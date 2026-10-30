//! API refinement and safety policy; independent C expectations are tested separately.
use super::*;
#[path = "../../../tests/support/m3g1_profiles.rs"]
mod inputs;
const COMMANDS: [TicCommand; 4] = [TicCommand {
    forward: 0,
    side: 0,
    angle_turn: 0,
    buttons: 0,
}; 4];
fn same(a: &Simulation, b: &Simulation) {
    let (a, b) = (a.snapshot(), b.snapshot());
    assert_eq!(a, b);
    assert_eq!(a.canonical_bytes(), b.canonical_bytes());
}
#[test]
fn every_profile_has_one_core_and_no_implicit_advance_capture() {
    for profile in inputs::PROFILES {
        let mut a = inputs::setup(profile);
        let mut b = a.clone();
        for _ in 0..32 {
            let count = snapshot::capture_count();
            let result = b.advance(COMMANDS);
            assert_eq!(snapshot::capture_count(), count, "{profile:?}");
            result.unwrap();
            let count = snapshot::capture_count();
            let observed = a.step(COMMANDS).unwrap();
            assert_eq!(snapshot::capture_count(), count + 1);
            assert_eq!(observed, b.snapshot());
            same(&a, &b);
        }
        if profile == Profile::OriginalDemon {
            let mut game = GameInstance::new(b);
            let key = game.id();
            let count = snapshot::capture_count();
            game.with_execution_context(|ctx| {
                ctx.advance(COMMANDS)?;
                Ok(())
            })
            .unwrap();
            assert_eq!(snapshot::capture_count(), count);
            assert_eq!(game.id(), key);
            game.with_execution_context(|ctx| {
                ctx.step(COMMANDS)?;
                Ok(())
            })
            .unwrap();
            assert_eq!(snapshot::capture_count(), count + 1);
        }
    }
}
#[test]
fn pause_precedes_buttons_and_overflow_without_capture_or_mutation() {
    for profile in inputs::PROFILES {
        let mut a = inputs::setup(profile);
        a.tic = u64::MAX;
        a.set_paused(true);
        let mut b = a.clone();
        let before = b.snapshot();
        let commands = [TicCommand {
            buttons: u8::MAX,
            ..Default::default()
        }; 4];
        let count = snapshot::capture_count();
        b.advance(commands).unwrap();
        assert_eq!(snapshot::capture_count(), count);
        assert_eq!(a.step(commands).unwrap(), before);
        assert_eq!(snapshot::capture_count(), count + 1);
        same(&a, &b);
        assert_eq!(b.snapshot(), before);
    }
}
#[test]
fn overflow_and_unsupported_buttons_match_and_preserve_complete_state() {
    for profile in inputs::PROFILES {
        for overflow in [false, true] {
            let mut a = inputs::setup(profile);
            if overflow {
                a.tic = u64::MAX;
            }
            let mut b = a.clone();
            let before = a.snapshot();
            let count = snapshot::capture_count();
            let commands = [TicCommand {
                buttons: u8::MAX,
                ..Default::default()
            }; 4];
            let error = b.advance(commands).unwrap_err();
            assert_eq!(a.step(commands).unwrap_err(), error);
            if overflow {
                assert_eq!(error, SimError::TicOverflow);
            }
            assert_eq!(snapshot::capture_count(), count);
            assert_eq!(a.snapshot(), before);
            assert_eq!(b.snapshot(), before);
        }
    }
}
#[test]
fn command_streams_keep_full_fields_events_errors_and_diagnostic_reads() {
    for profile in inputs::PROFILES {
        let mut a = inputs::setup(profile);
        let mut b = a.clone();
        for tic in 0..24 {
            let commands = [TicCommand {
                forward: if tic % 3 == 0 { 1 } else { 0 },
                side: 0,
                angle_turn: tic * 7,
                buttons: if tic % 7 == 0 { u8::MAX } else { 0 },
            }; 4];
            let before = a.snapshot();
            let x = a.step(commands).map(drop);
            let y = b.advance(commands);
            assert_eq!(x, y, "{profile:?}");
            if x.is_err() {
                assert_eq!(b.snapshot(), before);
            }
            same(&a, &b);
            let observed = b.snapshot();
            for _ in 0..3 {
                assert_eq!(b.snapshot(), observed);
            }
        }
    }
}
#[test]
fn both_rng_streams_wrap_without_observation_draws() {
    for profile in [
        Profile::Foundation,
        Profile::DemonExpandedMap,
        Profile::OriginalDemon,
    ] {
        let mut a = inputs::setup(profile);
        a.rng = DoomRng::from_indices(u8::MAX, u8::MAX);
        let mut b = a.clone();
        assert_eq!(a.gameplay_random(), b.gameplay_random());
        assert_eq!(a.rng.indices()[0], 0);
        assert_eq!(a.rng.m_random(), b.rng.m_random());
        assert_eq!(a.rng.indices(), [0, 0]);
        for _ in 0..32 {
            assert_eq!(a.gameplay_random(), b.gameplay_random());
            a.step(COMMANDS).unwrap();
            b.advance(COMMANDS).unwrap();
            same(&a, &b);
        }
    }
}
#[test]
fn run_commands_retains_its_snapshot_collection_contract() {
    for profile in inputs::PROFILES {
        let mut a = inputs::setup(profile);
        let mut b = a.clone();
        let snapshots = run_commands(&mut a, [COMMANDS; 3]).unwrap();
        for expected in snapshots {
            b.advance(COMMANDS).unwrap();
            assert_eq!(expected, b.snapshot());
        }
        same(&a, &b);
    }
}
