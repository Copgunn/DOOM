//! Wire 29 in the session (M6b step 7): the frames, the pause picture, the sound and the mixer of
//! `Configuration::FinaleAndMeltWire29` against the genuine linuxdoom-1.10 goldens of the M6b items,
//! every tic. No Rust output feeds an expectation: the frame streams (`tests/reference/m6b_frames/`),
//! the paused frames and music calls (`m6b_pause/`, the M6b stream's B and MP rows) and the sound
//! streams (`m6b_sound/`) are the genuine runner's (`tools/reference/m6b_goldens.py`, frozen), the
//! mixer goldens the genuine mixer's on those streams (`tools/reference/wire29_mixer.py`).
//!
//! Every tic of a run, as `wire28_genuine.rs` compares wire 28 (its helpers are private to it and
//! are not changed): a level frame's whole and face-zeroed digests, its F row rebuilt from the
//! renderer's counters, newly mapped lines, palette, face, `message_on` and `M_Random` index; a
//! frame outside a level whole with palette and index; for every paused tic its B row (digest,
//! `leveltime`, `gametic`, both indices; no B row on any other tic); the tic's music calls against
//! its MP rows; every sound row (actor handles matched to the genuine serials by a bijection); and
//! after the run every mixer buffer.
//!
//! Runs: PAUSE, PAUSEWI and PAUSEFIN at screenblocks 10 and 11, PAUSE0 at 10 (the pause items);
//! RESP4 and RESP3R at 10 (kills past 100 % on the intermission screen); BLOCK4 and GIB4 sound and
//! mixer only (no frame golden); DEADSPEC, LOADP (a paused load tic: the level drawn with the
//! picture before its tic 0, under a melt) and TURBOP at 10, their streams from the same runner
//! (`tools/reference/wire29_pause.py`, `tests/reference/wire29_session/`).
//!
//! Planted: `pause_hu` (the session's `PlantedFault::PauseHu`) is caught at the frame whose line its
//! `V` row's digest names, the port's line there hashing to the variant's; the self-test of
//! `m6b_goldens.py` (the picture one row lower) changes the first paused frame.
mod m6a;
#[path = "../../doom-sound/tests/common/rows.rs"]
mod rows;

use doom_game::session::finale_melt::FinaleMeltTicKind as Kind;
use doom_game::session::{PlantedFault, Session, SessionSettings};
use doom_render::{Frame, GameState, SCREENHEIGHT, SCREENWIDTH};
use doom_sim::{GameParameters, TicCommand};
use doom_sound::chain_pause::{MUSIC_HANDLE, MusicCall};
use doom_sound::logic::{SoundOrigin, SoundRecord};
use doom_test_evidence::sha256;
use m6a::{Rows, int};
use rows::{Carry, GoldenWriter, drain, stream_rows_with};
use std::collections::BTreeMap;

const SHARED: [&str; 5] = [
    "tests/reference/m5b_gfx.wad",
    "tests/reference/m5c_sfx.wad",
    "tests/reference/m6a_wads/m6a_gfx.wad",
    "tests/reference/m6a_wads/m6a_music.wad",
    "tests/reference/m6b_wads/m6b_gfx.wad",
];
/// (item, screen size, frames compared).
const RUNS: [(&str, u8, bool); 14] = [
    ("PAUSE", 10, true),
    ("PAUSE", 11, true),
    ("PAUSE0", 10, true),
    ("PAUSEWI", 10, true),
    ("PAUSEWI", 11, true),
    ("PAUSEFIN", 10, true),
    ("PAUSEFIN", 11, true),
    ("RESP4", 10, true),
    ("RESP3R", 10, true),
    ("BLOCK4", 10, false),
    ("GIB4", 10, false),
    ("DEADSPEC", 10, true),
    ("LOADP", 10, true),
    ("TURBOP", 10, true),
];

fn read(path: &str) -> String {
    std::fs::read_to_string(m6a::root(&format!("tests/reference/{path}")))
        .expect(path)
        .replace("\r\n", "\n")
}

