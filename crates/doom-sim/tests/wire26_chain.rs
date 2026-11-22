//! Wire 26's carried start against the genuine engine (M6a step 4): for every
//! committed M6a chain item, every level load the genuine session rows record
//! (tests/reference/m6a_session, the `L` rows) is rebuilt under wire 26 from those
//! rows alone (the session layer does not exist yet): the first load as a new game,
//! a later one as `Carried` from the `C` row of the tic before the load (the last
//! intermission tic, whose latches `WI_checkForAccelerate` wrote) or as `Reborn`
//! when that row's player is dead or reborn, each with the load's `P_Random` index
//! (`prndindex_before`). The level's tics are then replayed from the item's demo
//! (one command per session tic, M6a note §1.10) and every genuine row of a level
//! tic is compared, tic for tic:
//!
//! - the `L` row's `prndindex_after` with the index after the Rust load (the load's
//!   draws);
//! - every `G` row's `leveltime`, `prndindex`, `gameaction` (6, `ga_completed`, is
//!   an exit request) and `secretexit` with the simulation after that tic;
//! - every `C` row's player fields the simulation models (`playerstate`, `health`,
//!   armour, weapons, ammunition, maxima, backpack, keys, powers, `cheats`, the
//!   latches, the three counts, `refire`, `damagecount`, `bonuscount`);
//!   `viewheight`, `deltaviewheight` and `didsecret` are not simulation state and
//!   are not compared;
//! - every genuine sound start and stop of the tic (`S` and `T` rows of
//!   tests/reference/m6a_sound, in `seq` order) with the tic's sound records: kind,
//!   `sfx`, origin kind and, for a placed start, the five positions; the load's
//!   `sfx_sawup` is the first `S` row of the load's `G_Ticker`;
//! - after the level's last tic, the refusal of the next tic (`LevelExited` or
//!   `PlayerReborn`) and [`LevelFinish`]'s requests against the last rows.
//!
//! The genuine rows do not record `bob`, `psprites[ps_weapon].sx` or the flash
//! psprite's tics and coordinates, which a carried start also holds; they are built
//! as 0 here. No compared column reads them: `P_CalcHeight` recomputes `bob` before
//! any reader (p_user.c), and the psprite coordinates and a null flash's count are
//! read only by the drawing (p_pspr.c:331-334, 851-883). Every map of the items loads
//! under wire 26 (asserted): the walk exits 52 and 124 that wire 25 refuses are wire
//! 26's (p_spec.c:677-680, 760-763), so the levels ending by a walk exit, the rbx
//! dead-body crossings included, are replayed like the others.
//!
//! Genuine evidence for the carried start, from committed genuine rows; not a
//! comparison of whole DSIM captures (the genuine dumps of later levels are not
//! committed).

mod wire26;

use doom_sim::{demo::parse_demo_episode1_v2, sound::SoundOrigin, sound::SoundRecord, *};
use doom_types::Fixed;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// (lump, the WAD holding its maps) of every committed M6a chain item.
const ITEMS: [(&str, &str); 18] = [
    ("CHAIN", "m6a_wads/episode.wad"),
    ("INTSTA", "m6a_wads/episode.wad"),
    ("INTSTB", "m6a_wads/episode.wad"),
    ("INTIDLE", "m6a_wads/episode.wad"),
    ("INTHELD", "m6a_wads/episode.wad"),
    ("INTFAST", "m6a_wads/episode.wad"),
    ("REBHELD", "m6a_wads/episode.wad"),
    ("REBLATE", "m6a_wads/episode.wad"),
    ("SECRET", "m6a_wads/episode.wad"),
    ("REBSECRT", "m6a_wads/episode.wad"),
    ("PROBE", "m6a_wads/rbx.wad"),
    ("USEAT130", "m6a_wads/rbx.wad"),
    ("USEAT131", "m6a_wads/rbx.wad"),
    ("USEAT132", "m6a_wads/rbx.wad"),
    ("RBXDEAD", "m6a_wads/rbx.wad"),
    ("NOINTER", "m6a_wads/episode.wad"),
    ("NOE1M2", "m6a_wads/episode.wad"),
    ("NOE1M2AT", "m6a_wads/episode.wad"),
];

