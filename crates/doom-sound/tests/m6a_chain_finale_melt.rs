//! M6a part 2: the finale-and-melt configuration of the sound session across levels
//! ([`ChainMode::FinaleAndMelt`]) against the genuine engine's goldens, `DSND` version 3, and the
//! frozen configuration ([`ChainMode::RefusesMelt`]) beside it.
//!
//! The goldens are the committed genuine ones, never Rust output: `tests/reference/m6a_sound`
//! (the genuine M6a runner's sound-logic stream), `tests/reference/m6a_mixer` (the genuine M5c
//! mixer on those streams: the buffers, and the `w` rows that place each melt between two
//! buffers) and `tests/reference/m6a_melt` (the genuine runner under `--melt-clock unit`: one `Y`
//! row per melt and one `V` row per melt step, so the step counts). The items are the 29 of
//! `tests/reference/m6a_manifest.json`.
//!
//! **What is driven and what is projected** is exactly part 1's (`m6a_chain.rs`, whose driver
//! this file repeats with the display added): the first level of each item by the wire-25
//! simulation from the item's demo (a map with a W1 exit line 52 as a carrier with that special
//! cleared, its exit tic then projected from the genuine session rows); intermission and finale
//! tics against the finished level's simulation, the intermission's start requests and the
//! `M_Random` index projected from the genuine S and R rows; the load tic when its only record is
//! the carried chainsaw's `sfx_sawup` (projected), the new level's body a wire-25 carrier. Later
//! levels need wire 26's carried start (doom-game) and are not driven here.
//!
//! **The display** of every driven tic is added: under [`ChainMode::FinaleAndMelt`] the tic call
//! leaves its buffer owed; when the genuine display of that tic melts (a `Y` row), one
//! [`ChainSound::melt_step`] per genuine `V` row; then [`ChainSound::mix_after_display`]. The R
//! row's `updated` column is written from [`ChainSound::updates`] over the tic and its display,
//! and each item's buffers are compared, in order, with the genuine mixer golden's.

mod common;

use std::collections::BTreeMap;
use std::sync::Arc;

use common::gate::{RowCounts, compare_stream};
use common::rows::{Carry, stream_rows_with};
use common::{directory, reference, texture_count, wire25};
use doom_sim::demo::parse_demo_episode1_v2;
use doom_sim::{Simulation, Skill, TicCommand};
use doom_sound::chain::{
    ChainCall, ChainConfig, ChainRefusal, ChainSound, ChainTic, IntermissionSound, LevelLoad,
    NotYet,
};
use doom_sound::chain_melt::{ChainFault, ChainMode, ChainSoundStateV3, VERSION_FINALE_MELT};
use doom_sound::chain_state::{ChainSoundState, ChainStateError, MAX_BYTES, VERSION};
use doom_sound::logic::{Listener, SoundOrigin, SoundRecord};
use doom_sound::lumps::{SoundCaps, SoundSet};
use doom_sound::mixer::{Buffer, buffer_bytes};
use doom_sound::music::{Music, MusicLumps};
use doom_sound::schedule::Schedule;
use doom_sound::session::{SimulationPositions, simulation_listener};
use doom_sound::state::StateError;
use doom_test_evidence::sha256;

/// The 29 items of `tests/reference/m6a_manifest.json`: name, WAD set and demo (checked against
/// the manifest's text below).
const ITEMS: [(&str, &str, &str); 29] = [
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
    ("TURBO", "episode", "TURBO"),
    ("INTCMD", "episode", "INTCMD"),
    ("ATTACK", "attack", "ATTACK"),
];

