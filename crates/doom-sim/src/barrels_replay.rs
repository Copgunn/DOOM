//! The full replay of the committed M3h9b C scenarios (plan step 8): all 26
//! scenarios (`m3h9b_cases.csv`, every one at medium skill) under
//! `Simulation::original_player_barrels_map`, each driven through its declared
//! setup, per-op commands (`UC` rows) and controlled between-tic inputs (`hit_*`
//! through the shared `P_DamageMobj` transaction, `attack_at`, `chase_*` with the
//! monster's own see state; [`crate::movers_replay::controlled`]).
//!
//! Two passes compare everything. The shared engine
//! ([`crate::movers_replay::full::replay_all_in`]) compares every row kind the
//! M3h9 replay compares (sector heights and `specialdata`, line specials, use
//! latch, player state, crossings, uses, use intercepts, `spechit`-blocked moves,
//! the thinker list), the PBE1 event frame of every op (`m3h9b_events.dref`), and
//! at every capture the independent digest, size and field count
//! (`m3h9b_snapshots.csv`), the 16 pinned fixtures byte for byte and the
//! `decode_barrels` round trip with `decode_e1m1` rejecting version 37.
//! [`replay_barrels`] then compares the barrel rows: the spawn rows that pin the
//! ident map (map THINGs in lump order, the player body, later spawns in creation
//! order), every state-entry and action row (`A_Explode` among them), the `RA` row
//! of every explosion, every `RC` candidate in walk order with the `G` damage row
//! of each code-4 candidate, and the contract's barrel death chains. The walk is
//! read through the test-only [`crate::barrels::walk_trace`] observer. Expected
//! values come only from the C rows, the C contract and the independent DSIM37
//! goldens, never from Rust output.
//!
//! Between-tic records (the controlled damage) are part of the op whose tic
//! follows them, as in the C rows: each op's events start at the pending records
//! the setup or the previous op left, exactly as the shared engine slices them.
use crate::diagonal_room::Lumps;
use crate::movers_replay::full::{
    Capture, Frames, Spec, Totals, parse_captures, parse_frames, replay_all_in,
};
use crate::movers_replay::{Case, Evidence, Rows, parse_cases, parse_rows, setup_map_with};
use crate::*;
use std::{collections::BTreeMap, sync::OnceLock};

mod maps {
    include!("../../../tests/reference/m3h9b_maps.rs");
}

/// A committed M3h9b room (`tests/reference/m3h9b_cases.csv`, column `map`).
pub(crate) fn room(index: usize) -> Lumps {
    Lumps::parse(maps::MAP_BYTES[index])
}

/// Every M3h9b trace row keyed by (case, op).
pub(crate) fn rows() -> &'static Rows {
    static DATA: OnceLock<Rows> = OnceLock::new();
    DATA.get_or_init(|| parse_rows(include_bytes!("../../../tests/reference/m3h9b_trace.dref")))
}
/// The 26 scenarios, all at medium skill.
pub(crate) fn cases() -> Vec<Case> {
    parse_cases(
        include_str!("../../../tests/reference/m3h9b_cases.csv"),
        rows(),
    )
}
/// The C driver's setup in the committed M3h9b room of `c` under the barrels policy.
pub(crate) fn setup(c: &Case) -> (Simulation, ActorId, Vec<ActorId>) {
    setup_map_with(
        c,
        room(c.map).map(),
        Simulation::original_player_barrels_map,
    )
}
/// The M3h9b barrels evidence.
pub(crate) const M3H9B: Evidence = Evidence { rows, setup };

