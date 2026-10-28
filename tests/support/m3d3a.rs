//! Public source-C perception inputs and ordered evidence.
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
    include_str!("../reference/m3d3a_cases.csv")
        .lines()
        .skip(1)
        .map(|r| {
            let c = r.split(',').collect::<Vec<_>>();
            Case {
                label: c[1].into(),
                values: c[2..].iter().map(|v| v.parse().unwrap()).collect(),
            }
        })
        .collect()
}
pub fn map(index: usize) -> doom_map::Map {
    let bytes: &[u8] = match index {
        0 => include_bytes!("../reference/m3d3a_map_0.bin"),
        1 => include_bytes!("../reference/m3d3a_map_1.bin"),
        2 => include_bytes!("../reference/m3d3a_map_2.bin"),
        3 => include_bytes!("../reference/m3d3a_map_3.bin"),
        4 => include_bytes!("../reference/m3d3a_map_4.bin"),
        5 => include_bytes!("../reference/m3d3a_map_5.bin"),
        6 => include_bytes!("../reference/m3d3a_map_6.bin"),
        7 => include_bytes!("../reference/m3d3a_map_7.bin"),
        8 => include_bytes!("../reference/m3d3a_map_8.bin"),
        9 => include_bytes!("../reference/m3d3a_map_9.bin"),
        10 => include_bytes!("../reference/m3d3a_map_10.bin"),
        11 => include_bytes!("../reference/m3d3a_map_11.bin"),
        12 => include_bytes!("../reference/m3d3a_map_12.bin"),
        13 => include_bytes!("../reference/m3d3a_map_13.bin"),
        14 => include_bytes!("../reference/m3d3a_map_14.bin"),
        15 => include_bytes!("../reference/m3d3a_map_15.bin"),
        16 => include_bytes!("../reference/m3d3a_map_16.bin"),
        17 => include_bytes!("../reference/m3d3a_map_17.bin"),
        _ => panic!("fixture map"),
    };
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
pub fn registry(c: &Case) -> ActorRegistry {
    let r = ActorRegistry::vanilla();
    if c.n(0) < 2 {
        return r;
    }
    let mut states = r.states().to_vec();
    states[1] = ActorStateDef {
        name: "FIXTURE_LOOK".into(),
        sprite: 29,
        frame: 0,
        tics: c.n(15),
        action: ActorAction::Original(ActionId(43)),
        next: StateId(1),
        misc: [0, 0],
    };
    states[2] = ActorStateDef {
        name: "FIXTURE_SEE".into(),
        sprite: 29,
        frame: 1,
        tics: -1,
        action: ActorAction::None,
        next: StateId(0),
        misc: [0, 0],
    };
    let mut types = r.types().to_vec();
    for i in [19, 21, 136] {
        let mut t = types[0].clone();
        t.name = format!("FIXTURE_LOOKER_{i}");
        t.properties[0] = -1;
        t.properties[1] = 1;
        t.properties[3] = 2;
        t.properties[4] = c.n(8);
        t.properties[5] = 8;
        t.properties[17] = c.n(14);
        t.properties[21] = c.n(7);
        types[i] = t;
    }
    ActorRegistry::new(states, types).unwrap()
}
pub fn sight_setup(c: &Case) -> Simulation {
    setup(c)
}
pub fn setup(c: &Case) -> Simulation {
    let mut sim = Simulation::with_perception_registry(
        map(c.n(1) as usize),
        64,
        registry(c),
        ActorOptions::default(),
    )
    .unwrap();
    for _ in 0..c.n(9) {
        sim.gameplay_random();
    }
    sim.spawn_type(
        ActorTypeId(c.n(6) as u16),
        [c.f(2), c.f(3)],
        SpawnHeight::Absolute(c.f(4)),
    )
    .unwrap();
    for i in 0..4 {
        sim.spawn_type(
            ActorTypeId(0),
            [c.f(19 + i * 5), c.f(20 + i * 5)],
            SpawnHeight::Absolute(c.f(21 + i * 5)),
        )
        .unwrap();
    }
    configure(&mut sim, c);
    sim
}
pub fn configure(sim: &mut Simulation, c: &Case) {
    sim.configure_perception_observer(ActorId::from_parts(0, 1), c.n(5) as u32, c.n(16))
        .unwrap();
    for i in 0..4 {
        sim.set_perception_player(
            i,
            PerceptionPlayer {
                active: c.n(22 + i * 5) != 0,
                health: c.n(23 + i * 5),
                actor: Some(ActorId::from_parts(i as u32 + 1, 1)),
            },
        )
        .unwrap();
    }
}
pub fn flip(sim: &mut Simulation, c: &Case, op: usize) {
    if op == 2 && c.n(17) >= 0 {
        let slot = c.n(17) as usize;
        let mut p = sim.perception_players()[slot];
        p.health = c.n(18);
        sim.set_perception_player(slot, p).unwrap();
    }
}
pub type Row = (String, Vec<i64>);
pub fn rows(which: usize, op: usize) -> Vec<Row> {
    type Rows = std::collections::BTreeMap<(usize, usize), Vec<Row>>;
    static ROWS: std::sync::OnceLock<Rows> = std::sync::OnceLock::new();
    ROWS.get_or_init(|| {
        let mut out = Rows::new();
        for r in include_str!("../reference/m3d3a_trace.csv").lines() {
            let mut c = r.split(',');
            let tag = c.next().unwrap().to_string();
            let which = c.next().unwrap().parse().unwrap();
            let op = c.next().unwrap().parse().unwrap();
            out.entry((which, op))
                .or_default()
                .push((tag, c.map(|n| n.parse().unwrap()).collect()));
        }
        out
    })
    .get(&(which, op))
    .unwrap()
    .clone()
}
pub fn sight_visits(result: &SightResult) -> Vec<Vec<i64>> {
    result
        .visits
        .iter()
        .map(|v| match v {
            SightVisit::NodeStart { node, side } => vec![1, i64::from(*node), i64::from(*side)],
            SightVisit::NodeEnd { node, side } => vec![2, i64::from(*node), i64::from(*side)],
            SightVisit::Subsector(i) => vec![3, i64::from(*i)],
            SightVisit::Seg {
                subsector,
                seg,
                line,
                duplicate,
            } => vec![
                4,
                i64::from(*subsector),
                i64::from(*seg),
                i64::from(*line),
                i64::from(*duplicate),
            ],
            SightVisit::Sides { line, stage, sides } => vec![
                5,
                i64::from(*line),
                i64::from(*stage),
                i64::from(sides[0]),
                i64::from(sides[1]),
            ],
            SightVisit::Opening { line, heights } => vec![
                6,
                i64::from(*line),
                i64::from(heights[0].bits()),
                i64::from(heights[1].bits()),
            ],
            SightVisit::Clip {
                line,
                fraction,
                slopes,
            } => vec![
                7,
                i64::from(*line),
                i64::from(fraction.bits()),
                i64::from(slopes[0].bits()),
                i64::from(slopes[1].bits()),
            ],
            SightVisit::Blocked { line, reason } => vec![8, i64::from(*line), i64::from(*reason)],
        })
        .collect()
}
pub fn assert_sight(result: &SightResult, rows: &[Row], query: i64, observer: i64, target: i64) {
    let start = rows
        .iter()
        .find(|(tag, c)| tag == "V" && c[0] == query && c[1] == 0)
        .unwrap();
    assert_eq!(
        [
            observer,
            target,
            i64::from(result.reject_index),
            i64::from(result.rejected)
        ],
        start.1[2..]
    );
    let expected = rows
        .iter()
        .filter(|(tag, c)| tag == "V" && c[0] == query && c[1] != 0)
        .map(|(_, c)| c[1..].to_vec())
        .collect::<Vec<_>>();
    assert_eq!(
        sight_visits(result),
        expected,
        "sight candidate/decision order query {query}"
    );
    let end = rows
        .iter()
        .find(|(tag, c)| tag == "Q" && c[0] == query)
        .unwrap();
    assert_eq!(
        [
            i64::from(result.visible),
            result.eye.map_or(0, |v| i64::from(v.bits())),
            result.slopes.map_or(0, |s| i64::from(s[0].bits())),
            result.slopes.map_or(0, |s| i64::from(s[1].bits()))
        ],
        end.1[1..],
        "sight final interval query {query}"
    );
}
#[derive(Default)]
pub struct Trace {
    pub actor_fields: usize,
    pub events: usize,
    pub queries: usize,
    pub visits: usize,
    pub slots: usize,
    pub draws: usize,
    pub noise: usize,
    pub roster_fields: usize,
    pub sound_fields: usize,
    rng: doom_types::DoomRng,
}
fn ordinal(id: ActorId) -> i64 {
    assert_eq!(id.generation(), 1);
    i64::from(id.slot())
}
impl Trace {
    pub fn check(&mut self, sim: &Simulation, which: usize, op: usize, events: &[Event]) {
        let rows = rows(which, op);
        for (_, c) in rows.iter().filter(|(tag, c)| tag == "E" && c[0] == 8) {
            assert_eq!(
                i64::from(self.rng.p_random()),
                c[2],
                "case {which} op {op} RNG draw"
            );
            self.draws += 1;
        }
        assert_eq!(sim.rng().indices(), self.rng.indices());
        let mut out = Vec::<Row>::new();
        let mut query = 0_i64;
        for e in events {
            let value = match e {
                Event::GameplayRandom(v) => Some(("E", vec![9, -1, i64::from(*v)])),
                Event::Think(id) => Some(("E", vec![0, ordinal(*id), 0])),
                Event::RemovalMarked(id) => Some(("E", vec![2, ordinal(*id), 0])),
                Event::Retired(id) => Some(("E", vec![3, ordinal(*id), 0])),
                Event::ActorState(s) => match s {
                    ActorStateEvent::Initialized { actor, actor_type } => {
                        Some(("E", vec![1, ordinal(*actor), i64::from(actor_type.0)]))
                    }
                    ActorStateEvent::Entered { actor, state } => {
                        Some(("E", vec![4, ordinal(*actor), i64::from(state.0)]))
                    }
                    ActorStateEvent::Action {
                        actor,
                        action: ActorAction::Original(action),
                    } => Some(("E", vec![5, ordinal(*actor), i64::from(action.0)])),
                    ActorStateEvent::Target { actor, target } => {
                        Some(("E", vec![6, ordinal(*actor), target.map_or(-1, ordinal)]))
                    }
                    ActorStateEvent::Sound { actor, sound } => {
                        Some(("E", vec![7, actor.map_or(-1, ordinal), i64::from(*sound)]))
                    }
                    _ => None,
                },
                Event::Perception(p) => match p {
                    PerceptionEvent::LookStart { actor, threshold } => {
                        Some(("K", vec![0, ordinal(*actor), i64::from(*threshold)]))
                    }
                    PerceptionEvent::LookSound {
                        actor,
                        target,
                        shootable,
                        ambush,
                    } => Some((
                        "K",
                        vec![
                            1,
                            ordinal(*actor),
                            target.map_or(-1, ordinal),
                            i64::from(*shootable),
                            i64::from(*ambush),
                        ],
                    )),
                    PerceptionEvent::Wake { actor, state } => {
                        Some(("K", vec![8, ordinal(*actor), i64::from(state.0)]))
                    }
                    PerceptionEvent::Random { actor, value } => {
                        Some(("K", vec![9, ordinal(*actor), i64::from(*value)]))
                    }
                    PerceptionEvent::ScanStart {
                        actor,
                        allaround,
                        lastlook,
                        stop,
                    } => Some((
                        "K",
                        vec![
                            2,
                            ordinal(*actor),
                            i64::from(*allaround),
                            i64::from(*lastlook),
                            i64::from(*stop),
                        ],
                    )),
                    PerceptionEvent::Slot {
                        actor,
                        slot,
                        active,
                        count,
                    } => {
                        self.slots += 1;
                        Some((
                            "K",
                            vec![
                                3,
                                ordinal(*actor),
                                i64::from(*slot),
                                i64::from(*active),
                                i64::from(*count),
                            ],
                        ))
                    }
                    PerceptionEvent::Health {
                        actor,
                        slot,
                        health,
                    } => Some((
                        "K",
                        vec![4, ordinal(*actor), i64::from(*slot), i64::from(*health)],
                    )),
                    PerceptionEvent::Facing {
                        actor,
                        slot,
                        angle,
                        distance,
                        accepted,
                    } => Some((
                        "K",
                        vec![
                            5,
                            ordinal(*actor),
                            i64::from(*slot),
                            i64::from(*angle),
                            i64::from(distance.is_some()),
                            distance.map_or(0, |d| i64::from(d.bits())),
                            i64::from(*accepted),
                        ],
                    )),
                    PerceptionEvent::ScanEnd {
                        actor,
                        found,
                        lastlook,
                    } => Some((
                        "K",
                        vec![7, ordinal(*actor), i64::from(*found), i64::from(*lastlook)],
                    )),
                    PerceptionEvent::EmptyRoster(_) => {
                        panic!("nonterminating original roster is a separate safety test")
                    }
                    PerceptionEvent::Sight {
                        observer,
                        target,
                        result: r,
                    } => {
                        assert_sight(r, &rows, query, ordinal(*observer), ordinal(*target));
                        out.push((
                            "V".into(),
                            vec![
                                query,
                                0,
                                ordinal(*observer),
                                ordinal(*target),
                                i64::from(r.reject_index),
                                i64::from(r.rejected),
                            ],
                        ));
                        for v in sight_visits(r) {
                            let mut c = vec![query];
                            c.extend(v);
                            out.push(("V".into(), c));
                        }
                        out.push((
                            "Q".into(),
                            vec![
                                query,
                                i64::from(r.visible),
                                r.eye.map_or(0, |v| i64::from(v.bits())),
                                r.slopes.map_or(0, |s| i64::from(s[0].bits())),
                                r.slopes.map_or(0, |s| i64::from(s[1].bits())),
                            ],
                        ));
                        query += 1;
                        self.queries += 1;
                        self.visits += r.visits.len();
                        None
                    }
                },
                Event::Weapon(WeaponEvent::Noise {
                    sector,
                    blocks,
                    target,
                }) => {
                    self.noise += 1;
                    Some((
                        "N",
                        vec![i64::from(*sector), i64::from(*blocks), ordinal(*target)],
                    ))
                }
                _ => None,
            };
            if let Some((tag, v)) = value {
                out.push((tag.into(), v));
            }
        }
        let expected = rows
            .iter()
            .filter(|(tag, c)| {
                matches!(tag.as_str(), "E" | "K" | "V" | "Q" | "N") && !(tag == "E" && c[0] == 8)
            })
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(out, expected, "case {which} op {op} ordered decisions");
        self.events += out.len();
        for (_, c) in rows.iter().filter(|(tag, _)| tag == "A") {
            let id = ActorId::from_parts(c[0] as u32, 1);
            let a = sim.actor(id);
            assert_eq!(a.is_some(), c[1] != 0);
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
                    l.target.map_or(-1, ordinal),
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
        for (_, c) in rows.iter().filter(|(tag, _)| tag == "R") {
            let p = sim.perception_players()[c[0] as usize];
            assert_eq!(
                [
                    i64::from(p.active),
                    i64::from(p.health),
                    p.actor.map_or(-1, ordinal)
                ],
                c[1..],
                "case {which} op {op} player record"
            );
            self.roster_fields += 3;
        }
        for (_, c) in rows.iter().filter(|(tag, _)| tag == "S") {
            let p = sim.snapshot();
            let field = p
                .fields()
                .find(|f| f.path() == format!("sector.{}.soundtarget", c[0]))
                .unwrap();
            let mut expected = vec![0; 13];
            if c[1] >= 0 {
                expected[0] = 1;
                expected[1..5].copy_from_slice(&(c[1] as u32).to_le_bytes());
                expected[5..13].copy_from_slice(&1_u64.to_le_bytes());
            }
            assert_eq!(
                field.value(),
                expected,
                "case {which} op {op} sector sound target"
            );
            self.sound_fields += 1;
        }
    }
}
