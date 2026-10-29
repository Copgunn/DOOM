//! Combined genuine original-C gameplay and independently encoded DSIM-v20.
#[path = "../../../tests/support/m3f3_clipping.rs"]
mod fixtures;
#[test]
fn noncardinal_ai_sight_openings_and_clipping_match_original_c() {
    let cases = fixtures::cases();
    let mut counts = [0usize; 4];
    let mut raw = [0usize; 2];
    let mut events = 0;
    for (which, c) in cases.iter().enumerate() {
        let (s, _) = fixtures::map_only(c);
        let mut source = fixtures::SourceTrace::for_case(c);
        source.check_original_spawn(&s, which);
        raw[0] += source.actor_fields;
        raw[1] += source.draws;
        let mut sim = fixtures::setup(c);
        let mut trace = fixtures::SourceTrace::for_case(c);
        trace.check(&sim, which, 0, sim.pending_events());
        events += fixtures::assert_golden(&sim, which, 0);
        for op in 1..=c.v("tics") as usize {
            let pending = sim.pending_events().len();
            fixtures::advance(&mut sim, c, op);
            trace.check(&sim, which, op, &sim.events()[pending..]);
            events += fixtures::assert_golden(&sim, which, op);
        }
        for (sum, value) in counts.iter_mut().zip([
            trace.actor_fields,
            trace.draws,
            trace.instrumentation_rows,
            trace.statistics_fields,
        ]) {
            *sum += value;
        }
    }
    println!(
        "Original raw-spawn fields/RNG {raw:?}; combined fields/RNG/counted instrumentation rows/statistics {counts:?}; exact ordered event fields {events}"
    );
}