/// The committed full-byte fixtures (`m3h9b_snapshot_contract.json`).
pub(crate) const FIXTURES: &[(&str, &[u8])] = &[
    (
        "m3h9b_state_0_0.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_0_0.bin"),
    ),
    (
        "m3h9b_state_0_24.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_0_24.bin"),
    ),
    (
        "m3h9b_state_1_74.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_1_74.bin"),
    ),
    (
        "m3h9b_state_1_84.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_1_84.bin"),
    ),
    (
        "m3h9b_state_1_85.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_1_85.bin"),
    ),
    (
        "m3h9b_state_1_96.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_1_96.bin"),
    ),
    (
        "m3h9b_state_1_105.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_1_105.bin"),
    ),
    (
        "m3h9b_state_2_36.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_2_36.bin"),
    ),
    (
        "m3h9b_state_5_88.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_5_88.bin"),
    ),
    (
        "m3h9b_state_5_96.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_5_96.bin"),
    ),
    (
        "m3h9b_state_12_88.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_12_88.bin"),
    ),
    (
        "m3h9b_state_18_210.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_18_210.bin"),
    ),
    (
        "m3h9b_state_22_0.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_22_0.bin"),
    ),
    (
        "m3h9b_state_23_26.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_23_26.bin"),
    ),
    (
        "m3h9b_state_24_0.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_24_0.bin"),
    ),
    (
        "m3h9b_state_25_0.bin",
        include_bytes!("../../../tests/reference/m3h9b_state_25_0.bin"),
    ),
];
fn frames() -> &'static Frames {
    static DATA: OnceLock<Frames> = OnceLock::new();
    DATA.get_or_init(|| parse_frames(include_bytes!("../../../tests/reference/m3h9b_events.dref")))
}
fn captures() -> BTreeMap<(usize, usize), Capture> {
    parse_captures(include_str!("../../../tests/reference/m3h9b_snapshots.csv"))
}
/// The M3h9b set: DSIM37 read by `decode_barrels`, rejected by `decode_e1m1`.
pub(crate) const M3H9B_SPEC: Spec = Spec {
    evidence: M3H9B,
    cases,
    frames,
    captures,
    fixtures: FIXTURES,
    decode: |b, l| Diagnostic::decode_barrels(b, l),
    older: |b, l| Diagnostic::decode_e1m1(b, l),
    version: 37,
};

