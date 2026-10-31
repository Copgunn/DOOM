//! Synthetic C inputs and generation-aware projectile trace comparisons.
#![allow(dead_code)]
use doom_sim::*;
use doom_types::Fixed;
#[derive(Clone, Debug)]
pub struct Case {
    pub label: String,
    pub values: Vec<i64>,
}
impl Case {
    pub fn n(&self, i: usize) -> i32 {
        self.values[i] as i32
    }
    pub fn f(&self, i: usize) -> Fixed {
        Fixed::from_bits(self.n(i))
    }
}
pub fn cases() -> Vec<Case> {
    include_str!("../reference/m3d2b_cases.csv")
        .lines()
        .skip(1)
        .map(|r| {
            let c = r.split(',').collect::<Vec<_>>();
            Case {
                label: c[1].into(),
                values: c[2..].iter().map(|s| s.parse().unwrap()).collect(),
            }
        })
        .collect()
}
pub fn registry(c: &Case) -> ActorRegistry {
    let r = ActorRegistry::vanilla();
    let mut types = r.types().to_vec();
    for i in [15, 17, 136] {
        let mut t = types[0].clone();
        t.name = format!("FIXTURE_PAWN_{i}");
        t.properties[0] = -1;
        t.properties[2] = c.n(10);
        t.properties[8] = c.n(11);
        t.properties[18] = c.n(12);
        t.properties[21] = if i == c.n(9) as usize { c.n(13) } else { 6 };
        t.properties[5] = 8;
        types[i] = t;
    }
    ActorRegistry::new(r.states().to_vec(), types).unwrap()
}
pub fn map(index: usize) -> doom_map::Map {
    let bytes: &[u8] = match index {
        0 => include_bytes!("../reference/m3d2b_map_0.bin"),
        1 => include_bytes!("../reference/m3d2b_map_1.bin"),
        2 => include_bytes!("../reference/m3d2b_map_2.bin"),
        3 => include_bytes!("../reference/m3d2b_map_3.bin"),
        4 => include_bytes!("../reference/m3d2b_map_4.bin"),
        5 => include_bytes!("../reference/m3d2b_map_5.bin"),
        6 => include_bytes!("../reference/m3d2b_map_6.bin"),
        7 => include_bytes!("../reference/m3d2b_map_7.bin"),
        8 => include_bytes!("../reference/m3d2b_map_8.bin"),
        9 => include_bytes!("../reference/m3d2b_map_9.bin"),
        10 => include_bytes!("../reference/m3d2b_map_10.bin"),
        11 => include_bytes!("../reference/m3d2b_map_11.bin"),
        12 => include_bytes!("../reference/m3d2b_map_12.bin"),
        13 => include_bytes!("../reference/m3d2b_map_13.bin"),
        _ => panic!("fixture map"),
    };
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
pub fn setup(c: &Case) -> Simulation {
    let mut sim = Simulation::with_projectile_registry(
        map(c.n(1) as usize),
        64,
        registry(c),
        ActorOptions::default(),
    )
    .unwrap();
    sim.spawn_type(
        ActorTypeId(c.n(8) as u16),
        [c.f(2), c.f(3)],
        SpawnHeight::Absolute(c.f(4)),
    )
    .unwrap();
    sim.spawn_type(
        ActorTypeId(c.n(9) as u16),
        [c.f(5), c.f(6)],
        SpawnHeight::Absolute(c.f(7)),
    )
    .unwrap();
    if c.n(14) != 0 {
        sim.spawn_type(
            ActorTypeId(c.n(9) as u16),
            [c.f(5), c.f(6)],
            SpawnHeight::Absolute(c.f(7)),
        )
        .unwrap();
    }
    sim
}
pub fn request(c: &Case) -> ProjectileRequest {
    ProjectileRequest {
        source: ActorId::from_parts(0, 1),
        actor_type: ActorTypeId(34),
        aim: if c.n(0) == 1 {
            ProjectileAim::PlayerAutoAim
        } else {
            ProjectileAim::Actor(ActorId::from_parts(1, 1))
        },
    }
}
pub fn launch(sim: &mut Simulation, c: &Case) {
    for _ in 0..c.n(15) {
        sim.launch_projectile(request(c)).unwrap();
    }
}
pub fn rows(which: usize, op: usize) -> Vec<(String, Vec<i64>)> {
    type SourceRow = (usize, usize, String, Vec<i64>);
    static ROWS: std::sync::OnceLock<Vec<SourceRow>> = std::sync::OnceLock::new();
    ROWS.get_or_init(|| {
        doom_test_evidence::text(
            "tests/reference/m3d2b_trace.csv",
            include_bytes!("../reference/m3d2b_trace.csv.zst"),
            include_str!("../reference/zstd_manifest.json"),
        )
        .expect("verified historical zstd evidence")
        .lines()
        .map(|r| {
            let mut c = r.split(',');
            let tag = c.next().unwrap().to_owned();
            let which = c.next().unwrap().parse().unwrap();
            let op = c.next().unwrap().parse().unwrap();
            (which, op, tag, c.map(|s| s.parse().unwrap()).collect())
        })
        .collect()
    })
    .iter()
    .filter(|(w, o, _, _)| *w == which && *o == op)
    .map(|(_, _, tag, c)| (tag.clone(), c.clone()))
    .collect()
}
#[derive(Default)]
pub struct Trace {
    pub ids: Vec<ActorId>,
    pub actor_fields: usize,
    pub events: usize,
    pub moves: usize,
    pub candidates: usize,
    pub openings: usize,
    pub draws: usize,
    rng: doom_types::DoomRng,
}
impl Trace {
    fn ordinal(&self, id: ActorId) -> i64 {
        self.ids
            .iter()
            .position(|a| *a == id)
            .expect("generation-aware C allocation") as i64
    }
    pub fn check(&mut self, sim: &Simulation, which: usize, op: usize, events: &[Event]) {
        let rows = rows(which, op);
        for (_, c) in rows.iter().filter(|(tag, c)| tag == "E" && c[0] == 8) {
            assert_eq!(
                i64::from(self.rng.p_random()),
                c[2],
                "case {which} op {op} source RNG draw"
            );
            self.draws += 1;
        }
        assert_eq!(
            self.rng.indices(),
            sim.rng().indices(),
            "case {which} op {op} RNG index"
        );
        let mut observed = Vec::<(String, Vec<i64>)>::new();
        let mut attempt = 0;
        for e in events {
            if let Event::ActorState(ActorStateEvent::Initialized { actor, .. }) = e {
                self.ids.push(*actor);
            }
            let ord = |id| ord_id(&self.ids, id);
            let value = match e {
                Event::Combat(CombatEvent::Damage(r)) => Some((
                    "D",
                    vec![
                        ord(r.target),
                        r.inflictor.map_or(-1, ord),
                        r.source.map_or(-1, ord),
                        i64::from(r.amount),
                    ],
                )),
                Event::Think(a) => Some(("E", vec![0, ord(*a), 0])),
                Event::RemovalMarked(a) => Some(("E", vec![2, ord(*a), 0])),
                Event::Retired(a) => Some(("E", vec![3, ord(*a), 0])),
                Event::ActorState(a) => match a {
                    ActorStateEvent::Initialized { actor, actor_type } => {
                        Some(("E", vec![1, ord(*actor), i64::from(actor_type.0)]))
                    }
                    ActorStateEvent::Entered { actor, state } => {
                        Some(("E", vec![4, ord(*actor), i64::from(state.0)]))
                    }
                    ActorStateEvent::Action {
                        actor,
                        action: ActorAction::Original(action),
                    } => Some(("E", vec![5, ord(*actor), i64::from(action.0)])),
                    ActorStateEvent::Target { actor, target } => {
                        Some(("E", vec![6, ord(*actor), target.map_or(-1, ord)]))
                    }
                    ActorStateEvent::Sound { actor, sound } => {
                        Some(("E", vec![7, actor.map_or(-1, ord), i64::from(*sound)]))
                    }
                    _ => None,
                },

                Event::Projectile(p) => Some((
                    "P",
                    match p {
                        ProjectileEvent::Exploded(a) => vec![0, ord(*a)],
                        ProjectileEvent::Velocity {
                            actor,
                            angle,
                            momentum,
                        } => {
                            let mut v = vec![1, ord(*actor), i64::from(*angle)];
                            v.extend(momentum.map(|m| i64::from(m.bits())));
                            v
                        }
                        ProjectileEvent::Random { kind, value } => {
                            vec![2, i64::from(*kind), i64::from(*value)]
                        }
                        ProjectileEvent::HalfStep {
                            actor,
                            position,
                            tics,
                        } => {
                            let mut v = vec![4, ord(*actor)];
                            v.extend(position.map(|p| i64::from(p.bits())));
                            v.push(i64::from(*tics));
                            v
                        }
                        ProjectileEvent::Launch(r) => {
                            let (mode, dest) = match r.aim {
                                ProjectileAim::Actor(id) => (0, ord(id)),
                                ProjectileAim::PlayerAutoAim => (1, -1),
                            };
                            vec![5, ord(r.source), mode, dest, i64::from(r.actor_type.0)]
                        }
                    },
                )),
                Event::SectorMovement { actor, attempt: a } => {
                    let n = attempt;
                    attempt += 1;
                    let expected = rows
                        .iter()
                        .filter(|(tag, c)| tag == "C" && c[0] == n)
                        .map(|(_, c)| c[1..].to_vec())
                        .collect::<Vec<_>>();
                    let got = a
                        .movement
                        .actors
                        .iter()
                        .map(|id| vec![0, ord(*id)])
                        .chain(
                            a.movement
                                .lines
                                .candidates
                                .iter()
                                .map(|i| vec![1, i64::from(*i)]),
                        )
                        .collect::<Vec<_>>();
                    assert_eq!(
                        got, expected,
                        "case {which} op {op} attempt {n} candidate order"
                    );
                    self.candidates += got.len();
                    let expected = rows
                        .iter()
                        .filter(|(tag, c)| tag == "O" && c[0] == n)
                        .map(|(_, c)| c[1..].to_vec())
                        .collect::<Vec<_>>();
                    let got = a
                        .openings
                        .iter()
                        .map(|o| {
                            vec![
                                i64::from(o.line),
                                i64::from(o.opening.top.bits()),
                                i64::from(o.opening.bottom.bits()),
                                i64::from(o.opening.range.bits()),
                                i64::from(o.opening.low_floor.bits()),
                            ]
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(got, expected, "case {which} op {op} openings");
                    self.openings += got.len();
                    self.moves += 1;
                    Some((
                        "M",
                        vec![
                            n,
                            ord(*actor),
                            i64::from(a.movement.target[0].bits()),
                            i64::from(a.movement.target[1].bits()),
                            i64::from(a.allowed()),
                            i64::from(a.floor.bits()),
                            i64::from(a.ceiling.bits()),
                            i64::from(a.dropoff.bits()),
                            i64::from(a.float_ok),
                            a.failure as i64,
                            i64::from(a.location.subsector),
                            i64::from(a.location.sector),
                            a.movement.lines.blocked_by.map_or(-1, i64::from),
                            a.movement.blocked_by_actor.map_or(-1, ord),
                        ],
                    ))
                }
                Event::Vertical { actor, update: u } => {
                    let mut v = vec![ord(*actor)];
                    v.extend(
                        u.before
                            .into_iter()
                            .chain(u.after)
                            .chain(u.constraints)
                            .map(|v| i64::from(v.bits())),
                    );
                    v.extend([
                        i64::from(u.floor_contact),
                        i64::from(u.ceiling_contact),
                        i64::from(u.gravity_applied),
                    ]);
                    Some(("Z", v))
                }
                _ => None,
            };
            if let Some((tag, v)) = value {
                observed.push((tag.into(), v));
            }
        }
        let expected = rows
            .iter()
            .filter(|(tag, c)| {
                matches!(tag.as_str(), "E" | "P" | "M" | "Z" | "D") && !(tag == "E" && c[0] == 8)
            })
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(observed, expected, "case {which} op {op} event order");
        self.events += observed.len();
        for (_, c) in rows.iter().filter(|(tag, _)| tag == "A") {
            let id = self.ids[c[0] as usize];
            let a = sim.actor(id);
            assert_eq!(
                a.is_some(),
                c[1] != 0,
                "case {which} op {op} allocation {}",
                c[0]
            );
            if let Some(a) = a {
                let l = a.life().unwrap();
                let p = a.position();
                let m = a.momentum();
                let h = a.constraints();
                let got = vec![
                    i64::from(l.actor_type.0),
                    i64::from(p[0].bits()),
                    i64::from(p[1].bits()),
                    i64::from(a.z().bits()),
                    i64::from(m[0].bits()),
                    i64::from(m[1].bits()),
                    i64::from(a.vertical_momentum().bits()),
                    i64::from(l.health),
                    l.state.map_or(-1, |s| i64::from(s.0)),
                    i64::from(l.tics),
                    i64::from(l.flags),
                    i64::from(a.height().bits()),
                    i64::from(a.radius().bits()),
                    i64::from(l.reaction_time),
                    i64::from(l.threshold),
                    l.target.map_or(-1, |id| ord_id(&self.ids, id)),
                    i64::from(a.lastlook()),
                    i64::from(h[0].bits()),
                    i64::from(h[1].bits()),
                    i64::from(a.location().unwrap().subsector),
                    i64::from(sim.rng().indices()[0]),
                    i64::from(a.angle()),
                    i64::from(l.sprite),
                    i64::from(l.frame),
                ];
                assert_eq!(got, c[2..], "case {which} op {op} actor {}", c[0]);
                self.actor_fields += got.len();
            }
        }
    }
}
fn ord_id(ids: &[ActorId], id: ActorId) -> i64 {
    ids.iter().position(|a| *a == id).unwrap() as i64
}

pub fn assert_golden(sim: &Simulation, which: usize, tic: usize) {
    let row = include_str!("../reference/m3d2b_snapshots.csv")
        .lines()
        .skip(1)
        .find(|r| {
            let c = r.split(',').collect::<Vec<_>>();
            c[0].parse::<usize>().unwrap() == which && c[1].parse::<usize>().unwrap() == tic
        })
        .unwrap();
    let c = row.split(',').collect::<Vec<_>>();
    let snapshot = sim.snapshot();
    if c[4] != "-" {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/reference")
            .join(c[4]);
        let bytes = std::fs::read(path).unwrap();
        let count = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
        assert_eq!(
            snapshot.fields().count(),
            count,
            "case {which} tic {tic} field count"
        );
        let mut offset = 16;
        for f in snapshot.fields() {
            let size = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            let name = std::str::from_utf8(&bytes[offset..offset + size]).unwrap();
            offset += size;
            let size = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
            offset += 8;
            assert_eq!(f.path(), name, "case {which} tic {tic}");
            assert_eq!(
                f.value(),
                &bytes[offset..offset + size],
                "case {which} tic {tic} {name}"
            );
            offset += size;
        }
        assert_eq!(snapshot.canonical_bytes(), bytes);
    }
    assert_eq!(
        format!("{:016x}", snapshot.digest()),
        c[3],
        "case {which} tic {tic}"
    );
}
pub fn has_golden(which: usize, tic: usize) -> bool {
    include_str!("../reference/m3d2b_snapshots.csv")
        .lines()
        .skip(1)
        .any(|r| {
            let c = r.split(',').collect::<Vec<_>>();
            c[0].parse::<usize>().unwrap() == which && c[1].parse::<usize>().unwrap() == tic
        })
}
