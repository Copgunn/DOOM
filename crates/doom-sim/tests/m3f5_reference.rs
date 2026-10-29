//! Raw canonical C identities/behavior and independent frozen DSIM projection.
#[path = "../../../tests/support/m3f5.rs"]
mod fixtures;
#[test]
fn canonical_demon_requests_match_original_c_fields_rng_and_ordered_bytes() {
    let mut sums = [0; 6];
    let mut raw = [0; 3];
    for (which, c) in fixtures::cases().iter().enumerate() {
        let map = fixtures::map_only(c);
        let mut spawn = fixtures::SourceTrace::for_case(c);
        spawn.check(&map, which, 0, map.pending_events(), true);
        raw[0] += spawn.actor_fields;
        raw[1] += spawn.draws;
        raw[2] += spawn.spawnpoint_fields;
        let mut sim = fixtures::setup(c);
        let mut trace = fixtures::SourceTrace::for_case(c);
        trace.check(&sim, which, 0, sim.pending_events(), false);
        let mut events = fixtures::assert_golden(&sim, which, 0);
        for op in 1..=c.v("tics") as usize {
            let pending = sim.pending_events().len();
            fixtures::advance(&mut sim, c, op);
            trace.check(&sim, which, op, &sim.events()[pending..], false);
            events += fixtures::assert_golden(&sim, which, op);
        }
        for (sum, n) in sums.iter_mut().zip([
            trace.actor_fields,
            trace.canonical_fields,
            trace.draws,
            trace.statistics_fields,
            events,
            c.v("tics") as usize + 1,
        ]) {
            *sum += n;
        }
    }
    println!(
        "raw spawn fields/draws/spawnpoints{raw:?}; C actor fields/canonical definition fields/draws/statistics/ordered event fields/captures{sums:?}"
    );
    assert_eq!(sums[0], 93834);
    assert_eq!(sums[1], 1906);
    assert_eq!(sums[4], 14063);
    assert_eq!(sums[5], 664);
}