/// A minimal reader of the committed JSON contract (test evidence only): enough
/// for `m3h9b_contract.json`'s objects, arrays, strings, integers and `null`.
mod json {
    #[derive(Clone, Debug, PartialEq)]
    pub(crate) enum Value {
        Null,
        Bool(bool),
        Number(f64),
        Str(String),
        Array(Vec<Value>),
        Object(Vec<(String, Value)>),
    }
    impl Value {
        pub(crate) fn get(&self, key: &str) -> &Value {
            match self {
                Value::Object(o) => &o.iter().find(|(k, _)| k == key).expect(key).1,
                _ => panic!("not an object: {key}"),
            }
        }
        pub(crate) fn array(&self) -> &[Value] {
            match self {
                Value::Array(a) => a,
                _ => panic!("not an array"),
            }
        }
        /// An integer, or `None` for `null`.
        pub(crate) fn int(&self) -> Option<i64> {
            match self {
                Value::Null => None,
                Value::Number(n) => {
                    assert_eq!(n.fract(), 0.0);
                    Some(*n as i64)
                }
                // Idents are written as strings.
                Value::Str(s) => Some(s.parse().unwrap()),
                _ => panic!("not an integer"),
            }
        }
    }
    pub(crate) fn parse(text: &str) -> Value {
        let mut p = Parser {
            b: text.as_bytes(),
            at: 0,
        };
        let v = p.value();
        p.ws();
        assert_eq!(p.at, p.b.len());
        v
    }
    struct Parser<'a> {
        b: &'a [u8],
        at: usize,
    }
    impl Parser<'_> {
        fn ws(&mut self) {
            while self.b.get(self.at).is_some_and(u8::is_ascii_whitespace) {
                self.at += 1;
            }
        }
        fn eat(&mut self, c: u8) -> bool {
            self.ws();
            let hit = self.b.get(self.at) == Some(&c);
            self.at += usize::from(hit);
            hit
        }
        fn string(&mut self) -> String {
            assert!(self.eat(b'"'));
            let mut out = Vec::new();
            loop {
                let c = self.b[self.at];
                self.at += 1;
                match c {
                    b'"' => break,
                    b'\\' => {
                        let e = self.b[self.at];
                        self.at += 1;
                        match e {
                            b'u' => {
                                let hex =
                                    std::str::from_utf8(&self.b[self.at..self.at + 4]).unwrap();
                                self.at += 4;
                                let ch = char::from_u32(u32::from_str_radix(hex, 16).unwrap())
                                    .unwrap_or('?');
                                out.extend_from_slice(ch.to_string().as_bytes());
                            }
                            b'n' => out.push(b'\n'),
                            b't' => out.push(b'\t'),
                            other => out.push(other),
                        }
                    }
                    other => out.push(other),
                }
            }
            String::from_utf8(out).unwrap()
        }
        fn value(&mut self) -> Value {
            self.ws();
            match self.b[self.at] {
                b'{' => {
                    self.at += 1;
                    let mut o = Vec::new();
                    if !self.eat(b'}') {
                        loop {
                            let k = self.string();
                            assert!(self.eat(b':'));
                            o.push((k, self.value()));
                            if self.eat(b'}') {
                                break;
                            }
                            assert!(self.eat(b','));
                        }
                    }
                    Value::Object(o)
                }
                b'[' => {
                    self.at += 1;
                    let mut a = Vec::new();
                    if !self.eat(b']') {
                        loop {
                            a.push(self.value());
                            if self.eat(b']') {
                                break;
                            }
                            assert!(self.eat(b','));
                        }
                    }
                    Value::Array(a)
                }
                b'"' => Value::Str(self.string()),
                b'n' => {
                    self.at += 4;
                    Value::Null
                }
                b't' => {
                    self.at += 4;
                    Value::Bool(true)
                }
                b'f' => {
                    self.at += 5;
                    Value::Bool(false)
                }
                _ => {
                    let start = self.at;
                    while self
                        .b
                        .get(self.at)
                        .is_some_and(|c| c.is_ascii_digit() || b"+-.eE".contains(c))
                    {
                        self.at += 1;
                    }
                    let t = std::str::from_utf8(&self.b[start..self.at]).unwrap();
                    Value::Number(t.parse().unwrap())
                }
            }
        }
    }
}
/// The C evidence contract `m3h9b_contract.json`.
fn contract() -> &'static json::Value {
    static DATA: OnceLock<json::Value> = OnceLock::new();
    DATA.get_or_init(|| json::parse(include_str!("../../../tests/reference/m3h9b_contract.json")))
}

/// What the barrel comparisons of the replay compared.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Barrels {
    /// `RA` rows (one per `P_RadiusAttack`).
    pub explosions: usize,
    /// `RC` rows, by outcome code (0 not shootable, 1 boss, 2 out of range, 3 sight
    /// blocked, 4 damaged).
    pub codes: [usize; 5],
    /// The `G` damage rows following the code-4 candidates.
    pub damage_records: usize,
    /// `E` state-entry (kind 4) and action (kind 5) rows.
    pub state_rows: usize,
    pub action_rows: usize,
    /// `A_Explode` action rows (action 19).
    pub explode_rows: usize,
    /// `E` spawn rows (kind 1): the ident map.
    pub spawn_rows: usize,
    /// Barrel death chains (`barrel_deaths` of the contract), and how many gibbed.
    pub deaths: usize,
    pub gibbed: usize,
}
impl Barrels {
    pub(crate) fn candidates(&self) -> usize {
        self.codes.iter().sum()
    }
    fn add(&mut self, o: Self) {
        self.explosions += o.explosions;
        for (a, b) in self.codes.iter_mut().zip(o.codes) {
            *a += b;
        }
        self.damage_records += o.damage_records;
        self.state_rows += o.state_rows;
        self.action_rows += o.action_rows;
        self.explode_rows += o.explode_rows;
        self.spawn_rows += o.spawn_rows;
        self.deaths += o.deaths;
        self.gibbed += o.gibbed;
    }
}