/// The item as `m6a::data` takes it, its parameters and its commands (the demo's header and body).
fn item(name: &str) -> (m6a::Item, GameParameters, Vec<TicCommand>) {
    let episode = matches!(name, "PAUSEWI" | "PAUSEFIN" | "LOADP" | "DEADSPEC");
    let first = [
        "tests/reference/m6b_wads/m6b.wad",
        "tests/reference/m6a_wads/episode.wad",
    ];
    let dir = ["m6b_scripts", "wire29_session"][usize::from(new_item(name))];
    let lmp_path = format!("tests/reference/{dir}/{name}.lmp");
    let lmp = std::fs::read(m6a::root(&lmp_path)).expect("demo");
    let commands: Vec<TicCommand> = lmp[13..lmp.len() - 1]
        .chunks_exact(4)
        .map(|c| TicCommand {
            forward: c[0] as i8,
            side: c[1] as i8,
            angle_turn: (u16::from(c[2]) << 8) as i16,
            buttons: c[3],
        })
        .collect();
    let item = m6a::Item {
        name: name.into(),
        mode: "registered".into(),
        wads: std::iter::once(first[usize::from(episode)])
            .chain(SHARED)
            .map(String::from)
            .collect(),
        lmp: lmp_path,
        map: lmp[3],
        skill: lmp[1].min(4),
        end: "marker".into(),
        session_tics: commands.len(),
    };
    let parameters = GameParameters {
        respawn: lmp[5] == 1,
        fast: lmp[6] == 1,
    };
    (item, parameters, commands)
}

/// The wire 29 items, whose goldens are under `wire29_session/` (`tools/reference/wire29_pause.py`).
fn new_item(name: &str) -> bool {
    matches!(name, "DEADSPEC" | "LOADP" | "TURBOP")
}

/// The genuine sound stream's rows by tic (`m6b_sound/<item>.sound` or `wire29_session/`).
fn genuine_sound(path: &str) -> BTreeMap<i64, Vec<String>> {
    let mut by_tic: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    for l in read(path).lines() {
        if l.starts_with('#') || l.starts_with("Z,") || l.starts_with("W,") || l.starts_with("LV,")
        {
            continue;
        }
        let tic = l.split(',').nth(1).and_then(|t| t.parse().ok()).expect(l);
        by_tic.entry(tic).or_default().push(l.to_owned());
    }
    by_tic
}

/// The genuine goldens of one run.
struct Golden {
    frames: Option<BTreeMap<i64, Frame29>>,
    /// The M6b stream's B rows by tic and MP rows by tic (screenblocks `sb`).
    pause: Option<(Rows, BTreeMap<i64, Vec<String>>)>,
    sound: Option<BTreeMap<i64, Vec<String>>>,
    mixer: Option<Vec<String>>,
}

/// A frame of the frame stream: the F line (or the G row's columns) and the P and Q rows.
#[derive(Clone, Debug, Default)]
struct Frame29 {
    gamestate: i64,
    f_line: String,
    sha256: String,
    mapped: Vec<u16>,
    status: Vec<i64>,
    noface: String,
    q: [i64; 6],
}

fn read_frames(path: &str) -> BTreeMap<i64, Frame29> {
    let mut out: BTreeMap<i64, Frame29> = BTreeMap::new();
    for l in read(path).lines().filter(|l| !l.starts_with('#')) {
        let c: Vec<&str> = l.split(',').collect();
        let n = |i: usize| {
            c[i].parse::<i64>()
                .unwrap_or_else(|_| panic!("{path}: {l}"))
        };
        let at = n(1);
        match c[0] {
            "F" => drop(out.insert(
                at,
                Frame29 {
                    f_line: l.to_owned(),
                    sha256: c[2].into(),
                    ..Frame29::default()
                },
            )),
            "M" => {
                out.get_mut(&at).unwrap().mapped =
                    c[2].split(' ').map(|v| v.parse().unwrap()).collect()
            }
            "P" => out.get_mut(&at).unwrap().status = (2..=6).map(n).collect(),
            "Q" => {
                let g = out.get_mut(&at).unwrap();
                g.noface = c[2].into();
                g.q = std::array::from_fn(|i| n(3 + i));
            }
            "G" => drop(out.insert(
                at,
                Frame29 {
                    gamestate: n(2),
                    sha256: c[3].into(),
                    status: vec![n(4), n(5), n(6)],
                    ..Frame29::default()
                },
            )),
            "X" | "E" => {}
            _ => panic!("{path}: unexpected row {l}"),
        }
    }
    out
}

