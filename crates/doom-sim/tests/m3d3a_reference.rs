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
#[test]
fn original_player_scans_preserve_slot_health_lastlook_and_fov_order() {
    let mut totals = [0; 7];
    for (which, c) in fixtures::cases().iter().enumerate() {
        if c.n(0) != 1 {
            continue;
        }
        let mut sim = fixtures::sight_setup(c);
        let mut trace = fixtures::Trace::default();
        trace.check(&sim, which, 0, sim.pending_events());
        for op in 1..=c.n(13) as usize {
            fixtures::flip(&mut sim, c, op);
            let pending = sim.pending_events().len();
            sim.look_for_players(ActorId::from_parts(0, 1), c.n(10) != 0)
                .unwrap();
            trace.check(&sim, which, op, &sim.pending_events()[pending..]);
        }
        for (a, b) in totals.iter_mut().zip([
            trace.actor_fields,
            trace.events,
            trace.queries,
            trace.visits,
            trace.slots,
            trace.draws,
            trace.roster_fields,
        ]) {
            *a += b;
        }
    }
    println!("C scans: {totals:?}");
}
