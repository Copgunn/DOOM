//! New C inputs through shared execution services. Oracle-only body135 binding;
//! source birth identities map to Rust generations under declared schema policy.
#![allow(dead_code)]
#[path = "m3f7.rs"]
pub mod prior;
use doom_sim::{identity::*, *};
use doom_types::{DoomRng, Fixed};
pub use prior::historical::Case;
use std::{collections::BTreeMap, sync::OnceLock};
pub const OWNER: DefinitionIdentity = DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT);
pub const BODY: DefinitionIdentity = DefinitionIdentity::Context(ContextRole::UnboundBody);
pub fn id(slot: u32) -> ActorId {
    ActorId::from_parts(slot, 1)
}
pub fn cases() -> Vec<Case> {
    include_str!("../reference/m3f8_cases.csv")
        .lines()
        .skip(1)
        .map(|line| {
            let fields: Vec<_> = line.split(',').collect();
            Case {
                label: fields[1].into(),
                values: fields[2..].iter().map(|v| v.parse().unwrap()).collect(),
            }
        })
        .collect()
}
pub fn setup(which: usize, c: &Case) -> Simulation {
    let mut sim = prior::setup(c);
    sim.with_execution_context(|ctx| {
        let owner = ctx.admit_local(id(0), OWNER)?;
        let body = ctx.admit_local(id(1), BODY)?;
        ctx.set_target(owner, Some(body))?;
        if [4, 5].contains(&which) {
            ctx.mark_remove(body)?;
        }
        if which == 6 {
            ctx.mark_remove(owner)?;
        }
        Ok(())
    })
    .unwrap();
    sim
}
pub fn advance(sim: &mut Simulation, which: usize, c: &Case, op: usize) {
    sim.with_execution_context(|ctx| {
        match (which, op) {
            (0, 1) => {
                let owner = ctx.admit_local(id(0), OWNER)?;
                let body = ctx.admit_local(id(2), BODY)?;
                ctx.set_target(owner, Some(body))?;
            }
            (0, 2) => {
                let owner = ctx.admit_local(id(0), OWNER)?;
                ctx.set_target(owner, None)?;
            }
            (1, 1) => {
                let owner = ctx.admit_local(id(0), OWNER)?;
                let body = ctx.admit_local(id(2), BODY)?;
                ctx.damage(owner, None, Some(body), 1)?;
            }
            (2, 1) => {
                let owner = ctx.admit_local(id(0), OWNER)?;
                let body = ctx.admit_local(id(1), BODY)?;
                ctx.damage(body, None, Some(owner), 1000)?;
            }
            (3 | 7, 1) => {
                let owner = ctx.admit_local(id(0), OWNER)?;
                let body = ctx.admit_local(id(1), BODY)?;
                ctx.damage(owner, None, Some(body), 150)?;
            }
            (4 | 5, 1) => {
                let owner = ctx.admit_local(id(0), OWNER)?;
                ctx.set_target(owner, None)?;
                ctx.set_perception_player(0, None, false, c.v("p0health"))?;
            }
            (5, 2) => {
                let body = ctx.spawn_body(
                    [Fixed::from_bits(c.v("p0x")), Fixed::from_bits(c.v("p0y"))],
                    SpawnHeight::Absolute(Fixed::from_bits(c.v("p0z"))),
                )?;
                ctx.set_perception_player(0, Some(body), true, 100)?;
                let owner = ctx.admit_local(id(0), OWNER)?;
                ctx.set_target(owner, Some(body))?;
            }
            (7, 2) => {
                let body = ctx.admit_local(id(1), BODY)?;
                ctx.set_perception_player(0, None, false, c.v("p0health"))?;
                ctx.mark_remove(body)?;
            }
            _ => {}
        }
        ctx.step([TicCommand::default(); 4])?;
        Ok(())
    })
    .unwrap();
}
type Rows = BTreeMap<(usize, usize), Vec<(String, Vec<i64>)>>;
pub fn rows(which: usize, op: usize) -> &'static [(String, Vec<i64>)] {
    static ROWS: OnceLock<Rows> = OnceLock::new();
    ROWS.get_or_init(|| {
        let mut all = Rows::new();
        let mut lines = include_str!("../reference/m3f8_trace.csv").lines();
        assert_eq!(lines.next(), Some("DOOM-OWNERSHIP-v1"));
        for line in lines {
            let c: Vec<_> = line.split(',').collect();
            all.entry((c[1].parse().unwrap(), c[2].parse().unwrap()))
                .or_default()
                .push((
                    c[0].into(),
                    c[3..].iter().map(|s| s.parse().unwrap()).collect(),
                ));
        }
        all
    })
    .get(&(which, op))
    .unwrap()
}
#[derive(Default)]
pub struct SourceTrace {
    births: Vec<ActorId>,
    rng: DoomRng,
    pub actor_fields: usize,
    pub draws: usize,
    pub statistics_fields: usize,
}
impl SourceTrace {
    pub fn check(&mut self, sim: &Simulation, which: usize, op: usize, events: &[Event]) {
        for event in events {
            if let Event::Spawned(id) = event {
                self.births.push(*id);
            }
        }
        for (tag, c) in rows(which, op) {
            if tag == "E" && c[0] == 8 {
                assert_eq!(i64::from(self.rng.p_random()), c[2]);
                self.draws += 1;
            }
            if tag == "LS" {
                let s = sim.level_statistics().unwrap();
                assert_eq!(
                    [
                        s.total_kills,
                        s.total_items,
                        s.total_secrets,
                        s.single_player_kill_count
                    ]
                    .map(i64::from)
                    .as_slice(),
                    c
                );
                self.statistics_fields += 4;
            }
            if tag != "A" {
                continue;
            }
            let id = self.births[usize::try_from(c[0]).unwrap()];
            if c[1] == 0 {
                assert!(sim.actor(id).is_none());
                continue;
            }
            let a = sim.actor(id).unwrap();
            let l = a.life().unwrap();
            let actual = [
                i64::from(l.actor_type.0),
                i64::from(a.position()[0].bits()),
                i64::from(a.position()[1].bits()),
                i64::from(a.z().bits()),
                i64::from(a.momentum()[0].bits()),
                i64::from(a.momentum()[1].bits()),
                i64::from(a.vertical_momentum().bits()),
                i64::from(l.health),
                l.state.map_or(-1, |s| i64::from(s.0)),
                i64::from(l.tics),
                i64::from(l.flags),
                i64::from(a.height().bits()),
                i64::from(a.radius().bits()),
                i64::from(l.reaction_time),
                i64::from(l.threshold),
                l.target.map_or(-1, |id| {
                    i64::try_from(self.births.iter().position(|b| *b == id).unwrap()).unwrap()
                }),
                i64::from(a.lastlook()),
                i64::from(a.constraints()[0].bits()),
                i64::from(a.constraints()[1].bits()),
                i64::from(a.location().unwrap().subsector),
                i64::from(sim.rng().indices()[0]),
                i64::from(a.angle()),
                i64::from(l.sprite),
                i64::from(l.frame),
                i64::from(l.movedir),
                i64::from(l.movecount),
            ];
            for (field, (got, want)) in actual.iter().zip(&c[2..]).enumerate() {
                let expected = if field == 0 && *want == 135 {
                    i64::from(OriginalActorType::COUNT)
                } else {
                    *want
                };
                assert_eq!(
                    *got, expected,
                    "case{which} op{op} birth{} field{field}",
                    c[0]
                );
                self.actor_fields += 1;
            }
        }
        assert_eq!(
            sim.rng().indices(),
            self.rng.indices(),
            "case{which} op{op} RNG consumption"
        );
    }
}
type EventFields = BTreeMap<(usize, usize), Vec<(String, Vec<u8>)>>;
fn event_fields() -> &'static EventFields {
    static FIELDS: OnceLock<EventFields> = OnceLock::new();
    FIELDS.get_or_init(|| {
        let bytes = include_bytes!("../reference/m3f8_events.bin");
        let mut at = 4;
        assert_eq!(&bytes[..at], b"EV21");
        fn number(bytes: &[u8], at: &mut usize, n: usize) -> usize {
            let mut value = 0_u64;
            for (i, b) in bytes[*at..*at + n].iter().enumerate() {
                value |= u64::from(*b) << (8 * i);
            }
            *at += n;
            usize::try_from(value).unwrap()
        }
        let mut all = EventFields::new();
        while at < bytes.len() {
            let which = number(bytes, &mut at, 4);
            let op = number(bytes, &mut at, 4);
            let count = number(bytes, &mut at, 4);
            let mut fields = Vec::new();
            for _ in 0..count {
                let n = number(bytes, &mut at, 4);
                let path = std::str::from_utf8(&bytes[at..at + n]).unwrap().to_owned();
                at += n;
                let n = number(bytes, &mut at, 8);
                let value = bytes[at..at + n].to_vec();
                at += n;
                fields.push((path, value));
            }
            assert!(all.insert((which, op), fields).is_none());
        }
        all
    })
}
pub fn assert_golden(sim: &Simulation, which: usize, op: usize) -> usize {
    let line = include_str!("../reference/m3f8_snapshots.csv")
        .lines()
        .skip(1)
        .find(|line| {
            let c: Vec<_> = line.split(',').collect();
            c[0].parse::<usize>().unwrap() == which && c[1].parse::<usize>().unwrap() == op
        })
        .unwrap();
    let row: Vec<_> = line.split(',').collect();
    let s = sim.snapshot();
    let expected = &event_fields()[&(which, op)];
    let actual: Vec<_> = s
        .fields()
        .filter(|f| f.path().starts_with("events."))
        .collect();
    assert_eq!(actual.len(), expected.len());
    for (a, (path, bytes)) in actual.iter().zip(expected) {
        assert_eq!(a.path(), path);
        assert_eq!(a.value(), bytes, "case{which} op{op} {path}");
    }
    assert_eq!(
        s.digest(),
        u64::from_str_radix(row[3], 16).unwrap(),
        "case{which} op{op}"
    );
    if row[4] != "-" {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/reference")
            .join(row[4]);
        let expected = std::fs::read(path).unwrap();
        assert_eq!(s.canonical_bytes(), expected);
    }
    let bytes = s.diagnostic_bytes(DiagnosticLimits::default()).unwrap();
    let view = Diagnostic::decode(&bytes, DiagnosticLimits::default()).unwrap();
    assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
    expected.len() - 2
}
pub fn run(which: usize, c: &Case) -> [usize; 5] {
    let mut sim = setup(which, c);
    let mut source = SourceTrace::default();
    source.check(&sim, which, 0, sim.pending_events());
    let mut event_fields = assert_golden(&sim, which, 0);
    for op in 1..=usize::try_from(c.v("tics")).unwrap() {
        let pending = sim.pending_events().len();
        advance(&mut sim, which, c, op);
        source.check(&sim, which, op, &sim.events()[pending..]);
        event_fields += assert_golden(&sim, which, op);
    }
    [
        source.actor_fields,
        source.draws,
        source.statistics_fields,
        event_fields,
        usize::try_from(c.v("tics")).unwrap() + 1,
    ]
}