/// The rows of one genuine stream, by kind, as name → value maps.
struct Rows {
    by_kind: BTreeMap<String, Vec<BTreeMap<String, String>>>,
}
impl Rows {
    fn parse(text: &str) -> Self {
        let mut columns: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut by_kind: BTreeMap<String, Vec<BTreeMap<String, String>>> = BTreeMap::new();
        for line in text.lines() {
            let f: Vec<&str> = line.split(',').collect();
            if f[0] == "#columns" {
                columns.insert(
                    f[1].to_owned(),
                    f[2..].iter().map(|s| (*s).to_owned()).collect(),
                );
                continue;
            }
            if f[0].starts_with('#') {
                continue;
            }
            if let Some(names) = columns.get(f[0]) {
                if names.len() + 1 == f.len() {
                    let row = names
                        .iter()
                        .zip(&f[1..])
                        .map(|(n, v)| (n.clone(), (*v).to_owned()))
                        .collect();
                    by_kind.entry(f[0].to_owned()).or_default().push(row);
                }
            }
        }
        Self { by_kind }
    }
    fn of(&self, kind: &str) -> &[BTreeMap<String, String>] {
        self.by_kind.get(kind).map_or(&[], Vec::as_slice)
    }
    fn at(&self, kind: &str, tic: i64) -> Option<&BTreeMap<String, String>> {
        self.of(kind).iter().find(|r| int(r, "tic") == tic)
    }
}

fn int(row: &BTreeMap<String, String>, name: &str) -> i64 {
    row.get(name)
        .unwrap_or_else(|| panic!("column {name}"))
        .parse()
        .unwrap_or_else(|_| panic!("column {name}: {:?}", row.get(name)))
}

/// The carried start the genuine `C` row of the tic before a load describes.
fn carried_from(c: &BTreeMap<String, String>, p_random: u8) -> CarriedStart {
    let i = |n: &str| i32::try_from(int(c, n)).expect("i32");
    let armor = PlayerArmorState {
        kind: match i("armortype") {
            0 => ArmorType::None,
            1 => ArmorType::Green,
            2 => ArmorType::Blue,
            other => panic!("armortype {other}"),
        },
        points: i("armorpoints"),
    };
    let ready = match i("readyweapon") {
        0 => Weapon::Fist,
        1 => Weapon::Pistol,
        2 => Weapon::Shotgun,
        3 => Weapon::Chaingun,
        4 => Weapon::Rocket,
        7 => Weapon::Chainsaw,
        other => panic!("readyweapon {other}"),
    };
    CarriedStart {
        health: i("health"),
        armor,
        weapons_owned: u16::try_from(i("weaponowned")).expect("bits"),
        ready_weapon: ready,
        ammo: [i("ammo0"), i("ammo1"), i("ammo2"), i("ammo3")],
        max_ammo: [i("maxammo0"), i("maxammo1"), i("maxammo2"), i("maxammo3")],
        backpack: i("backpack") != 0,
        cheats: i("cheats"),
        attack_down: i("attackdown") != 0,
        use_down: i("usedown") != 0,
        bob: Fixed::ZERO,
        weapon_x: Fixed::ZERO,
        flash_tics: 0,
        flash_position: [Fixed::ZERO; 2],
        p_random,
    }
}