/// One barrel's death chain: killed op, `t0`, `d`, the ops of `S_BEXP2`
/// (`A_Scream`), `S_BEXP4` (`A_Explode`), `S_NULL` and `S_GIBS`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Death {
    barrel: i64,
    killed: i64,
    t0: i64,
    d: i64,
    scream: Option<i64>,
    explode: Option<i64>,
    removed: Option<i64>,
    gibbed: Option<i64>,
}

const A_EXPLODE: i64 = 19;
fn is_barrel(s: &Simulation, id: ActorId) -> bool {
    s.actor(id)
        .and_then(|a| a.life())
        .is_some_and(|l| l.actor_type.0 == crate::identity::OriginalActorType::MT_BARREL.ordinal())
}

/// Replays `c` against the barrel rows: at every op the spawn rows (`E` kind 1,
/// which pin the ident map: map THINGs in lump order, the player body, later
/// spawns in creation order), every state entry and action row (`E` kinds 4 and 5,
/// the `A_Explode` rows among them), the `RA` row of each explosion (spot, source,
/// damage, the 3 × 3 window, the spot and the wrapped `dist`), every `RC`
/// candidate in walk order (block, ordinal, outcome, `dist`, damage) with the `G`
/// damage row following each code-4 candidate, and after the last op the
/// contract's death chains. The walk is read through the test-only
/// [`crate::barrels::walk_trace`] observer.
pub(crate) fn replay_barrels(c: &Case) -> Barrels {
    use crate::barrels::walk_trace::{self, Row};
    use crate::identity::OriginalState as S;
    let mut b = Barrels::default();
    let (mut s, body, things) = setup(c);
    let mut idents: BTreeMap<ActorId, i64> = things
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i as i64))
        .collect();
    idents.insert(body, things.len() as i64);
    let mut next = things.len() as i64 + 1;
    let mut deaths: Vec<Death> = Vec::new();
    walk_trace::enable();
    for op in 0..=c.tics {
        let pending = s.pending_events().len();
        if op != 0 {
            crate::movers_replay::controlled(&mut s, c, body, &things, op);
            s.advance(crate::movers_replay::command_in(rows(), c, op))
                .unwrap_or_else(|e| panic!("{} op{op}: {e}", c.label));
        }
        let events: Vec<Event> = if op == 0 {
            s.pending_events().to_vec()
        } else {
            s.events()[pending..].to_vec()
        };
        let walk = walk_trace::take();
        let rows = rows().get(&(c.which, op)).map_or(&[][..], Vec::as_slice);
        let at = |what: &str| format!("{} op{op} {what}", c.label);
        // Spawns: assign idents in creation order, then compare with `E` kind 1.
        let mut spawned = Vec::new();
        for e in &events {
            if let Event::ActorState(lifecycle::ActorStateEvent::Initialized {
                actor,
                actor_type,
            }) = e
            {
                if op != 0 {
                    assert!(!idents.contains_key(actor), "{}", at("respawned ident"));
                    idents.insert(*actor, next);
                    next += 1;
                }
                spawned.push([idents[actor], i64::from(actor_type.0)]);
            }
        }
        let id = |a: &ActorId| {
            *idents
                .get(a)
                .unwrap_or_else(|| panic!("{}", at("unmapped actor")))
        };
        let opt = |a: &Option<ActorId>| a.as_ref().map_or(-1, id);
        let c_rows = |kind: i64| -> Vec<[i64; 2]> {
            rows.iter()
                .filter(|(t, v)| t == "E" && v[0] == kind)
                .map(|(_, v)| [v[1], v[2]])
                .collect()
        };
        assert_eq!(spawned, c_rows(1), "{}", at("spawns"));
        b.spawn_rows += spawned.len();
        // State entries and actions, in execution order.
        let entered: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::ActorState(lifecycle::ActorStateEvent::Entered { actor, state }) => {
                    Some([id(actor), i64::from(state.0)])
                }
                _ => None,
            })
            .collect();
        assert_eq!(entered, c_rows(4), "{}", at("state entries"));
        b.state_rows += entered.len();
        let actions: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::ActorState(lifecycle::ActorStateEvent::Action {
                    actor,
                    action: lifecycle::ActorAction::Original(a),
                }) => Some([id(actor), i64::from(a.0)]),
                _ => None,
            })
            .collect();
        assert_eq!(actions, c_rows(5), "{}", at("actions"));
        b.action_rows += actions.len();
        b.explode_rows += actions.iter().filter(|a| a[1] == A_EXPLODE).count();
        // RA and RC rows against the observed walk, in order.
        let c_walk: Vec<_> = rows
            .iter()
            .filter(|(t, _)| t == "RA" || t == "RC")
            .collect();
        assert_eq!(walk.len(), c_walk.len(), "{}", at("radius rows"));
        for (row, (tag, v)) in walk.iter().zip(&c_walk) {
            match *row {
                Row::Attack {
                    spot,
                    source,
                    window,
                    position,
                    dist,
                } => {
                    assert_eq!(tag, "RA", "{}", at("RA order"));
                    let got = [
                        id(&spot),
                        opt(&source),
                        i64::from(crate::barrels::BOMB_DAMAGE),
                        window[0].into(),
                        window[1].into(),
                        window[2].into(),
                        window[3].into(),
                        position[0].into(),
                        position[1].into(),
                        position[2].into(),
                        dist.into(),
                    ];
                    assert_eq!(&got[..], &v[..], "{}", at("RA"));
                    b.explosions += 1;
                }
                Row::Candidate {
                    thing,
                    block,
                    position,
                    code,
                    dist,
                } => {
                    assert_eq!(tag, "RC", "{}", at("RC order"));
                    let damage = if code == 4 {
                        i64::from(crate::barrels::BOMB_DAMAGE - dist)
                    } else {
                        0
                    };
                    let got = [
                        id(&thing),
                        block.into(),
                        position as i64,
                        code.into(),
                        dist.into(),
                        damage,
                    ];
                    assert_eq!(&got[..], &v[..], "{}", at("RC"));
                    b.codes[usize::from(code)] += 1;
                }
            }
        }
        // The G damage row after each code-4 RC row, against the damage records
        // whose inflictor is an exploding barrel, in order.
        let mut expected = Vec::new();
        for (k, (tag, v)) in rows.iter().enumerate() {
            if tag == "RC" && v[3] == 4 {
                let g = rows[k + 1..]
                    .iter()
                    .find(|(t, g)| t == "G" && g[0] == 2)
                    .map(|(_, g)| [g[1], g[2], g[3], g[4]])
                    .unwrap();
                assert_eq!(g[0], v[0], "{}", at("G target"));
                expected.push(g);
            }
        }
        let damage: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::Combat(CombatEvent::Damage(r))
                    if r.inflictor.is_some_and(|i| is_barrel(&s, i)) =>
                {
                    Some([
                        id(&r.target),
                        opt(&r.inflictor),
                        opt(&r.source),
                        i64::from(r.amount),
                    ])
                }
                _ => None,
            })
            .collect();
        assert_eq!(damage, expected, "{}", at("explosion damage"));
        b.damage_records += damage.len();
        // Death chains of barrels.
        for e in &events {
            match e {
                Event::Combat(CombatEvent::Kill { target, .. }) if is_barrel(&s, *target) => {
                    deaths.push(Death {
                        barrel: id(target),
                        killed: op as i64,
                        t0: -1,
                        d: -1,
                        scream: None,
                        explode: None,
                        removed: None,
                        gibbed: None,
                    });
                }
                Event::Combat(CombatEvent::DeathTimer { target, tics }) => {
                    if let Some(d) = deaths
                        .iter_mut()
                        .find(|d| d.barrel == id(target) && d.t0 < 0)
                    {
                        d.t0 = i64::from(*tics);
                    }
                }
                Event::ActorState(lifecycle::ActorStateEvent::Entered { actor, state }) => {
                    let Some(d) = idents
                        .get(actor)
                        .and_then(|i| deaths.iter_mut().find(|d| d.barrel == *i))
                    else {
                        continue;
                    };
                    let slot = match S::try_from(state.0).unwrap() {
                        S::S_BEXP2 => &mut d.scream,
                        S::S_BEXP4 => &mut d.explode,
                        S::S_NULL => &mut d.removed,
                        S::S_GIBS => &mut d.gibbed,
                        _ => continue,
                    };
                    slot.get_or_insert(op as i64);
                }
                _ => {}
            }
        }
        // `d`: the victim's countdown after the killing op is `t0 − 1` when its
        // thinker ran after the kill in that tic (d = 0), `t0` when before (d = 1).
        for d in deaths.iter_mut().filter(|d| d.killed == op as i64) {
            let actor = idents.iter().find(|(_, i)| **i == d.barrel).unwrap().0;
            let tics = s.actor(*actor).unwrap().life().unwrap().tics;
            d.d = i64::from(tics) - d.t0 + 1;
            assert!(matches!(d.d, 0 | 1), "{}", at("death d"));
        }
    }
    walk_trace::disable();
    // The contract's death chains (killed, t0, d, scream, explode, removed, gibbed).
    let case = &contract().get("cases").array()[c.which];
    assert_eq!(case.get("label"), &json::Value::Str(c.label.clone()));
    let expected: Vec<_> = case
        .get("barrel_deaths")
        .array()
        .iter()
        .map(|v| Death {
            barrel: v.get("barrel").int().unwrap(),
            killed: v.get("killed").int().unwrap(),
            t0: v.get("t0").int().unwrap(),
            d: v.get("d").int().unwrap(),
            scream: v.get("scream").int(),
            explode: v.get("explode").int(),
            removed: v.get("removed").int(),
            gibbed: v.get("gibbed").int(),
        })
        .collect();
    assert_eq!(deaths, expected, "{} death chains", c.label);
    b.deaths = deaths.len();
    b.gibbed = deaths.iter().filter(|d| d.gibbed.is_some()).count();
    b
}

