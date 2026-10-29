//! Genuine original Sarg/Face/range/damage and complete thinker comparisons.
#[path = "../../../tests/support/m3d3h.rs"]
mod fixtures;
#[test]
fn original_demon_stand_run_and_whole_tics_match_c_and_independent_v15() {
    let cases = fixtures::cases();
    assert_eq!(cases.len(), 390);
    assert_eq!(
        cases
            .iter()
            .map(|c| c.v("map"))
            .collect::<std::collections::BTreeSet<_>>(),
        (0..21).collect()
    );
    let mut totals = [0usize; 3];
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
        }
        for (sum, value) in
            totals
                .iter_mut()
                .zip([trace.actor_fields, trace.draws, trace.observations])
        {
            *sum += value;
        }
    }
    assert_eq!(totals, [935012, 38578, 151474]);
    println!(
        "Original Demon stand/run comparisons {} cases: {totals:?}",
        cases.len()
    );
}