/// The simulation's values of the compared `C` columns, by column name.
fn player_columns(s: &Simulation) -> BTreeMap<&'static str, i64> {
    let p = s.original_player(PlayerSlot::One).expect("player");
    let w = s
        .original_player_weapons(PlayerSlot::One)
        .expect("weapons")
        .expect("pack");
    let armor = s.original_player_armor(PlayerSlot::One).expect("armor");
    let powers = s.original_player_powers(PlayerSlot::One).expect("powers");
    let pickups = s.original_player_pickups(PlayerSlot::One).expect("pickups");
    let finish = s.level_finish().expect("finish");
    let stats = s.level_statistics().expect("statistics");
    let state = match (p.state, finish.reborn) {
        (PlayerBodyState::Live, _) => 0,
        (PlayerBodyState::Dead, false) => 1,
        (PlayerBodyState::Dead, true) => 2,
    };
    let mut m = BTreeMap::new();
    m.insert("playerstate", state);
    m.insert("health", i64::from(p.health));
    m.insert("armorpoints", i64::from(armor.points));
    m.insert("armortype", i64::from(armor.kind.wire_id()));
    m.insert("readyweapon", w.ready as i64);
    m.insert("pendingweapon", w.pending.map_or(10, |x| x as i64));
    m.insert("weaponowned", i64::from(w.owned));
    m.insert("ammo0", i64::from(w.bullets));
    m.insert("ammo1", i64::from(w.shells));
    m.insert("ammo2", i64::from(w.cells));
    m.insert("ammo3", i64::from(w.rockets));
    let max = [200_i64, 50, 300, 50].map(|v| v << u32::from(w.backpack));
    for (k, v) in max.iter().enumerate() {
        m.insert(["maxammo0", "maxammo1", "maxammo2", "maxammo3"][k], *v);
    }
    m.insert("backpack", i64::from(w.backpack));
    let cards = powers
        .cards
        .iter()
        .enumerate()
        .map(|(k, c)| i64::from(*c) << k)
        .sum();
    m.insert("cards", cards);
    for (k, v) in powers.powers.iter().enumerate() {
        m.insert(["pw0", "pw1", "pw2", "pw3", "pw4", "pw5"][k], i64::from(*v));
    }
    m.insert("cheats", 0);
    m.insert("attackdown", i64::from(w.attack_down));
    // `usedown` is read through what a living player carries.
    if let Some(c) = finish.carried {
        m.insert("usedown", i64::from(c.use_down));
    }
    m.insert("killcount", i64::from(stats.single_player_kill_count));
    m.insert("itemcount", i64::from(pickups.item_count));
    m.insert(
        "secretcount",
        i64::from(s.original_player_secret_count().expect("secrets")),
    );
    m.insert("refire", i64::from(w.refire));
    m.insert("damagecount", i64::from(p.damage_count));
    m.insert("bonuscount", i64::from(pickups.bonus_count));
    m
}

/// One record as the genuine stream's columns describe it: (row kind, sfx, origin
/// kind, placement).
type Shape = (char, i64, String, Option<[i64; 5]>);

fn record_shape(r: &SoundRecord, player: Option<ActorId>) -> Shape {
    match r {
        SoundRecord::Start { sfx, origin, at } => {
            let kind = match origin {
                SoundOrigin::None => "none",
                SoundOrigin::Actor(id) if Some(*id) == player => "player",
                SoundOrigin::Actor(_) => "mobj",
                SoundOrigin::Sector(_) => "sector",
                SoundOrigin::Button(_) => "button",
            };
            let at = at.map(|a| {
                [
                    i64::from(a.origin[0]),
                    i64::from(a.origin[1]),
                    i64::from(a.listener[0]),
                    i64::from(a.listener[1]),
                    i64::from(a.listener_angle),
                ]
            });
            ('S', i64::from(*sfx), kind.to_owned(), at)
        }
        SoundRecord::Stop { actor } => (
            'T',
            0,
            if Some(*actor) == player {
                "player"
            } else {
                "mobj"
            }
            .to_owned(),
            None,
        ),
    }
}

fn genuine_shapes(sound: &Rows, tic: i64) -> Vec<(i64, Shape)> {
    let mut out = Vec::new();
    for row in sound.of("S").iter().filter(|r| int(r, "tic") == tic) {
        let at =
            (row["ox"] != "-").then(|| ["ox", "oy", "lx", "ly", "langle"].map(|k| int(row, k)));
        out.push((
            int(row, "seq"),
            ('S', int(row, "sfx"), row["okind"].clone(), at),
        ));
    }
    for row in sound.of("T").iter().filter(|r| int(r, "tic") == tic) {
        out.push((int(row, "seq"), ('T', 0, row["okind"].clone(), None)));
    }
    out.sort_by_key(|(seq, _)| *seq);
    out
}

