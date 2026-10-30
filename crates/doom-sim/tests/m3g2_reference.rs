//! New compiled-C player behavior, independent diagnostic bytes and ordered events.
#[path = "../../../tests/support/m3g2.rs"]
mod source;
#[test]
fn original_player_fields_rng_states_statistics_and_complete_events_match_c() {
    let mut sums = [0; 5];
    let mut captures = 0;
    let mut events = 0;
    for (which, c) in source::cases().iter().enumerate() {
        let mut sim = source::setup(which, c);
        let mut trace = source::SourceTrace::default();
        trace.check(&sim, which, 0, sim.pending_events());
        events += source::assert_events(&sim, which, 0);
        captures += usize::from(source::assert_snapshot(&sim, which, 0));
        for op in 1..=c.tics {
            let pending = sim.pending_events().len();
            source::operations(&mut sim, which, op);
            sim.advance(source::COMMANDS).unwrap();
            trace.check(&sim, which, op, &sim.events()[pending..]);
            events += source::assert_events(&sim, which, op);
            captures += usize::from(source::assert_snapshot(&sim, which, op));
        }
        for (sum, value) in sums.iter_mut().zip(trace.counts) {
            *sum += value;
        }
    }
    println!(
        "NEW C actor/player/association/draw/statistics fields{sums:?}, captures{captures}, ordered events{events}"
    );
    assert_eq!(sums, [5850, 1110, 225, 372, 740]);
    assert_eq!(captures, 185);
    assert_eq!(events, 4256);
}
