//! Reusable committed C-row/PBE1/DSIM evidence reader; never a gameplay oracle.
#![allow(dead_code)]
use doom_sim::*;
use std::collections::BTreeMap;

pub type Rows = BTreeMap<(usize, usize), Vec<(String, Vec<i64>)>>;
type Frames = BTreeMap<(usize, usize), [Vec<Vec<u8>>; 2]>;
struct Expected {
    digest: u64,
    bytes: usize,
    fields: usize,
    full: Option<&'static [u8]>,
}
pub struct PlayerEvidence {
    pub rows: Rows,
    frames: Frames,
    captures: BTreeMap<(usize, usize), Expected>,
}
fn take<'a>(bytes: &'a [u8], at: &mut usize, n: usize) -> &'a [u8] {
    let end = at.checked_add(n).expect("bounded committed evidence");
    let data = bytes.get(*at..end).expect("complete committed evidence");
    *at = end;
    data
}
fn word(bytes: &[u8], at: &mut usize) -> usize {
    usize::try_from(u32::from_le_bytes(take(bytes, at, 4).try_into().unwrap())).unwrap()
}
impl PlayerEvidence {
    pub fn new(
        trace: &str,
        schema: &str,
        events: &[u8],
        index: &str,
        full: &[(&str, &'static [u8])],
    ) -> Self {
        let mut rows = Rows::new();
        let mut lines = trace.lines();
        assert_eq!(lines.next(), Some(schema));
        for line in lines {
            let v: Vec<_> = line.split(',').collect();
            rows.entry((v[1].parse().unwrap(), v[2].parse().unwrap()))
                .or_default()
                .push((
                    v[0].into(),
                    v[3..].iter().map(|v| v.parse().unwrap()).collect(),
                ));
        }
        let mut at = 0;
        assert_eq!(take(events, &mut at, 4), b"PBE1");
        let mut frames = Frames::new();
        while at < events.len() {
            let key = (word(events, &mut at), word(events, &mut at));
            let counts = [word(events, &mut at), word(events, &mut at)];
            let mut frame = [Vec::new(), Vec::new()];
            for (family, count) in frame.iter_mut().zip(counts) {
                assert!(count <= events.len());
                for _ in 0..count {
                    let n = word(events, &mut at);
                    family.push(take(events, &mut at, n).to_vec());
                }
            }
            assert!(frames.insert(key, frame).is_none());
        }
        assert_eq!(at, events.len());
        let mut captures = BTreeMap::new();
        for line in index.lines().skip(1) {
            let v: Vec<_> = line.split(',').collect();
            let fixture = if v[4] == "-" {
                None
            } else {
                Some(full.iter().find(|(name, _)| *name == v[4]).unwrap().1)
            };
            assert!(
                captures
                    .insert(
                        (v[0].parse().unwrap(), v[1].parse().unwrap()),
                        Expected {
                            digest: u64::from_str_radix(v[3], 16).unwrap(),
                            bytes: v[5].parse().unwrap(),
                            fields: v[6].parse().unwrap(),
                            full: fixture,
                        },
                    )
                    .is_none()
            );
        }
        Self {
            rows,
            frames,
            captures,
        }
    }
    pub fn assert_events(&self, snapshot: &Snapshot, which: usize, op: usize) -> usize {
        let expected = &self.frames[&(which, op)];
        let mut fields = Vec::new();
        for (prefix, events) in ["events.last", "events.pending"].into_iter().zip(expected) {
            fields.push((
                format!("{prefix}.count"),
                u64::try_from(events.len()).unwrap().to_le_bytes().to_vec(),
            ));
            fields.extend(
                events
                    .iter()
                    .enumerate()
                    .map(|(i, e)| (format!("{prefix}.{i}"), e.clone())),
            );
        }
        let actual: Vec<_> = snapshot
            .fields()
            .filter(|f| f.path().starts_with("events."))
            .map(|f| (f.path().to_owned(), f.value().to_vec()))
            .collect();
        for (index, (actual, expected)) in actual.iter().zip(&fields).enumerate() {
            assert_eq!(actual, expected, "event case{which} op{op} field{index}");
        }
        assert_eq!(actual.len(), fields.len(), "event count case{which} op{op}");
        fields.len() - 2
    }
    pub fn assert_snapshot(&self, snapshot: &Snapshot, which: usize, op: usize) -> bool {
        let Some(expected) = self.captures.get(&(which, op)) else {
            return false;
        };
        let bytes = snapshot
            .diagnostic_bytes(DiagnosticLimits::default())
            .unwrap();
        if let Some(full) = expected.full {
            let view = Diagnostic::decode_player(full, DiagnosticLimits::default()).unwrap();
            for field in snapshot.fields() {
                assert_eq!(
                    Some(field.value()),
                    view.field(field.path()),
                    "full case{which} op{op} {}",
                    field.path()
                );
            }
            assert_eq!(bytes, full, "full bytes case{which} op{op}");
        }
        assert_eq!(
            snapshot.digest(),
            expected.digest,
            "digest case{which} op{op}"
        );
        assert_eq!(bytes.len(), expected.bytes);
        assert_eq!(snapshot.fields().count(), expected.fields);
        let view = Diagnostic::decode_player(&bytes, DiagnosticLimits::default()).unwrap();
        assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
        true
    }
}
