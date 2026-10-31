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
        let i = include_str!("../reference/m3f3_cases.csv")
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
        let i = include_str!("../reference/m3f3_cases.csv")
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
    include_str!("../reference/m3f3_cases.csv")
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
mod public_maps {
    include!("../reference/m3f3_maps.rs");
}
pub fn map_bytes(index: usize) -> &'static [u8] {
    public_maps::MAP_BYTES[index]
}
pub fn map(index: usize) -> doom_map::Map {
    let bytes = map_bytes(index);
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
pub fn registry(_c: &Case) -> ActorRegistry {
    ActorRegistry::demon_map_context_fixture()
}
pub fn options(c: &Case) -> ActorOptions {
    ActorOptions {
        skill: [Skill::Baby, Skill::Easy, Skill::Medium, Skill::Hard][c.v("skill") as usize],
        no_monsters: c.v("no_monsters") != 0,
        ..ActorOptions::default()
    }
}
pub fn map_only(c: &Case) -> (Simulation, MapSpawnReport) {
    let mut s = Simulation::with_demon_expanded_map_registry(
        map(c.v("map") as usize),
        64,
        registry(c),
        options(c),
        MapContextOptions {
            initial_rng: doom_types::DoomRng::from_indices(c.v("seed") as u8, 0),
            respawn_monsters: false,
        },
    )
    .unwrap();
    for _ in 0..c.v("skip") {
        s.gameplay_random();
    }
    let report = s.spawn_map_things().unwrap();
    (s, report)
}
pub fn setup(c: &Case) -> Simulation {
    let (sim, _) = map_only(c);
    install_targets(sim, c)
}
pub fn install_targets(mut sim: Simulation, c: &Case) -> Simulation {
    let demons = sim.level_statistics().unwrap().total_kills as usize;
    for i in 0..4 {
        let actor = sim
            .spawn_type(
                ActorTypeId(135),
                [
                    Fixed::from_bits(c.v(&format!("p{i}x"))),
                    Fixed::from_bits(c.v(&format!("p{i}y"))),
                ],
                SpawnHeight::Absolute(Fixed::from_bits(c.v(&format!("p{i}z")))),
            )
            .unwrap();
        let mut p = PursuitSetup::from_actor(sim.actor(actor).unwrap()).unwrap();
        p.health = c.v("body_health");
        sim.configure_pursuit_actor(actor, p).unwrap();
    }
    for i in 0..4 {
        sim.set_perception_player(
            i,
            PerceptionPlayer {
                active: c.v(&format!("p{i}active")) != 0,
                health: c.v(&format!("p{i}health")),
                actor: Some(ActorId::from_parts((demons + i) as u32, 1)),
            },
        )
        .unwrap();
    }
    if c.v("enter_all") != 0 {
        for id in (0..demons).map(|i| ActorId::from_parts(i as u32, 1)) {
            sim.set_actor_target(id, Some(ActorId::from_parts(demons as u32, 1)))
                .unwrap();
            sim.enter_actor_state(id, StateId(c.v("enter_all") as u16))
                .unwrap();
        }
    }
    sim
}
pub fn damage_source(sim: &Simulation, kind: i32) -> Option<ActorId> {
    match kind {
        0 => None,
        1 => sim.perception_players()[0].actor,
        2 => Some(ActorId::from_parts(0, 1)),
        3 => Some(ActorId::from_parts(1, 1)),
        _ => panic!("source class"),
    }
}
pub fn advance(sim: &mut Simulation, c: &Case, op: usize) {
    let owner = ActorId::from_parts(0, 1);
    let body = sim.perception_players()[0].actor;
    if op == c.v("damage_tick") as usize {
        sim.damage_actor(DamageRequest {
            target: owner,
            source: damage_source(sim, c.v("source_kind")),
            inflictor: damage_source(sim, c.v("inflictor_kind")),
            amount: c.v("damage"),
        })
        .unwrap();
    }
    if op == c.v("kill_tick") as usize {
        if c.v("hitscan") != 0 {
            sim.line_attack(
                body.unwrap(),
                LineAttack {
                    angle: 0x80000000,
                    range: Fixed::from_map_units(512),
                    slope: Fixed::ZERO,
                    damage: c.v("kill"),
                },
            )
            .unwrap();
        } else {
            let count = if c.v("kill_all") != 0 {
                sim.level_statistics().unwrap().total_kills as u32
            } else {
                1
            };
            for index in 0..count {
                sim.damage_actor(DamageRequest {
                    target: ActorId::from_parts(index, 1),
                    source: damage_source(sim, c.v("source_kind")),
                    inflictor: damage_source(sim, c.v("inflictor_kind")),
                    amount: c.v("kill"),
                })
                .unwrap();
            }
        }
    }
    if op == c.v("remove_corpse") as usize {
        sim.mark_remove(owner).unwrap();
    }
    if op == c.v("sound_tick") as usize {
        sim.noise_alert(body.unwrap(), body.unwrap()).unwrap();
    }
    if op == c.v("retire_tick") as usize {
        sim.set_perception_player(
            0,
            PerceptionPlayer {
                active: false,
                health: c.v("p0health"),
                actor: None,
            },
        )
        .unwrap();
        sim.mark_remove(body.unwrap()).unwrap();
    }
    if c.v("retire_tick") >= 0 && op == c.v("retire_tick") as usize + 2 {
        let actor = sim
            .spawn_type(
                ActorTypeId(135),
                [Fixed::from_bits(c.v("p0x")), Fixed::from_bits(c.v("p0y"))],
                SpawnHeight::Absolute(Fixed::from_bits(c.v("p0z"))),
            )
            .unwrap();
        assert_eq!(actor.generation(), 2);
        sim.set_perception_player(
            0,
            PerceptionPlayer {
                active: true,
                health: 100,
                actor: Some(actor),
            },
        )
        .unwrap();
    }
    sim.step([TicCommand::default(); 4])
        .unwrap_or_else(|e| panic!("{} op{op}: {e}", c.label));
}
pub fn assert_golden(sim: &Simulation, which: usize, op: usize) -> usize {
    let row = include_str!("../reference/m3f3_snapshots.csv")
        .lines()
        .skip(1)
        .find(|r| {
            let c = r.split(',').collect::<Vec<_>>();
            c[0].parse::<usize>().unwrap() == which && c[1].parse::<usize>().unwrap() == op
        })
        .unwrap();
    let c = row.split(',').collect::<Vec<_>>();
    let snapshot = sim.snapshot();
    let events = assert_ordered_events(&snapshot, which, op);
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
    events
}
pub fn has_golden(which: usize, op: usize) -> bool {
    include_str!("../reference/m3f3_snapshots.csv")
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
            "tests/reference/m3f3_trace.csv",
            include_bytes!("../reference/m3f3_trace.csv.zst"),
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
    pub instrumentation_rows: usize,
    pub statistics_fields: usize,
}
impl SourceTrace {
    pub fn for_case(c: &Case) -> Self {
        Self {
            rng: doom_types::DoomRng::from_indices(c.v("seed") as u8, 0),
            ..Self::default()
        }
    }
    pub fn check_original_spawn(&mut self, sim: &Simulation, which: usize) {
        for event in sim.pending_events() {
            if let Event::Spawned(id) = event {
                self.births.push(*id);
            }
        }
        let rows = include_str!("../reference/m3f3_spawn.csv")
            .lines()
            .filter_map(|line| {
                let mut fields = line.split(',');
                let tag = fields.next().unwrap().to_string();
                let scenario = fields.next().unwrap().parse::<usize>().unwrap();
                let _op = fields.next();
                if scenario != which {
                    return None;
                }
                let mut c = fields
                    .map(|x| x.parse::<i64>().unwrap())
                    .collect::<Vec<_>>();
                if tag == "A" {
                    assert_eq!(c[2], 12);
                    assert_eq!(c[10], 475);
                    c[2] = 136;
                    c[10] = 1;
                    // Original COUNTKILL is preserved; only identity/state aliasing remains.
                }
                if tag == "F" && c[0] >= 0 && c[7] > c[6] {
                    let actor = sim.actor(ActorId::from_parts(c[6] as u32, 1)).unwrap();
                    let point = actor.life().unwrap().spawnpoint.unwrap();
                    assert_eq!(
                        [
                            i64::from(point.position.x),
                            i64::from(point.position.y),
                            i64::from(point.angle),
                            i64::from(point.kind),
                            i64::from(point.options)
                        ],
                        c[1..6]
                    );
                }
                Some((tag, c))
            })
            .collect();
        self.compare_rows(sim, which, 0, rows);
    }
    pub fn check(&mut self, sim: &Simulation, which: usize, op: usize, events: &[Event]) {
        for event in events {
            if let Event::Spawned(id) = event {
                self.births.push(*id);
            }
        }
        let rows = source_rows(which, op);
        self.compare_rows(sim, which, op, rows);
    }
    pub fn compare_rows(
        &mut self,
        sim: &Simulation,
        which: usize,
        op: usize,
        rows: Vec<(String, Vec<i64>)>,
    ) {
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
        self.instrumentation_rows += rows
            .iter()
            .filter(|(tag, _)| {
                [
                    "E", "J", "G", "X", "H", "V", "K", "D", "B", "U", "I", "T", "M", "C", "O", "Z",
                    "Y", "ST",
                ]
                .contains(&tag.as_str())
            })
            .count();
        for (_, c) in rows.iter().filter(|(tag, _)| tag == "LS") {
            let stats = sim.level_statistics().unwrap();
            assert_eq!(
                [
                    stats.total_kills,
                    stats.total_items,
                    stats.total_secrets,
                    stats.single_player_kill_count
                ]
                .map(i64::from)
                .as_slice(),
                c.as_slice(),
                "case {which} op {op} source statistics"
            );
            self.statistics_fields += 4;
        }
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

/// Exact ordered C-derived event bytes for every capture, with field diagnostics.
fn assert_ordered_events(snapshot: &Snapshot, which: usize, op: usize) -> usize {
    type Index = std::collections::BTreeMap<(usize, usize), (usize, usize, usize)>;
    static INDEX: std::sync::OnceLock<Index> = std::sync::OnceLock::new();
    let bytes = include_bytes!("../reference/m3f3_events.bin").as_slice();
    let index = INDEX.get_or_init(|| {
        assert_eq!(&bytes[..4], b"EV20");
        let mut index = Index::new();
        let mut at = 4;
        while at < bytes.len() {
            let which = read_u32(bytes, &mut at);
            let op = read_u32(bytes, &mut at);
            let count = read_u32(bytes, &mut at);
            let start = at;
            for _ in 0..count {
                let size = read_u32(bytes, &mut at);
                at += size;
                let size = read_u64(bytes, &mut at);
                at += size;
            }
            assert!(index.insert((which, op), (count, start, at)).is_none());
        }
        assert_eq!(at, bytes.len());
        index
    });
    let &(count, mut at, end) = index.get(&(which, op)).unwrap();
    let mut actual_count = 0;
    for field in snapshot
        .fields()
        .filter(|f| f.path().starts_with("events."))
    {
        let size = read_u32(bytes, &mut at);
        let path = std::str::from_utf8(&bytes[at..at + size]).unwrap();
        at += size;
        let size = read_u64(bytes, &mut at);
        assert_eq!(field.path(), path, "case {which} op {op} event order");
        let expected = &bytes[at..at + size];
        if field.value() != expected {
            let first = field.value().iter().zip(expected).position(|(a, b)| a != b);
            panic!(
                "case {which} op {op} {path}: first differing event byte {first:?}; lengths {} / {}",
                field.value().len(),
                expected.len()
            );
        }
        at += size;
        actual_count += 1;
    }
    assert_eq!(
        actual_count, count,
        "case {which} op {op} event-field count"
    );
    assert_eq!(at, end, "case {which} op {op} event payload extent");
    count - 2
}
fn read_u32(bytes: &[u8], at: &mut usize) -> usize {
    let value = u32::from_le_bytes(bytes[*at..*at + 4].try_into().unwrap());
    *at += 4;
    value as usize
}
fn read_u64(bytes: &[u8], at: &mut usize) -> usize {
    let value = u64::from_le_bytes(bytes[*at..*at + 8].try_into().unwrap());
    *at += 8;
    usize::try_from(value).unwrap()
}