fn golden(name: &str, sb: u8, frames: bool) -> Golden {
    let exists = |p: &str| m6a::root(&format!("tests/reference/{p}")).exists();
    let (pause, sound, frame_path) = if new_item(name) {
        let d = "wire29_session";
        (
            format!("{d}/{name}.sb{sb}.pause"),
            format!("{d}/{name}.sound"),
            format!("{d}/{name}.sb{sb}.frames"),
        )
    } else {
        (
            format!("m6b_pause/{name}.sb{sb}.txt"),
            format!("m6b_sound/{name}.sound"),
            format!("m6b_frames/{name}.sb{sb}.frames"),
        )
    };
    let mixer = format!("wire29_mixer/goldens/{name}.golden");
    Golden {
        frames: frames.then(|| read_frames(&frame_path)),
        pause: exists(&pause).then(|| {
            let text = read(&pause);
            let mut mp: BTreeMap<i64, Vec<String>> = BTreeMap::new();
            for l in text.lines().filter(|l| l.starts_with("MP,")) {
                let f: Vec<&str> = l.split(',').collect();
                mp.entry(f[1].parse().unwrap())
                    .or_default()
                    .push(format!("{},{}", f[2], f[3]));
            }
            (Rows::parse(&text), mp)
        }),
        sound: exists(&sound).then(|| genuine_sound(&sound)),
        mixer: exists(&mixer).then(|| {
            read(&mixer)
                .lines()
                .filter(|l| !l.starts_with("d ") && !l.starts_with("q "))
                .map(String::from)
                .collect()
        }),
    }
}

/// What a run compared, and where it first differed.
#[derive(Debug, Default)]
struct Outcome {
    first: Option<(i64, String)>,
    /// The port's F line at the first differing frame.
    f_line: Option<String>,
    tics: usize,
    level_frames: usize,
    other_frames: usize,
    paused_frames: usize,
    music_calls: usize,
    sound_rows: usize,
    mixer_rows: usize,
    buffers: u64,
}

/// Per-run state: the handle bijection, the cumulative draws, the level ordinal.
#[derive(Default)]
struct Run {
    to_c: BTreeMap<String, String>,
    to_rust: BTreeMap<String, String>,
    carry: Carry,
    ordinal: usize,
    /// Cumulative sound, status bar and wipe draws.
    draws: [i64; 3],
}

impl Run {
    /// One sound row against the genuine one (as `wire28_genuine.rs`): equal but for actor
    /// handles, matched to serials one to one; an unplaced start's listener columns unrecorded.
    fn row(&mut self, rust: &str, genuine: &str) -> Result<(), String> {
        let (r, mut g): (Vec<&str>, Vec<&str>) =
            (rust.split(',').collect(), genuine.split(',').collect());
        let unplaced = |f: &[&str]| f.len() == 22 && f[0] == "S";
        if unplaced(&r)
            && unplaced(&g)
            && r[17..].iter().all(|f| *f == "-")
            && g[17..19] == ["-", "-"]
        {
            g.truncate(19);
            g.extend(["-", "-", "-"]);
        }
        if r.len() != g.len() {
            return Err("field count".into());
        }
        for (i, (a, b)) in r.iter().zip(&g).enumerate() {
            let Some(h) = a.strip_prefix('A') else {
                if a != b {
                    return Err(format!("field {i}"));
                }
                continue;
            };
            let (h, s) = (h.to_string(), b.to_string());
            match (self.to_c.get(&h), self.to_rust.get(&s)) {
                (None, None) => {
                    self.to_c.insert(h.clone(), s.clone());
                    self.to_rust.insert(s, h);
                }
                (Some(c), Some(x)) if *c == s && *x == h => {}
                _ => return Err(format!("field {i}: handle {h} against serial {s}")),
            }
        }
        Ok(())
    }
}

/// `gamestate` (doomdef.h:127-133).
fn state_of(s: &Session) -> i64 {
    match s.gamestate() {
        GameState::Level => 0,
        GameState::Intermission => 1,
        GameState::Finale => 2,
    }
}

