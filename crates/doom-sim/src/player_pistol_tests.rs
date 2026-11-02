//! Independent query observations and Rust-only transaction/invariant policies.
use super::*;
#[path = "../../../tests/support/m3g7.rs"]
mod source;

fn object(value: TraceObject, births: &source::SourceTrace) -> [i64; 2] {
    match value {
        TraceObject::Line(line) => [1, i64::from(line)],
        TraceObject::Actor(actor) => [0, births.birth(Some(actor))],
    }
}
#[test]
fn actual_pistol_aim_queries_match_independent_source_intercepts_and_windows() {
    let mut count = 0;
    for c in source::cases() {
        let (mut sim, _) = source::setup(&c);
        let mut births = source::SourceTrace::seeded(c.seed);
        births.check(
            &sim,
            c.which,
            0,
            sim.pending_events(),
            source::rows(c.which, 0),
        );
        AIM_OBSERVATIONS.with(|capture| *capture.borrow_mut() = Some(Vec::new()));
        for op in 1..=c.tics {
            let pending = sim.pending_events().len();
            sim.advance(source::commands(&c, op)).unwrap();
            let rows = source::rows(c.which, op);
            births.check(&sim, c.which, op, &sim.events()[pending..], rows);
            let actual = AIM_OBSERVATIONS
                .with(|capture| std::mem::take(capture.borrow_mut().as_mut().unwrap()));
            let mut expected = Vec::new();
            let mut query = Vec::new();
            let mut inside = false;
            let mut shot = false;
            for (tag, v) in rows {
                if tag == "X" {
                    shot = v[0] == 0;
                }
                if tag == "D" && !shot {
                    inside = true;
                }
                if inside {
                    query.push((tag.as_str(), v.as_slice()));
                }
                if tag == "J" && v[0] == 5 {
                    expected.push(std::mem::take(&mut query));
                    inside = false;
                }
            }
            assert_eq!(actual.len(), expected.len(), "queries {} op{op}", c.label);
            for (actual, expected) in actual.iter().zip(expected) {
                let d = expected.iter().find(|(tag, _)| *tag == "D").unwrap().1;
                assert_eq!(
                    actual.trace.origin.map(|v| i64::from(v.bits())).as_slice(),
                    &d[1..3]
                );
                assert_eq!(
                    actual.trace.delta.map(|v| i64::from(v.bits())).as_slice(),
                    &d[3..5]
                );
                let cells: Vec<_> = expected
                    .iter()
                    .filter(|(tag, _)| *tag == "B")
                    .map(|(_, v)| v[1..].to_vec())
                    .collect();
                assert_eq!(
                    actual
                        .trace
                        .cells
                        .iter()
                        .map(|c| c.map(i64::from).to_vec())
                        .collect::<Vec<_>>(),
                    cells
                );
                let candidates: Vec<_> = expected
                    .iter()
                    .filter(|(tag, _)| *tag == "U")
                    .map(|(_, v)| [v[1], v[2]])
                    .collect();
                assert_eq!(
                    actual
                        .trace
                        .candidates
                        .iter()
                        .map(|v| object(*v, &births))
                        .collect::<Vec<_>>(),
                    candidates
                );
                let mut intercepts: Vec<_> = expected
                    .iter()
                    .filter(|(tag, _)| *tag == "I")
                    .map(|(_, v)| [v[1], v[2], v[3]])
                    .collect();
                intercepts.sort_by_key(|v| v[2]);
                assert_eq!(
                    actual
                        .trace
                        .intercepts
                        .iter()
                        .map(|i| {
                            let [kind, id] = object(i.object, &births);
                            [kind, id, i64::from(i.fraction.bits())]
                        })
                        .collect::<Vec<_>>(),
                    intercepts
                );
                let result = expected.last().unwrap().1;
                let stop = actual
                    .stopped
                    .map_or([-1, -1], |value| object(value, &births));
                assert_eq!(
                    [
                        births.birth(actual.target),
                        i64::from(actual.slope.bits()),
                        i64::from(actual.window[0].bits()),
                        i64::from(actual.window[1].bits()),
                        i64::from(actual.traversed),
                        stop[0],
                        stop[1]
                    ]
                    .as_slice(),
                    &result[2..]
                );
                count += 1;
            }
        }
        AIM_OBSERVATIONS.with(|capture| *capture.borrow_mut() = None);
    }
    assert_eq!(count, 223);
}

