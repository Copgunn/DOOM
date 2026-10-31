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
    pub fn v(&self, name: &str) -> i32 {
        let i = include_str!("../reference/m3d3b_cases.csv")
            .lines()
            .next()
            .unwrap()
            .split(',')
            .position(|k| k == name)
            .unwrap()
            - 2;
        self.n(i)
    }
    pub fn n(&self, i: usize) -> i32 {
        self.values[i] as i32
    }
    pub fn f(&self, i: usize) -> Fixed {
        Fixed::from_bits(self.n(i))
    }
}
pub fn cases() -> Vec<Case> {
    include_str!("../reference/m3d3b_cases.csv")
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
        0 => include_bytes!("../reference/m3d3b_map_0.bin"),
        1 => include_bytes!("../reference/m3d3b_map_1.bin"),
        2 => include_bytes!("../reference/m3d3b_map_2.bin"),
        3 => include_bytes!("../reference/m3d3b_map_3.bin"),
        4 => include_bytes!("../reference/m3d3b_map_4.bin"),
        5 => include_bytes!("../reference/m3d3b_map_5.bin"),
        6 => include_bytes!("../reference/m3d3b_map_6.bin"),
        7 => include_bytes!("../reference/m3d3b_map_7.bin"),
        8 => include_bytes!("../reference/m3d3b_map_8.bin"),
        9 => include_bytes!("../reference/m3d3b_map_9.bin"),
        10 => include_bytes!("../reference/m3d3b_map_10.bin"),
        11 => include_bytes!("../reference/m3d3b_map_11.bin"),
        12 => include_bytes!("../reference/m3d3b_map_12.bin"),
        13 => include_bytes!("../reference/m3d3b_map_13.bin"),
        14 => include_bytes!("../reference/m3d3b_map_14.bin"),
        15 => include_bytes!("../reference/m3d3b_map_15.bin"),
        16 => include_bytes!("../reference/m3d3b_map_16.bin"),
        _ => panic!("fixture map"),
    };
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
pub fn registry(c: &Case) -> ActorRegistry {
    let r = ActorRegistry::vanilla();
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
        name: "FIXTURE_CHASE".into(),
        sprite: 29,
        frame: 1,
        tics: c.v("see"),
        action: ActorAction::Original(ActionId(15)),
        next: StateId(2),
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
        t.properties[11] = 0;
        t.properties[15] = c.v("speed");
        t.properties[16] = c.v("radius");
        t.properties[20] = c.v("active_sound");
        types[i] = t;
    }
    ActorRegistry::new(states, types).unwrap()
}
pub fn sight_setup(c: &Case) -> Simulation {
    setup(c)
}
pub fn setup(c: &Case) -> Simulation {
    let mut sim = Simulation::with_pursuit_registry(
        map(c.n(1) as usize),
        64,
        registry(c),
        ActorOptions {
            skill: match c.v("skill") {
                0 => Skill::Baby,
                1 => Skill::Easy,
                2 => Skill::Medium,
                3 => Skill::Hard,
                4 => Skill::Nightmare,
                _ => panic!("fixture skill"),
            },
            ..ActorOptions::default()
        },
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
    .unwrap_or_else(|e| panic!("spawn observer {}: {e}", c.label));
    for i in 0..4 {
        sim.spawn_type(
            ActorTypeId(0),
            [c.f(19 + i * 5), c.f(20 + i * 5)],
            SpawnHeight::Absolute(c.f(21 + i * 5)),
        )
        .unwrap();
    }
    configure(&mut sim, c);
    if c.v("other") != 0 {
        sim.spawn_type(
            ActorTypeId(136),
            [Fixed::from_map_units(-64), Fixed::from_map_units(32)],
            SpawnHeight::Floor,
        )
        .unwrap();
    }
    if c.v("enter") != 0 {
        sim.enter_actor_state(ActorId::from_parts(0, 1), StateId(c.v("enter") as u16))
            .unwrap();
    }
    sim
}
pub fn configure(sim: &mut Simulation, c: &Case) {
    if c.n(0) != 6 {
        let id = ActorId::from_parts(0, 1);
        let mut setup = PursuitSetup::from_actor(sim.actor(id).unwrap()).unwrap();
        setup.movedir = c.v("direction") as u8;
        setup.movecount = c.v("count");
        setup.reaction_time = c.v("reaction");
        setup.threshold = c.n(16);
        setup.angle = c.n(5) as u32;
        sim.configure_pursuit_actor(id, setup).unwrap();
        sim.set_momentum(
            id,
            [Fixed::from_bits(c.v("mx")), Fixed::from_bits(c.v("my"))],
        )
        .unwrap();
        sim.set_vertical_momentum(id, Fixed::from_bits(c.v("mz")))
            .unwrap();
        let target = ActorId::from_parts(1, 1);
        let mut setup = PursuitSetup::from_actor(sim.actor(target).unwrap()).unwrap();
        setup.health = c.v("target_health");
        setup.flags = c.v("target_flags") as u32;
        sim.configure_pursuit_actor(target, setup).unwrap();
        if c.v("target") >= 0 {
            sim.set_actor_target(id, Some(ActorId::from_parts(c.v("target") as u32, 1)))
                .unwrap();
        }
    }
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
    if op != c.v("flip_tick") as usize {
        return;
    }
    if c.n(17) >= 0 {
        let slot = c.n(17) as usize;
        let mut p = sim.perception_players()[slot];
        p.health = c.n(18);
        sim.set_perception_player(slot, p).unwrap();
    }
    if c.v("flip_flags") >= 0 || c.v("flip_actorhealth") != -999 {
        let id = ActorId::from_parts(1, 1);
        let mut setup = PursuitSetup::from_actor(sim.actor(id).unwrap()).unwrap();
        if c.v("flip_flags") >= 0 {
            setup.flags = c.v("flip_flags") as u32;
        }
        if c.v("flip_actorhealth") != -999 {
            setup.health = c.v("flip_actorhealth");
        }
        sim.configure_pursuit_actor(id, setup).unwrap();
    }
    if c.v("flip_target") >= -1 {
        let n = c.v("flip_target");
        sim.set_actor_target(
            ActorId::from_parts(0, 1),
            if n < 0 {
                None
            } else {
                Some(ActorId::from_parts(n as u32, 1))
            },
        )
        .unwrap();
    }
}
/// Execute one source operation; captures contain all setup/pending observations.
pub fn advance(sim: &mut Simulation, c: &Case, op: usize) {
    flip(sim, c, op);
    let id = ActorId::from_parts(0, 1);
    if op == 1 && c.n(11) >= 0 {
        sim.noise_alert(
            ActorId::from_parts(c.n(11) as u32, 1),
            ActorId::from_parts(c.n(12) as u32, 1),
        )
        .unwrap();
    }
    match c.n(0) {
        0 => {
            sim.move_pursuer(id).unwrap();
        }
        1 => {
            sim.try_walk(id).unwrap();
        }
        2 => {
            sim.new_chase_direction(id).unwrap();
        }
        3 => {
            sim.chase(id).unwrap();
        }
        4 => {
            sim.step([TicCommand::default(); 4]).unwrap();
        }
        6 => {}
        _ => panic!("fixture operation"),
    }
}
const ACTOR_FIELDS: [&str; 26] = [
    "type",
    "x",
    "y",
    "z",
    "momx",
    "momy",
    "momz",
    "health",
    "state",
    "tics",
    "flags",
    "height",
    "radius",
    "reaction_time",
    "threshold",
    "target",
    "lastlook",
    "floor",
    "ceiling",
    "subsector",
    "rng.gameplay",
    "angle",
    "sprite",
    "frame",
    "movedir",
    "movecount",
];
pub type Row = (String, Vec<i64>);
pub fn rows(which: usize, op: usize) -> Vec<Row> {
    type Rows = std::collections::BTreeMap<(usize, usize), Vec<Row>>;
    static ROWS: std::sync::OnceLock<Rows> = std::sync::OnceLock::new();
    ROWS.get_or_init(|| {
        let mut out = Rows::new();
        for r in doom_test_evidence::text(
            "tests/reference/m3d3b_trace.csv",
            include_bytes!("../reference/m3d3b_trace.csv.zst"),
            include_str!("../reference/zstd_manifest.json"),
        )
        .expect("verified historical zstd evidence")
        .lines()
        {
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
pub fn assert_sight(
    result: &SightResult,
    rows: &[Row],
    query: i64,
    observer: i64,
    target: i64,
    context: (usize, usize),
) {
    let (which, op) = context;
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
        start.1[2..],
        "case {which} op {op} sight {query} REJECT"
    );
    let expected = rows
        .iter()
        .filter(|(tag, c)| tag == "V" && c[0] == query && c[1] != 0)
        .map(|(_, c)| c[1..].to_vec())
        .collect::<Vec<_>>();
    let actual = sight_visits(result);
    assert_eq!(
        actual.len(),
        expected.len(),
        "case {which} op {op} sight {query} visit count"
    );
    for (index, (got, want)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(
            got, want,
            "case {which} op {op} sight {query} visit {index}"
        );
    }
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
        "case {which} op {op} sight {query} final interval"
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
    pub moves: usize,
    pub candidates: usize,
    pub directions: usize,
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
        let mut attempt_index = 0_i64;
        for e in events {
            let value = match e {
                Event::Pursuit(e) => Some((
                    "H",
                    match e {
                        PursuitEvent::Start { actor, operation } => {
                            vec![0, ordinal(*actor), *operation as i64]
                        }
                        PursuitEvent::Return {
                            actor,
                            operation,
                            result,
                        } => vec![1, ordinal(*actor), *operation as i64, i64::from(*result)],
                        PursuitEvent::Direction { actor, direction } => {
                            self.directions += 1;
                            vec![2, ordinal(*actor), i64::from(*direction)]
                        }
                        PursuitEvent::Random { actor, kind, value } => {
                            vec![3, ordinal(*actor), i64::from(*kind), i64::from(*value)]
                        }
                    },
                )),
                Event::SectorMovement { actor, attempt: a } => {
                    for id in &a.movement.actors {
                        out.push(("C".into(), vec![attempt_index, 0, ordinal(*id)]));
                        self.candidates += 1;
                    }
                    for line in &a.movement.lines.candidates {
                        out.push(("C".into(), vec![attempt_index, 1, i64::from(*line)]));
                        self.candidates += 1;
                        for o in a.openings.iter().filter(|o| o.line == *line) {
                            out.push((
                                "O".into(),
                                vec![
                                    attempt_index,
                                    i64::from(o.line),
                                    i64::from(o.opening.top.bits()),
                                    i64::from(o.opening.bottom.bits()),
                                    i64::from(o.opening.range.bits()),
                                    i64::from(o.opening.low_floor.bits()),
                                ],
                            ));
                        }
                    }
                    out.push((
                        "M".into(),
                        vec![
                            attempt_index,
                            ordinal(*actor),
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
                            a.movement.blocked_by_actor.map_or(-1, ordinal),
                        ],
                    ));
                    attempt_index += 1;
                    self.moves += 1;
                    None
                }
                Event::Vertical { actor, update: u } => Some((
                    "Z",
                    std::iter::once(ordinal(*actor))
                        .chain(
                            u.before
                                .into_iter()
                                .chain(u.after)
                                .chain(u.constraints)
                                .map(|x| i64::from(x.bits())),
                        )
                        .chain([
                            i64::from(u.floor_contact),
                            i64::from(u.ceiling_contact),
                            i64::from(u.gravity_applied),
                        ])
                        .collect(),
                )),
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
                        assert_sight(
                            r,
                            &rows,
                            query,
                            ordinal(*observer),
                            ordinal(*target),
                            (which, op),
                        );
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
                matches!(
                    tag.as_str(),
                    "E" | "K" | "V" | "Q" | "N" | "H" | "C" | "O" | "M" | "Z"
                ) && !(tag == "E" && c[0] == 8)
            })
            .cloned()
            .collect::<Vec<_>>();
        for (i, (got, want)) in out.iter().zip(&expected).enumerate() {
            assert_eq!(got, want, "case {which} op {op} observation {i}");
        }
        assert_eq!(
            out.len(),
            expected.len(),
            "case {which} op {op} observation count"
        );
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
                    i64::from(l.movedir),
                    i64::from(l.movecount),
                ];
                for (field, (got, want)) in got.iter().zip(&c[2..]).enumerate() {
                    assert_eq!(
                        got, want,
                        "case {which} op {op} actor {} field {}",
                        c[0], ACTOR_FIELDS[field]
                    );
                }
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
pub fn assert_golden(sim: &Simulation, which: usize, op: usize) {
    let row = include_str!("../reference/m3d3b_snapshots.csv")
        .lines()
        .skip(1)
        .find(|r| {
            let c = r.split(',').collect::<Vec<_>>();
            c[0].parse::<usize>().unwrap() == which && c[1].parse::<usize>().unwrap() == op
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
            "case {which} op {op} field count"
        );
        let mut offset = 16;
        for f in snapshot.fields() {
            let size = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            let name = std::str::from_utf8(&bytes[offset..offset + size]).unwrap();
            offset += size;
            let size = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
            offset += 8;
            assert_eq!(f.path(), name, "case {which} op {op}");
            assert_eq!(
                f.value(),
                &bytes[offset..offset + size],
                "case {which} op {op} {name}"
            );
            offset += size;
        }
        assert_eq!(snapshot.canonical_bytes(), bytes);
    }
    assert_eq!(
        format!("{:016x}", snapshot.digest()),
        c[3],
        "case {which} op {op}"
    );
}
pub fn has_golden(which: usize, op: usize) -> bool {
    include_str!("../reference/m3d3b_snapshots.csv")
        .lines()
        .skip(1)
        .any(|r| {
            let c = r.split(',').collect::<Vec<_>>();
            c[0].parse::<usize>().unwrap() == which && c[1].parse::<usize>().unwrap() == op
        })
}