/// The full replay: the shared comparisons of [`replay_all_in`] (every M3h9 row
/// kind, the PBE1 frames, every capture, the fixtures, the DSIM37 round trip) and
/// the barrel rows of [`replay_barrels`] for every scenario.
pub(crate) fn replay_all() -> (Totals, Barrels) {
    let t = replay_all_in(&M3H9B_SPEC);
    let mut b = Barrels::default();
    for c in cases() {
        b.add(replay_barrels(&c));
    }
    (t, b)
}

/// The C rows of `tag` (and, for `E`, of `kind`) inside the replayed ops.
fn rows_named(tag: &str, kind: Option<i64>) -> usize {
    let last: BTreeMap<_, _> = cases().iter().map(|c| (c.which, c.tics)).collect();
    rows()
        .iter()
        .filter(|((which, op), _)| last.get(which).is_some_and(|t| op <= t))
        .flat_map(|(_, r)| r)
        .filter(|(t, v)| t == tag && kind.is_none_or(|k| v[0] == k))
        .count()
}

#[test]
fn full_replay_matches_the_c_rows_and_the_independent_dsim37_goldens() {
    let (t, b) = replay_all();
    println!(
        "player_barrels full replay: cases={} ops={} event_frames={} event_records={} captures={} fixtures={} thinker_rows={} crossings={} uses={} blocked={} explosions={} candidates={} not_shootable={} boss={} out_of_range={} sight_blocked={} damaged={} explosion_damage={} spawns={} state_entries={} actions={} explode_actions={} barrel_deaths={} gibbed={} digest={:016x}",
        t.cases,
        t.ops,
        t.event_frames,
        t.event_records,
        t.captures,
        t.fixtures,
        t.thinker_rows,
        t.partial.crossings,
        t.partial.uses,
        t.partial.blocked,
        b.explosions,
        b.candidates(),
        b.codes[0],
        b.codes[1],
        b.codes[2],
        b.codes[3],
        b.codes[4],
        b.damage_records,
        b.spawn_rows,
        b.state_rows,
        b.action_rows,
        b.explode_rows,
        b.deaths,
        b.gibbed,
        t.digest
    );
    // Every case, op, frame, capture, fixture and row was reached.
    assert_eq!(t.cases, 26);
    assert_eq!(t.ops, frames().len());
    assert_eq!(t.event_frames, frames().len());
    assert_eq!(t.captures, captures().len());
    assert_eq!(t.captures, 454);
    assert_eq!(t.fixtures, FIXTURES.len());
    assert_eq!(t.fixtures, 16);
    assert_eq!(t.thinker_rows, rows_named("TK", None));
    assert_eq!(t.button_rows, rows_named("MB", None));
    assert_eq!(t.partial.crossings, rows_named("MX", None));
    assert_eq!(t.partial.blocked, rows_named("MP", None));
    assert_eq!(
        t.partial.uses,
        rows_named("MU", None) + rows_named("MI", None)
    );
    // The barrel rows, against the C row counts and the contract's inventory
    // (27 explosions; 79 candidates: 43 not shootable, 3 out of range, 1 sight
    // blocked, 32 damaged; 28 barrel deaths, 1 gibbed).
    assert_eq!(b.explosions, rows_named("RA", None));
    assert_eq!(b.candidates(), rows_named("RC", None));
    assert_eq!(b.spawn_rows, rows_named("E", Some(1)));
    assert_eq!(b.state_rows, rows_named("E", Some(4)));
    assert_eq!(b.action_rows, rows_named("E", Some(5)));
    let inventory = contract().get("inventory");
    let n = |v: &json::Value| v.int().unwrap() as usize;
    assert_eq!(b.explosions, n(inventory.get("explosions")));
    assert_eq!(b.explosions, 27);
    assert_eq!(b.explode_rows, 27);
    assert_eq!(b.candidates(), n(inventory.get("radius_candidates")));
    assert_eq!(b.candidates(), 79);
    let outcomes = inventory.get("candidate_outcomes");
    assert_eq!(
        b.codes,
        [
            n(outcomes.get("not_shootable")),
            0,
            n(outcomes.get("out_of_range")),
            n(outcomes.get("sight_blocked")),
            n(outcomes.get("damaged")),
        ]
    );
    assert_eq!(b.codes, [43, 0, 3, 1, 32]);
    assert_eq!(b.damage_records, b.codes[4]);
    assert_eq!(b.deaths, n(inventory.get("barrel_deaths")));
    assert_eq!((b.deaths, b.gibbed), (28, 1));
    assert_eq!(b.gibbed, n(inventory.get("barrel_gibs")));
}

