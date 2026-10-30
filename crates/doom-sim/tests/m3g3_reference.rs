//! Genuine new original-C fields and independent complete ordered event evidence.
#[path = "../../../tests/support/m3g3.rs"]
mod source;
#[test]
fn original_player_thrust_grounded_motion_collision_friction_and_lifecycle_match_c() {
    let mut sums = [0; 5];
    let mut captures = 0;
    let mut event_fields = 0;
    for (which, c) in source::cases().iter().enumerate() {
        let (mut sim, a) = source::setup(c);
        let mut trace = source::SourceTrace::default();
        trace.check(&sim, which, 0, sim.pending_events(), source::rows(which, 0));
        event_fields += source::assert_events(&sim, which, 0);
        captures += usize::from(source::assert_snapshot(&sim, which, 0));
        for op in 1..=c.tics {
            let pending = sim.pending_events().len();
            source::operations(&mut sim, a, c, op);
            sim.advance(source::COMMANDS).unwrap();
            trace.check(
                &sim,
                which,
                op,
                &sim.events()[pending..],
                source::rows(which, op),
            );
            event_fields += source::assert_events(&sim, which, op);
            captures += usize::from(source::assert_snapshot(&sim, which, op));
        }
        for (sum, value) in sums.iter_mut().zip(trace.counts) {
            *sum += value;
        }
    }
    println!(
        "New C actor/player/association/RNG/statistic fields{sums:?}; captures{captures}; ordered events{event_fields}"
    );
    assert_eq!(sums, [12922, 1416, 497, 247, 944]);
    assert_eq!(captures, 236);
    assert_eq!(event_fields, 6652);
}