fn tag(rows: Vec<String>, n: usize) -> Vec<String> {
    let field = |f: &str| {
        f.strip_prefix('A')
            .map_or_else(|| f.to_string(), |h| format!("A{n}/{h}"))
    };
    rows.into_iter()
        .map(|r| r.split(',').map(field).collect::<Vec<_>>().join(","))
        .collect()
}

/// Drives `name` at `sb` (with `fault`) against its goldens.
fn drive(name: &str, sb: u8, frames: bool, fault: Option<PlantedFault>) -> Outcome {
    let (item, parameters, cmds) = item(name);
    let g = golden(name, sb, frames);
    let settings = SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: sb,
    };
    let mut s = Session::new_finale_and_melt_wire29(m6a::data(&item, None), settings, parameters)
        .expect("starts");
    if let Some(f) = fault {
        s.plant_fault(f);
    }
    let mut out = Outcome::default();
    let mut run = Run::default();
    let mut writer = GoldenWriter::new(s.sound().sound().mixer().sound_set());
    let mut listener = s.level().sound_listener().map(|l| l.actor);
    let mut picture = Frame::new();
    let mut owed = 0;
    for (t, &cmd) in cmds.iter().enumerate() {
        let tic = t as i64;
        let r = match s.tic_finale_and_melt(cmd) {
            Ok(r) => r,
            Err(e) => {
                out.first = Some((tic, format!("refused {e}")));
                return out;
            }
        };
        out.tics += 1;
        let new_level = matches!(r.kind, Kind::Load { .. });
        let level = matches!(r.kind, Kind::Level | Kind::Load { .. });
        run.ordinal += usize::from(new_level);
        let add = [
            i64::from(r.sound.sound.draws),
            i64::from(level),
            320 * i64::from(r.wiped),
        ];
        (0..3).for_each(|k| run.draws[k] += add[k]);
        let old = std::mem::replace(&mut listener, s.level().sound_listener().map(|l| l.actor));
        let px = s.frame().pixels().to_vec();
        let index = i64::from(s.display().random_index());
        let mut check = |what: Result<(), String>, f_line: Option<String>| {
            if let (None, Err(e)) = (&out.first, what) {
                out.first = Some((tic, e));
                out.f_line = f_line;
            }
        };
        if let Some(frames) = &g.frames {
            let gf = frames.get(&tic);
            let (res, line) = match (&r.level_frame, gf) {
                (_, None) => (Err("no genuine frame".to_string()), None),
                (Some(f), Some(gf)) => {
                    let c = f.view.counters;
                    let line = format!(
                        "F,{tic},{},-,{},{},0,{},{},{},{},{},{},{},0",
                        sha256(&px),
                        f.view.fuzzpos,
                        c.mapped,
                        c.visplanes,
                        c.drawsegs,
                        c.vissprites,
                        c.openings,
                        c.solidsegs_peak,
                        c.drawseg_drops,
                        u8::from(c.vissprite_drop)
                    );
                    let q = gf.q;
                    let mut nf = px.clone();
                    for y in q[3] as usize..(q[5] as usize).min(SCREENHEIGHT) {
                        nf[y * SCREENWIDTH + q[2] as usize
                            ..y * SCREENWIDTH + (q[4] as usize).min(SCREENWIDTH)]
                            .fill(0);
                    }
                    let status = [
                        i64::from(f.palette),
                        f.palette_set.map_or(-1, i64::from),
                        i64::from(f.face),
                        i64::from(f.message_on),
                        index,
                    ];
                    let ok = gf.gamestate == 0
                        && line == gf.f_line
                        && sha256(&nf) == gf.noface
                        && [q[0], q[1]] == [run.draws[0], run.draws[2]]
                        && f.view.newly_mapped == gf.mapped
                        && !f.view.switch_parity_unknown
                        && f.view.flagged_columns.is_empty()
                        && status[..] == gf.status[..];
                    out.level_frames += usize::from(ok);
                    (
                        if ok {
                            Ok(())
                        } else {
                            Err("level frame differs".into())
                        },
                        Some(line),
                    )
                }
                (None, Some(gf)) => {
                    let state = state_of(&s);
                    let status = vec![
                        i64::from(r.palette),
                        r.palette_set.map_or(-1, i64::from),
                        index,
                    ];
                    let ok = state != 0
                        && (gf.gamestate, gf.sha256.as_str(), &gf.status)
                            == (state, sha256(&px).as_str(), &status);
                    out.other_frames += usize::from(ok);
                    (
                        if ok {
                            Ok(())
                        } else {
                            Err(format!("game state {state} frame differs"))
                        },
                        None,
                    )
                }
            };
            check(res, line);
        }
        if let Some((pause, mp)) = &g.pause {
            let b = pause.at("B", tic);
            let res = match (s.paused(), b) {
                (false, None) => Ok(()),
                (true, Some(b)) => {
                    let lv = s.level();
                    let got = [
                        state_of(&s),
                        lv.tic() as i64,
                        s.gametic() as i64,
                        i64::from(lv.rng().indices()[0]),
                        index,
                        1,
                    ];
                    let want = [
                        "gamestate",
                        "leveltime",
                        "gametic",
                        "prndindex",
                        "rndindex",
                        "drawn",
                    ]
                    .map(|k| int(b, k));
                    if got == want && b["frame_sha256"] == sha256(&px) {
                        out.paused_frames += 1;
                        Ok(())
                    } else {
                        Err(format!("B row {want:?} genuine, the port {got:?}"))
                    }
                }
                (p, b) => Err(format!("paused {p}, genuine B row {}", b.is_some())),
            };
            check(res, None);
            let calls: Vec<String> = s
                .sound()
                .music_calls()
                .iter()
                .map(|c| match c {
                    MusicCall::Pause => format!("pause,{MUSIC_HANDLE}"),
                    MusicCall::Resume => format!("resume,{MUSIC_HANDLE}"),
                })
                .collect();
            let want = mp.get(&tic).cloned().unwrap_or_default();
            out.music_calls += calls.len();
            check(
                if calls == want {
                    Ok(())
                } else {
                    Err(format!("music calls {calls:?}, genuine {want:?}"))
                },
                None,
            );
        }
        if let Some(sound) = &g.sound {
            // A paused load tic gives the sound the load's records (its tic 0 has not run); the
            // level's tic 0, run later, publishes them again at the head of its own, which the
            // session does not give twice (module documentation of doom-game's session).
            let lv = s.level();
            let records: Vec<SoundRecord> = if new_level && lv.tic() == 0 {
                owed = lv.load_sound_records().len();
                lv.load_sound_records().to_vec()
            } else if level && lv.tic() == 1 && owed > 0 {
                std::mem::take(&mut owed);
                lv.sound_records()[lv.load_sound_records().len()..].to_vec()
            } else if level {
                lv.sound_records().to_vec()
            } else {
                r.wi_starts
                    .iter()
                    .map(|x| SoundRecord::Start {
                        sfx: x.sfx(),
                        origin: SoundOrigin::None,
                        at: None,
                    })
                    .collect()
            };
            let t = &r.sound.sound;
            let first_seq = run.carry.seq;
            let mut rows = stream_rows_with(
                t,
                &records,
                s.sound().sound(),
                s.level().sound_listener().map(|l| l.actor),
                &mut run.carry,
                t.rndindex.wrapping_add(1),
                s.display().random_index(),
            );
            let origin = |o: SoundOrigin| match o {
                SoundOrigin::None => "none,-1".to_string(),
                SoundOrigin::Actor(a) => format!(
                    "{},A{}:{}",
                    if Some(a) == old { "player" } else { "mobj" },
                    a.slot(),
                    a.generation()
                ),
                SoundOrigin::Sector(x) => format!("sector,{x}"),
                SoundOrigin::Button(b) => format!("button,{b}"),
            };
            let k_rows: Vec<String> = r
                .sound
                .level_start
                .iter()
                .map(|k| {
                    format!(
                        "K,{tic},0,{},{},{},{},level_start,{}",
                        k.cnum,
                        k.entry.sfx.get(),
                        origin(k.entry.origin),
                        k.entry.handle,
                        u8::from(k.playing)
                    )
                })
                .collect();
            let last = rows.last_mut().expect("an R row");
            let mut f: Vec<String> = last.split(',').map(str::to_string).collect();
            (f[6], f[7]) = (run.draws[1].to_string(), run.draws[2].to_string());
            if !level {
                (f[2], f[3]) = ("-1".into(), "-1".into());
            }
            *last = f.join(",");
            let mut rows = tag(rows, run.ordinal);
            rows.splice(0..0, tag(k_rows, run.ordinal - usize::from(new_level)));
            let genuine = sound.get(&tic).cloned().unwrap_or_default();
            for i in 0..rows.len().max(genuine.len()) {
                let (a, b) = (
                    rows.get(i).map_or("<none>", String::as_str),
                    genuine.get(i).map_or("<none>", String::as_str),
                );
                let res = run
                    .row(a, b)
                    .map_err(|e| format!("sound row {i} ({e}):\n  genuine: {b}\n  rust:    {a}"));
                out.sound_rows += usize::from(res.is_ok());
                check(res, None);
            }
            writer.starts(t, first_seq);
            if let Some(b) = &t.buffer {
                writer.buffer(t.tic, b, s.sound().sound().mixer().mask());
            }
        }
        while s.melt_in_progress() {
            let m = s.melt_step(&mut picture).expect("a melt step");
            if let (true, Some(b)) = (m.done, &m.buffer) {
                writer.buffer(tic as u64, b, s.sound().sound().mixer().mask());
            }
        }
    }
    if let Some(want) = &g.mixer {
        drain(s.sound().sound(), &mut writer);
        out.buffers = writer.buffers();
        let got = writer.finish();
        out.mixer_rows = want.len();
        if let Some(i) = (0..want.len().max(got.len())).find(|i| want.get(*i) != got.get(*i)) {
            out.first.get_or_insert((
                -1,
                format!("mixer row {i}: {:?} genuine {:?}", got.get(i), want.get(i)),
            ));
        }
    }
    if let Some((pause, mp)) = &g.pause {
        let (b, calls) = (
            pause.of("B").len(),
            mp.values().map(Vec::len).sum::<usize>(),
        );
        if (out.paused_frames, out.music_calls) != (b, calls) && out.first.is_none() {
            out.first = Some((
                -1,
                format!(
                    "{} paused frames and {} music calls, genuine {b} and {calls}",
                    out.paused_frames, out.music_calls
                ),
            ));
        }
    }
    out
}

