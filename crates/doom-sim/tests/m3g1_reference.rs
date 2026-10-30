//! Reused original C and independent v21 fixtures; no new source coverage.
#[path = "../../../tests/support/m3g1.rs"]
mod fixture;
use fixture::ownership as frozen;
#[test]
fn both_apis_match_every_frozen_c_ownership_capture_and_ordered_event_field() {
    legacy_lifecycle();
    let mut totals = [0; 5];
    for (which, case) in frozen::cases().iter().enumerate() {
        let mut observed = frozen::setup(which, case);
        let mut direct = observed.clone();
        let mut source = frozen::SourceTrace::default();
        let mut legacy_source = frozen::SourceTrace::default();
        let mut events = 0;
        let tics = usize::try_from(case.v("tics")).unwrap();
        for op in 0..=tics {
            let pending = direct.pending_events().len();
            if op != 0 {
                frozen::advance(&mut observed, which, case, op);
                fixture::ownership_tick(&mut direct, which, case, op);
            }
            assert_eq!(observed.snapshot(), direct.snapshot());
            assert_eq!(
                observed.snapshot().canonical_bytes(),
                direct.snapshot().canonical_bytes()
            );
            source.check(
                &direct,
                which,
                op,
                if op == 0 {
                    direct.pending_events()
                } else {
                    &direct.events()[pending..]
                },
            );
            legacy_source.check(
                &observed,
                which,
                op,
                if op == 0 {
                    observed.pending_events()
                } else {
                    &observed.events()[pending..]
                },
            );
            let count = frozen::assert_golden(&direct, which, op);
            assert_eq!(frozen::assert_golden(&observed, which, op), count);
            events += count;
        }
        assert_eq!(
            [source.actor_fields, source.draws, source.statistics_fields],
            [
                legacy_source.actor_fields,
                legacy_source.draws,
                legacy_source.statistics_fields
            ]
        );
        for (total, count) in totals.iter_mut().zip([
            source.actor_fields,
            source.draws,
            source.statistics_fields,
            events,
            tics + 1,
        ]) {
            *total += count;
        }
    }
    assert_eq!(totals, [11856, 63, 376, 1138, 94]);
    println!(
        "M3g1 reused per API: actor_fields={} draws={} statistics={} events={} captures={}",
        totals[0], totals[1], totals[2], totals[3], totals[4]
    );
}

// Explicit historical v20 advance path includes external pain/death inputs,
// terminal corpse chronology and complete frozen event payloads.
fn legacy_lifecycle() {
    let cases = fixture::expanded::cases();
    let which = cases
        .iter()
        .position(|c| c.label == "long_lifecycle")
        .unwrap();
    let case = &cases[which];
    let mut observed = fixture::expanded::setup(case);
    let mut direct = fixture::Workload::setup("long_lifecycle", "20").unwrap();
    let mut source = fixture::expanded::SourceTrace::for_case(case);
    let mut legacy_source = fixture::expanded::SourceTrace::for_case(case);
    let mut events = 0;
    let tics = usize::try_from(case.v("tics")).unwrap();
    for op in 0..=tics {
        let pending = direct.sim.pending_events().len();
        if op != 0 {
            fixture::expanded::advance(&mut observed, case, op);
            direct.before_tick(op).unwrap();
            direct
                .sim
                .advance([doom_sim::TicCommand::default(); 4])
                .unwrap();
        }
        assert_eq!(observed.snapshot(), direct.sim.snapshot());
        source.check(
            &direct.sim,
            which,
            op,
            if op == 0 {
                direct.sim.pending_events()
            } else {
                &direct.sim.events()[pending..]
            },
        );
        legacy_source.check(
            &observed,
            which,
            op,
            if op == 0 {
                observed.pending_events()
            } else {
                &observed.events()[pending..]
            },
        );
        let count = fixture::expanded::assert_golden(&direct.sim, which, op);
        assert_eq!(
            count,
            fixture::expanded::assert_golden(&observed, which, op)
        );
        events += count;
    }
    assert_eq!(
        [source.actor_fields, source.draws, source.statistics_fields],
        [
            legacy_source.actor_fields,
            legacy_source.draws,
            legacy_source.statistics_fields
        ]
    );
    println!(
        "M3g1 reused v20 per API: actor_fields={} draws={} statistics={} events={events} captures={}",
        source.actor_fields,
        source.draws,
        source.statistics_fields,
        tics + 1
    );
}
