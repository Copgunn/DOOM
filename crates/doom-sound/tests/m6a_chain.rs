//! M6a step 6: the sound session across levels ([`doom_sound::chain::ChainSound`]) against the
//! genuine engine's goldens (`tests/reference/m6a_sound`, the genuine M6a runner;
//! `tests/reference/m6a_mixer`, the genuine M5c mixer on those streams; both from
//! `tools/reference/m6a_goldens.py` and `tools/reference/m6a_mixer.py`, never from Rust output),
//! `DSND` version 2, and the refusals.
//!
//! What is driven and what is projected (stated per test):
//!
//! - **The first level of each item is driven** by the wire-25 simulation from the item's demo, its
//!   records, listener and positions fed to the session: genuine rows against Rust rows. A map
//!   with a W1 exit line 52, which wire 25 refuses at load (`Pursuit(Specials)`), runs as a
//!   *carrier* with that special cleared (crossing it changes nothing else in the tic, p_spec.c:
//!   677-680: it only sets `gameaction`); its exit tic is then taken from the genuine session rows
//!   (a projection), while for every other map the simulation's own `LevelExited` or
//!   `PlayerReborn` refusal of the next tic must fall on the genuine boundary.
//! - **Intermission and finale tics are driven** against the finished level's simulation (frozen
//!   listener and origins), with the intermission's start requests **projected** from the genuine
//!   S rows of each tic (the game layer's intermission logic is step 7) and the `M_Random` index
//!   **projected** from the genuine R rows (the display owns it; `WI_Start`'s ten animation draws
//!   come before the first tic's starts, wi_stuff.c:1838-1850, 1318-1328, 514-528).
//! - **The load tic is driven** when the genuine tic's only records are the load-time start: the
//!   carried chainsaw's `sfx_sawup` is **projected** from the genuine row, and the new level's body
//!   is a wire-25 carrier of the new map (a fresh start, with no record of its own). Later levels
//!   need wire 26's carried start and are not compared here.

mod common;

use std::collections::BTreeMap;
use std::sync::Arc;

use common::gate::{RowCounts, compare_stream};
use common::rows::{Carry, stream_rows_with};
use common::{directory, golden_rows, parse_starts, reference, replay, texture_count, wire25};
use doom_sim::demo::parse_demo_episode1_v2;
use doom_sim::{Simulation, Skill, TicCommand};
use doom_sound::chain::{
    ChainCall, ChainConfig, ChainRefusal, ChainSound, ChainState, IntermissionSound, LevelLoad,
    NotYet,
};
use doom_sound::chain_state::{ChainSoundState, ChainStateError, MAX_BYTES, VERSION};
use doom_sound::logic::{Listener, SoundOrigin, SoundRecord};
use doom_sound::lumps::{SoundCaps, SoundSet};
use doom_sound::mixer::buffer_bytes;
use doom_sound::music::{Music, MusicLumps};
use doom_sound::schedule::Schedule;
use doom_sound::session::{SimulationPositions, simulation_listener};
use doom_sound::state::{SoundState, StateError};
use doom_test_evidence::sha256;

/// The 26 items of `tests/reference/m6a_manifest.json`: name, WAD set and demo (checked against
/// the manifest's text below).
const ITEMS: [(&str, &str, &str); 26] = [
    ("CHAIN", "episode", "CHAIN"),
    ("INTSTA", "episode", "INTSTA"),
    ("INTSTB", "episode", "INTSTB"),
    ("INTIDLE", "episode", "INTIDLE"),
    ("INTHELD", "episode", "INTHELD"),
    ("INTFAST", "episode", "INTFAST"),
    ("REBHELD", "episode", "REBHELD"),
    ("REBLATE", "episode", "REBLATE"),
    ("SECRET", "episode", "SECRET"),
    ("REBSECRT", "episode", "REBSECRT"),
    ("PROBE", "rbx", "PROBE"),
    ("USEAT130", "rbx", "USEAT130"),
    ("USEAT131", "rbx", "USEAT131"),
    ("USEAT132", "rbx", "USEAT132"),
    ("RBXDEAD", "rbx", "RBXDEAD"),
    ("NOINTER", "nointer", "NOINTER"),
    ("NOE1M2", "noe1m2", "NOE1M2"),
    ("NOE1M2AT", "noe1m2", "NOE1M2AT"),
    ("FINREG", "episode", "FINALE"),
    ("FINRET", "episode", "FINALE"),
    ("NOHELP2", "nohelp2", "FINALE"),
    ("NOVICTOR", "novictor", "FINALE"),
    ("SKILL0", "episode", "SKILL0"),
    ("SKILL1", "episode", "SKILL1"),
    ("SKILL2", "episode", "SKILL2"),
    ("SKILL3", "episode", "SKILL3"),
];

fn wadset(set: &str) -> Vec<&'static str> {
    let (iwad, gfx, music) = match set {
        "episode" => ("episode.wad", "m6a_gfx.wad", "m6a_music.wad"),
        "rbx" => ("rbx.wad", "m6a_gfx.wad", "m6a_music.wad"),
        "nohelp2" => ("episode.wad", "m6a_gfx_nohelp2.wad", "m6a_music.wad"),
        "nointer" => ("episode.wad", "m6a_gfx.wad", "m6a_music_nointer.wad"),
        "novictor" => ("episode.wad", "m6a_gfx.wad", "m6a_music_novictor.wad"),
        "noe1m2" => ("episode.wad", "m6a_gfx.wad", "m6a_music_noe1m2.wad"),
        s => panic!("WAD set {s}"),
    };
    let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
    vec![
        leak(format!("m6a_wads/{iwad}")),
        "m5b_gfx.wad",
        "m5c_sfx.wad",
        leak(format!("m6a_wads/{gfx}")),
        leak(format!("m6a_wads/{music}")),
    ]
}

