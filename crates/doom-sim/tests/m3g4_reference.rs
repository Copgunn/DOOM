//! New compiled-C command/gate observations and independent full diagnostic bytes.
#[path = "../../../tests/support/m3g4.rs"]
mod source;

#[test]
fn canonical_player_commands_reaction_gate_movement_and_order_match_source() {
    let mut sums = [0; 5];
    let mut motion = [0; 2];
    let mut captures = 0;
    let mut event_fields = 0;
    for (which, case) in source::cases().iter().enumerate() {
        let (mut sim, a) = source::setup(case);
        let mut trace = source::SourceTrace::default();
        for op in 0..=case.tics {
            let pending = sim.pending_events().len();
            if op != 0 {
                source::operations(&mut sim, a, case, op);
                sim.advance(source::commands(case, op))
                    .unwrap_or_else(|e| panic!("{} op{op}: {e}", case.label));
            }
            let events = if op == 0 {
                sim.pending_events()
            } else {
                &sim.events()[pending..]
            };
            trace.check(&sim, which, op, events, source::rows(which, op));
            let snapshot = sim.snapshot();
            let m = source::check_commands_and_motion(&sim, a, &snapshot, source::rows(which, op));
            for (sum, value) in motion.iter_mut().zip(m) {
                *sum += value;
            }
            event_fields += source::evidence().assert_events(&snapshot, which, op);
            captures += usize::from(source::evidence().assert_snapshot(&snapshot, which, op));
        }
        for (sum, value) in sums.iter_mut().zip(trace.counts) {
            *sum += value;
        }
    }
    assert_eq!(sums, [9776, 1896, 376, 39, 1264]);
    assert_eq!(motion, [19019, 5852]);
    assert_eq!(captures, 316);
    assert_eq!(event_fields, 10530);
}
