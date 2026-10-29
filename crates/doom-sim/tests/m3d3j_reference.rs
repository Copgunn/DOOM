//! Genuine C observations, explicit identity/state projection and independent bytes.
#[path = "../../../tests/support/m3d3j.rs"]
mod fixtures;
#[test]
fn original_map_totals_credit_corpse_and_rng_match_independent_v17() {
    let cases = fixtures::cases();
    assert_eq!(cases.len(), 351);
    assert_eq!(
        cases
            .iter()
            .map(|c| c.v("map"))
            .collect::<std::collections::BTreeSet<_>>(),
        (0..54).collect()
    );
    let mut totals = [0usize; 4];
    let mut raw = [0usize; 2];
    for (which, c) in cases.iter().enumerate() {
        let (s, report) = fixtures::map_only(c);
        let mut source = fixtures::SourceTrace::for_case(c);
        source.check_original_spawn(&s, which);
        raw[0] += source.actor_fields;
        raw[1] += source.draws;
        assert_eq!(
            s.level_statistics().unwrap().total_kills,
            report.spawned.len() as i32
        );
        let mut sim = fixtures::setup(c);
        let mut trace = fixtures::SourceTrace::for_case(c);
        trace.check(&sim, which, 0, sim.pending_events());
        fixtures::assert_golden(&sim, which, 0);
        for op in 1..=c.v("tics") as usize {
            let pending = sim.pending_events().len();
            fixtures::advance(&mut sim, c, op);
            trace.check(&sim, which, op, &sim.events()[pending..]);
            fixtures::assert_golden(&sim, which, op);
        }
        for (sum, value) in totals.iter_mut().zip([
            trace.actor_fields,
            trace.draws,
            trace.observations,
            trace.statistics_fields,
        ]) {
            *sum += value;
        }
    }
    println!(
        "Unadapted spawn fields/RNG {raw:?}; adapted gameplay fields/RNG/instrumentation/statistics {totals:?}"
    );
    assert_eq!(raw, [11622, 37742]);
    assert_eq!(totals, [922324, 39340, 149398, 23188]);
}
