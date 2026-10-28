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
        _ => panic!("fixture map"),
    };
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
pub fn sight_setup(c: &Case) -> Simulation {
    let mut sim = Simulation::monster_perception(map(c.n(1) as usize)).unwrap();
    sim.spawn_type(
        ActorTypeId(0),
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
    sim
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
