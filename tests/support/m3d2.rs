//! Public synthetic inputs and C-backed player weapon setup.
#![allow(dead_code)]
#[path = "m3d1.rs"]
mod previous;
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
    include_str!("../reference/m3d2_cases.csv")
        .lines()
        .skip(1)
        .map(|line| {
            let c = line.split(',').collect::<Vec<_>>();
            Case {
                label: c[1].into(),
                values: c[2..]
                    .iter()
                    .enumerate()
                    .map(|(i, s)| if i == 13 { 0 } else { s.parse().unwrap() })
                    .collect(),
            }
        })
        .collect()
}
pub fn weapon(v: i32) -> Weapon {
    match v {
        0 => Weapon::Fist,
        1 => Weapon::Pistol,
        2 => Weapon::Shotgun,
        3 => Weapon::Chaingun,
        _ => panic!("fixture weapon"),
    }
}
pub fn registry(c: &Case) -> ActorRegistry {
    let r = ActorRegistry::vanilla();
    let mut types = r.types().to_vec();
    let mut t = types[0].clone();
    t.name = "FIXTURE_PAWN".into();
    t.properties[0] = -1;
    t.properties[2] = c.n(5);
    t.properties[8] = c.n(6);
    t.properties[18] = c.n(14);
    t.properties[21] = 6;
    t.properties[5] = 8;
    t.properties[17] = c.n(10);
    types[136] = t;
    ActorRegistry::new(r.states().to_vec(), types).unwrap()
}
pub fn map(index: usize) -> doom_map::Map {
    if index < 9 {
        return previous::map(index);
    }
    let bytes: &[u8] = match index {
        9 => include_bytes!("../reference/m3d2_map_9.bin"),
        10 => include_bytes!("../reference/m3d2_map_10.bin"),
        11 => include_bytes!("../reference/m3d2_map_11.bin"),
        _ => panic!("fixture map"),
    };
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
pub fn setup(c: &Case) -> Simulation {
    let mut sim = Simulation::with_weapon_registry(
        map(c.n(12) as usize),
        64,
        registry(c),
        ActorOptions::default(),
    )
    .unwrap();
    let id = sim
        .spawn_type(
            ActorTypeId(0),
            [Fixed::from_map_units(-80), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    sim.spawn_type(
        ActorTypeId(136),
        [c.f(7), c.f(8)],
        SpawnHeight::Absolute(c.f(9)),
    )
    .unwrap();
    if c.n(15) != 0 {
        sim.spawn_type(
            ActorTypeId(136),
            [Fixed::from_map_units(96), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    }
    sim.initialize_weapon_player(
        c.n(11) as usize,
        id,
        WeaponInventory {
            ready: weapon(c.n(0)),
            owned: c.n(1) as u16,
            ammo: [c.n(2), c.n(3), 0, 0],
        },
    )
    .unwrap();
    sim
}
pub fn commands(which: usize) -> Vec<[TicCommand; 4]> {
    include_str!("../reference/m3d2_commands.csv")
        .lines()
        .skip(1)
        .filter_map(|line| {
            let c = line
                .split(',')
                .map(|x| x.parse::<i32>().unwrap())
                .collect::<Vec<_>>();
            if c[0] as usize != which {
                return None;
            }
            let mut cmd = [TicCommand::default(); 4];
            cmd[c[2] as usize] = TicCommand {
                forward: c[3] as i8,
                side: c[4] as i8,
                angle_turn: c[5] as i16,
                buttons: c[6] as u8,
            };
            Some(cmd)
        })
        .collect()
}
/// Full independent byte/field comparison when stored, plus every recorded digest.
pub fn assert_golden(sim: &Simulation, which: usize, tic: usize) {
    let row = include_str!("../reference/m3d2_snapshots.csv")
        .lines()
        .skip(1)
        .find(|r| {
            let c = r.split(',').collect::<Vec<_>>();
            c[0].parse::<usize>().unwrap() == which && c[1].parse::<usize>().unwrap() == tic
        })
        .unwrap();
    let c = row.split(',').collect::<Vec<_>>();
    let snapshot = sim.snapshot();
    if c[3] != "-" {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/reference")
            .join(c[3]);
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
        c[2],
        "case {which} tic {tic}"
    );
}
pub fn has_golden(which: usize, tic: usize) -> bool {
    include_str!("../reference/m3d2_snapshots.csv")
        .lines()
        .skip(1)
        .any(|r| {
            let c = r.split(',').collect::<Vec<_>>();
            c[0].parse::<usize>().unwrap() == which && c[1].parse::<usize>().unwrap() == tic
        })
}
/// Source player_t weapon fields in the reference row's explicit order.
pub fn player_values(sim: &Simulation) -> Vec<i64> {
    let p = sim.weapon_player().unwrap();
    let mut v = vec![
        i64::from(p.slot),
        0,
        i64::from(p.health),
        p.ready as i64,
        p.pending.map_or(10, |w| w as i64),
        i64::from(p.owned),
    ];
    v.extend(p.ammo.map(i64::from));
    v.extend(p.max_ammo.map(i64::from));
    v.extend([
        i64::from(p.attack_down),
        i64::from(p.refire),
        i64::from(p.bob.bits()),
        i64::from(p.extra_light),
    ]);
    for ps in p.psprites {
        v.extend([
            ps.state.map_or(-1, |s| i64::from(s.0)),
            i64::from(ps.tics),
            i64::from(ps.position[0].bits()),
            i64::from(ps.position[1].bits()),
        ]);
    }
    v.push(i64::from(sim.rng().indices()[0]));
    v
}
pub fn player_reference(which: usize) -> Vec<Vec<i64>> {
    doom_test_evidence::text(
        "tests/reference/m3d2_trace.csv",
        include_bytes!("../reference/m3d2_trace.csv.zst"),
        include_str!("../reference/zstd_manifest.json"),
    )
    .expect("verified historical zstd evidence")
    .lines()
    .filter(|r| {
        r.starts_with("P,") && r.split(',').nth(1).unwrap().parse::<usize>().unwrap() == which
    })
    .map(|r| r.split(',').skip(3).map(|x| x.parse().unwrap()).collect())
    .collect()
}