/// The independent encoder's start groups (tests/reference/m6a_dsim42_starts.txt,
/// tools/reference/level_snapshot_m6a.py from the genuine session and sound rows),
/// by (item, load): (player slot, load sawups, framed start-group bytes in hex).
fn encoder_starts() -> BTreeMap<(String, usize), (u32, usize, String)> {
    let text = std::fs::read_to_string(root().join("tests/reference/m6a_dsim42_starts.txt"))
        .expect("encoder results");
    let mut out = BTreeMap::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split(',').collect();
        assert_eq!(f.len(), 10, "{line}");
        out.insert(
            (f[0].to_owned(), f[1].parse().expect("load")),
            (
                f[7].parse().expect("slot"),
                f[8].parse().expect("sawups"),
                f[9].to_owned(),
            ),
        );
    }
    out
}

/// The start group of a capture in DSIM framing without the header, in hex.
fn start_group_hex(s: &Simulation) -> String {
    let mut out = String::new();
    for (path, value) in wire26::start_group(&s.snapshot()) {
        let mut b = (path.len() as u32).to_le_bytes().to_vec();
        b.extend(path.as_bytes());
        b.extend((value.len() as u64).to_le_bytes());
        b.extend(value);
        for x in b {
            out.push_str(&format!("{x:02x}"));
        }
    }
    out
}

#[derive(Default)]
struct Counts {
    items: usize,
    loads: usize,
    replayed: usize,
    refused_maps: usize,
    kinds: [usize; 3],
    tics: usize,
    rows: usize,
    sounds: usize,
    sawups: usize,
    ends: usize,
    groups: usize,
    differences: Vec<String>,
}

/// A fault planted in the carried start built from the rows, to prove the
/// comparison can fail.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Fault {
    None,
    /// The carried `P_Random` index one higher.
    Random,
    /// One point of carried health less.
    Health,
    /// The carried `attackdown` latch inverted.
    AttackDown,
    /// The carried ready weapon the pistol (a carried chainsaw loses its sawup).
    Pistol,
}

fn plant(start: LevelStart, fault: Fault) -> LevelStart {
    let LevelStart::Carried(mut c) = start else {
        return start;
    };
    match fault {
        Fault::None => {}
        Fault::Random => c.p_random = c.p_random.wrapping_add(1),
        Fault::Health => c.health -= 1,
        Fault::AttackDown => c.attack_down = !c.attack_down,
        Fault::Pistol => c.ready_weapon = Weapon::Pistol,
    }
    LevelStart::Carried(c)
}

fn replay_item(lump: &str, wad_path: &str, counts: &mut Counts) {
    replay_item_with(lump, wad_path, counts, Fault::None);
}