/// Every run equals the genuine goldens at every tic; the totals compared are printed.
#[test]
fn wire_29_frames_sound_and_mixer_equal_the_genuine_engine() {
    let mut totals = Outcome::default();
    for (name, sb, frames) in RUNS {
        let o = drive(name, sb, frames, None);
        assert_eq!(o.first, None, "{name} sb{sb} {:?}", o.f_line);
        assert_eq!(o.tics, item(name).2.len(), "{name} sb{sb}: every tic");
        eprintln!(
            "{name} sb{sb}: {} tics, {} level and {} other frames, {} paused frames, {} music calls, {} sound rows, {} mixer rows ({} buffers)",
            o.tics,
            o.level_frames,
            o.other_frames,
            o.paused_frames,
            o.music_calls,
            o.sound_rows,
            o.mixer_rows,
            o.buffers
        );
        totals.tics += o.tics;
        totals.level_frames += o.level_frames;
        totals.other_frames += o.other_frames;
        totals.paused_frames += o.paused_frames;
        totals.music_calls += o.music_calls;
        totals.sound_rows += o.sound_rows;
        totals.mixer_rows += o.mixer_rows;
        totals.buffers += o.buffers;
    }
    eprintln!("wire 29 frames: {} runs, {totals:?}", RUNS.len());
}

