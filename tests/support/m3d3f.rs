//! Public source-C perception inputs and ordered evidence.
#![allow(dead_code)]
#[path = "m3d3b.rs"]
mod sight_support;
use doom_sim::*;
use doom_types::Fixed;
#[derive(Clone, Debug)]
pub struct Case {
    pub label: String,
    pub values: Vec<i64>,
}
impl Case {
    pub fn v(&self, name: &str) -> i32 {
        let i = include_str!("../reference/m3d3f_cases.csv")
            .lines()
            .next()
            .unwrap()
            .split(',')
            .position(|k| k == name)
            .unwrap()
            - 2;
        self.n(i)
    }
    pub fn set(&mut self, name: &str, value: i32) {
        let i = include_str!("../reference/m3d3f_cases.csv")
            .lines()
            .next()
            .unwrap()
            .split(',')
            .position(|k| k == name)
            .unwrap()
            - 2;
        self.values[i] = i64::from(value);
    }
    pub fn n(&self, i: usize) -> i32 {
        self.values[i] as i32
    }
    pub fn f(&self, i: usize) -> Fixed {
        Fixed::from_bits(self.n(i))
    }
}
pub fn cases() -> Vec<Case> {
    include_str!("../reference/m3d3f_cases.csv")
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
pub fn map_bytes(index: usize) -> &'static [u8] {
    match index {
        0 => include_bytes!("../reference/m3d3f_map_0.bin"),
        1 => include_bytes!("../reference/m3d3f_map_1.bin"),
        2 => include_bytes!("../reference/m3d3f_map_2.bin"),
        3 => include_bytes!("../reference/m3d3f_map_3.bin"),
        4 => include_bytes!("../reference/m3d3f_map_4.bin"),
        5 => include_bytes!("../reference/m3d3f_map_5.bin"),
        6 => include_bytes!("../reference/m3d3f_map_6.bin"),
        7 => include_bytes!("../reference/m3d3f_map_7.bin"),
        8 => include_bytes!("../reference/m3d3f_map_8.bin"),
        9 => include_bytes!("../reference/m3d3f_map_9.bin"),
        10 => include_bytes!("../reference/m3d3f_map_10.bin"),
        11 => include_bytes!("../reference/m3d3f_map_11.bin"),
        12 => include_bytes!("../reference/m3d3f_map_12.bin"),
        13 => include_bytes!("../reference/m3d3f_map_13.bin"),
        14 => include_bytes!("../reference/m3d3f_map_14.bin"),
        15 => include_bytes!("../reference/m3d3f_map_15.bin"),
        16 => include_bytes!("../reference/m3d3f_map_16.bin"),
        17 => include_bytes!("../reference/m3d3f_map_17.bin"),
        18 => include_bytes!("../reference/m3d3f_map_18.bin"),
        19 => include_bytes!("../reference/m3d3f_map_19.bin"),
        _ => panic!("fixture map"),
    }
}
pub fn map(index: usize) -> doom_map::Map {
    let bytes = map_bytes(index);
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
pub fn registry(c: &Case) -> ActorRegistry {
    let r = ActorRegistry::vanilla();
    let mut states = r.states().to_vec();
    states[1] = ActorStateDef {
        name: "FIXTURE_SARG_LOOK".into(),
        sprite: 39,
        frame: 0,
        tics: c.n(15),
        action: ActorAction::Original(ActionId(43)),
        next: StateId(1),
        misc: [0, 0],
    };
    states[2] = ActorStateDef {
        name: "FIXTURE_SARG_CHASE".into(),
        sprite: 39,
        frame: 0,
        tics: c.v("see"),
        action: ActorAction::Original(ActionId(15)),
        next: StateId(2),
        misc: [0, 0],
    };
    for (i, name) in [(4, "S_SARG_ATK1"), (5, "S_SARG_ATK2"), (6, "S_SARG_ATK3")] {
        states[i] = r.states().iter().find(|s| s.name == name).unwrap().clone();
        states[i].name = format!("FIXTURE_SARG_ATK{}", i - 3);
        states[i].next = StateId(if i == 6 { 2 } else { i as u16 + 1 });
    }
    let mut types = r.types().to_vec();
    for i in [136] {
        let mut t = types[0].clone();
        t.name = format!("FIXTURE_DEMON_MELEE_{i}");
        for (key, value) in [
            (0, -1),
            (1, 1),
            (2, 150),
            (18, 400),
            (3, 2),
            (4, c.n(8)),
            (5, 8),
            (17, c.n(14)),
            (21, c.n(7)),
            (10, c.v("melee")),
            (
                11,
                if i != 136 && c.v("missile") == 4 {
                    7
                } else {
                    c.v("missile")
                },
            ),
            (6, c.v("attacksound")),
            (15, c.v("speed")),
            (16, c.v("radius")),
            (20, c.v("active_sound")),
        ] {
            t.properties[key] = value;
        }
        types[i] = t;
    }
    let mut target = types[0].clone();
    target.name = "FIXTURE_DAMAGE_TARGET".into();
    for (key, value) in [
        (0, -1),
        (2, c.v("spawn_health")),
        (8, c.v("pain")),
        (21, c.v("target_flags")),
        (16, c.v("target_radius")),
        (17, c.v("target_height")),
    ] {
        target.properties[key] = value;
    }
    types[135] = target;
    ActorRegistry::new(states, types).unwrap()
}
pub fn sight_setup(c: &Case) -> Simulation {
    setup(c)
}
pub fn setup(c: &Case) -> Simulation {
    let mut sim = Simulation::with_demon_melee_registry(
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
            ActorTypeId(135),
            [c.f(19 + i * 5), c.f(20 + i * 5)],
            SpawnHeight::Absolute(c.f(21 + i * 5)),
        )
        .unwrap();
    }
    configure(&mut sim, c);
    sim.set_momentum(
        ActorId::from_parts(1, 1),
        [
            Fixed::from_bits(c.v("target_mx")),
            Fixed::from_bits(c.v("target_my")),
        ],
    )
    .unwrap();
    for i in 2..=4 {
        let id = ActorId::from_parts(i, 1);
        let mut p = PursuitSetup::from_actor(sim.actor(id).unwrap()).unwrap();
        p.health = c.v("secondary_health");
        if c.v("secondary_flags") >= 0 {
            p.flags = c.v("secondary_flags") as u32;
        }
        sim.configure_pursuit_actor(id, p).unwrap();
    }
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
    if op == c.v("extra_strike_tick") as usize {
        sim.demon_melee_attack(id).unwrap();
    }
    if c.v("reuse_tick") >= 0 {
        if op == c.v("reuse_tick") as usize {
            sim.set_actor_target(id, None).unwrap();
            let mut player = sim.perception_players()[0];
            player.active = false;
            player.actor = None;
            sim.set_perception_player(0, player).unwrap();
            sim.mark_remove(ActorId::from_parts(1, 1)).unwrap();
        }
        if op == c.v("reuse_tick") as usize + 2 {
            let fresh = sim
                .spawn_type(
                    ActorTypeId(135),
                    [c.f(19), c.f(20)],
                    SpawnHeight::Absolute(c.f(21)),
                )
                .unwrap();
            assert_eq!(fresh, ActorId::from_parts(1, 2));
            sim.set_perception_player(
                0,
                PerceptionPlayer {
                    active: true,
                    health: c.v("spawn_health"),
                    actor: Some(fresh),
                },
            )
            .unwrap();
            sim.set_actor_target(id, Some(fresh)).unwrap();
        }
    }
    if op == 1 && c.n(11) >= 0 {
        sim.noise_alert(
            ActorId::from_parts(c.n(11) as u32, 1),
            ActorId::from_parts(c.n(12) as u32, 1),
        )
        .unwrap();
    }
    match c.n(0) {
        0 => {
            let before = sim.snapshot();
            let result = sim.check_melee_range(id).unwrap();
            assert_eq!(before, sim.snapshot());
            let rows = source_rows(
                cases()
                    .iter()
                    .position(|case| case.label == c.label)
                    .unwrap(),
                op,
            );
            let row = rows.iter().find(|(tag, _)| tag == "J").unwrap();
            let d = &row.1;
            if let Some(sight) = &result.sight {
                sight_support::assert_sight(
                    sight,
                    &rows,
                    0,
                    0,
                    1,
                    (
                        cases()
                            .iter()
                            .position(|case| case.label == c.label)
                            .unwrap(),
                        op,
                    ),
                );
            }
            assert_eq!(result.accepted, d[3] != 0);
            assert_eq!(result.reason, i32::try_from(d[4]).unwrap() as u8);
            assert_eq!(
                result.distance,
                if d[5] != 0 { Some(d[6] as i32) } else { None }
            );
            assert_eq!(
                result.threshold,
                if d[7] != 0 { Some(d[8] as i32) } else { None }
            );
        }
        1 => {
            sim.check_missile_range(id).unwrap();
        }
        2 => {
            sim.face_target(id).unwrap();
        }
        3 => {
            sim.chase(id).unwrap();
        }
        4 => {
            sim.step([TicCommand::default(); 4]).unwrap();
        }
        5 => {
            sim.demon_melee_attack(id).unwrap();
        }
        6 => {}
        7 => panic!("refire is outside the Demon profile"),
        _ => panic!("fixture operation"),
    }
}
pub fn assert_golden(sim: &Simulation, which: usize, op: usize) {
    let row = include_str!("../reference/m3d3f_snapshots.csv")
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
    include_str!("../reference/m3d3f_snapshots.csv")
        .lines()
        .skip(1)
        .any(|r| {
            let c = r.split(',').collect::<Vec<_>>();
            c[0].parse::<usize>().unwrap() == which && c[1].parse::<usize>().unwrap() == op
        })
}

