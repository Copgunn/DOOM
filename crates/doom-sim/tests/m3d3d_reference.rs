//! Genuine original SPosAttack with per-pellet and whole-tic independent evidence.
#[path = "../../../tests/support/m3d3d.rs"]
mod fixtures;
use doom_sim::*;
#[test]
fn original_sequential_pellets_and_complete_shotgunner_tics_match_c() {
    assert_eq!(fixtures::cases().len(), 105);
    let mut totals = [0; 3];
    let cases = fixtures::cases();
    for (which, c) in cases.iter().enumerate() {
        let mut sim = fixtures::setup(c);
        let mut trace = fixtures::SourceTrace::default();
        trace.check(&sim, which, 0, sim.pending_events());
        fixtures::assert_golden(&sim, which, 0);
        for op in 1..=c.v("tics") as usize {
            let pending = sim.pending_events().len();
            fixtures::advance(&mut sim, c, op);
            let events = if c.v("mode") == 4 {
                sim.events()
            } else {
                sim.pending_events()
            };
            trace.check(&sim, which, op, &events[pending..]);
            fixtures::assert_golden(&sim, which, op);
            let actual = events[pending..]
                .iter()
                .filter_map(|e| {
                    if let Event::Shotgun(ShotgunEvent::PelletDone { index, rng, .. }) = e {
                        Some((i64::from(*index), i64::from(*rng)))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let expected = fixtures::source_rows(which, op)
                .into_iter()
                .filter_map(|(tag, c)| {
                    if tag == "W" && c[0] == 3 {
                        Some((c[2], c[3]))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(
                actual, expected,
                "case {which} op {op} each source pellet RNG index"
            );
        }
        for (sum, n) in totals
            .iter_mut()
            .zip([trace.actor_fields, trace.draws, trace.observations])
        {
            *sum += n;
        }
    }
    assert_eq!(totals, [147940, 3894, 34350]);
    println!(
        "Source shotgunner comparisons {} cases: {totals:?}",
        cases.len()
    );
}