fn replay_item_with(lump: &str, wad_path: &str, counts: &mut Counts, fault: Fault) {
    let reference = root().join("tests/reference");
    let session = Rows::parse(
        &std::fs::read_to_string(reference.join(format!("m6a_session/{lump}.session")))
            .expect("session golden"),
    );
    let sound = Rows::parse(
        &std::fs::read_to_string(reference.join(format!("m6a_sound/{lump}.sound")))
            .expect("sound golden"),
    );
    let demo = std::fs::read(reference.join(format!("m6a_scripts/{lump}.lmp"))).expect("demo");
    let (_, demo) = parse_demo_episode1_v2(&demo, demo[3]).expect("demo parse");
    let bytes = std::fs::read(reference.join(wad_path)).expect("wad");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("wad parse");
    counts.items += 1;
    let loads = session.of("L").to_vec();
    for (k, l) in loads.iter().enumerate() {
        counts.loads += 1;
        let (u, map_number, skill) = (int(l, "tic"), int(l, "map") as u8, int(l, "skill") as u8);
        let p_random = int(l, "prndindex_before") as u8;
        let start = if k == 0 {
            assert_eq!(p_random, 0, "{lump}: a new game starts at index 0");
            LevelStart::NewGame
        } else {
            let c = session.at("C", u - 1).expect("the C row before the load");
            if int(c, "playerstate") == 0 {
                LevelStart::Carried(carried_from(c, p_random))
            } else {
                LevelStart::Reborn { p_random }
            }
        };
        let start = plant(start, fault);
        let map = doom_map::Map::from_wad(
            &wad,
            &format!("E1M{map_number}"),
            doom_map::MapLimits::default(),
        )
        .expect("map");
        // The load's `gametic` (`levelstarttic`, g_game.c:470) from the genuine L row.
        let mut sim = match Simulation::original_player_episode1_v4_level_at(
            map,
            map_number,
            wire26::skill(skill),
            MapContextOptions::default(),
            start,
            int(l, "levelstarttic") as u64,
        ) {
            Ok(sim) => sim,
            Err(LevelStartError::Registry(e)) => {
                println!("{lump} load {k} (E1M{map_number}, tic {u}): refused at load, {e:?}");
                counts.refused_maps += 1;
                continue;
            }
            Err(e) => panic!("{lump} load {k}: {e:?}"),
        };
        counts.replayed += 1;
        counts.kinds[usize::from(start.kind().wire_id())] += 1;
        let mut diff = |what: String| counts.differences.push(format!("{lump} load {k}: {what}"));
        let (slot, sawups, hex) = encoder_starts()
            .remove(&(lump.to_owned(), k))
            .expect("the encoder's start group of every load");
        let mo = sim
            .original_player(PlayerSlot::One)
            .expect("player")
            .mo
            .expect("body");
        if start_group_hex(&sim) != hex
            || mo.slot() != slot
            || sim.load_sound_records().len() != sawups
        {
            diff("the start group differs from the encoder's".into());
        }
        counts.groups += 1;
        if i64::from(sim.rng().indices()[0]) != int(l, "prndindex_after") {
            diff(format!(
                "after the load P_Random index {} against {}",
                sim.rng().indices()[0],
                l["prndindex_after"]
            ));
        }
        let saw = sim.load_sound_records().len();
        let player = sim.original_player(PlayerSlot::One).expect("player").mo;
        let mut s = u.max(0);
        let mut level_tics = 0;
        while let Some(g) = session.at("G", s) {
            if int(g, "gamestate") != 0 || (s > u.max(0) && session.at("L", s).is_some()) {
                break;
            }
            let Some(c) = session.at("C", s) else { break };
            let cmd = *demo
                .commands
                .get(s as usize)
                .expect("a command for every tic");
            let cmd = [
                cmd,
                TicCommand::default(),
                TicCommand::default(),
                TicCommand::default(),
            ];
            if let Err(e) = sim.advance(cmd) {
                diff(format!("session tic {s}: refused {e:?}"));
                break;
            }
            level_tics += 1;
            counts.tics += 1;
            let finish = sim.level_finish().expect("finish");
            // `gameaction` 6 (`ga_completed`) is the exit request. The C's
            // `secretexit` is a session global, rewritten at every exit
            // (g_game.c:1004, 1014-1016) and read only between an exit and the next
            // load (G_DoCompleted 1071, 1087; G_WorldDone 1151, 1160), so it keeps a
            // previous level's value through the next level; the simulation's flag
            // is this level's request: compared on the exit tic, 0 on every other.
            let exit_row = int(g, "gameaction") == 6;
            let checks = [
                ("leveltime", sim.tic() as i64, int(g, "leveltime")),
                (
                    "prndindex",
                    i64::from(sim.rng().indices()[0]),
                    int(g, "prndindex"),
                ),
                (
                    "gameaction 6",
                    i64::from(finish.exit.is_some()),
                    i64::from(exit_row),
                ),
                (
                    "secretexit",
                    i64::from(finish.exit == Some(LevelExit::Secret)),
                    int(g, "secretexit") * i64::from(exit_row),
                ),
            ];
            for (name, value, want) in checks {
                if want != value {
                    diff(format!("session tic {s}: G {name} {value} against {want}"));
                }
            }
            for (name, value) in player_columns(&sim) {
                if int(c, name) != value {
                    diff(format!(
                        "session tic {s}: C {name} {value} against {}",
                        c[name]
                    ));
                }
            }
            counts.rows += 2;
            let genuine = genuine_shapes(&sound, s);
            let rust: Vec<Shape> = sim
                .sound_records()
                .iter()
                .map(|r| record_shape(r, player))
                .collect();
            let genuine: Vec<Shape> = genuine.into_iter().map(|(_, x)| x).collect();
            if genuine != rust {
                diff(format!(
                    "session tic {s}: sound {rust:?} against {genuine:?}"
                ));
            }
            counts.sounds += rust.len();
            s += 1;
        }
        if level_tics > 0 && saw > 0 {
            counts.sawups += saw;
        }
        // The next session tic is not a level tic of this load: the simulation must
        // refuse it with the request the last rows show.
        let last = s - 1;
        if let (Some(g), Some(c)) = (session.at("G", last), session.at("C", last)) {
            if session.at("G", s).is_some() && level_tics > 0 {
                let finish = sim.level_finish().expect("finish");
                let exit = int(g, "gameaction") == 6;
                let reborn = int(c, "playerstate") == 2;
                let refusal = sim.clone().advance([TicCommand::default(); 4]);
                let expected = if exit {
                    Err(SimError::LevelExited)
                } else if reborn {
                    Err(SimError::PlayerReborn)
                } else {
                    Ok(())
                };
                if refusal != expected || finish.exit.is_some() != exit || finish.reborn != reborn {
                    diff(format!(
                        "after session tic {last}: {refusal:?}, exit {:?}, reborn {} against \
                         exit {exit}, reborn {reborn}",
                        finish.exit, finish.reborn
                    ));
                }
                counts.ends += 1;
            }
        }
        println!(
            "{lump} load {k} (E1M{map_number}, tic {u}, {:?}): {level_tics} tics, {saw} load \
             sound(s)",
            start.kind()
        );
    }
}