/// Rust-only lane `m3h9b_embedded_barrel.bin` (never run in C): a barrel at
/// (−506, 0), radius 10, whose box overlaps the west wall `x = −512` by 4 units.
/// S′ (decision F3) needs `(31 − 10)·w(θ)` = 21 units in front of the wall for a
/// mover whose open box meets it; the centre is 6 units in front, so the spawn
/// batch is refused with `MonsterError::Arithmetic`, nothing spawned and the
/// snapshot and RNG unchanged. The same room with the barrel moved 21 units clear
/// of the wall's line (centre at −491, box clear) spawns it.
#[test]
fn an_embedded_barrel_is_refused_at_spawn_by_s_prime() {
    let lumps = Lumps::parse(include_bytes!(
        "../../../tests/reference/m3h9b_embedded_barrel.bin"
    ));
    let options = || MapContextOptions {
        initial_rng: DoomRng::from_indices(6, 0),
        respawn_monsters: false,
    };
    let mut s = Simulation::original_player_barrels_map(lumps.map(), options()).unwrap();
    let before = s.snapshot();
    let rng = s.rng();
    assert_eq!(
        s.spawn_map_things().map(|_| ()),
        Err(SimError::Monster(MonsterError::Arithmetic))
    );
    assert_eq!(s.snapshot(), before);
    assert_eq!(s.rng(), rng);
    assert_eq!(s.actor_order().count(), 0);
    let mut clear = lumps.clone();
    clear.clear_things();
    clear.add_thing([-491, 0, 0, 2035, 7]);
    let mut s = Simulation::original_player_barrels_map(clear.map(), options()).unwrap();
    assert_eq!(s.spawn_map_things().unwrap().spawned.len(), 1);
}

