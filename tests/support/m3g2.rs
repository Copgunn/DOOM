//! Actual original-player source inputs and independent C/DSIM observations.
#![allow(dead_code)]
use doom_sim::{identity::*, *};
use doom_types::{DoomRng, Fixed};
use std::{collections::BTreeMap, sync::OnceLock};
pub const COMMANDS: [TicCommand; 4] = [TicCommand {
    forward: 0,
    side: 0,
    angle_turn: 0,
    buttons: 0,
}; 4];
#[derive(Clone)]
pub struct Case {
    pub label: String,
    pub tics: usize,
    pub skip: usize,
}
pub fn cases() -> Vec<Case> {
    let mut lines = include_str!("../reference/m3g2_cases.csv").lines();
    let columns: Vec<_> = lines.next().unwrap().split(',').collect();
    let t = columns.iter().position(|s| *s == "tics").unwrap();
    let skip = columns.iter().position(|s| *s == "skip").unwrap();
    lines
        .map(|line| {
            let v: Vec<_> = line.split(',').collect();
            Case {
                label: v[1].into(),
                tics: v[t].parse().unwrap(),
                skip: v[skip].parse().unwrap(),
            }
        })
        .collect()
}
pub fn map() -> doom_map::Map {
    let wad = doom_wad::Wad::parse(
        include_bytes!("../reference/m3f5_map_0.bin"),
        doom_wad::Limits::default(),
    )
    .unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
pub fn id(slot: u32) -> ActorId {
    ActorId::from_parts(slot, 1)
}
pub fn damage(sim: &mut Simulation, target: ActorId, source: Option<ActorId>, amount: i32) {
    sim.with_execution_context(|ctx| {
        let target = ctx.admit_local(target, sim_definition(target))?;
        let source = source
            .map(|source| ctx.admit_local(source, sim_definition(source)))
            .transpose()?;
        ctx.damage(target, None, source, amount)?;
        Ok(())
    })
    .unwrap();
}
fn sim_definition(id: ActorId) -> DefinitionIdentity {
    DefinitionIdentity::Original(if id == self::id(0) {
        OriginalActorType::MT_SERGEANT
    } else {
        OriginalActorType::MT_PLAYER
    })
}
pub fn setup(which: usize, c: &Case) -> Simulation {
    let mut sim = Simulation::original_player_map(map(), 64, MapContextOptions::default()).unwrap();
    for _ in 0..c.skip {
        sim.gameplay_random();
    }
    sim.spawn_original_actor(OriginalActorType::MT_SERGEANT)
        .unwrap();
    sim.with_execution_context(|ctx| {
        let body = ctx.spawn_player(PlayerSlot::One, [Fixed::from_map_units(96), Fixed::ZERO])?;
        assert_eq!(body.raw_id(), id(1));
        Ok(())
    })
    .unwrap();
    if which == 3 {
        damage(&mut sim, id(0), Some(id(1)), 150);
    } else {
        if [2, 12, 14].contains(&which) {
            damage(&mut sim, id(1), Some(id(0)), 1);
        }
        if (7..=10).contains(&which) {
            damage(
                &mut sim,
                id(1),
                Some(id(0)),
                [150, 151, 200, 201][which - 7],
            );
        }
        sim.mark_remove(id(0)).unwrap();
    }
    sim
}
pub fn operations(sim: &mut Simulation, which: usize, op: usize) {
    if op == 1 && [1, 4].contains(&which) {
        damage(sim, id(1), None, 1);
    }
    if op == 1 && which == 3 {
        damage(sim, id(1), Some(id(0)), 1);
    }
    if which == 5 {
        match op {
            1 => damage(sim, id(1), Some(id(1)), 1),
            2 => damage(sim, id(1), None, 0),
            3 => damage(sim, id(1), None, 10),
            _ => {}
        }
    }
    if which == 6 && op == 1 {
        damage(sim, id(1), None, 100);
    }
    if which == 11 && op == 1 {
        damage(sim, id(1), Some(id(1)), 100);
    }
    if (6..=11).contains(&which) && op == 2 {
        damage(sim, id(1), None, 1);
    }
    if which == 12 && op == 2 {
        sim.spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::from_map_units(-96), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    }
    if which == 13 && op == 2 {
        sim.mark_remove(id(1)).unwrap();
    }
    if which == 13 && op == 3 {
        for x in [-96, 96] {
            sim.spawn_type(
                ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
                [Fixed::from_map_units(x), Fixed::ZERO],
                SpawnHeight::Floor,
            )
            .unwrap();
        }
    }
}
type Rows = BTreeMap<(usize, usize), Vec<(String, Vec<i64>)>>;
pub fn rows(which: usize, op: usize) -> &'static [(String, Vec<i64>)] {
    static ROWS: OnceLock<Rows> = OnceLock::new();
    ROWS.get_or_init(|| {
        let mut all = Rows::new();
        let mut lines = include_str!("../reference/m3g2_trace.csv").lines();
        assert_eq!(lines.next(), Some("DOOM-PLAYER-BODY-v1"));
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
    pub counts: [usize; 5],
}
impl SourceTrace {
    fn birth(&self, id: Option<ActorId>) -> i64 {
        id.map_or(-1, |id| {
            self.births.iter().position(|b| *b == id).unwrap() as i64
        })
    }
    pub fn check(&mut self, sim: &Simulation, which: usize, op: usize, events: &[Event]) {
        for e in events {
            if let Event::Spawned(id) = e {
                self.births.push(*id);
            }
        }
        for (tag, c) in rows(which, op) {
            match tag.as_str() {
                "E" if c[0] == 8 => {
                    assert_eq!(i64::from(self.rng.p_random()), c[2]);
                    self.counts[3] += 1;
                }
                "P" => {
                    let p = sim.original_player(PlayerSlot::One).unwrap();
                    let fields = [
                        self.birth(p.mo),
                        i64::from(p.health),
                        i64::from(p.state == PlayerBodyState::Dead),
                        self.birth(p.attacker),
                        i64::from(p.damage_count),
                        i64::from(p.frags),
                    ];
                    assert_eq!(fields.as_slice(), c, "player case{which} op{op}");
                    self.counts[1] += 6;
                }
                "AP" => {
                    let actor = self.births[c[0] as usize];
                    let p = sim.original_player(PlayerSlot::One).unwrap();
                    assert_eq!(if p.mo == Some(actor) { 0 } else { -1 }, c[1]);
                    self.counts[2] += 1;
                }
                "LS" => {
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
                    self.counts[4] += 4;
                }
                "A" => {
                    let id = self.births[c[0] as usize];
                    if c[1] == 0 {
                        assert!(sim.actor(id).is_none());
                        continue;
                    }
                    let a = sim.actor(id).unwrap();
                    let l = a.life().unwrap();
                    let got = [
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
                        self.birth(l.target),
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
                    for (field, (actual, expected)) in got.iter().zip(&c[2..]).enumerate() {
                        assert_eq!(
                            actual, expected,
                            "case{which} op{op} birth{} field{field}",
                            c[0]
                        );
                        self.counts[0] += 1;
                    }
                }
                _ => {}
            }
        }
        assert_eq!(
            sim.rng().indices(),
            self.rng.indices(),
            "RNG case{which} op{op}"
        );
    }
}
fn word(bytes: &[u8], at: &mut usize) -> usize {
    let value = u32::from_le_bytes(bytes[*at..*at + 4].try_into().unwrap()) as usize;
    *at += 4;
    value
}
type EventFields = BTreeMap<(usize, usize), Vec<(String, Vec<u8>)>>;
pub fn assert_events(sim: &Simulation, which: usize, op: usize) -> usize {
    static FIELDS: OnceLock<EventFields> = OnceLock::new();
    let fields = FIELDS.get_or_init(|| {
        let bytes = include_bytes!("../reference/m3g2_events.bin");
        assert_eq!(&bytes[..4], b"EV22");
        let mut at = 4;
        let mut all = EventFields::new();
        while at < bytes.len() {
            let case = word(bytes, &mut at);
            let op = word(bytes, &mut at);
            let count = word(bytes, &mut at);
            let mut fields = Vec::new();
            for _ in 0..count {
                let n = word(bytes, &mut at);
                let path = std::str::from_utf8(&bytes[at..at + n]).unwrap().to_owned();
                at += n;
                let n = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()) as usize;
                at += 8;
                fields.push((path, bytes[at..at + n].to_vec()));
                at += n;
            }
            all.insert((case, op), fields);
        }
        all
    });
    let snapshot = sim.snapshot();
    let actual: Vec<_> = snapshot
        .fields()
        .filter(|f| f.path().starts_with("events."))
        .map(|f| (f.path().to_owned(), f.value().to_vec()))
        .collect();
    let expected = &fields[&(which, op)];
    assert_eq!(actual, *expected, "event case{which} op{op}");
    actual.len() - 2
}
pub fn assert_snapshot(sim: &Simulation, which: usize, op: usize) -> bool {
    let line = include_str!("../reference/m3g2_snapshots.csv")
        .lines()
        .skip(1)
        .find(|line| {
            let v: Vec<_> = line.split(',').collect();
            v[0].parse::<usize>().unwrap() == which && v[1].parse::<usize>().unwrap() == op
        });
    let Some(line) = line else {
        return false;
    };
    let v: Vec<_> = line.split(',').collect();
    let snapshot = sim.snapshot();
    let bytes = snapshot
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    assert_eq!(
        snapshot.digest(),
        u64::from_str_radix(v[3], 16).unwrap(),
        "digest case{which} op{op}"
    );
    if v[4] != "-" {
        let expected: &[u8] = match v[4] {
            "m3g2_state_0_0.bin" => include_bytes!("../reference/m3g2_state_0_0.bin"),
            "m3g2_state_6_72.bin" => include_bytes!("../reference/m3g2_state_6_72.bin"),
            "m3g2_state_10_52.bin" => include_bytes!("../reference/m3g2_state_10_52.bin"),
            "m3g2_state_13_3.bin" => include_bytes!("../reference/m3g2_state_13_3.bin"),
            _ => panic!("unknown declared fixture"),
        };
        assert_eq!(bytes, expected, "full fixture{which}/{op}");
    }
    let diagnostic = Diagnostic::decode_player(&bytes, DiagnosticLimits::default()).unwrap();
    assert_eq!(
        diagnostic.encode(DiagnosticLimits::default()).unwrap(),
        bytes
    );
    true
}