/// `pause_hu` (HU_Ticker stopped while paused, g_game.c:733) first differs at a level frame, the
/// message's (hu_stuff.c:512-516: the timer frozen, the message kept on screen), and the port's F
/// line there hashes to the digest of the variant's line that its `V` row names in
/// `m6b_genuine.txt` (the row's place counts the lines of the runner's raw frame stream, whose
/// D and other rows the committed golden leaves out, so the digest, not the line number, pins the
/// frame); the picture one row lower (the genuine self-test's fault) changes the first paused frame.
#[test]
fn the_pause_faults_are_caught_at_the_genuine_places() {
    let v = read("m6b_genuine.txt")
        .lines()
        .find(|l| l.starts_with("V,pause_hu,PAUSE,frames,"))
        .expect("the V row")
        .to_owned();
    let digest = v.rsplit(',').next().unwrap();
    let o = drive("PAUSE", 10, true, Some(PlantedFault::PauseHu));
    let (t, e) = o.first.clone().expect("pause_hu differs");
    assert_eq!(e, "level frame differs", "{o:?}");
    assert_eq!(
        sha256(o.f_line.as_deref().unwrap().as_bytes()),
        digest,
        "the variant's F line"
    );
    eprintln!("planted pause_hu: PAUSE caught at frame {t} with the variant's line digest");
    let o = drive("PAUSE", 10, true, Some(PlantedFault::PausePictureLower));
    let first_paused = int(&read_b_first("PAUSE"), "tic");
    assert_eq!(o.first.as_ref().map(|x| x.0), Some(first_paused), "{o:?}");
    eprintln!(
        "planted picture one row lower: caught at tic {first_paused} ({:?})",
        o.first
    );
}

