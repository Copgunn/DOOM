//! Independent original-C sight and perception decisions.
#[path = "../../../tests/support/m3d3a.rs"]
mod fixtures;
use doom_sim::*;
#[test]
fn original_reject_bsp_segs_clipping_and_quirks_match_c() {
    let mut visits = 0;
    let mut cases = 0;
    for (which, c) in fixtures::cases().iter().enumerate() {
        if c.n(0) != 0 {
            continue;
        }
        let sim = fixtures::sight_setup(c);
        let before = sim.snapshot();
        let result = sim
            .check_sight(ActorId::from_parts(0, 1), ActorId::from_parts(1, 1))
            .unwrap();
        fixtures::assert_sight(&result, &fixtures::rows(which, 1), 0, 0, 1);
        assert_eq!(sim.snapshot(), before);
        assert_eq!(
            sim.check_sight(ActorId::from_parts(0, 1), ActorId::from_parts(1, 1))
                .unwrap(),
            result
        );
        visits += result.visits.len();
        cases += 1;
    }
    assert_eq!(cases, 21);
    println!("C sight: {cases} scenarios, {visits} ordered visits");
}