fn wadset(set: &str) -> Vec<&'static str> {
    let (iwad, gfx, music) = match set {
        "episode" => ("episode.wad", "m6a_gfx.wad", "m6a_music.wad"),
        "rbx" => ("rbx.wad", "m6a_gfx.wad", "m6a_music.wad"),
        "attack" => ("attack.wad", "m6a_gfx.wad", "m6a_music.wad"),
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

/// The map of an item's first WAD; with `carrier`, every W1 exit line 52 cleared.
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

/// One genuine sound golden split by tic; the LV rows (load tic, map); the E row; the Z row's
/// `updates` total (the runner's count of `S_UpdateSounds` calls), when the run wrote one.
#[derive(Default)]
struct Genuine {
    by_tic: BTreeMap<i64, Vec<String>>,
    loads: BTreeMap<i64, i32>,
    error: Option<(i64, String)>,
    updates: Option<u64>,
}

fn genuine(name: &str) -> Genuine {
    let mut g = Genuine::default();
    for l in read(&format!("m6a_sound/{name}.sound")).lines() {
        if l.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        if f[0] == "Z" {
            let at = f.iter().position(|x| *x == "updates").expect("updates");
            g.updates = Some(f[at + 1].parse().expect("updates"));
            continue;
        }
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

/// The melts of an item: the session tic whose display starts each and its number of steps (the
/// `Y` rows and the count of `V` rows of `m6a_melt/<ITEM>.melt`), checked against the `w` rows of
/// `m6a_mixer/melts/<ITEM>.txt` (same tics and steps, the melt after buffer K - 1 =
/// `buffers_before(TIC)` - 1).
fn melts(name: &str) -> BTreeMap<i64, u32> {
    let text = read(&format!("m6a_melt/{name}.melt"));
    let mut out = BTreeMap::new();
    for l in text.lines() {
        if let Some(r) = l.strip_prefix("Y,") {
            let tic: i64 = r.split(',').next().unwrap().parse().unwrap();
            assert!(
                out.insert(tic, 0u32).is_none(),
                "{name}: two melts at {tic}"
            );
        }
    }
    for l in text.lines() {
        if let Some(r) = l.strip_prefix("V,") {
            let tic: i64 = r.split(',').next().unwrap().parse().unwrap();
            *out.get_mut(&tic).expect("a V row of a melt") += 1;
        }
    }
    let w: BTreeMap<i64, u32> = read(&format!("m6a_mixer/melts/{name}.txt"))
        .lines()
        .filter_map(|l| l.strip_prefix("w "))
        .map(|w| {
            let f: Vec<u64> = w.split(' ').map(|x| x.parse().unwrap()).collect();
            assert_eq!(f[2], Schedule::buffers_before(f[0]), "{name}: {w}");
            (f[0] as i64, f[1] as u32)
        })
        .collect();
    assert_eq!(out, w, "{name}: the melt golden and the mixer's w rows");
    out
}

/// One buffer: the session tic after which it is mixed, its digest, the busy mask after it.
type BufferRow = (u64, String, u8);

/// The genuine mixer golden's buffers in order (`b` rows, and `r` rows repeating the previous
/// buffer), each mixed after tic floor(512k/315) under the declared schedule.
fn golden_buffers(name: &str) -> Vec<BufferRow> {
    let mut out: Vec<BufferRow> = Vec::new();
    for l in read(&format!("m6a_mixer/goldens/{name}.golden")).lines() {
        if let Some(r) = l.strip_prefix("b ") {
            let f: Vec<&str> = r.split(' ').collect();
            let k: u64 = f[0].parse().unwrap();
            let tic: u64 = f[1].parse().unwrap();
            assert_eq!(k, out.len() as u64, "{name}: {l}");
            assert_eq!(tic, 512 * k / 315, "{name}: {l}");
            out.push((tic, f[2].to_string(), u8::from_str_radix(f[3], 16).unwrap()));
        } else if let Some(r) = l.strip_prefix("r ") {
            let f: Vec<u64> = r.split(' ').map(|x| x.parse().unwrap()).collect();
            let (_, d, m) = out.last().cloned().expect("a b row before an r row");
            for _ in 0..f[0] {
                let k = out.len() as u64;
                out.push((512 * k / 315, d.clone(), m));
            }
            assert_eq!(out.len() as u64 - 1, f[1], "{name}: {l}");
            assert_eq!(out.last().unwrap().0, f[2], "{name}: {l}");
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
            assert_eq!(field(r, 4), "none", "an intermission start: {r}");
            match field(r, 3) {
                "1" => IntermissionSound::Pistol,
                "82" => IntermissionSound::Barexp,
                "3" => IntermissionSound::Sgcock,
                s => panic!("intermission sfx {s}"),
            }
        })
        .collect()
}

/// Where a drive restores the session from its own saved bytes and continues.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Point {
    /// Before session tic `t` (no display pending).
    Before(i64),
    /// In the display of tic `t` after `k` of its melt steps (0: right after the tic call;
    /// [`ChainMode::FinaleAndMelt`] only).
    Melt(i64, u32),
}

/// One item's drive.
struct Drive {
    name: &'static str,
    mode: ChainMode,
    sim: Simulation,
    carrier: bool,
    commands: Vec<TicCommand>,
    chain: ChainSound,
    genuine: Genuine,
    states: BTreeMap<i64, i64>,
    melts: BTreeMap<i64, u32>,
    file: &'static str,
    skill: Skill,
    carry: Carry,
    /// Rust rows per compared tic.
    rows: Vec<Vec<String>>,
    /// Per compared tic, the buffer mixed after it and its display (digest, mask).
    tic_buffers: Vec<Option<(String, u8)>>,
    /// The buffers in the order mixed.
    mixed: Vec<BufferRow>,
    listener: Option<doom_sim::ActorId>,
    /// The saved bytes before each driven tic (version 3, or version 2 under the frozen
    /// configuration; empty when the session holds a planted fault).
    saved: Vec<Vec<u8>>,
    level_index: usize,
    set: Arc<SoundSet>,
    music: MusicLumps,
    config: ChainConfig,
    restore: Vec<Point>,
    restores: usize,
    /// At a load tic, the simulation a restore in its display checks against: the new level's
    /// carrier after its tic 0 (a restore checks the level tics run and the origins).
    load_sim: Option<Simulation>,
    plant: Option<ChainFault>,
    /// Melt steps after which the session differed from before the step (tic, step).
    melt_changed: Vec<(i64, u32)>,
    melts_run: usize,
    steps_run: u64,
    frozen_refusals: usize,
    refusal_tic: Option<i64>,
}

/// Facts of one drive.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Outcome {
    tics: usize,
    level_tics: usize,
    intermission_tics: usize,
    finale_tics: usize,
    loads: usize,
    refusal: Option<String>,
}

fn listener_of(sim: &Simulation) -> Listener {
    simulation_listener(sim).expect("listener")
}

fn digest(b: &Buffer) -> String {
    sha256(&buffer_bytes(b))
}

impl Drive {
    fn new(name: &'static str, set: &str, demo: &str, mode: ChainMode) -> Result<Self, String> {
        let wads = wadset(set);
        let dir = directory(&wads);
        let sound_set = Arc::new(SoundSet::load(&dir, SoundCaps::default()).expect("sound set"));
        let music = MusicLumps::from_directory(&dir);
        let config = ChainConfig::fresh(texture_count(&dir));
        let lmp = std::fs::read(reference(&format!("m6a_scripts/{demo}.lmp"))).expect("demo");
        let map = lmp[3];
        let chain = ChainSound::new_with(sound_set.clone(), music, config, i32::from(map), mode)
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
            mode,
            listener: simulation_listener(&sim).map(|l| l.actor),
            sim,
            carrier,
            commands: demo.commands,
            chain,
            genuine: genuine(name),
            states: gamestates(name),
            melts: melts(name),
            file: wads[0],
            skill,
            carry: Carry::default(),
            rows: Vec::new(),
            tic_buffers: Vec::new(),
            mixed: Vec::new(),
            saved: Vec::new(),
            level_index: 0,
            set: sound_set,
            music,
            config,
            restore: Vec::new(),
            restores: 0,
            load_sim: None,
            plant: None,
            melt_changed: Vec::new(),
            melts_run: 0,
            steps_run: 0,
            frozen_refusals: 0,
            refusal_tic: None,
        })
    }

    fn index_in(&self, t: i64, entering_intermission: bool) -> u8 {
        // (A tic after one the genuine engine never ran, only under a planted fault, takes 0.)
        let prev = match self.genuine.by_tic.get(&(t - 1)) {
            Some(rows) if t > 0 => r_indices(rows)[2],
            _ => 0,
        };
        (prev + if entering_intermission { 10 } else { 0 }) as u8
    }

    /// The session's saved bytes in its configuration's version.
    fn save_bytes(&self) -> Option<Vec<u8>> {
        match self.mode {
            ChainMode::RefusesMelt => Some(self.chain.save().encode()),
            ChainMode::FinaleAndMelt => self.chain.save_v3().map(|s| s.encode()),
        }
    }

    /// Restores the session from its own saved bytes against `self.sim`, or at a load tic the new
    /// level's (byte-equal round trip, the restored session equal to the one it came from).
    fn restore_here(&mut self, at: Point) {
        let bytes = self.save_bytes().expect("a saved state");
        let sim = self.load_sim.as_ref().unwrap_or(&self.sim);
        let restored = match self.mode {
            ChainMode::RefusesMelt => {
                let st = ChainSoundState::decode(&bytes).expect("decode");
                assert_eq!(st.encode(), bytes, "{} {at:?}: round trip", self.name);
                ChainSound::restore(self.set.clone(), self.music, &st, sim, self.config)
            }
            ChainMode::FinaleAndMelt => {
                let st = ChainSoundStateV3::decode(&bytes).expect("decode v3");
                assert_eq!(st.encode(), bytes, "{} {at:?}: round trip", self.name);
                assert_eq!(st.state.melt, matches!(at, Point::Melt(..)));
                ChainSound::restore_v3(self.set.clone(), self.music, &st, sim, self.config)
            }
        }
        .unwrap_or_else(|e| panic!("{} {at:?}: restore: {e:?}", self.name));
        assert_eq!(
            restored, self.chain,
            "{} {at:?}: the restored session",
            self.name
        );
        self.chain = restored;
        self.restores += 1;
    }

    /// The display of tic `t` after its tic call (module documentation); the buffer mixed after
    /// it. `restorable`: `self.sim` is the simulation a restore checks against (not at a load).
    fn display(&mut self, t: i64, ct: &ChainTic, restorable: bool) -> Option<Buffer> {
        let steps = self.melts.get(&t).copied().unwrap_or(0);
        match self.mode {
            ChainMode::RefusesMelt => {
                if steps > 0 {
                    let before = self.chain.clone();
                    assert_eq!(
                        self.chain.melt_step(),
                        Err(ChainRefusal::NotYet(NotYet::MeltSteps))
                    );
                    assert_eq!(
                        self.chain, before,
                        "{}: a frozen refusal changed it",
                        self.name
                    );
                    self.frozen_refusals += 1;
                }
                assert_eq!(
                    self.chain.mix_after_display(),
                    Err(ChainRefusal::NoDisplayPending)
                );
                ct.sound.buffer
            }
            ChainMode::FinaleAndMelt => {
                assert!(
                    ct.sound.buffer.is_none(),
                    "{} tic {t}: mixed in the tic",
                    self.name
                );
                assert!(self.chain.display_pending());
                for k in 0..=steps {
                    if self.restore.contains(&Point::Melt(t, k)) {
                        assert!(restorable || self.load_sim.is_some(), "{}: {t}", self.name);
                        self.restore_here(Point::Melt(t, k));
                    }
                    if k == steps {
                        break;
                    }
                    let before = self.chain.clone();
                    self.chain
                        .melt_step()
                        .unwrap_or_else(|e| panic!("{} tic {t} step {}: {e}", self.name, k + 1));
                    if self.chain != before {
                        self.melt_changed.push((t, k + 1));
                    }
                    self.steps_run += 1;
                }
                if steps > 0 {
                    self.melts_run += 1;
                }
                let b = self.chain.mix_after_display().expect("the display's mix");
                assert!(!self.chain.display_pending());
                b
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_rows(
        &mut self,
        t: i64,
        ct: &ChainTic,
        records: &[SoundRecord],
        level: bool,
        old_listener: Option<doom_sim::ActorId>,
        new_level: bool,
        buffer: Option<Buffer>,
        updates: u64,
    ) {
        // A tic the genuine engine never ran (only under a planted fault) has no genuine row: its
        // projected columns are written as -1, and the comparison fails on it.
        let none = vec![format!("R,{t},-1,-1,-1,-1,-1,-1,-1,-1")];
        let genuine_rows = self.genuine.by_tic.get(&t).unwrap_or(&none).clone();
        let g = r_indices(&genuine_rows);
        let rows = stream_rows_with(
            &ct.sound,
            records,
            self.chain.sound(),
            self.listener,
            &mut self.carry,
            g[1] as u8,
            g[2] as u8,
        );
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
        // The status-bar and wipe draw counts are the display's (doom-render); projected from the
        // genuine row. Outside a level the C writes -1 for the two indices after P_Ticker and
        // ST_Ticker. The `updated` column is the session's own count of S_UpdateSounds calls over
        // the tic and its display (the genuine runner writes 1: it ran exactly one).
        let genuine_r = genuine_rows
            .iter()
            .find(|r| r.starts_with("R,"))
            .expect("R row")
            .clone();
        let gf: Vec<&str> = genuine_r.split(',').collect();
        let r = rows.last_mut().expect("R row");
        let mut f: Vec<String> = r.split(',').map(str::to_string).collect();
        f[6] = gf[6].into();
        f[7] = gf[7].into();
        f[8] = updates.to_string();
        if !level {
            f[2] = "-1".into();
            f[3] = "-1".into();
        }
        *r = f.join(",");
        let mask = self.chain.sound().mixer().mask();
        let row = buffer.as_ref().map(|b| (digest(b), mask));
        if let Some((d, m)) = &row {
            self.mixed.push((t as u64, d.clone(), *m));
        }
        self.tic_buffers.push(row);
        self.rows.push(rows);
    }

    /// Drives the item as far as the module documentation allows.
    fn run(&mut self) -> Outcome {
        if self.plant.is_some() {
            self.chain.plant_fault(self.plant);
        }
        let mut out = Outcome::default();
        let mut first_level = true;
        let last = *self.states.keys().last().unwrap_or(&-1);
        let refused_tic = self
            .genuine
            .error
            .as_ref()
            .map(|(t, m)| if m.contains("HELP2") { *t } else { t + 1 });
        let mut t: i64 = 0;
        let mut last_gs = 0;
        loop {
            let refusal_here = refused_tic == Some(t);
            // Under a planted fault the session may run the genuine refusal tic: one more tic is
            // then driven, in the same game state, to see where the session refuses.
            let past = refused_tic.is_some_and(|r| t == r + 1) && self.refusal_tic.is_none();
            if t > last && !refusal_here && !(past && self.plant.is_some()) {
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
                self.states.get(&t).copied().unwrap_or(last_gs)
            };
            let prev = if t == 0 {
                0
            } else {
                self.states.get(&(t - 1)).copied().unwrap_or(last_gs)
            };
            last_gs = gs;
            let load = self.genuine.loads.get(&t).copied();
            self.saved.push(self.save_bytes().unwrap_or_default());
            if self.restore.contains(&Point::Before(t)) {
                self.restore_here(Point::Before(t));
            }
            let before = self.chain.clone();
            let updates = self.chain.updates();
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
                    let b = self.display(t, &ct, true);
                    let n = self.chain.updates() - updates;
                    self.push_rows(t, &ct, &records, true, self.listener, false, b, n);
                    out.level_tics += 1;
                }
                0 => {
                    let map = load.expect("load");
                    if prev == 0 {
                        let e = self
                            .sim
                            .advance([TicCommand::default(); 4])
                            .expect_err("reborn");
                        assert!(format!("{e:?}").contains("PlayerReborn"), "{e:?}");
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
                    let load_records = if sawup {
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
                                }
                            );
                            assert_eq!(self.chain, before, "{}: a refusal changed it", self.name);
                            out.refusal = Some(format!("{e}"));
                            self.refusal_tic = Some(t);
                            break;
                        }
                        r => {
                            let ct = r.unwrap_or_else(|e| panic!("{} tic {t}: {e}", self.name));
                            let old = self.listener;
                            self.listener = Some(listener.actor);
                            self.level_index += 1;
                            if self
                                .restore
                                .iter()
                                .any(|p| matches!(p, Point::Melt(m, _) if *m == t))
                            {
                                let mut next = carrier.clone();
                                next.advance([TicCommand::default(); 4])
                                    .unwrap_or_else(|e| panic!("{} tic {t}: {e:?}", self.name));
                                self.load_sim = Some(next);
                            }
                            let b = self.display(t, &ct, false);
                            self.load_sim = None;
                            let n = self.chain.updates() - updates;
                            self.push_rows(t, &ct, &load_records, true, old, true, b, n);
                            out.loads += 1;
                            out.tics += 1;
                            break; // the new level's later tics need wire 26
                        }
                    }
                }
                1 | 2 => {
                    let entering = prev == 0;
                    if entering {
                        if !self.carrier {
                            let e = self
                                .sim
                                .advance([TicCommand::default(); 4])
                                .expect_err("exit");
                            assert!(format!("{e:?}").contains("LevelExited"), "{e:?}");
                        }
                        first_level = false;
                    }
                    let rows = self.genuine.by_tic.get(&t).cloned().unwrap_or_default();
                    let listener = listener_of(&self.sim);
                    let sim = self.sim.clone();
                    let positions = SimulationPositions(&sim);
                    let (r, records) = if gs == 1 {
                        let starts = projected_starts(&rows);
                        let idx = self.index_in(t, entering);
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
                                }
                            );
                            assert_eq!(self.chain, before, "{}: a refusal changed it", self.name);
                            out.refusal = Some(format!("{e}"));
                            self.refusal_tic = Some(t);
                            break;
                        }
                        Err(e) => {
                            // Only a planted fault gets here (a refusal the genuine engine does
                            // not make at this tic): recorded, and the drive stops.
                            out.refusal = Some(format!("{e}"));
                            self.refusal_tic = Some(t);
                            break;
                        }
                        Ok(ct) => {
                            let b = self.display(t, &ct, true);
                            let n = self.chain.updates() - updates;
                            self.push_rows(t, &ct, &records, false, self.listener, false, b, n);
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

    /// The genuine refusal tic (the tic the genuine engine stops at, for a sound refusal).
    fn genuine_refusal_tic(&self) -> Option<i64> {
        self.genuine
            .error
            .as_ref()
            .filter(|(_, m)| !m.contains("HELP2"))
            .map(|(t, _)| t + 1)
    }

    /// The mixed buffers against the genuine golden's, in order; the number compared.
    fn compare_buffers(&self) -> Result<usize, String> {
        let want = golden_buffers(self.name);
        for (k, got) in self.mixed.iter().enumerate() {
            match want.get(k) {
                Some(w) if w == got => {}
                w => {
                    return Err(format!(
                        "{}: buffer {k}:\n  golden: {w:?}\n  rust:   {got:?}",
                        self.name
                    ));
                }
            }
        }
        Ok(self.mixed.len())
    }
}

fn item(name: &str) -> (&'static str, &'static str, &'static str) {
    ITEMS.iter().copied().find(|i| i.0 == name).expect("item")
}

fn drive(name: &str, mode: ChainMode) -> Drive {
    let (n, set, demo) = item(name);
    Drive::new(n, set, demo, mode).expect("drive")
}

#[test]
fn the_item_table_is_the_manifests() {
    let text = read("m6a_manifest.json");
    assert_eq!(text.matches("\"lmp\": ").count(), ITEMS.len());
    for (name, set, demo) in ITEMS {
        let at = text
            .find(&format!("\"name\": \"{name}\""))
            .unwrap_or_else(|| panic!("{name}"));
        let rest = &text[at..];
        let end = rest.find('}').unwrap();
        assert!(
            rest[..end].contains(&format!("\"wadset\": \"{set}\"")),
            "{name}"
        );
        assert!(
            rest[..end].contains(&format!(
                "\"lmp\": \"tests/reference/m6a_scripts/{demo}.lmp\""
            )),
            "{name}"
        );
    }
}

/// Totals over the drivable items.
#[derive(Debug, Default)]
struct Totals {
    items: usize,
    tics: usize,
    finale_tics: usize,
    intermission_tics: usize,
    loads: usize,
    buffers: usize,
    golden_buffers: usize,
    melts: usize,
    steps: u64,
    melts_in_goldens: usize,
    steps_in_goldens: u64,
    genuine_tics: usize,
    updates_totals_checked: usize,
}

/// Every drivable item under [`ChainMode::FinaleAndMelt`]: every driven tic's rows (S, T, K, C,
/// L, R, the R row's `updated` from the session's update count) equal to the genuine rows; every
/// buffer mixed equal, in order, to the genuine mixer golden's; a melt step at every genuine melt
/// step of the driven tics, each leaving the session unchanged; the Z row's `updates` total equal
/// to the session's where the whole run is driven; each refusal at the genuine tic.
#[test]
fn finale_and_melt_against_the_genuine_goldens() {
    let mut counts = RowCounts::default();
    let mut tot = Totals::default();
    let mut gaps = Vec::new();
    for (name, set, demo) in ITEMS {
        let m = melts(name);
        tot.melts_in_goldens += m.len();
        tot.steps_in_goldens += m.values().map(|s| u64::from(*s)).sum::<u64>();
        tot.golden_buffers += golden_buffers(name).len();
        tot.genuine_tics += genuine(name).by_tic.len();
        let mut d = match Drive::new(name, set, demo, ChainMode::FinaleAndMelt) {
            Ok(d) => d,
            Err(e) if name == "NOE1M2AT" => {
                assert_eq!(e, "W_GetNumForName: d_e1m2 not found! (at load)");
                assert_eq!(genuine(name).error.map(|e| e.0), Some(-1));
                gaps.push(format!(
                    "{name}: refused at load, as the genuine engine stops"
                ));
                continue;
            }
            Err(e) if name == "SKILL0" || name == "SKILL1" => {
                // The version-2 demo parser admits skills 2 and 3 only (easy skills: the next pull
                // request).
                gaps.push(format!("{name}: {e}"));
                continue;
            }
            Err(e) => panic!("{name}: {e}"),
        };
        let o = d.run();
        compare_stream(name, &d.rows, &d.genuine_rows(), &mut counts)
            .unwrap_or_else(|e| panic!("{e}"));
        let n = d.compare_buffers().unwrap_or_else(|e| panic!("{e}"));
        assert!(d.melt_changed.is_empty(), "{name}: {:?}", d.melt_changed);
        assert_eq!(
            d.refusal_tic,
            d.genuine_refusal_tic(),
            "{name}: refusal tic"
        );
        let whole = d.rows.len() == d.genuine.by_tic.len();
        if let (true, Some(u)) = (whole, d.genuine.updates) {
            assert_eq!(d.chain.updates(), u, "{name}: S_UpdateSounds calls");
            tot.updates_totals_checked += 1;
        }
        eprintln!(
            "{name}: {o:?}, {} of {} tics, {n} buffers, {} melts, {} steps",
            d.rows.len(),
            d.genuine.by_tic.len(),
            d.melts_run,
            d.steps_run
        );
        tot.items += 1;
        tot.tics += d.rows.len();
        tot.finale_tics += o.finale_tics;
        tot.intermission_tics += o.intermission_tics;
        tot.loads += o.loads;
        tot.buffers += n;
        tot.melts += d.melts_run;
        tot.steps += d.steps_run;
    }
    eprintln!("finale and melt: {tot:?}, rows {counts:?}, gaps {gaps:?}");
    // What exists: 125 melts of 4,946 steps, 10,425 genuine buffers, 16,925 genuine tics over the
    // 29 items. What is driven (the extent of the part 1 driver, module documentation):
    assert_eq!(
        (
            tot.melts_in_goldens,
            tot.steps_in_goldens,
            tot.golden_buffers
        ),
        (125, 4946, 10425)
    );
    assert_eq!((tot.items, tot.tics, tot.genuine_tics), (26, 10064, 16925));
    assert_eq!(
        (tot.finale_tics, tot.intermission_tics, tot.loads),
        (4830, 1276, 17)
    );
    assert_eq!((tot.melts, tot.steps, tot.buffers), (65, 2608, 6204));
    assert_eq!(tot.updates_totals_checked, 5);
}

/// The frozen configuration: wherever both run, its rows and buffers equal the finale-and-melt
/// configuration's, its saved version 2 bytes before every tic are the version 3 bytes with the
/// version field changed, and it still refuses as before: a melt step is
/// `NotYet(MeltSteps)` with nothing changed, it has no display to mix, and its restore refuses a
/// melt in progress, including the one a finale-and-melt session's version 2 view holds.
#[test]
fn the_frozen_configuration_is_unchanged_where_both_run() {
    let mut items = 0;
    let mut refusals = 0;
    let mut tics = 0;
    for (name, set, demo) in ITEMS {
        let (Ok(mut a), Ok(mut b)) = (
            Drive::new(name, set, demo, ChainMode::RefusesMelt),
            Drive::new(name, set, demo, ChainMode::FinaleAndMelt),
        ) else {
            continue;
        };
        a.run();
        b.run();
        assert_eq!(a.rows, b.rows, "{name}: rows");
        assert_eq!(a.tic_buffers, b.tic_buffers, "{name}: buffers");
        assert_eq!(a.mixed, b.mixed, "{name}: buffers in order");
        assert_eq!(a.saved.len(), b.saved.len());
        for (t, (v2, v3)) in a.saved.iter().zip(&b.saved).enumerate() {
            assert_eq!(&v2[4..6], &VERSION.to_le_bytes());
            assert_eq!(&v3[4..6], &VERSION_FINALE_MELT.to_le_bytes());
            assert_eq!(v2[..4], v3[..4]);
            assert_eq!(v2[6..], v3[6..], "{name} tic {t}: v2 and v3 bytes");
        }
        assert_eq!(a.refusal_tic, b.refusal_tic);
        assert_eq!(a.chain.mode(), ChainMode::RefusesMelt);
        refusals += a.frozen_refusals;
        tics += a.rows.len();
        items += 1;
    }
    eprintln!("frozen: {items} items, {tics} tics equal, {refusals} melt steps refused");
    assert_eq!((items, tics, refusals), (26, 10064, 65));

    // A finale-and-melt session in a display, seen through version 2: a melt in progress, which
    // version 2's reader accepts and its restore refuses as before.
    let mut d = drive("FINRET", ChainMode::FinaleAndMelt);
    d.states.retain(|t, _| *t < 321);
    d.run();
    let mut c = d.chain.clone();
    let l = listener_of(&d.sim);
    let r = c
        .finale_tic(0, &l, &SimulationPositions(&d.sim))
        .expect("tic");
    assert!(c.display_pending());
    let v2 = c.save();
    assert!(v2.melt);
    let back = ChainSoundState::decode(&v2.encode()).expect("version 2 reads a melt");
    assert_eq!(
        ChainSound::restore(d.set.clone(), d.music, &back, &d.sim, d.config),
        Err(ChainStateError::NotYet(NotYet::MeltInProgress))
    );
    // The frozen session at the same point mixed the tic's buffer in the call.
    let mut f = drive("FINRET", ChainMode::RefusesMelt);
    f.states.retain(|t, _| *t < 321);
    f.run();
    let mut fc = f.chain.clone();
    let fr = fc
        .finale_tic(0, &l, &SimulationPositions(&f.sim))
        .expect("tic");
    assert_eq!(fr.sound.records, r.sound.records);
    assert_eq!(fr.sound.update, r.sound.update);
    assert!(r.sound.buffer.is_none());
    assert_eq!(c.mix_after_display().expect("mix"), fr.sound.buffer);
    assert_eq!(c.save().encode(), fc.save().encode());
    assert_eq!(
        fc.save_v3(),
        None,
        "the frozen configuration saves version 2"
    );
}

/// `DSND` version 3 restore-and-continue: in the display of every driven melt of CHAIN, INTFAST,
/// REBHELD and FINRET, before each of its steps (all six kinds: the session start, level to
/// intermission, intermission to level, the reborn reload, level to finale and the finale's
/// text-to-art melt), and before several tics in a level, the intermission and the finale, the
/// session is saved, decoded (byte-equal round trip), restored against the level's simulation, the
/// finished level's, or at a load the new level's carrier after its tic 0 (equal to the session it
/// came from) and continued in the same drive: every later row and buffer equal to the
/// uninterrupted drive's.
#[test]
fn dsnd_v3_restore_mid_melt_and_in_the_finale() {
    let mut points = 0;
    let mut melts_covered = 0;
    for (name, before) in [
        ("CHAIN", vec![1, 60, 125, 126, 127, 135, 142]),
        ("INTFAST", vec![55, 56, 60, 69]),
        ("REBHELD", vec![100, 161, 162]),
        ("FINRET", vec![320, 321, 322, 1000, 1890, 1891, 1892, 1950]),
    ] {
        let mut whole = drive(name, ChainMode::FinaleAndMelt);
        whole.run();
        let mut d = drive(name, ChainMode::FinaleAndMelt);
        d.restore = before.iter().map(|t| Point::Before(*t)).collect();
        for (&t, &steps) in &whole.melts {
            if t as usize >= whole.rows.len() {
                continue;
            }
            melts_covered += 1;
            d.restore.extend((0..steps).map(|k| Point::Melt(t, k)));
        }
        d.run();
        assert_eq!(d.rows, whole.rows, "{name}: rows after the restores");
        assert_eq!(d.mixed, whole.mixed, "{name}: buffers after the restores");
        assert_eq!(d.restores, d.restore.len(), "{name}: every point reached");
        points += d.restores;
        for (t, bytes) in whole.saved.iter().enumerate() {
            let st =
                ChainSoundStateV3::decode(bytes).unwrap_or_else(|e| panic!("{name} {t}: {e:?}"));
            assert_eq!(&st.encode(), bytes, "{name} tic {t}");
            assert!(bytes.len() <= MAX_BYTES);
        }
    }
    eprintln!("DSND v3: {points} restore points, {melts_covered} melts restored at every step");
    // 22 points before a tic and every step of 11 melts (440 steps of the genuine melt golden).
    assert_eq!(melts_covered, 11);
    assert_eq!(points, 462);
}

/// The call order and the reader of version 3 against a session's own states: a tic while a
/// display is pending, a melt step or a mix with none; version 2 and 3 refuse each other's
/// bytes; a mid-melt state restores with its buffer owed; the restore checks of version 2.
#[test]
fn dsnd_v3_call_order_and_cross_version() {
    let mut d = drive("CHAIN", ChainMode::FinaleAndMelt);
    d.states.retain(|t, _| *t < 130);
    d.run();
    let (sim, l) = (d.sim.clone(), listener_of(&d.sim));
    let p = SimulationPositions(&sim);
    let mut c = d.chain.clone();
    // No display pending: a melt step and a mix are refused, nothing changed.
    let before = c.clone();
    assert_eq!(c.melt_step(), Err(ChainRefusal::NoDisplayPending));
    assert_eq!(c.mix_after_display(), Err(ChainRefusal::NoDisplayPending));
    assert_eq!(c, before);
    // A tic, then a second tic before its display is mixed.
    c.intermission_tic(&[], 0, &l, &p).expect("tic");
    let pending = c.clone();
    let r = c.intermission_tic(&[], 0, &l, &p);
    assert_eq!(
        r.map(|_| ()),
        Err(ChainRefusal::DisplayPending {
            call: ChainCall::Intermission
        })
    );
    assert_eq!(c, pending);
    for (r, call) in [
        (
            c.level_tic(None, &[], 0, &l, &p).map(|_| ()),
            ChainCall::Level,
        ),
        (c.finale_tic(0, &l, &p).map(|_| ()), ChainCall::Finale),
    ] {
        // The state check comes first, as in part 1: in the intermission both are WrongState.
        assert!(matches!(r, Err(ChainRefusal::WrongState { call: c2, .. }) if c2 == call));
    }
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
    assert_eq!(
        r.map(|_| ()),
        Err(ChainRefusal::DisplayPending {
            call: ChainCall::Load
        })
    );
    assert_eq!(c, pending);
    // Mid-melt (pending) state: version 3 restores it, version 2's reader refuses its bytes.
    let v3 = c.save_v3().expect("v3").encode();
    assert_eq!(
        ChainSoundState::decode(&v3),
        Err(ChainStateError::State(StateError::Version(3)))
    );
    let v2 = c.save().encode();
    assert_eq!(
        ChainSoundStateV3::decode(&v2),
        Err(ChainStateError::State(StateError::Version(2)))
    );
    let st = ChainSoundStateV3::decode(&v3).expect("v3");
    let owed =
        Schedule::buffers_before(st.state.tics) != Schedule::buffers_before(st.state.tics - 1);
    assert_eq!(
        st.state.buffers,
        Schedule::buffers_before(st.state.tics - 1)
    );
    let mut restored = ChainSound::restore_v3(d.set.clone(), d.music, &st, &sim, d.config)
        .expect("restore mid-melt");
    assert_eq!(restored, c);
    assert_eq!(restored.buffer_owed(), owed);
    assert_eq!(restored.buffers_mixed(), st.state.buffers);
    for _ in 0..43 {
        restored.melt_step().expect("step");
    }
    assert_eq!(restored, c, "melt steps change nothing");
    assert_eq!(restored.mix_after_display(), c.clone().mix_after_display());
    // The restore checks of version 2, through version 3.
    let mut other = d.config;
    other.sfx_volume = 9;
    assert!(matches!(
        ChainSound::restore_v3(d.set.clone(), d.music, &st, &sim, other),
        Err(ChainStateError::State(StateError::ConfigMismatch { .. }))
    ));
    assert_eq!(
        ChainSound::restore_v3(
            d.set.clone(),
            d.music.without(Music::Inter),
            &st,
            &sim,
            d.config
        ),
        Err(ChainStateError::MusicMissing(Music::Inter))
    );
    let fresh = drive("CHAIN", ChainMode::FinaleAndMelt);
    assert!(matches!(
        ChainSound::restore_v3(d.set.clone(), d.music, &st, &fresh.sim, d.config),
        Err(ChainStateError::TicMismatch {
            state: 126,
            simulation: 0
        })
    ));
    // A hand-built pending state with no tic run is refused, never a panic.
    let mut zero = st.clone();
    zero.state.tics = 0;
    zero.state.buffers = 0;
    zero.state.level_end = 0;
    assert!(ChainSound::restore_v3(d.set.clone(), d.music, &zero, &sim, d.config).is_err());
    // A planted fault blocks the version 3 save.
    let mut f = c.clone();
    f.plant_fault(Some(ChainFault::UpdateInMelt));
    assert_eq!(f.save_v3(), None);
}

/// Planted faults, each caught by the comparison with the genuine goldens: a buffer mixed during
/// a melt step (the owed buffer at the first step: the buffer sequence differs from the genuine
/// mixer's at the session-start melt), a sound update run during a melt step (the R row's update
/// count differs at the first melt, and the Z row's total), and the `D_VICTOR` refusal one tic
/// late (a finale tic the genuine engine never ran, and the refusal tic). The Rust-only check that
/// a melt step leaves the session unchanged also flags the first two.
#[test]
fn the_comparisons_can_fail() {
    // A buffer mixed during a melt step.
    let mut d = drive("FINRET", ChainMode::FinaleAndMelt);
    d.plant = Some(ChainFault::MixInMelt);
    d.run();
    let e = d.compare_buffers().expect_err("caught");
    assert!(e.contains("buffer 0:"), "{e}");
    assert_eq!(d.melt_changed.first(), Some(&(0, 1)));

    // A sound update during a melt step.
    let mut d = drive("FINRET", ChainMode::FinaleAndMelt);
    d.plant = Some(ChainFault::UpdateInMelt);
    d.run();
    let e = compare_stream(
        "FINRET",
        &d.rows,
        &d.genuine_rows(),
        &mut RowCounts::default(),
    )
    .expect_err("caught");
    assert!(e.contains("tic 0 ") && e.contains("field 8"), "{e}");
    assert_ne!(Some(d.chain.updates()), d.genuine.updates);
    assert_eq!(d.melt_changed.first(), Some(&(0, 1)));
    assert_eq!(d.compare_buffers(), Ok(d.mixed.len()), "buffers unchanged");

    // The D_VICTOR refusal one tic late.
    let mut d = drive("NOVICTOR", ChainMode::FinaleAndMelt);
    d.plant = Some(ChainFault::VictorLate);
    d.run();
    assert_eq!(d.genuine_refusal_tic(), Some(321));
    assert_eq!(d.refusal_tic, Some(322));
    let e = compare_stream(
        "NOVICTOR",
        &d.rows,
        &d.genuine_rows(),
        &mut RowCounts::default(),
    )
    .expect_err("caught");
    assert!(e.contains("tic 321 "), "{e}");
    // Without the fault the refusal is at the genuine tic, with the genuine message.
    let mut d = drive("NOVICTOR", ChainMode::FinaleAndMelt);
    let o = d.run();
    assert_eq!(d.refusal_tic, Some(321));
    assert_eq!(
        o.refusal.as_deref(),
        Some("W_GetNumForName: d_victor not found! (tic 321)")
    );
}

/// Pins what the plain part 1 call `ChainSound::save` does on a `ChainMode::FinaleAndMelt` chain
/// (owner's ruling of 2026-11-24: the plain calls keep their signatures; the checked calls
/// refuse by type). It gives a `DSND` version 2 blob, and version 2's `restore` refuses that blob
/// only while a display is pending (`NotYet::MeltInProgress`, the chain_state.rs `state.melt`
/// test); outside a pending display `restore` ACCEPTS it and yields a chain of the frozen
/// configuration (`ChainMode::RefusesMelt`), which the checked saves show. The public API reaches
/// everything here; no guard below it is pinned.
#[test]
fn plain_save_on_a_finale_and_melt_chain_is_version_2() {
    // Outside a pending display: in a level, in the intermission and in the finale.
    let mut accepted = 0;
    for (name, before, want) in [
        ("CHAIN", Some(60), doom_sound::chain::ChainState::Level),
        (
            "CHAIN",
            Some(130),
            doom_sound::chain::ChainState::Intermission,
        ),
        ("FINRET", None, doom_sound::chain::ChainState::Finale),
    ] {
        let mut d = drive(name, ChainMode::FinaleAndMelt);
        if let Some(t) = before {
            d.states.retain(|k, _| *k < t);
        }
        d.run();
        let c = d.chain.clone();
        assert_eq!(c.state(), want, "{name}");
        assert!(!c.display_pending(), "{name}");
        let v2 = c.save().encode();
        assert_eq!(&v2[..6], b"DSND\x02\0", "{name}: version 2");
        let st = ChainSoundState::decode(&v2).expect("decode v2");
        assert!(!st.melt, "{name}");
        assert_eq!(
            ChainSoundStateV3::decode(&v2),
            Err(ChainStateError::State(StateError::Version(2)))
        );
        assert_eq!(
            c.save_v2_checked(),
            Err(doom_sound::chain_melt::ChainSaveRefusal::FinaleAndMeltSavesVersion3)
        );
        let back = ChainSound::restore(d.set.clone(), d.music, &st, &d.sim, d.config)
            .unwrap_or_else(|e| panic!("{name}: v2 restore: {e:?}"));
        // The restored chain is the frozen configuration: its checked saves say so.
        assert_eq!(back.save(), c.save(), "{name}");
        assert!(back.save_v2_checked().is_ok(), "{name}");
        assert_eq!(
            back.save_v3_checked(),
            Err(doom_sound::chain_melt::ChainSaveRefusal::RefusesMeltSavesVersion2),
            "{name}"
        );
        accepted += 1;
    }
    assert_eq!(accepted, 3);
    // Mid-display (a tic called, its display pending): the blob is version 2 with the melt flag,
    // and version 2's restore refuses it.
    let mut d = drive("CHAIN", ChainMode::FinaleAndMelt);
    d.states.retain(|t, _| *t < 130);
    d.run();
    let (sim, l) = (d.sim.clone(), listener_of(&d.sim));
    let p = SimulationPositions(&sim);
    let mut c = d.chain.clone();
    c.intermission_tic(&[], 0, &l, &p).expect("tic");
    assert!(c.display_pending());
    let v2 = c.save().encode();
    assert_eq!(&v2[..6], b"DSND\x02\0");
    let st = ChainSoundState::decode(&v2).expect("decode v2");
    assert!(st.melt);
    assert_eq!(
        ChainSound::restore(d.set.clone(), d.music, &st, &sim, d.config),
        Err(ChainStateError::NotYet(NotYet::MeltInProgress))
    );
    assert_eq!(
        c.save_v2_checked(),
        Err(doom_sound::chain_melt::ChainSaveRefusal::FinaleAndMeltSavesVersion3)
    );
}