fn read_b_first(name: &str) -> BTreeMap<String, String> {
    Rows::parse(&read(&format!("m6b_pause/{name}.sb10.txt"))).of("B")[0].clone()
}

/// `DDSP` 5 and `DSND` 4 inside `DSES` 4 (Rust-only safety): saved at paused points (a level, the
/// intermission, the finale, LOADP's paused load tic before its tic 0) and around them, every state
/// round-trips, restores and continues 30 tics with the uninterrupted session's frames, music calls
/// and sound; each new reader refuses the old version and the old readers the new one.
#[test]
fn paused_states_restore_and_continue() {
    use doom_game::session::state_v4::SessionStateV4;
    use doom_render::{SessionDisplayStateV4, SessionDisplayStateV5, StateDecodeError};
    use doom_sound::chain_melt::ChainSoundStateV3;
    use doom_sound::chain_pause::ChainSoundStateV4;
    use doom_sound::chain_state::ChainStateError;
    use doom_sound::state::StateError;
    let points = [
        ("PAUSE", &[72, 73, 113, 135, 200][..]),
        ("PAUSEWI", &[128, 136, 141][..]),
        ("PAUSEFIN", &[401, 1901, 1950][..]),
        ("LOADP", &[143, 145, 146][..]),
    ];
    let mut checked = 0;
    for (name, at) in points {
        let (item, parameters, cmds) = item(name);
        let data = m6a::data(&item, None);
        let settings = SessionSettings {
            map: item.map,
            skill: item.skill,
            screenblocks: 10,
        };
        let mut s =
            Session::new_finale_and_melt_wire29(data.clone(), settings, parameters).unwrap();
        let mut unseen = Frame::new();
        for (t, &c) in cmds.iter().enumerate() {
            if at.contains(&t) {
                let state = s.save_v4().expect("saveable");
                let bytes = state.encode();
                let decoded = SessionStateV4::decode(&bytes).expect("decodes");
                assert_eq!(decoded, state, "{name} {t}");
                let mut r = Session::restore_v4(data.clone(), &decoded, s.level().clone())
                    .expect("restores");
                assert_eq!(r.save_v4().unwrap().encode(), bytes, "{name} {t}");
                let mut u = s.clone();
                for &c in &cmds[t..(t + 30).min(cmds.len())] {
                    let (a, b) = (
                        r.tic_finale_and_melt(c).unwrap(),
                        u.tic_finale_and_melt(c).unwrap(),
                    );
                    assert_eq!(
                        (a.kind, a.paused, &a.sound),
                        (b.kind, b.paused, &b.sound),
                        "{name} {t}"
                    );
                    assert_eq!(r.frame().pixels(), u.frame().pixels(), "{name} {t}");
                    assert_eq!(
                        r.sound().music_calls(),
                        u.sound().music_calls(),
                        "{name} {t}"
                    );
                    while u.melt_in_progress() {
                        assert_eq!(
                            r.melt_step(&mut unseen).unwrap(),
                            u.melt_step(&mut unseen).unwrap()
                        );
                    }
                }
                checked += 1;
                if name == "PAUSE" && t == 73 {
                    let d = SessionDisplayStateV5 {
                        display: decoded.base.display.clone(),
                        pause: decoded.v4.display,
                    }
                    .encode();
                    assert_eq!(
                        SessionDisplayStateV4::decode(&d),
                        Err(StateDecodeError::Version(5))
                    );
                    let v4 = decoded.base.display.encode();
                    assert_eq!(
                        SessionDisplayStateV5::decode(&v4),
                        Err(StateDecodeError::Version(4))
                    );
                    let mut bad = d.clone();
                    bad[8] = 2;
                    assert_eq!(
                        SessionDisplayStateV5::decode(&bad),
                        Err(StateDecodeError::Invalid("paused"))
                    );
                    let mut early = SessionDisplayStateV5::decode(&d).unwrap();
                    early.pause.level_paused += 1;
                    assert!(
                        early.check().is_err(),
                        "a level tic plus its paused tics is the session's"
                    );
                    let snd = ChainSoundStateV4 {
                        state: decoded.base.sound.state.clone(),
                        pause: decoded.v4.sound,
                    };
                    let b = snd.encode();
                    assert_eq!(
                        ChainSoundStateV3::decode(&b),
                        Err(ChainStateError::State(StateError::Version(4)))
                    );
                    let mut bad = b.clone();
                    bad[67] = 2;
                    assert_eq!(
                        ChainSoundStateV4::decode(&bad),
                        Err(ChainStateError::Pause("mus_paused"))
                    );
                    let mut more = snd.clone();
                    more.pause.level_paused = 74;
                    assert_eq!(
                        ChainSoundStateV4::decode(&more.encode()),
                        Err(ChainStateError::Pause("level paused tics"))
                    );
                }
            }
            s.tic_finale_and_melt(c).expect("a tic");
            while s.melt_in_progress() {
                s.melt_step(&mut unseen).expect("a melt step");
            }
        }
    }
    assert_eq!(checked, 14);
}