#[test]
fn every_carried_start_replays_the_genuine_rows() {
    let mut counts = Counts::default();
    for (lump, wad) in ITEMS {
        replay_item(lump, wad, &mut counts);
    }
    println!(
        "wire26_chain: {} items, {} loads: {} replayed (new game {}, carried {}, reborn {}), {} \
         refused at load by wire 26 as by wire 25; {} level tics, {} G and C rows, {} sound \
         records, {} load sawups, {} level ends, {} start groups equal to the encoder's; {} \
         differences",
        counts.items,
        counts.loads,
        counts.replayed,
        counts.kinds[0],
        counts.kinds[1],
        counts.kinds[2],
        counts.refused_maps,
        counts.tics,
        counts.rows,
        counts.sounds,
        counts.sawups,
        counts.ends,
        counts.groups,
        counts.differences.len()
    );
    for d in counts.differences.iter().take(40) {
        println!("  {d}");
    }
    assert!(
        counts.differences.is_empty(),
        "{} differences",
        counts.differences.len()
    );
    assert_eq!(
        counts.refused_maps, 0,
        "every map of the items loads under wire 26"
    );
}

/// The comparison can fail: each planted fault in CHAIN's carried starts is reported,
/// first at the first carried load (load 1, E1M2, whose carried ready weapon is
/// the chainsaw), and nowhere in the new game's level.
#[test]
fn planted_faults_in_the_carried_start_are_caught() {
    for fault in [
        Fault::Random,
        Fault::Health,
        Fault::AttackDown,
        Fault::Pistol,
    ] {
        let mut counts = Counts::default();
        replay_item_with("CHAIN", "m6a_wads/episode.wad", &mut counts, fault);
        let first = counts.differences.first().cloned().unwrap_or_default();
        println!(
            "{fault:?}: {} differences, first: {first}",
            counts.differences.len()
        );
        assert!(first.starts_with("CHAIN load 1:"), "{fault:?}: {first}");
        assert!(
            !counts
                .differences
                .iter()
                .any(|d| d.starts_with("CHAIN load 0:"))
        );
    }
}

/// `rbx.wad` with every line special 52 of `E1M1` made 124 (the W1 secret exit),
/// patched in memory: the LINEDEFS record is 14 bytes, the special at offset 6.
fn rbx_with_124() -> Vec<u8> {
    let mut bytes = std::fs::read(root().join("tests/reference/m6a_wads/rbx.wad")).expect("wad");
    let at = |b: &[u8], o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) as usize;
    let (count, dir) = (at(&bytes, 4), at(&bytes, 8));
    let name = |b: &[u8], k: usize| b[dir + 16 * k + 8..dir + 16 * k + 16].to_vec();
    let map = (0..count)
        .find(|&k| name(&bytes, k).starts_with(b"E1M1"))
        .expect("E1M1");
    assert!(name(&bytes, map + 2).starts_with(b"LINEDEFS"));
    let (pos, size) = (
        at(&bytes, dir + 16 * (map + 2)),
        at(&bytes, dir + 16 * (map + 2) + 4),
    );
    let mut patched = 0;
    for r in (pos..pos + size).step_by(14) {
        if i16::from_le_bytes([bytes[r + 6], bytes[r + 7]]) == 52 {
            bytes[r + 6..r + 8].copy_from_slice(&124_i16.to_le_bytes());
            patched += 1;
        }
    }
    assert!(patched > 0);
    bytes
}