#[test]
fn late_failure_after_an_actual_shot_discards_ammo_rng_effects_and_damage() {
    let c = &source::cases()[0];
    let (mut sim, a) = source::setup(c);
    for op in 1..19 {
        sim.advance(source::commands(c, op)).unwrap();
    }
    let before = sim.snapshot();
    let pistol = sim.original_player_pistol(PlayerSlot::One).unwrap();
    let result = sim.checked_demon_operation(|host| {
        let mut sound = Some(a.body);
        stage(
            host,
            source::commands(c, 19)[0],
            18,
            std::slice::from_mut(&mut sound),
        )
        .unwrap();
        assert!(
            host.access
                .get(a.origin.unwrap())
                .unwrap()
                .life()
                .unwrap()
                .health
                < 150
        );
        assert!(
            host.player
                .as_ref()
                .unwrap()
                .weapon
                .unwrap()
                .pistol()
                .unwrap()
                .bullets
                < pistol.unwrap().bullets
        );
        host.fail_sight(PerceptionError::WorkLimit.into());
    });
    assert_eq!(result, Err(PerceptionError::WorkLimit.into()));
    assert_eq!(sim.snapshot(), before);
}

#[test]
fn capacity_and_late_noise_sight_failures_are_atomic_and_capture_nothing() {
    let c = &source::cases()[0];
    let (mut sim, _) = source::setup(c);
    for op in 1..19 {
        sim.advance(source::commands(c, op)).unwrap();
    }
    // Existing current capacity is two actors; both source owners remain live.
    sim.actors.max_slots = 2;
    let before = sim.snapshot();
    let captures = snapshot::capture_count();
    assert_eq!(
        sim.advance(source::commands(c, 19)),
        Err(StoreError::Capacity.into())
    );
    assert_eq!(snapshot::capture_count(), captures);
    assert_eq!(sim.snapshot(), before);
    // A separate legal stand timer makes the ambush Demon look after the new
    // weapon noise. Injected query budget failure is Rust policy, not C input.
    let c = &source::cases()[16];
    let (mut sim, a) = source::setup(c);
    for op in 1..21 {
        sim.advance(source::commands(c, op)).unwrap();
    }
    sim.actors
        .get_mut(a.origin.unwrap())
        .unwrap()
        .life
        .as_mut()
        .unwrap()
        .tics = 1;
    sim.world.extent_sight_work = Some(1_048_576);
    assert_eq!(sim.sound_targets, [None]);
    let before = sim.snapshot();
    let captures = snapshot::capture_count();
    assert_eq!(
        sim.advance(source::commands(c, 21)),
        Err(PerceptionError::WorkLimit.into())
    );
    assert_eq!(snapshot::capture_count(), captures);
    assert_eq!(sim.snapshot(), before);
}

#[test]
fn no_implicit_snapshot_in_winning_losing_and_paused_weapon_execution() {
    for index in [0, 9, 13] {
        let c = &source::cases()[index];
        let (mut s, _) = source::setup(c);
        let before = snapshot::capture_count();
        for op in 1..=c.tics {
            s.advance(source::commands(c, op)).unwrap();
        }
        s.set_paused(true);
        s.advance([TicCommand::default(); 4]).unwrap();
        assert_eq!(snapshot::capture_count(), before);
        s.step([TicCommand::default(); 4]).unwrap();
        assert_eq!(snapshot::capture_count(), before + 1);
    }
}

#[test]
fn invalid_internal_weapon_fields_reject_before_commands_rng_or_events() {
    for mode in 0..5 {
        let (mut s, _) = source::setup(&source::cases()[0]);
        let state = s
            .original_player
            .as_mut()
            .unwrap()
            .weapon
            .as_mut()
            .unwrap()
            .pistol_mut()
            .unwrap();
        match mode {
            0 => state.psprites[0].state = Some(StateId(OriginalState::S_SGUN.ordinal())),
            1 => state.psprites[0].tics = 2,
            2 => state.bullets = 201,
            3 => state.refire = -1,
            _ => state.extra_light = 1,
        }
        let before = s.snapshot();
        assert!(s.advance(source::commands(&source::cases()[0], 1)).is_err());
        assert_eq!(s.snapshot(), before);
    }
}

#[test]
fn pending_stale_and_reused_body_generations_never_gain_pistol_ownership() {
    for mode in 0..3 {
        let (mut s, a) = source::setup(&source::cases()[0]);
        if mode == 0 {
            s.actors.mark_remove(a.body).unwrap();
        } else if mode == 1 {
            s.original_player.as_mut().unwrap().mo =
                Some(ActorId::from_parts(a.body.slot(), a.body.generation() + 1));
        } else {
            // Deliberate Rust-only invalid-world setup. Public active-player
            // retirement is rejected; no C freed-pointer behavior is imitated.
            let body = *s.actors.get(a.body).unwrap();
            s.spatial.unlink(a.body);
            s.actors.mark_remove(a.body).unwrap();
            s.actors.run(64, |_, _| {}).unwrap();
            let new = s.actors.spawn(body).unwrap();
            s.spatial.link(new);
            assert_eq!(new.slot(), a.body.slot());
            assert_ne!(new.generation(), a.body.generation());
            assert!(s.actor(a.body).is_none());
        }
        let before = s.snapshot();
        assert!(s.advance(source::commands(&source::cases()[0], 1)).is_err());
        assert_eq!(s.snapshot(), before);
    }
}