/// The pause picture below screen size 10 (`tools/reference/wire29_border.py`, our own border
/// lumps): where the genuine engine drew `M_PAUSE` in the first paused frame of PAUSE at screen
/// sizes 10, 9, 5 and 3. The port's display admits sizes 10 and 11 only (its session start refuses
/// the others), and its renderer's view tables 9 to 11: at 10 and 9 the picture's place from
/// those tables (d_main.c:303-311, the patch offsets subtracted, v_video.c:219-220) equals the
/// genuine one; at 5 and 3 the tables are refused, so nothing is compared there (the genuine frame
/// digests are kept in `tests/reference/wire29_border/` for a later small-size renderer).
#[test]
fn the_pause_picture_below_screen_size_10() {
    use doom_render::d_display::session::pause::pause_origin_for;
    use doom_render::r_main::ViewSize;
    use doom_render::{ConfigError, Detail, RenderConfig};
    let text = read("wire29_border/border.txt");
    let head: Vec<&str> = text
        .lines()
        .find(|l| l.starts_with("#patch,"))
        .unwrap()
        .split(',')
        .collect();
    let (left, top): (i32, i32) = (head[7].parse().unwrap(), head[9].parse().unwrap());
    let (mut compared, mut refused) = (Vec::new(), Vec::new());
    for o in text.lines().filter(|l| l.starts_with("O,")) {
        let f: Vec<&str> = o.split(',').collect();
        let sb: u8 = f[2].parse().unwrap();
        let genuine: (i32, i32) = (f[4].parse().unwrap(), f[5].parse().unwrap());
        match ViewSize::new(RenderConfig {
            screenblocks: sb,
            detail: Detail::High,
        }) {
            Ok(v) => {
                let (x, y) = pause_origin_for(&v);
                assert_eq!((x - left, y - top), genuine, "screen size {sb}");
                compared.push(sb);
            }
            Err(e) => {
                assert_eq!(e, ConfigError::ScreenBlocks(sb));
                refused.push(sb);
            }
        }
        let (item, parameters, _) = item("PAUSE");
        let settings = SessionSettings {
            map: item.map,
            skill: item.skill,
            screenblocks: sb,
        };
        let start =
            Session::new_finale_and_melt_wire29(m6a::data(&item, None), settings, parameters);
        assert_eq!(
            start.is_ok(),
            sb >= 10,
            "screen size {sb}: the session start"
        );
    }
    assert_eq!((compared, refused), (vec![10, 9], vec![5, 3]));
}
