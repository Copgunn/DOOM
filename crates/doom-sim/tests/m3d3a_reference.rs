//! Independent original-C sight and perception decisions.
#[path = "../../../tests/support/m3d3a.rs"]
mod fixtures;
use doom_sim::*;
#[test]
fn original_reject_bsp_segs_clipping_and_quirks_match_c() {
    let mut visits = 0;
    let mut cases = 0;
    for (which, c) in fixtures::cases().iter().enumerate() {
        if c.n(0) != 0 {
            continue;
        }
        let sim = fixtures::sight_setup(c);
        let before = sim.snapshot();
        let result = sim
            .check_sight(ActorId::from_parts(0, 1), ActorId::from_parts(1, 1))
            .unwrap();
        fixtures::assert_sight(&result, &fixtures::rows(which, 1), 0, 0, 1);
        assert_eq!(sim.snapshot(), before);
        assert_eq!(
            sim.check_sight(ActorId::from_parts(0, 1), ActorId::from_parts(1, 1))
                .unwrap(),
            result
        );
        visits += result.visits.len();
        cases += 1;
    }
    assert_eq!(cases, 24);
    println!("C sight: {cases} scenarios, {visits} ordered visits");
}
#[test]
fn original_player_scans_preserve_slot_health_lastlook_and_fov_order() {
    let mut totals = [0; 7];
    for (which, c) in fixtures::cases().iter().enumerate() {
        if c.n(0) != 1 {
            continue;
        }
        let mut sim = fixtures::sight_setup(c);
        let mut trace = fixtures::Trace::default();
        trace.check(&sim, which, 0, sim.pending_events());
        for op in 1..=c.n(13) as usize {
            fixtures::flip(&mut sim, c, op);
            let pending = sim.pending_events().len();
            sim.look_for_players(ActorId::from_parts(0, 1), c.n(10) != 0)
                .unwrap();
            trace.check(&sim, which, op, &sim.pending_events()[pending..]);
        }
        for (a, b) in totals.iter_mut().zip([
            trace.actor_fields,
            trace.events,
            trace.queries,
            trace.visits,
            trace.slots,
            trace.draws,
            trace.roster_fields,
        ]) {
            *a += b;
        }
    }
    println!("C scans: {totals:?}");
}
#[test]
fn existing_noise_propagation_handles_distinct_emitter_and_sound_blocks() {
    let cases = fixtures::cases();
    for which in [54, 55, 56, 57, 58] {
        let c = &cases[which];
        let mut sim = fixtures::sight_setup(c);
        let pending = sim.pending_events().len();
        sim.noise_alert(
            ActorId::from_parts(c.n(11) as u32, 1),
            ActorId::from_parts(c.n(12) as u32, 1),
        )
        .unwrap();
        let got = sim.pending_events()[pending..]
            .iter()
            .filter_map(|e| {
                if let Event::Weapon(WeaponEvent::Noise {
                    sector,
                    blocks,
                    target,
                }) = e
                {
                    Some(vec![
                        i64::from(*sector),
                        i64::from(*blocks),
                        i64::from(target.slot()),
                    ])
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let expected = fixtures::rows(which, 1)
            .into_iter()
            .filter(|(tag, _)| tag == "N")
            .map(|(_, v)| v)
            .collect::<Vec<_>>();
        assert_eq!(got, expected, "noise source case {which}");
    }
}
#[test]
fn original_a_look_sound_ambush_wake_rng_and_thinker_handoff_match_c() {
    let mut totals = [0; 9];
    let mut count = 0;
    for (which, c) in fixtures::cases().iter().enumerate() {
        if c.n(0) < 2 {
            continue;
        }
        count += 1;
        let mut sim = fixtures::setup(c);
        let mut trace = fixtures::Trace::default();
        trace.check(&sim, which, 0, sim.pending_events());
        for op in 1..=c.n(13) as usize {
            fixtures::flip(&mut sim, c, op);
            let pending = sim.pending_events().len();
            if op == 1 && c.n(11) >= 0 {
                sim.noise_alert(
                    ActorId::from_parts(c.n(11) as u32, 1),
                    ActorId::from_parts(c.n(12) as u32, 1),
                )
                .unwrap();
            }
            if c.n(0) == 2 {
                sim.look(ActorId::from_parts(0, 1)).unwrap();
                trace.check(&sim, which, op, &sim.pending_events()[pending..]);
            } else {
                sim.step([TicCommand::default(); 4]).unwrap();
                trace.check(&sim, which, op, &sim.events()[pending..]);
            }
        }
        for (a, b) in totals.iter_mut().zip([
            trace.actor_fields,
            trace.events,
            trace.queries,
            trace.visits,
            trace.slots,
            trace.draws,
            trace.noise,
            trace.roster_fields,
            trace.sound_fields,
        ]) {
            *a += b;
        }
    }
    println!("C A_Look: {count} scenarios, {totals:?}");
}
