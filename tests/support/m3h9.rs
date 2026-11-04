//! M3h9 two-sided scrolling walls, static decorations and spawn-filter inputs and
//! committed source rows, read through the public API only (the asserted example
//! and measurements). The full replay, which also needs the crate-internal
//! controlled health of `script` 30 and compares the thinker list, the PBE1 event
//! frames and the pinned DSIM36 fixtures, is the library test `e1m1_replay`.
#![allow(dead_code)]
#[allow(clippy::duplicate_mod)]
#[path = "evidence_pack.rs"]
mod packed;
use doom_sim::{identity::*, *};
use doom_types::{DoomRng, Fixed};
use std::{collections::BTreeMap, sync::OnceLock};
include!("../reference/m3h9_maps.rs");

#[derive(Clone, Debug)]
pub struct Case {
    pub which: usize,
    pub label: String,
    pub tics: usize,
    pub position: [i16; 2],
    pub seed: u8,
    pub reaction: u8,
    pub clips: i32,
    pub ready: Weapon,
    pub owned: u16,
    /// Controlled between-tic player target plus S_TROO_ATK1 entry.
    pub attack_at: usize,
    pub attack_thing: usize,
    /// Controlled between-tic player target plus see-state entry (S_POSS_RUN1).
    pub chase_at: usize,
    pub chase_thing: usize,
    /// 30: the declared controlled 200 health of the player and body
    /// (crate-internal; not replayed here).
    pub script: u8,
    /// The genuine `gameskill` of the C case: 2 (medium) for every scenario Rust
    /// replays; 1 and 3 are the C-only census rows.
    pub skill: u8,
    pub map: usize,
}
type Rows = BTreeMap<(usize, usize), Vec<(String, Vec<i64>)>>;
/// Every row of the committed C trace keyed by (case, op).
pub fn all_rows() -> &'static Rows {
    static DATA: OnceLock<Rows> = OnceLock::new();
    DATA.get_or_init(|| {
        let text = packed::unpack(include_bytes!("../reference/m3h9_trace.dref")).unwrap();
        let mut rows = Rows::new();
        for line in std::str::from_utf8(&text).unwrap().lines().skip(1) {
            let mut fields = line.split(',');
            let tag = fields.next().unwrap().to_string();
            let values: Vec<i64> = fields.map(|f| f.parse().unwrap()).collect();
            if values.len() >= 2 {
                rows.entry((values[0] as usize, values[1] as usize))
                    .or_default()
                    .push((tag, values[2..].to_vec()));
            }
        }
        rows
    })
}
pub fn rows(which: usize, op: usize) -> &'static [(String, Vec<i64>)] {
    all_rows().get(&(which, op)).map_or(&[], Vec::as_slice)
}
pub fn row<'a>(rows: &'a [(String, Vec<i64>)], tag: &str) -> Option<&'a [i64]> {
    rows.iter()
        .find(|(t, _)| t == tag)
        .map(|(_, v)| v.as_slice())
}
/// Every committed case, the C-only skills included.
pub fn all_cases() -> Vec<Case> {
    let mut lines = include_str!("../reference/m3h9_cases.csv").lines();
    let header: Vec<_> = lines.next().unwrap().split(',').collect();
    lines
        .enumerate()
        .map(|(which, line)| {
            let v: BTreeMap<_, _> = header.iter().copied().zip(line.split(',')).collect();
            let n = |k: &str| v[k].parse::<i64>().unwrap();
            let last = all_rows()
                .range((which, 0)..(which + 1, 0))
                .filter(|(_, r)| r.iter().any(|(t, _)| t == "UC"))
                .map(|((_, op), _)| *op)
                .max()
                .unwrap();
            Case {
                which,
                label: v["label"].into(),
                tics: (n("tics") as usize).min(last),
                position: [n("px") as i16, n("py") as i16],
                seed: n("seed") as u8,
                reaction: n("reaction") as u8,
                clips: n("clips") as i32,
                ready: match n("ready") {
                    0 => Weapon::Fist,
                    1 => Weapon::Pistol,
                    2 => Weapon::Shotgun,
                    _ => Weapon::Chaingun,
                },
                owned: n("owned") as u16,
                attack_at: n("attack_at") as usize,
                attack_thing: n("attack_thing") as usize,
                chase_at: n("chase_at") as usize,
                chase_thing: n("chase_thing") as usize,
                script: n("script") as u8,
                skill: n("skill") as u8,
                map: n("map") as usize,
            }
        })
        .collect()
}
/// The 18 medium-skill cases (the policy's skill); the E1M1 policy loads every one.
pub fn cases() -> Vec<Case> {
    all_cases().into_iter().filter(|c| c.skill == 2).collect()
}
pub fn case(label: &str) -> Case {
    all_cases().into_iter().find(|c| c.label == label).unwrap()
}
pub fn case_map(index: usize) -> doom_map::Map {
    let wad = doom_wad::Wad::parse(MAP_BYTES[index], doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
pub fn options(seed: u8) -> MapContextOptions {
    MapContextOptions {
        initial_rng: DoomRng::from_indices(seed, 0),
        respawn_monsters: false,
    }
}
/// `m3h9_snapshots.csv`: (case, op) to the independent capture digest.
pub fn capture_digests() -> BTreeMap<(usize, usize), u64> {
    include_str!("../reference/m3h9_snapshots.csv")
        .lines()
        .skip(1)
        .map(|line| {
            let v: Vec<_> = line.split(',').collect();
            (
                (v[0].parse().unwrap(), v[1].parse().unwrap()),
                u64::from_str_radix(v[3], 16).unwrap(),
            )
        })
        .collect()
}
/// The C driver's setup: admission, THINGs, the player body, weapons,
/// `P_SpawnSpecials`, the reaction time.
pub fn setup(c: &Case) -> Result<(Simulation, ActorId, Vec<ActorId>), SimError> {
    let mut s = Simulation::original_player_e1m1_map(case_map(c.map), options(c.seed))
        .expect("committed room admission");
    let things = s.spawn_map_things()?.spawned;
    let body = s.spawn_player_body(PlayerSlot::One, c.position.map(Fixed::from_map_units))?;
    s.initialize_player_weapons(
        PlayerSlot::One,
        WeaponInventory {
            ready: c.ready,
            owned: c.owned,
            ammo: [c.clips, 0, 0, 0],
        },
    )?;
    s.spawn_level_specials()?;
    s.set_initial_player_reaction_time(PlayerSlot::One, c.reaction)?;
    Ok((s, body, things))
}
/// The case's command at `op` (its `UC` row).
pub fn command(c: &Case, op: usize) -> [TicCommand; 4] {
    let v = row(rows(c.which, op), "UC").expect("UC row");
    let mut commands = [TicCommand::default(); 4];
    commands[0] = TicCommand {
        forward: v[0] as i8,
        side: v[1] as i8,
        angle_turn: v[2] as i16,
        buttons: v[3] as u8,
    };
    commands
}
/// The declared between-tic inputs through the public API: the attack entry, then
/// the see-state entry (no M3h9 case declares a controlled damage).
pub fn controlled(
    s: &mut Simulation,
    c: &Case,
    body: ActorId,
    things: &[ActorId],
    op: usize,
) -> Result<(), SimError> {
    if op == c.attack_at {
        let imp = things[c.attack_thing];
        s.set_actor_target(imp, Some(body))?;
        s.enter_original_state(imp, OriginalState::S_TROO_ATK1)
            .expect("declared controlled attack entry");
    }
    if op == c.chase_at {
        let zombie = things[c.chase_thing];
        s.set_actor_target(zombie, Some(body))?;
        s.enter_original_state(zombie, OriginalState::S_POSS_RUN1)
            .expect("declared controlled see-state entry");
    }
    Ok(())
}
/// A deterministic patrol command stream (xorshift): walking, turning, firing and
/// `BT_USE` in bursts; a dead player gets the empty command.
pub struct Patrol(pub u32);
impl Patrol {
    pub fn command(&mut self, dead: bool) -> [TicCommand; 4] {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        let r = self.0;
        let mut commands = [TicCommand::default(); 4];
        if !dead {
            commands[0] = TicCommand {
                forward: ((r >> 8) % 51) as i8 - 20,
                side: ((r >> 14) % 21) as i8 - 10,
                angle_turn: (((r >> 19) % 9) as i16 - 4) * 1024,
                buttons: u8::from(r % 7 == 0) | (u8::from((r >> 3) % 9 == 0) << 1),
            };
        }
        commands
    }
}
