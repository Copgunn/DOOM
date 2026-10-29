//! Original C spawn projection, admitted gameplay and independent canonical bytes.
#[path = "../../../tests/support/m3d3i.rs"]
mod fixtures;
#[test]
fn genuine_original_spawn_and_ordered_gameplay_match_independent_v16() {
    let cases = fixtures::cases();
    assert_eq!(cases.len(), 334);
    assert_eq!(
        cases
            .iter()
            .map(|c| c.v("map"))
            .collect::<std::collections::BTreeSet<_>>(),
        (0..53).collect()
    );
    let mut totals = [0usize; 3];
    let mut raw = [0usize; 2];
    for (which, c) in cases.iter().enumerate() {
        let (s, report) = fixtures::map_only(c);
        let mut source = fixtures::SourceTrace::default();
        source.check_original_spawn(&s, which);
        raw[0] += source.actor_fields;
        raw[1] += source.draws;
        let f = include_str!("../../../tests/reference/m3d3i_spawn.csv")
            .lines()
            .find(|line| line.starts_with(&format!("F,{which},0,-1,")))
            .unwrap()
            .split(',')
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert_eq!(report.spawned.len(), f[9].parse::<usize>().unwrap());
        assert_eq!(s.rng().indices()[0], f[10].parse::<u8>().unwrap());
        assert_eq!(
            report.spawned.len(),
            f[11].parse::<usize>().unwrap(),
            "original totals are observed only, not a Rust scoring claim"
        );
        let mut sim = fixtures::setup(c);
        let mut trace = fixtures::SourceTrace::default();
        trace.check(&sim, which, 0, sim.pending_events());
        fixtures::assert_golden(&sim, which, 0);
        for op in 1..=c.v("tics") as usize {
            let pending = sim.pending_events().len();
            fixtures::advance(&mut sim, c, op);
            trace.check(&sim, which, op, &sim.events()[pending..]);
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
    println!(
        "Original spawn projected fields/RNG {raw:?}; admitted gameplay fields/RNG/instrumentation {totals:?}"
    );
    assert_eq!(raw, [10868, 37437]);
    assert_eq!(totals, [585936, 38944, 131946]);
}
