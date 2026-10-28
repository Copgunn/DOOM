//! Genuine original P_Move/TryWalk/NewChaseDir/A_Chase comparisons.
#[path = "../../../tests/support/m3d3b.rs"]
mod fixtures;
#[test]
fn original_movement_decisions_and_complete_pursuit_tics_match_c() {
    let mut totals = [0; 10];
    for (which, c) in fixtures::cases().iter().enumerate() {
        let mut sim = fixtures::setup(c);
        let mut trace = fixtures::Trace::default();
        trace.check(&sim, which, 0, sim.pending_events());
        fixtures::assert_golden(&sim, which, 0);
        for op in 1..=c.n(13) as usize {
            let pending = sim.pending_events().len();
            fixtures::advance(&mut sim, c, op);
            let events = if c.n(0) == 4 {
                sim.events()
            } else {
                sim.pending_events()
            };
            trace.check(&sim, which, op, &events[pending..]);
            fixtures::assert_golden(&sim, which, op);
        }
        for (sum, value) in totals.iter_mut().zip([
            trace.actor_fields,
            trace.events,
            trace.moves,
            trace.candidates,
            trace.directions,
            trace.draws,
            trace.queries,
            trace.visits,
            trace.slots,
            trace.roster_fields,
        ]) {
            *sum += value;
        }
    }
    assert_eq!(fixtures::cases().len(), 96);
    assert_eq!(
        totals,
        [108992, 29416, 2401, 11458, 2103, 1380, 13, 158, 116, 10020]
    );
    println!("C pursuit comparisons {totals:?}");
}