/// Line special 124 is 52 with `G_SecretExitLevel` (p_spec.c:677-680, 760-763;
/// g_game.c:1002-1018, a non-commercial game takes the secret exit): on the PROBE
/// demo, whose dead player's body slides across the walk exit (the genuine rows put
/// the exit at session tic 131, compared by the test above), the patched map exits
/// on the same tic as the committed one, with the secret flag, every other state
/// field equal, and neither special ever cleared. Wire 25 refuses both maps at load.
#[test]
fn walk_secret_exit_124_is_52_with_the_secret_flag() {
    let reference = root().join("tests/reference");
    let plain = std::fs::read(reference.join("m6a_wads/rbx.wad")).expect("wad");
    let secret = rbx_with_124();
    let demo = std::fs::read(reference.join("m6a_scripts/PROBE.lmp")).expect("demo");
    let (_, demo) = parse_demo_episode1_v2(&demo, 1).expect("demo parse");
    let load = |bytes: &[u8]| {
        let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).expect("wad parse");
        doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).expect("map")
    };
    type Tic = (
        Option<LevelExit>,
        bool,
        Vec<(String, Vec<u8>)>,
        Vec<Vec<u8>>,
    );
    let mut runs: Vec<Vec<Tic>> = Vec::new();
    for bytes in [&plain, &secret] {
        let refused = Simulation::original_player_episode1_v3_map(
            load(bytes),
            1,
            Skill::Medium,
            MapContextOptions::default(),
        );
        assert_eq!(
            refused.err(),
            Some(OriginalRegistryError::Runtime(SimError::Pursuit(
                PursuitError::Specials
            )))
        );
        let mut sim = Simulation::original_player_episode1_v4_level(
            load(bytes),
            1,
            Skill::Medium,
            MapContextOptions::default(),
            LevelStart::NewGame,
        )
        .expect("wire 26 loads a walk exit");
        let mut trace = Vec::new();
        for c in &demo.commands {
            let cmd = [
                *c,
                TicCommand::default(),
                TicCommand::default(),
                TicCommand::default(),
            ];
            if sim.advance(cmd).is_err() {
                break;
            }
            let f = sim.level_finish().expect("finish");
            let snapshot = sim.snapshot();
            let fields = snapshot
                .fields()
                .filter(|f| {
                    let p = f.path();
                    !p.starts_with("map.")
                        && !p.starts_with("line.")
                        && !p.starts_with("events.")
                        && p != "level.secret_exit"
                })
                .map(|f| (f.path().to_owned(), f.value().to_vec()))
                .collect();
            let specials = snapshot
                .fields()
                .filter(|f| f.path().starts_with("line.") && f.path().ends_with(".special"))
                .map(|f| f.value().to_vec())
                .collect();
            trace.push((f.exit, f.player_dead, fields, specials));
        }
        runs.push(trace);
    }
    let (a, b) = (&runs[0], &runs[1]);
    assert_eq!(a.len(), b.len());
    let exit = a.iter().position(|t| t.0.is_some()).expect("the walk exit");
    assert_eq!(exit, 131, "session tic 131, as the genuine PROBE rows");
    assert!(a[exit].1, "a dead player's body takes the exit");
    for (k, (x, y)) in a.iter().zip(b).enumerate() {
        assert_eq!(x.2, y.2, "tic {k}");
        assert_eq!(x.0.is_some(), y.0.is_some(), "tic {k}");
        assert!(x.3.iter().all(|v| v == &52_i16.to_le_bytes()), "tic {k}");
        assert!(y.3.iter().all(|v| v == &124_i16.to_le_bytes()), "tic {k}");
    }
    assert_eq!(a[exit].0, Some(LevelExit::Normal));
    assert_eq!(b[exit].0, Some(LevelExit::Secret));
    println!(
        "PROBE: exit on tic {exit} with 52 (normal) and 124 (secret), {} tics",
        a.len()
    );
}