fn read(name: &str) -> String {
    std::fs::read_to_string(reference(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The map of an item's first WAD; with `carrier`, every W1 exit line 52 cleared (module doc).
fn map_of(file: &str, n: u8, carrier: bool) -> doom_map::Map {
    let mut bytes = std::fs::read(reference(file)).expect("WAD");
    if carrier {
        let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("WAD");
        let lumps = wad.lumps();
        let at = lumps
            .iter()
            .position(|l| {
                l.name()
                    .as_bytes()
                    .starts_with(format!("E1M{n}").as_bytes())
            })
            .expect("map marker");
        let lines = lumps[at..]
            .iter()
            .find(|l| l.name().as_bytes().starts_with(b"LINEDEFS"))
            .expect("LINEDEFS")
            .range();
        drop(wad);
        for l in lines.step_by(14) {
            if bytes[l + 6..l + 8] == 52i16.to_le_bytes() {
                bytes[l + 6..l + 8].copy_from_slice(&0i16.to_le_bytes());
            }
        }
    }
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("WAD");
    doom_map::Map::from_wad(&wad, &format!("E1M{n}"), Default::default()).expect("map")
}

/// A wire-25 simulation of map `n` after its setup calls, and whether it is a carrier.
fn level(file: &str, n: u8, skill: Skill) -> (Simulation, bool) {
    match wire25::build(true, map_of(file, n, false), n, skill) {
        Ok(s) => (s, false),
        Err(e) if e.contains("Pursuit(Specials)") => (
            wire25::build(true, map_of(file, n, true), n, skill).expect("carrier"),
            true,
        ),
        Err(e) => panic!("E1M{n}: {e}"),
    }
}

/// One genuine sound golden split by tic: the S, T, K, C, L and R rows; the LV rows (load tic,
/// map) and the E row (its tic and message).
#[derive(Default)]
struct Genuine {
    by_tic: BTreeMap<i64, Vec<String>>,
    loads: BTreeMap<i64, i32>,
    error: Option<(i64, String)>,
}

fn genuine(name: &str) -> Genuine {
    let mut g = Genuine::default();
    for l in read(&format!("m6a_sound/{name}.sound")).lines() {
        if l.starts_with('#') || l.starts_with("Z,") {
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        let tic: i64 = f[1].parse().expect("tic");
        match f[0] {
            "LV" => {
                g.loads.insert(tic, f[3].parse().expect("map"));
            }
            "E" => g.error = Some((tic, f[2..].join(","))),
            _ => g.by_tic.entry(tic).or_default().push(l.to_string()),
        }
    }
    g
}

/// The game state of every completed session tic (the G rows of the session golden).
fn gamestates(name: &str) -> BTreeMap<i64, i64> {
    let mut out = BTreeMap::new();
    for l in read(&format!("m6a_session/{name}.session")).lines() {
        if let Some(rest) = l.strip_prefix("G,") {
            let f: Vec<&str> = rest.split(',').collect();
            out.insert(f[0].parse().unwrap(), f[1].parse().unwrap());
        }
    }
    out
}

fn field(row: &str, i: usize) -> &str {
    row.split(',').nth(i).expect("field")
}

/// The R row's three indices of a genuine tic (-1 where the C writes it).
fn r_indices(rows: &[String]) -> [i64; 3] {
    let r = rows.iter().find(|r| r.starts_with("R,")).expect("R row");
    [2, 3, 4].map(|i| field(r, i).parse().unwrap())
}

/// The projected intermission starts of a genuine tic (S rows with no origin).
fn projected_starts(rows: &[String]) -> Vec<IntermissionSound> {
    rows.iter()
        .filter(|r| r.starts_with("S,"))
        .map(|r| {
            assert_eq!(
                field(r, 4),
                "none",
                "an intermission start with an origin: {r}"
            );
            match field(r, 3) {
                "1" => IntermissionSound::Pistol,
                "82" => IntermissionSound::Barexp,
                "3" => IntermissionSound::Sgcock,
                s => panic!("intermission sfx {s}"),
            }
        })
        .collect()
}

/// One item's drive.
struct Drive {
    name: &'static str,
    sim: Simulation,
    carrier: bool,
    commands: Vec<TicCommand>,
    chain: ChainSound,
    genuine: Genuine,
    states: BTreeMap<i64, i64>,
    file: &'static str,
    skill: Skill,
    carry: Carry,
    /// Rust rows per compared tic.
    rows: Vec<Vec<String>>,
    /// Per compared tic, the buffer mixed after it.
    buffers: Vec<Option<String>>,
    /// The listener actor of the current level (for the origin column).
    listener: Option<doom_sim::ActorId>,
    /// The session's saved state before each driven tic.
    saved: Vec<Vec<u8>>,
    /// The levels loaded so far in the drive (each level's actor handles are their own namespace:
    /// the Rust handles are tagged with it before the bijection with the genuine serials, which
    /// run on across levels).
    level_index: usize,
    /// The sound set, music lumps and configuration the session was built with.
    set: Arc<SoundSet>,
    music: MusicLumps,
    config: ChainConfig,
    /// Restore the session from its own saved bytes before this tic (restore-and-continue).
    restore_at: Option<i64>,
    /// A planted fault (the comparisons must catch it).
    plant: Option<Plant>,
}

/// Faults planted in a drive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Plant {
    /// The first projected intermission start of this tic is dropped.
    DropStart(i64),
    /// The load-time `sfx_sawup` is not given.
    NoSawup,
    /// The `M_Random` index handed in at this tic is one more.
    IndexOff(i64),
}

/// Facts of one drive.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Outcome {
    tics: usize,
    level_tics: usize,
    intermission_tics: usize,
    finale_tics: usize,
    loads: usize,
    real_boundaries: usize,
    projected_boundaries: usize,
    refusal: Option<String>,
}

fn listener_of(sim: &Simulation) -> Listener {
    simulation_listener(sim).expect("listener")
}

impl Drive {
    fn new(name: &'static str, set: &str, demo: &str) -> Result<Self, String> {
        let wads = wadset(set);
        let dir = directory(&wads);
        let sound_set = Arc::new(SoundSet::load(&dir, SoundCaps::default()).expect("sound set"));
        let music = MusicLumps::from_directory(&dir);
        let config = ChainConfig::fresh(texture_count(&dir));
        let lmp = std::fs::read(reference(&format!("m6a_scripts/{demo}.lmp"))).expect("demo");
        let map = lmp[3];
        let chain = ChainSound::new(sound_set.clone(), music, config, i32::from(map))
            .map_err(|e| format!("{e}"))?;
        let (header, demo) =
            parse_demo_episode1_v2(&lmp, map).map_err(|e| format!("demo: {e:?}"))?;
        let skill = if header.skill == 3 {
            Skill::Hard
        } else {
            Skill::Medium
        };
        let (sim, carrier) = level(wads[0], map, skill);
        Ok(Self {
            name,
            listener: simulation_listener(&sim).map(|l| l.actor),
            sim,
            carrier,
            commands: demo.commands,
            chain,
            genuine: genuine(name),
            states: gamestates(name),
            file: wads[0],
            skill,
            carry: Carry::default(),
            rows: Vec::new(),
            buffers: Vec::new(),
            saved: Vec::new(),
            level_index: 0,
            set: sound_set,
            music,
            config,
            restore_at: None,
            plant: None,
        })
    }

    /// The index handed to the session at tic `t` (projected, module documentation).
    fn index_in(&self, t: i64, entering_intermission: bool) -> u8 {
        let prev = if t == 0 {
            0
        } else {
            r_indices(&self.genuine.by_tic[&(t - 1)])[2]
        };
        (prev + if entering_intermission { 10 } else { 0 }) as u8
    }

    fn push_rows(
        &mut self,
        t: i64,
        ct: &doom_sound::chain::ChainTic,
        records: &[SoundRecord],
        level: bool,
        old_listener: Option<doom_sim::ActorId>,
        new_level: bool,
    ) {
        let g = r_indices(&self.genuine.by_tic[&t]);
        let rows = stream_rows_with(
            &ct.sound,
            records,
            self.chain.sound(),
            self.listener,
            &mut self.carry,
            g[1] as u8,
            g[2] as u8,
        );
        // Load-time stops of S_Start come first in the genuine stream (written inside
        // S_StopChannel, before the tic's records).
        let old = old_listener;
        let origin = |o: SoundOrigin| match o {
            SoundOrigin::None => "none,-1".to_string(),
            SoundOrigin::Actor(a) => format!(
                "{},A{}:{}",
                if Some(a) == old { "player" } else { "mobj" },
                a.slot(),
                a.generation()
            ),
            SoundOrigin::Sector(s) => format!("sector,{s}"),
            SoundOrigin::Button(b) => format!("button,{b}"),
        };
        let k_rows: Vec<String> = ct
            .level_start
            .iter()
            .map(|s| {
                format!(
                    "K,{t},0,{},{},{},{},level_start,{}",
                    s.cnum,
                    s.entry.sfx.get(),
                    origin(s.entry.origin),
                    s.entry.handle,
                    u8::from(s.playing)
                )
            })
            .collect();
        let tag = |rows: Vec<String>, n: usize| -> Vec<String> {
            rows.into_iter()
                .map(|r| {
                    r.split(',')
                        .map(|f| match f.strip_prefix('A') {
                            Some(h) if n > 0 => format!("A{n}/{h}"),
                            _ => f.to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .collect()
        };
        let before = if new_level {
            self.level_index - 1
        } else {
            self.level_index
        };
        let mut rows = tag(rows, self.level_index);
        rows.splice(0..0, tag(k_rows, before));
        // The R row's status-bar and wipe draw counts are the display's (compared by doom-render's
        // m6a_display test); projected here from the genuine row. Outside a level no P_Ticker or
        // ST_Ticker runs, so the C writes -1 for the two indices after them.
        let genuine_r = self.genuine.by_tic[&t]
            .iter()
            .find(|r| r.starts_with("R,"))
            .expect("R row")
            .clone();
        let gf: Vec<&str> = genuine_r.split(',').collect();
        let r = rows.last_mut().expect("R row");
        let mut f: Vec<String> = r.split(',').map(str::to_string).collect();
        f[6] = gf[6].into();
        f[7] = gf[7].into();
        if !level {
            f[2] = "-1".into();
            f[3] = "-1".into();
        }
        *r = f.join(",");
        self.buffers
            .push(ct.sound.buffer.as_ref().map(|b| sha256(&buffer_bytes(b))));
        self.rows.push(rows);
    }

    /// Drives the item as far as the module documentation allows; returns what was done.
    fn run(&mut self) -> Outcome {
        let mut out = Outcome::default();
        let mut first_level = true;
        let last = *self.states.keys().last().unwrap_or(&-1);
        let refused_tic = self
            .genuine
            .error
            .as_ref()
            .map(|(t, m)| if m.contains("HELP2") { *t } else { t + 1 });
        let mut t: i64 = 0;
        loop {
            let refusal_here = refused_tic == Some(t);
            if t > last && !refusal_here {
                break;
            }
            let msg = self
                .genuine
                .error
                .as_ref()
                .map(|e| e.1.clone())
                .unwrap_or_default();
            if refusal_here && msg.contains("HELP2") {
                // A display refusal (doom-render's); the sound of that tic has no genuine row.
                out.refusal = Some("display: HELP2".into());
                break;
            }
            let gs = if refusal_here {
                if msg.contains("d_inter") {
                    1
                } else if msg.contains("d_victor") {
                    2
                } else {
                    0
                }
            } else {
                self.states[&t]
            };
            let prev = if t == 0 { 0 } else { self.states[&(t - 1)] };
            let load = self.genuine.loads.get(&t).copied();
            self.saved.push(self.chain.save().encode());
            if self.restore_at == Some(t) {
                let st = ChainSoundState::decode(&self.saved[t as usize]).expect("decode");
                assert_eq!(
                    st.encode(),
                    self.saved[t as usize],
                    "{} tic {t}: round trip",
                    self.name
                );
                let restored =
                    ChainSound::restore(self.set.clone(), self.music, &st, &self.sim, self.config)
                        .unwrap_or_else(|e| panic!("{} tic {t}: restore: {e:?}", self.name));
                assert_eq!(
                    restored, self.chain,
                    "{} tic {t}: the restored session",
                    self.name
                );
                self.chain = restored;
            }
            let before = self.chain.clone();
            match gs {
                0 if load.is_none() => {
                    assert!(first_level, "{}: a later level is not driven", self.name);
                    self.sim
                        .advance([
                            self.commands[t as usize],
                            TicCommand::default(),
                            TicCommand::default(),
                            TicCommand::default(),
                        ])
                        .unwrap_or_else(|e| panic!("{} tic {t}: {e:?}", self.name));
                    let records = self.sim.sound_records().to_vec();
                    let idx = self.index_in(t, false);
                    let ct = self
                        .chain
                        .level_tic(
                            None,
                            &records,
                            idx,
                            &listener_of(&self.sim),
                            &SimulationPositions(&self.sim),
                        )
                        .unwrap_or_else(|e| panic!("{} tic {t}: {e}", self.name));
                    self.push_rows(t, &ct, &records, true, self.listener, false);
                    out.level_tics += 1;
                }
                0 => {
                    let map = load.expect("load");
                    // The boundary: the level simulation refuses the next tic (a real check unless
                    // it is a carrier).
                    if prev == 0 {
                        let e = self
                            .sim
                            .advance([TicCommand::default(); 4])
                            .expect_err("reborn");
                        assert!(
                            format!("{e:?}").contains("PlayerReborn"),
                            "{}: {e:?}",
                            self.name
                        );
                        out.real_boundaries += 1;
                    }
                    let rows = self.genuine.by_tic.get(&t).cloned().unwrap_or_default();
                    let starts: Vec<&String> = rows
                        .iter()
                        .filter(|r| r.starts_with("S,") || r.starts_with("T,"))
                        .collect();
                    let sawup = starts
                        .first()
                        .is_some_and(|r| field(r, 3) == "10" && field(r, 4) == "player");
                    if starts.len() > usize::from(sawup) {
                        break; // tic 0's own records need wire 26
                    }
                    let (carrier, _) = level(self.file, map as u8, self.skill);
                    let listener = listener_of(&carrier);
                    let load_records = if sawup && self.plant != Some(Plant::NoSawup) {
                        vec![SoundRecord::Start {
                            sfx: 10,
                            origin: SoundOrigin::Actor(listener.actor),
                            at: None,
                        }]
                    } else {
                        Vec::new()
                    };
                    let idx = self.index_in(t, false);
                    let r = self.chain.level_tic(
                        Some(LevelLoad {
                            map,
                            records: &load_records,
                        }),
                        &[],
                        idx,
                        &listener,
                        &SimulationPositions(&carrier),
                    );
                    match r {
                        Err(e) if refusal_here => {
                            assert_eq!(
                                e,
                                ChainRefusal::MusicMissing {
                                    music: Music::level(map).unwrap(),
                                    tic: Some(t as u64)
                                },
                                "{}",
                                self.name
                            );
                            assert_eq!(
                                self.chain, before,
                                "{}: a refusal changed the session",
                                self.name
                            );
                            out.refusal = Some(format!("{e}"));
                            break;
                        }
                        r => {
                            let ct = r.unwrap_or_else(|e| panic!("{} tic {t}: {e}", self.name));
                            let old = self.listener;
                            self.listener = Some(listener.actor);
                            self.level_index += 1;
                            self.push_rows(t, &ct, &load_records, true, old, true);
                            out.loads += 1;
                            out.tics += 1;
                            break; // the new level's later tics need wire 26
                        }
                    }
                }
                1 | 2 => {
                    let entering = prev == 0;
                    if entering {
                        if self.carrier {
                            out.projected_boundaries += 1;
                        } else {
                            let e = self
                                .sim
                                .advance([TicCommand::default(); 4])
                                .expect_err("exit");
                            assert!(
                                format!("{e:?}").contains("LevelExited"),
                                "{}: {e:?}",
                                self.name
                            );
                            out.real_boundaries += 1;
                        }
                        first_level = false;
                    }
                    let rows = self.genuine.by_tic.get(&t).cloned().unwrap_or_default();
                    let listener = listener_of(&self.sim);
                    let positions = SimulationPositions(&self.sim);
                    let (r, records) = if gs == 1 {
                        let mut starts = projected_starts(&rows);
                        if self.plant == Some(Plant::DropStart(t)) {
                            starts.remove(0);
                        }
                        let idx = self
                            .index_in(t, entering)
                            .wrapping_add(u8::from(self.plant == Some(Plant::IndexOff(t))));
                        let records: Vec<SoundRecord> = starts
                            .iter()
                            .map(|s| SoundRecord::Start {
                                sfx: s.sfx(),
                                origin: SoundOrigin::None,
                                at: None,
                            })
                            .collect();
                        (
                            self.chain
                                .intermission_tic(&starts, idx, &listener, &positions),
                            records,
                        )
                    } else {
                        (
                            self.chain
                                .finale_tic(self.index_in(t, false), &listener, &positions),
                            Vec::new(),
                        )
                    };
                    match r {
                        Err(e) if refusal_here => {
                            let music = if gs == 1 { Music::Inter } else { Music::Victor };
                            assert_eq!(
                                e,
                                ChainRefusal::MusicMissing {
                                    music,
                                    tic: Some(t as u64)
                                },
                                "{}",
                                self.name
                            );
                            assert_eq!(
                                self.chain, before,
                                "{}: a refusal changed the session",
                                self.name
                            );
                            out.refusal = Some(format!("{e}"));
                            break;
                        }
                        r => {
                            let ct = r.unwrap_or_else(|e| panic!("{} tic {t}: {e}", self.name));
                            self.push_rows(t, &ct, &records, false, self.listener, false);
                            if gs == 1 {
                                out.intermission_tics += 1;
                            } else {
                                out.finale_tics += 1;
                            }
                        }
                    }
                }
                s => panic!("{} tic {t}: game state {s}", self.name),
            }
            out.tics += 1;
            t += 1;
        }
        out
    }

    /// The genuine rows of the compared tics.
    fn genuine_rows(&self) -> Vec<Vec<String>> {
        (0..self.rows.len() as i64)
            .map(|t| self.genuine.by_tic.get(&t).cloned().unwrap_or_default())
            .collect()
    }
}

#[test]
fn the_item_table_is_the_manifests() {
    let text = read("m6a_manifest.json");
    for (name, set, demo) in ITEMS {
        assert!(text.contains(&format!("\"name\": \"{name}\"")), "{name}");
        let at = text.find(&format!("\"name\": \"{name}\"")).unwrap();
        let rest = &text[at..];
        let end = rest.find('}').unwrap();
        assert!(
            rest[..end].contains(&format!("\"wadset\": \"{set}\"")),
            "{name}: {set}"
        );
        assert!(
            rest[..end].contains(&format!(
                "\"lmp\": \"tests/reference/m6a_scripts/{demo}.lmp\""
            )),
            "{name}: {demo}"
        );
    }
}

/// Every drivable item: the Rust rows of the driven tics equal the genuine rows (S, T, K, C, L
/// and R; actor origins by a bijection of handles and serials over the item).
#[test]
fn chain_rows_against_the_genuine_sound_goldens() {
    let mut counts = RowCounts::default();
    let mut total = Outcome::default();
    let mut items = 0;
    let mut skipped = Vec::new();
    for (name, set, demo) in ITEMS {
        let mut d = match Drive::new(name, set, demo) {
            Ok(d) => d,
            Err(e) if name == "NOE1M2AT" => {
                // The first map's music at the session's load: the genuine E row at tic -1.
                assert_eq!(e, "W_GetNumForName: d_e1m2 not found! (at load)");
                assert_eq!(genuine(name).error.map(|e| e.0), Some(-1));
                total.refusal = Some(e);
                continue;
            }
            Err(e) if name == "SKILL0" || name == "SKILL1" => {
                // The version-2 demo parser admits skills 2 and 3 only (easy skills: step 10).
                skipped.push(format!("{name}: {e}"));
                continue;
            }
            Err(e) => panic!("{name}: {e}"),
        };
        let o = d.run();
        let g = d.genuine_rows();
        compare_stream(name, &d.rows, &g, &mut counts).unwrap_or_else(|e| panic!("{e}"));
        eprintln!("{name}: {o:?}");
        items += 1;
        total.tics += o.tics;
        total.level_tics += o.level_tics;
        total.intermission_tics += o.intermission_tics;
        total.finale_tics += o.finale_tics;
        total.loads += o.loads;
        total.real_boundaries += o.real_boundaries;
        total.projected_boundaries += o.projected_boundaries;
    }
    eprintln!("chain sound: {items} items driven, {total:?}, rows {counts:?}, skipped {skipped:?}");
    assert_eq!(items, 23);
    assert!(total.intermission_tics > 1000 && total.finale_tics > 1000 && total.loads >= 5);
}

/// The genuine mixer across boundaries: every start list of the M6a items through the Rust mixer
/// (the M5c replay of `tests/common`), row for row equal to the genuine golden; every melt row in
/// its place under the declared schedule; and U8 quirk 9 as a mixer observation: buffers mixed
/// after the first intermission or finale tic while a channel the level started is still mixing.
#[test]
fn mixer_goldens_across_boundaries() {
    let set = common::sound_set("m5c_sfx.wad");
    let (mut starts, mut buffers, mut melts, mut heard) = (0, 0, 0, Vec::new());
    for (name, _set, _demo) in ITEMS {
        let list = parse_starts(&read(&format!("m6a_mixer/starts/{name}.txt")));
        let got = replay(&set, &list, false, 2_000_000);
        let want = golden_rows(&read(&format!("m6a_mixer/goldens/{name}.golden")));
        if let Some((i, w, g)) = common::first_difference(&want, &got.rows) {
            panic!("{name}: first differing row {i}:\n  golden: {w}\n  rust:   {g}");
        }
        starts += got.starts;
        buffers += got.buffers;
        let states = gamestates(name);
        for l in read(&format!("m6a_mixer/melts/{name}.txt")).lines() {
            let Some(w) = l.strip_prefix("w ") else {
                continue;
            };
            let f: Vec<u64> = w.split(' ').map(|x| x.parse().unwrap()).collect();
            assert_eq!(f[2], Schedule::buffers_before(f[0]), "{name}: {l}");
            assert!((34..=43).contains(&f[1]), "{name}: {l}");
            melts += 1;
        }
        // Quirk 9: the first tic outside a level, and the buffers mixed after it during which a
        // channel the level started is still mixing (its slot busy after the buffer, and the
        // slot's latest start before that tic).
        let Some((&first, _)) = states
            .iter()
            .find(|(t, s)| **s != 0 && states.get(&(**t - 1)) == Some(&0))
        else {
            continue;
        };
        let first = first as u64;
        let mut last_start = [None::<u64>; 8];
        let (mut late, mut prev_late) = (0u64, false);
        for r in &got.rows {
            let f: Vec<&str> = r.split(' ').collect();
            match f[0] {
                "s" => {
                    let slot: usize = f[11].parse().unwrap();
                    last_start[slot] = Some(f[1].parse().unwrap());
                    prev_late = false;
                }
                "b" => {
                    let tic: u64 = f[2].parse().unwrap();
                    let mask = u8::from_str_radix(f[4], 16).unwrap();
                    prev_late = tic >= first
                        && (0..8).any(|k| {
                            mask & (1 << k) != 0 && last_start[k].is_some_and(|s| s < first)
                        });
                    late += u64::from(prev_late);
                }
                "r" if prev_late => late += f[1].parse::<u64>().unwrap(),
                _ => {}
            }
        }
        if late > 0 {
            heard.push((name, first, late));
        }
    }
    eprintln!(
        "m6a mixer: 26 items, {starts} starts, {buffers} buffers, {melts} melts; quirk 9 (level \
         sounds mixed after the level ended): {heard:?}"
    );
    assert_eq!(melts, 113);
    assert!(!heard.is_empty(), "quirk 9 seen in the mixer");
}

/// One item's drive, by name.
fn drive(name: &str) -> Drive {
    let (n, set, demo) = ITEMS.iter().copied().find(|i| i.0 == name).expect("item");
    Drive::new(n, set, demo).expect("drive")
}

/// `DSND` version 2 restore-and-continue: at every kind of point (mid-level, the last level tic,
/// the first and a later intermission tic, before a load that stops a playing channel, before
/// the finale, mid-finale) the session is saved, decoded (byte-equal round trip), restored
/// against the level's or the finished level's simulation (equal to the session it came from)
/// and continued: every later row equal to the uninterrupted drive's.
#[test]
fn dsnd_v2_restore_and_continue_across_boundaries() {
    let mut points = 0;
    for (name, at) in [
        ("CHAIN", vec![0, 60, 125, 126, 127, 135, 142]),
        ("INTFAST", vec![55, 56, 60, 69]),
        ("REBHELD", vec![100, 162]),
        ("FINREG", vec![320, 321, 322, 1000, 1890]),
    ] {
        let mut whole = drive(name);
        whole.run();
        for t in at {
            let mut d = drive(name);
            d.restore_at = Some(t);
            d.run();
            assert_eq!(d.rows, whole.rows, "{name}: continued from tic {t}");
            assert_eq!(d.buffers, whole.buffers, "{name}: buffers from tic {t}");
            points += 1;
        }
        // Every saved state of the drive round-trips byte for byte.
        for (t, bytes) in whole.saved.iter().enumerate() {
            let st = ChainSoundState::decode(bytes).unwrap_or_else(|e| panic!("{name} {t}: {e:?}"));
            assert_eq!(&st.encode(), bytes, "{name} tic {t}");
            assert!(bytes.len() <= MAX_BYTES);
        }
    }
    assert_eq!(points, 18);
}

fn put(bytes: &mut [u8], at: usize, v: &[u8]) {
    bytes[at..at + v.len()].copy_from_slice(v);
}

/// Forged or inconsistent `DSND` version 2 bytes are refused; version 1 and 2 refuse each other.
#[test]
fn dsnd_v2_forgeries_are_refused() {
    let mut d = drive("CHAIN");
    d.run();
    let level = d.saved[100].clone(); // in E1M1
    let inter = d.saved[130].clone(); // in the intermission
    let st = ChainSoundState::decode(&inter).unwrap();
    assert_eq!(st.state, ChainState::Intermission);
    assert_eq!((st.level_start, st.level_end, st.gamemap), (0, 126, 1));
    assert_eq!(u16::from_le_bytes([inter[4], inter[5]]), VERSION);
    // Version 1 refuses version 2 and the reverse.
    assert_eq!(SoundState::decode(&inter), Err(StateError::Version(2)));
    let v1 = d.chain.sound().state().encode();
    assert_eq!(
        ChainSoundState::decode(&v1),
        Err(ChainStateError::State(StateError::Version(1)))
    );
    // Every truncation refused, never a panic; a trailing byte and an oversize input.
    for n in 0..inter.len() {
        assert!(ChainSoundState::decode(&inter[..n]).is_err(), "prefix {n}");
    }
    let mut long = inter.clone();
    long.push(0);
    assert_eq!(
        ChainSoundState::decode(&long),
        Err(ChainStateError::State(StateError::Trailing))
    );
    assert_eq!(
        ChainSoundState::decode(&vec![0; MAX_BYTES + 1]),
        Err(ChainStateError::State(StateError::TooLong))
    );
    let forged = |base: &[u8], at: usize, v: &[u8]| {
        let mut b = base.to_vec();
        put(&mut b, at, v);
        ChainSoundState::decode(&b)
    };
    assert_eq!(forged(&inter, 28, &[3]), Err(ChainStateError::GameState(3)));
    assert_eq!(
        forged(&inter, 29, &0i32.to_le_bytes()),
        Err(ChainStateError::Map(0))
    );
    assert_eq!(
        forged(&inter, 29, &10i32.to_le_bytes()),
        Err(ChainStateError::Map(10))
    );
    // A finale on map 1.
    assert_eq!(forged(&inter, 28, &[2]), Err(ChainStateError::Map(1)));
    // Level start and end.
    assert_eq!(
        forged(&level, 41, &5u64.to_le_bytes()),
        Err(ChainStateError::LevelTics)
    );
    assert_eq!(
        forged(&inter, 41, &0u64.to_le_bytes()),
        Err(ChainStateError::LevelTics)
    );
    assert_eq!(
        forged(&inter, 41, &130u64.to_le_bytes()),
        Err(ChainStateError::LevelTics)
    );
    assert_eq!(
        forged(&level, 33, &100u64.to_le_bytes()),
        Err(ChainStateError::LevelTics)
    );
    // The buffer count.
    assert!(matches!(
        forged(&inter, 14, &(st.buffers + 1).to_le_bytes()),
        Err(ChainStateError::State(StateError::Schedule(_)))
    ));
    // The melt byte; a melt in progress decodes with the last tic's buffer owed and is refused
    // on restore (not yet).
    assert_eq!(forged(&inter, 49, &[2]), Err(ChainStateError::Melt(2)));
    let owed = (1..d.saved.len())
        .find(|&t| Schedule::buffers_before(t as u64) != Schedule::buffers_before(t as u64 - 1))
        .unwrap();
    let mut melt = d.saved[owed].clone();
    put(&mut melt, 49, &[1]);
    assert!(matches!(
        ChainSoundState::decode(&melt),
        Err(ChainStateError::State(StateError::Schedule(_)))
    ));
    put(
        &mut melt,
        14,
        &Schedule::buffers_before(owed as u64 - 1).to_le_bytes(),
    );
    let m = ChainSoundState::decode(&melt).expect("a melt in progress decodes");
    assert!(m.melt);
    assert_eq!(m.encode(), melt);
    let fresh = drive("CHAIN");
    assert_eq!(
        ChainSound::restore(fresh.set.clone(), fresh.music, &m, &fresh.sim, fresh.config),
        Err(ChainStateError::NotYet(NotYet::MeltInProgress))
    );
    // Restore against the wrong configuration, music lumps or simulation.
    let mut other = d.config;
    other.sfx_volume = 9;
    assert!(matches!(
        ChainSound::restore(d.set.clone(), d.music, &st, &d.sim, other),
        Err(ChainStateError::State(StateError::ConfigMismatch { .. }))
    ));
    assert_eq!(
        ChainSound::restore(
            d.set.clone(),
            d.music.without(Music::Inter),
            &st,
            &d.sim,
            d.config
        ),
        Err(ChainStateError::MusicMissing(Music::Inter))
    );
    assert!(matches!(
        ChainSound::restore(d.set.clone(), d.music, &st, &fresh.sim, d.config),
        Err(ChainStateError::TicMismatch {
            state: 126,
            simulation: 0
        })
    ));
}

/// Every refusal of the session leaves it, and its saved bytes, exactly as they were.
#[test]
fn refusals_leave_the_session_unchanged() {
    let check = |chain: &mut ChainSound,
                 r: Result<doom_sound::chain::ChainTic, ChainRefusal>,
                 want: ChainRefusal,
                 before: &ChainSound| {
        assert_eq!(r.map(|_| ()), Err(want));
        assert_eq!(chain, before);
    };
    // In E1M1, after 100 tics.
    let mut d = drive("CHAIN");
    let mut lvl = drive("CHAIN");
    lvl.states.retain(|t, _| *t < 100);
    lvl.run();
    let (mut c, sim) = (lvl.chain.clone(), &lvl.sim);
    let before = c.clone();
    let l = listener_of(sim);
    let p = SimulationPositions(sim);
    let r = c.finale_tic(0, &l, &p);
    check(&mut c, r, ChainRefusal::FinaleMap { map: 1 }, &before);
    let r = c.level_tic(
        Some(LevelLoad {
            map: 2,
            records: &[],
        }),
        &[],
        0,
        &l,
        &p,
    );
    check(
        &mut c,
        r,
        ChainRefusal::ReloadMap {
            current: 1,
            requested: 2,
        },
        &before,
    );
    let r = c.level_tic(
        Some(LevelLoad {
            map: 10,
            records: &[],
        }),
        &[],
        0,
        &l,
        &p,
    );
    check(&mut c, r, ChainRefusal::Map { map: 10 }, &before);
    let wrong = [SoundRecord::Start {
        sfx: 11,
        origin: SoundOrigin::Actor(l.actor),
        at: None,
    }];
    let r = c.level_tic(
        Some(LevelLoad {
            map: 1,
            records: &wrong,
        }),
        &[],
        0,
        &l,
        &p,
    );
    check(&mut c, r, ChainRefusal::LoadRecord { index: 0 }, &before);
    let saw = SoundRecord::Start {
        sfx: 10,
        origin: SoundOrigin::Actor(l.actor),
        at: None,
    };
    let r = c.level_tic(
        Some(LevelLoad {
            map: 1,
            records: &[saw, saw],
        }),
        &[],
        0,
        &l,
        &p,
    );
    check(&mut c, r, ChainRefusal::LoadRecord { index: 1 }, &before);
    assert_eq!(c.melt_step(), Err(ChainRefusal::NotYet(NotYet::MeltSteps)));
    assert_eq!(c, before);
    // A start of an actor other than the listener without a placement (the M5c rule).
    let other = SoundRecord::Start {
        sfx: 1,
        origin: SoundOrigin::Actor(doom_sim::ActorId::from_parts(5, 1)),
        at: None,
    };
    let r = c.level_tic(None, &[other], 0, &l, &p);
    assert!(matches!(r, Err(ChainRefusal::Session(_))));
    assert_eq!(c, before);
    // In the intermission.
    d.states.retain(|t, _| *t < 130);
    d.run();
    let (mut c, sim) = (d.chain.clone(), &d.sim);
    let before = c.clone();
    let l = listener_of(sim);
    let p = SimulationPositions(sim);
    let r = c.level_tic(None, &[], 0, &l, &p);
    check(
        &mut c,
        r,
        ChainRefusal::WrongState {
            state: ChainState::Intermission,
            call: ChainCall::Level,
        },
        &before,
    );
    let r = c.finale_tic(0, &l, &p);
    check(
        &mut c,
        r,
        ChainRefusal::WrongState {
            state: ChainState::Intermission,
            call: ChainCall::Finale,
        },
        &before,
    );
    let st = c.save();
    let mut nomus = ChainSound::restore(
        d.set.clone(),
        d.music.without(Music::Level(2)),
        &st,
        sim,
        d.config,
    )
    .unwrap();
    let before2 = nomus.clone();
    let r = nomus.level_tic(
        Some(LevelLoad {
            map: 2,
            records: &[],
        }),
        &[],
        0,
        &l,
        &p,
    );
    check(
        &mut nomus,
        r,
        ChainRefusal::MusicMissing {
            music: Music::Level(2),
            tic: Some(130),
        },
        &before2,
    );
    // In the finale.
    let mut f = drive("FINREG");
    f.states.retain(|t, _| *t < 400);
    f.run();
    let (mut c, sim) = (f.chain.clone(), &f.sim);
    let before = c.clone();
    let l = listener_of(sim);
    let p = SimulationPositions(sim);
    let r = c.intermission_tic(&[], 0, &l, &p);
    check(
        &mut c,
        r,
        ChainRefusal::WrongState {
            state: ChainState::Finale,
            call: ChainCall::Intermission,
        },
        &before,
    );
    let r = c.level_tic(
        Some(LevelLoad {
            map: 1,
            records: &[],
        }),
        &[],
        0,
        &l,
        &p,
    );
    check(
        &mut c,
        r,
        ChainRefusal::WrongState {
            state: ChainState::Finale,
            call: ChainCall::Load,
        },
        &before,
    );
    assert_eq!(c.music_playing(), Music::Victor);
    // The tic limit: a state at the last session tic `gametic` can count.
    let mut st = lvl.chain.save();
    st.tics = Schedule::buffers_before(0) + doom_sound::schedule::MAX_TIC + 1;
    st.buffers = Schedule::buffers_before(st.tics);
    st.level_start = st.tics - lvl.sim.tic();
    let st = ChainSoundState::decode(&st.encode()).expect("the last tic decodes");
    let mut c = ChainSound::restore(lvl.set.clone(), lvl.music, &st, &lvl.sim, lvl.config).unwrap();
    let before = c.clone();
    let l = listener_of(&lvl.sim);
    let p = SimulationPositions(&lvl.sim);
    let r = c.level_tic(None, &[], 0, &l, &p);
    check(
        &mut c,
        r,
        ChainRefusal::Session(doom_sound::session::SessionRefusal::TicLimit),
        &before,
    );
}

/// Planted faults are caught by the row comparison: a dropped intermission start, a load
/// without the carried chainsaw's start, an `M_Random` index off by one; and a changed buffer
/// digest in a copy of a mixer golden.
#[test]
fn the_comparisons_can_fail() {
    for (name, plant, at) in [
        ("CHAIN", Plant::DropStart(127), 127),
        ("CHAIN", Plant::NoSawup, 142),
        ("INTSTA", Plant::IndexOff(128), 128),
    ] {
        let mut d = drive(name);
        d.plant = Some(plant);
        d.run();
        let g = d.genuine_rows();
        let e = compare_stream(name, &d.rows, &g, &mut RowCounts::default()).expect_err("caught");
        assert!(e.contains(&format!("tic {at} ")), "{name} {plant:?}: {e}");
    }
    let set = common::sound_set("m5c_sfx.wad");
    let list = parse_starts(&read("m6a_mixer/starts/CHAIN.txt"));
    let got = replay(&set, &list, false, 2_000_000);
    let mut want = golden_rows(&read("m6a_mixer/goldens/CHAIN.golden"));
    let k = want
        .iter()
        .position(|r| r.starts_with("b ") && r.split(' ').nth(2) == Some("130"))
        .unwrap();
    let mut f: Vec<String> = want[k].split(' ').map(str::to_string).collect();
    f[3] = "0".repeat(64);
    want[k] = f.join(" ");
    assert_eq!(
        common::first_difference(&want, &got.rows).map(|d| d.0),
        Some(k)
    );
}
