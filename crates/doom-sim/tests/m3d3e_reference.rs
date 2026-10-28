//! Original CPOS actions, refire short-circuit decisions and whole-tic evidence.
#[path = "../../../tests/support/m3d3e.rs"]
mod fixtures;
use doom_sim::*;

#[test]
fn original_cpos_shots_refire_and_nested_chase_match_c() {
    let cases = fixtures::cases();
    assert_eq!(cases.len(), 153);
    let mut totals = [0; 3];
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
                .filter_map(|event| match event {
                    Event::Refire(event) => Some(event.decision),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let expected = fixtures::source_rows(which, op)
                .into_iter()
                .filter_map(|(tag, c)| {
                    (tag == "Y").then(|| match c[1] {
                        0 => RefireDecision::EarlyContinue,
                        1 => RefireDecision::NoTarget,
                        2 => RefireDecision::DeadTarget,
                        3 => RefireDecision::NotVisible,
                        4 => RefireDecision::VisibleContinue,
                        _ => panic!("source refire branch"),
                    })
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "case {which} op {op} refire branches");
        }
        for (sum, count) in
            totals
                .iter_mut()
                .zip([trace.actor_fields, trace.draws, trace.observations])
        {
            *sum += count;
        }
    }
    assert_eq!(totals, [268606, 10697, 63308]);
    println!("Source CPOS comparisons {} cases: {totals:?}", cases.len());
}
