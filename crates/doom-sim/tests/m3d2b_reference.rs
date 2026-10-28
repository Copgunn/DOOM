//! Original C projectile evidence, with generation-aware allocation correspondence.
#[path = "../../../tests/support/m3d2b.rs"]
mod fixtures;
use doom_sim::TicCommand;
#[test]
fn complete_original_plasma_lifecycles_match_c() {
    let mut totals = [0; 6];
    for (which, c) in fixtures::cases().iter().enumerate() {
        if c.n(0) == 2 {
            continue;
        }
        let mut sim = fixtures::setup(c);
        let mut trace = fixtures::Trace::default();
        trace.check(&sim, which, 0, sim.pending_events());
        if fixtures::has_golden(which, 0) {
            fixtures::assert_golden(&sim, which, 0);
        }
        let old = sim.pending_events().len();
        fixtures::launch(&mut sim, c);
        trace.check(&sim, which, 1, &sim.pending_events()[old..]);
        if fixtures::has_golden(which, 1) {
            fixtures::assert_golden(&sim, which, 1);
        }
        let mut op = 1;
        for batch in 0..if c.n(0) == 3 { 2 } else { 1 } {
            if batch != 0 {
                op += 1;
                fixtures::launch(&mut sim, c);
                trace.check(&sim, which, op, sim.pending_events());
                if fixtures::has_golden(which, op) {
                    fixtures::assert_golden(&sim, which, op);
                }
            }
            for _ in 0..c.n(16) {
                op += 1;
                let pending = sim.pending_events().len();
                sim.step([TicCommand::default(); 4]).unwrap();
                trace.check(&sim, which, op, &sim.events()[pending..]);
                if fixtures::has_golden(which, op) {
                    fixtures::assert_golden(&sim, which, op);
                }
            }
        }
        for (a, b) in totals.iter_mut().zip([
            trace.actor_fields,
            trace.events,
            trace.moves,
            trace.candidates,
            trace.openings,
            trace.draws,
        ]) {
            *a += b;
        }
    }

    assert_eq!(totals, [155352, 12991, 1151, 6013, 16, 542]);
    println!(
        "C comparisons: actor fields {}, ordered events {}, moves {}, candidates {}, openings {}, RNG draws {}",
        totals[0], totals[1], totals[2], totals[3], totals[4], totals[5]
    );
}