pub fn source_rows(which: usize, op: usize) -> Vec<(String, Vec<i64>)> {
    type Rows = std::collections::BTreeMap<(usize, usize), Vec<(String, Vec<i64>)>>;
    static ROWS: std::sync::OnceLock<Rows> = std::sync::OnceLock::new();
    ROWS.get_or_init(|| {
        let mut rows = Rows::new();
        for line in doom_test_evidence::text(
            "tests/reference/m3d3f_trace.csv",
            include_bytes!("../reference/m3d3f_trace.csv.zst"),
            include_str!("../reference/zstd_manifest.json"),
        )
        .expect("verified historical zstd evidence")
        .lines()
        {
            let mut values = line.split(',');
            let tag = values.next().unwrap().to_string();
            let which = values.next().unwrap().parse().unwrap();
            let op = values.next().unwrap().parse().unwrap();
            rows.entry((which, op))
                .or_default()
                .push((tag, values.map(|s| s.parse().unwrap()).collect()));
        }
        rows
    })
    .get(&(which, op))
    .unwrap()
    .clone()
}
#[derive(Default)]
pub struct SourceTrace {
    births: Vec<ActorId>,
    rng: doom_types::DoomRng,
    pub actor_fields: usize,
    pub draws: usize,
    pub observations: usize,
}
impl SourceTrace {
    pub fn check(&mut self, sim: &Simulation, which: usize, op: usize, events: &[Event]) {
        for event in events {
            if let Event::Spawned(id) = event {
                self.births.push(*id);
            }
        }
        let rows = source_rows(which, op);
        for (_, c) in rows.iter().filter(|(tag, c)| tag == "E" && c[0] == 8) {
            assert_eq!(
                i64::from(self.rng.p_random()),
                c[2],
                "case {which} op {op} source RNG"
            );
            self.draws += 1;
        }
        assert_eq!(
            sim.rng().indices(),
            self.rng.indices(),
            "case {which} op {op} RNG index"
        );
        self.observations += rows
            .iter()
            .filter(|(tag, _)| {
                [
                    "E", "J", "G", "X", "H", "V", "K", "D", "B", "U", "I", "T", "M", "C", "O", "Z",
                    "Y",
                ]
                .contains(&tag.as_str())
            })
            .count();
        for (_, c) in rows.iter().filter(|(tag, _)| tag == "A") {
            let id = self.births[c[0] as usize];
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
                    self.births.iter().position(|b| *b == id).unwrap() as i64
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
                assert_eq!(
                    got, want,
                    "case {which} op {op} birth {} source actor field {field}",
                    c[0]
                );
                self.actor_fields += 1;
            }
        }
    }
}
