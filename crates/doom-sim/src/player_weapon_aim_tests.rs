//! Independent query observations and Rust-only transaction/invariant policies.
use super::*;
use crate::weapon_pack_evidence as source;

fn object(value: TraceObject, births: &source::SourceTrace) -> [i64; 2] {
    match value {
        TraceObject::Line(line) => [1, i64::from(line)],
        TraceObject::Actor(actor) => [0, births.birth(Some(actor))],
    }
}
#[test]
fn actual_weapon_pack_aim_queries_match_independent_source_intercepts_and_windows() {
    let mut count = 0;
    for c in source::cases() {
        let (mut sim, _) = source::setup(&c);
        let mut births = source::SourceTrace::seeded(c.seed);
        births.check(
            &sim,
            c.which,
            0,
            sim.pending_events(),
            source::rows(c.which, 0),
        );
        AIM_OBSERVATIONS.with(|capture| *capture.borrow_mut() = Some(Vec::new()));
        for op in 1..=c.tics {
            if c.which == 6 && op == 45 {
                break;
            }
            let pending = sim.pending_events().len();
            sim.advance(source::commands(&c, op)).unwrap();
            let rows = source::rows(c.which, op);
            births.check(&sim, c.which, op, &sim.events()[pending..], rows);
            let actual = AIM_OBSERVATIONS
                .with(|capture| std::mem::take(capture.borrow_mut().as_mut().unwrap()));
            let mut expected = Vec::new();
            let mut query = Vec::new();
            let mut inside = false;
            let mut shot = false;
            for (tag, v) in rows {
                if tag == "X" {
                    shot = v[0] == 0;
                }
                if tag == "D" && !shot {
                    inside = true;
                }
                if inside {
                    query.push((tag.as_str(), v.as_slice()));
                }
                if tag == "J" && v[0] == 5 {
                    expected.push(std::mem::take(&mut query));
                    inside = false;
                }
            }
            assert_eq!(actual.len(), expected.len(), "queries {} op{op}", c.label);
            for (actual, expected) in actual.iter().zip(expected) {
                let d = expected.iter().find(|(tag, _)| *tag == "D").unwrap().1;
                assert_eq!(
                    actual.trace.origin.map(|v| i64::from(v.bits())).as_slice(),
                    &d[1..3]
                );
                assert_eq!(
                    actual.trace.delta.map(|v| i64::from(v.bits())).as_slice(),
                    &d[3..5]
                );
                let cells: Vec<_> = expected
                    .iter()
                    .filter(|(tag, _)| *tag == "B")
                    .map(|(_, v)| v[1..].to_vec())
                    .collect();
                assert_eq!(
                    actual
                        .trace
                        .cells
                        .iter()
                        .map(|c| c.map(i64::from).to_vec())
                        .collect::<Vec<_>>(),
                    cells
                );
                let candidates: Vec<_> = expected
                    .iter()
                    .filter(|(tag, _)| *tag == "U")
                    .map(|(_, v)| [v[1], v[2]])
                    .collect();
                assert_eq!(
                    actual
                        .trace
                        .candidates
                        .iter()
                        .map(|v| object(*v, &births))
                        .collect::<Vec<_>>(),
                    candidates
                );
                let mut intercepts: Vec<_> = expected
                    .iter()
                    .filter(|(tag, _)| *tag == "I")
                    .map(|(_, v)| [v[1], v[2], v[3]])
                    .collect();
                intercepts.sort_by_key(|v| v[2]);
                assert_eq!(
                    actual
                        .trace
                        .intercepts
                        .iter()
                        .map(|i| {
                            let [kind, id] = object(i.object, &births);
                            [kind, id, i64::from(i.fraction.bits())]
                        })
                        .collect::<Vec<_>>(),
                    intercepts
                );
                let result = expected.last().unwrap().1;
                let stop = actual
                    .stopped
                    .map_or([-1, -1], |value| object(value, &births));
                assert_eq!(
                    [
                        births.birth(actual.target),
                        i64::from(actual.slope.bits()),
                        i64::from(actual.window[0].bits()),
                        i64::from(actual.window[1].bits()),
                        i64::from(actual.traversed),
                        stop[0],
                        stop[1]
                    ]
                    .as_slice(),
                    &result[2..]
                );
                count += 1;
            }
        }
        AIM_OBSERVATIONS.with(|capture| *capture.borrow_mut() = None);
    }
    assert_eq!(count, 515);
}
