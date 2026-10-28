//! Original Linux 1.10 attack bodies plus independent full canonical observations.
#[path = "../../../tests/support/m3d3c.rs"]
mod fixtures;
#[test]
fn original_attack_decisions_actions_and_complete_tics_match_c() {
    assert_eq!(fixtures::cases().len(), 158);
    let mut totals = [0; 3];
    for (which, c) in fixtures::cases().iter().enumerate() {
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
    assert_eq!(totals, [178178, 2430, 20785]);
    println!("Original C attack comparisons: {totals:?}");
}