/// Rust-only lane `m3h9b_intercepts129.bin` (never run in C; the C harness aborts
/// before a 129th intercept): a 2,304-unit lane split by 79 two-sided lines with
/// 64 armor bonuses along it. The player at (−1100, 0) facing east fires the
/// pistol; the 2,048-unit shoot trace crosses 71 of the lines and all 64 bonuses
/// (at least 135 intercepts by geometry), so `P_LineAttack` would write past
/// `intercepts[128]`: the tic is refused with `MapScaleError::Intercepts` through
/// both `advance` and `step`, the snapshot and RNG unchanged, before which every
/// tic is admitted.
#[test]
fn the_intercept_lane_refuses_the_shot_with_the_intercept_witness() {
    let lumps = Lumps::parse(include_bytes!(
        "../../../tests/reference/m3h9b_intercepts129.bin"
    ));
    assert_eq!(lumps.map().linedefs().len(), 241);
    let lane = |lumps: &Lumps, things: usize| {
        let mut s = Simulation::original_player_barrels_map(
            lumps.map(),
            MapContextOptions {
                initial_rng: DoomRng::from_indices(6, 0),
                respawn_monsters: false,
            },
        )
        .unwrap();
        assert_eq!(s.spawn_map_things().unwrap().spawned.len(), things);
        s.spawn_player_body(PlayerSlot::One, [-1100, 0].map(Fixed::from_map_units))
            .unwrap();
        s.initialize_player_weapons(
            PlayerSlot::One,
            WeaponInventory {
                ready: Weapon::Pistol,
                owned: 3,
                ammo: [50, 0, 0, 0],
            },
        )
        .unwrap();
        s.spawn_level_specials().unwrap();
        s
    };
    let mut fire = [TicCommand::default(); 4];
    fire[0].buttons = BT_ATTACK;
    // Control: the same lane without its 64 bonuses (71 line intercepts) admits
    // every tic; its first pistol shot is the tic the full lane refuses.
    let mut bare = lumps.clone();
    bare.clear_things();
    let mut control = lane(&bare, 0);
    let mut shot = None;
    for op in 1..=40 {
        control.advance(fire).unwrap();
        let fired = control.events().iter().any(|e| {
            matches!(e, Event::Combat(CombatEvent::Attack { attack, .. }) if attack.range == Fixed::from_map_units(2048))
        });
        if fired && shot.is_none() {
            shot = Some(op);
        }
    }
    let shot = shot.expect("the control lane fires");
    let mut s = lane(&lumps, 64);
    let mut stepped = s.clone();
    let mut refused = None;
    for op in 1..=40 {
        let before = s.snapshot();
        let rng = s.rng();
        let result = s.advance(fire);
        assert_eq!(result, stepped.step(fire).map(|_| ()), "tic {op}");
        if let Err(error) = result {
            assert_eq!(error, SimError::MapScale(MapScaleError::Intercepts));
            assert_eq!(s.snapshot(), before);
            assert_eq!(stepped.snapshot(), before);
            assert_eq!(s.rng(), rng);
            refused = Some(op);
            break;
        }
    }
    assert_eq!(refused, Some(shot));
}
