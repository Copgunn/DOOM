//! Wire 28 in the session (`Session::new_finale_and_melt_wire28`) against the genuine
//! linuxdoom-1.10 goldens of the M6a items SKILL0 (`sk_baby`) and SKILL1 (`sk_easy`), which the
//! part 2 gate (`m6a2_session.rs`) names as not driven: every stream that gate compares for a
//! driven item, at screenblocks 10 and 11. The goldens are the genuine engine's only
//! (`tools/reference/m6a_goldens.py`, `m6a_mixer.py`, `m6a2_goldens.py`; the state rows
//! `wire28_skill_state.txt` from `tools/reference/wire28_goldens.py` through the independent
//! DSIM44 encoder). That gate's driver (`m6a2_run`) builds the wire 27 session, which refuses
//! skills 0 and 1, and is not changed here; this file drives the wire 28 session with the same
//! comparisons, reusing its readers.
//!
//! Every tic: the G row (19 columns), the C row (the player; outside a level the finished
//! player), the I and J rows (the intermission), no F row, the frame (a level frame's whole and
//! face-zeroed digests, renderer counters, newly mapped lines, palette, face, message and index;
//! an intermission frame whole with palette and index), every sound row (actor handles matched to
//! the genuine serials by a bijection), and every melt (screens, palette, each step's picture and
//! `done`, a tic asked for during it refused with nothing changed, no sound change before the last
//! step); after the run, every mixer buffer and, at screenblocks 10, the level's whole DSIM44
//! state after every level tic (sequence digest and the sampled `D` rows).
//!
//! Self-test: SKILL1's session (no `sk_baby` rules) driven against SKILL0's goldens is caught.
//! FIN0 and FIN1 (FINRET's demo at skills 0 and 1, to the victory and the finale's stage-1 melt;
//! `tools/reference/wire28_fin.py`, the same layout under `wire28_fin/`) also compare the F row.
mod m6a;
mod m6a2_run;
#[path = "../../doom-sound/tests/common/rows.rs"]
mod rows;

use doom_game::session::demo::SessionDemo;
use doom_game::session::finale_melt::{FinaleMeltTicKind as Kind, FinaleMeltTicReport as Report};
use doom_game::session::{Session, SessionSettings, SessionWitness};
use doom_render::{Frame, GameState, SCREENHEIGHT, SCREENWIDTH};
use doom_sim::{ActorId, PlayerBodyState, PlayerSlot, Snapshot, TicCommand};
use doom_sound::logic::{SoundOrigin, SoundRecord};
use doom_test_evidence::sha256;
use m6a::{Item, Rows, int};
use m6a2_run::{GoldenFrame, GoldenMelt, read_frames, read_melts};
use rows::{Carry, GoldenWriter, drain, stream_rows_with};
use std::collections::BTreeMap;

type Res = Result<(), String>;
/// A level of the state rows: its L row and its D rows by leveltime (session tic, digest).
type StateLevel = (Vec<String>, BTreeMap<usize, (String, String)>);
/// A level of the run: load tic, map, start kind, (digest, session tic) per tic, map digest.
type RunLevel = (i64, u8, String, Vec<(String, String)>, Option<String>);

/// A row's columns as (name, value) pairs, one table per row kind.
macro_rules! cols {
    ($($n:literal => $v:expr),* $(,)?) => { vec![$(($n, $v)),*] };
}

fn read(path: &str) -> String {
    let p = m6a::root("tests/reference").join(path);
    std::fs::read_to_string(p)
        .expect(path)
        .replace("\r\n", "\n")
}

/// The genuine sound stream at `path` (as `m6a::read_sound` reads `m6a_sound`; a copy of the
/// private reader of `m6a2_run`, which stays as it is).
fn read_sound(path: &str) -> m6a::GenuineSound {
    let mut g = m6a::GenuineSound::default();
    for l in read(path).lines() {
        if l.starts_with('#') || l.starts_with("Z,") || l.starts_with("W,") {
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        let tic: i64 = f[1].parse().expect("tic");
        match f[0] {
            "LV" => {
                g.loads.insert(tic, f[3].parse().expect("map"));
            }
            "E" => g.error = Some((tic, f[2..].join(","))),
            _ => g.by_tic.entry(tic).or_default().push(l.to_owned()),
        }
    }
    g
}

/// Compares `got` with the genuine row's columns of the same names.
fn check(row: Option<&BTreeMap<String, String>>, kind: &str, got: &[(&str, i64)]) -> Res {
    let row = row.ok_or(format!("no genuine {kind} row"))?;
    match got.iter().find(|(n, v)| int(row, n) != *v) {
        Some((n, v)) => Err(format!("{kind} {n} {v} genuine {}", row[*n])),
        None => Ok(()),
    }
}

/// `wire28_skill_state.txt`'s rows of `name`: per level its L row and its D rows by leveltime
/// (session tic, state digest).
fn state_levels(name: &str) -> Vec<StateLevel> {
    let text = read(&format!("{}wire28_skill_state.txt", dir(name)));
    assert!(text.starts_with("#schema,DOOM-WIRE28-SKILL-STATE-v1\n"));
    let mut out: Vec<StateLevel> = Vec::new();
    for l in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<String> = l.split(',').map(String::from).collect();
        match (f[0].as_str(), f[1] == name) {
            ("L", true) => out.push((f, BTreeMap::new())),
            ("D", true) => drop(
                out.last_mut()
                    .unwrap()
                    .1
                    .insert(f[4].parse().unwrap(), (f[3].clone(), f[6].clone())),
            ),
            _ => {}
        }
    }
    out
}

/// The state and map digests of a DSIM44 capture (`m6a_state.txt`'s definition).
fn digests(s: &Snapshot) -> (String, String) {
    assert_eq!(s.version(), 44, "wire 28 captures are DSIM 44");
    let framed = |out: &mut Vec<u8>, p: &str, v: &[u8]| {
        out.extend((p.len() as u32).to_le_bytes());
        out.extend(p.as_bytes());
        out.extend((v.len() as u64).to_le_bytes());
        out.extend(v);
    };
    let kept = |p: &str| !p.starts_with("map.") && !p.starts_with("events.");
    let (mut body, mut map) = (b"DSIM".to_vec(), Vec::new());
    body.extend(44_u32.to_le_bytes());
    body.extend((s.fields().filter(|f| kept(f.path())).count() as u64).to_le_bytes());
    for f in s.fields().filter(|f| kept(f.path())) {
        framed(&mut body, f.path(), f.value());
    }
    for f in s.fields() {
        if f.path().starts_with("events.sounds.") {
            framed(&mut body, f.path(), f.value());
        } else if f.path().starts_with("map.") {
            framed(&mut map, f.path(), f.value());
        }
    }
    (sha256(&body), sha256(&map))
}

/// The C row's columns (as `m6a2_run`'s `player_columns`).
fn player(s: &Session) -> Vec<(&'static str, i64)> {
    let (l, outside) = (s.level(), s.gamestate() != GameState::Level);
    let one = PlayerSlot::One;
    let (p, armor) = (
        l.original_player(one).unwrap(),
        l.original_player_armor(one).unwrap(),
    );
    let w = l.original_player_weapons(one).unwrap().unwrap();
    let (pw, pick) = (
        l.original_player_powers(one).unwrap(),
        l.original_player_pickups(one).unwrap(),
    );
    let finish = l.level_finish().unwrap();
    let keep = i64::from(!outside);
    let state = match (p.state, finish.reborn) {
        (PlayerBodyState::Live, _) => 0,
        (PlayerBodyState::Dead, false) => 1,
        (PlayerBodyState::Dead, true) => 2,
    };
    let cards: i64 = pw
        .cards
        .iter()
        .enumerate()
        .map(|(k, c)| i64::from(*c) << k)
        .sum();
    let max = |v: i64| v << u32::from(w.backpack);
    let kills = l.level_statistics().unwrap().single_player_kill_count;
    let pwr = |k: usize| i64::from(pw.powers[k]) * keep;
    let mut m = cols![
        "playerstate" => state, "health" => i64::from(p.health), "armorpoints" => i64::from(armor.points),
        "armortype" => i64::from(armor.kind.wire_id()), "readyweapon" => w.ready as i64,
        "pendingweapon" => w.pending.map_or(10, |x| x as i64), "weaponowned" => i64::from(w.owned),
        "ammo0" => i64::from(w.bullets), "ammo1" => i64::from(w.shells), "ammo2" => i64::from(w.cells),
        "ammo3" => i64::from(w.rockets), "maxammo0" => max(200), "maxammo1" => max(50), "maxammo2" => max(300),
        "maxammo3" => max(50), "backpack" => i64::from(w.backpack), "cards" => cards * keep, "pw0" => pwr(0),
        "pw1" => pwr(1), "pw2" => pwr(2), "pw3" => pwr(3), "pw4" => pwr(4), "pw5" => pwr(5), "cheats" => 0,
        "killcount" => i64::from(kills), "itemcount" => i64::from(pick.item_count),
        "secretcount" => i64::from(l.original_player_secret_count().unwrap()), "refire" => i64::from(w.refire),
        "damagecount" => i64::from(p.damage_count) * keep, "bonuscount" => i64::from(pick.bonus_count) * keep,
        "didsecret" => i64::from(s.didsecret()),
    ];
    m.extend(if outside {
        let c = s.carried().unwrap();
        cols!["attackdown" => i64::from(c.latches.attackdown), "usedown" => i64::from(c.latches.usedown),
              "viewheight" => i64::from(c.viewheight), "deltaviewheight" => i64::from(c.deltaviewheight)]
    } else {
        let snap = || l.snapshot().fields().any(|f| f.path() == "original_player.0.use_down" && f.value() == [1]);
        let usedown = finish.carried.map_or_else(snap, |c| c.use_down);
        let v = s.display().level_view().unwrap();
        cols!["attackdown" => i64::from(w.attack_down), "usedown" => i64::from(usedown),
              "viewheight" => i64::from(v.viewheight), "deltaviewheight" => i64::from(v.deltaviewheight)]
    });
    m
}

/// The per-run state: the handle bijection, the genuine row counters, the levels captured.
#[derive(Default)]
struct Run {
    to_c: BTreeMap<String, String>,
    to_rust: BTreeMap<String, String>,
    carry: Carry,
    ordinal: usize,
    loads: i64,
    wipes: i64,
    /// Cumulative sound, status bar, wipe and intermission draws.
    draws: [i64; 4],
    /// Per level: load tic, map, start kind, (state digest, session tic) per level tic, map digest.
    levels: Vec<RunLevel>,
    melts: usize,
    steps: usize,
    sound_rows: usize,
}

impl Run {
    /// One sound row against the genuine one: equal but for actor handles, which must map to
    /// serials one to one (an unplaced start's listener columns are not recorded, s_sound.c).
    fn row(&mut self, rust: &str, genuine: &str) -> Res {
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

/// Tags every actor handle with the level ordinal `n`.
fn tag(rows: Vec<String>, n: usize) -> Vec<String> {
    let field = |f: &str| {
        f.strip_prefix('A')
            .map_or_else(|| f.to_string(), |h| format!("A{n}/{h}"))
    };
    rows.into_iter()
        .map(|r| r.split(',').map(field).collect::<Vec<_>>().join(","))
        .collect()
}

/// The genuine goldens of one item at one screen size.
struct Golden {
    session: Rows,
    frames: BTreeMap<i64, GoldenFrame>,
    sound: m6a::GenuineSound,
    mixer: Vec<String>,
    melts: BTreeMap<i64, GoldenMelt>,
}

/// Where the goldens of `name` are: `wire28_fin/` for FIN0 and FIN1.
fn dir(name: &str) -> &'static str {
    ["", "wire28_fin/"][usize::from(name.starts_with("FIN"))]
}

/// Drives `item`'s demo through the wire 28 session at `sb` against the goldens of `golden`:
/// Ok(tics compared) or the first difference.
fn gate(item: &Item, golden: &str, sb: u8) -> Result<usize, String> {
    let dir = dir(golden);
    let mixer = read(&format!("{dir}m6a_mixer/goldens/{golden}.golden"));
    let melts = if sb == 10 {
        format!("{dir}m6a_melt/{golden}.melt")
    } else {
        format!("{dir}m6a2_melt/{golden}.sb11.melt")
    };
    let g = Golden {
        session: Rows::parse(&read(&format!("{dir}m6a_session/{golden}.session"))),
        frames: read_frames(&format!("{dir}m6a_frames/{golden}.sb{sb}.frames")).0,
        sound: read_sound(&format!("{dir}m6a_sound/{golden}.sound")),
        mixer: mixer
            .lines()
            .filter(|l| !l.starts_with("d ") && !l.starts_with("q "))
            .map(String::from)
            .collect(),
        melts: read_melts(&melts),
    };
    let lmp = std::fs::read(m6a::root(&item.lmp)).expect("demo");
    let cmds = SessionDemo::parse_v3(&lmp)
        .expect("the wire 28 reader")
        .commands;
    let (map, skill) = (item.map, item.skill);
    let settings = SessionSettings {
        map,
        skill,
        screenblocks: sb,
    };
    let mut s =
        Session::new_finale_and_melt_wire28(m6a::data(item, None), settings).expect("starts");
    let mut run = Run {
        levels: vec![(-1, map, "new".into(), Vec::new(), None)],
        ..Run::default()
    };
    let mut writer = GoldenWriter::new(s.sound().sound().mixer().sound_set());
    let mut listener = s.level().sound_listener().map(|l| l.actor);
    for (t, &cmd) in cmds.iter().enumerate() {
        let tic = t as i64;
        let d = |what: String| format!("{} sb{sb} tic {tic}: {what}", item.name);
        let r = s
            .tic_finale_and_melt(cmd)
            .map_err(|e| d(format!("refused {e}")))?;
        let (new_level, level) = (
            matches!(r.kind, Kind::Load { .. }),
            matches!(r.kind, Kind::Level | Kind::Load { .. }),
        );
        (run.loads, run.ordinal, run.wipes) = (
            run.loads + i64::from(new_level),
            run.ordinal + usize::from(new_level),
            run.wipes + i64::from(r.wiped),
        );
        let add = [
            i64::from(r.sound.sound.draws),
            i64::from(level),
            320 * i64::from(r.wiped),
            i64::from(r.wi_draws),
        ];
        (0..4).for_each(|k| run.draws[k] += add[k]);
        let old = std::mem::replace(&mut listener, s.level().sound_listener().map(|l| l.actor));
        let state = [GameState::Level, GameState::Intermission, GameState::Finale]
            .iter()
            .position(|x| *x == s.gamestate())
            .unwrap() as i64;
        tic_rows(&s, &g, &run, tic, state).map_err(d)?;
        frame(&s, &r, &g, &run, tic, state).map_err(d)?;
        sound(&s, &r, &g, &mut run, &mut writer, tic, old, new_level).map_err(d)?;
        if let Kind::Load { map, start } = r.kind {
            run.levels.push((
                tic,
                map,
                format!("{start:?}").to_lowercase(),
                Vec::new(),
                None,
            ));
        }
        if level {
            let (st, map) = digests(&s.level().snapshot());
            let lv = run.levels.last_mut().unwrap();
            lv.4.get_or_insert(map);
            lv.3.push((st, tic.to_string()));
        }
        melt(&mut s, &r, &g, &mut run, &mut writer, tic).map_err(d)?;
    }
    if (item.end.as_str(), cmds.len(), run.melts) != ("marker", item.session_tics, g.melts.len()) {
        return Err(format!(
            "{}: {} tics and {} melts, genuine {} and {}",
            item.name,
            cmds.len(),
            run.melts,
            item.session_tics,
            g.melts.len()
        ));
    }
    let want = if sb == 10 {
        state_levels(&item.name)
    } else {
        Vec::new()
    };
    if sb == 10 && want.len() != run.levels.len() {
        return Err(format!(
            "{}: {} levels, genuine {}",
            item.name,
            run.levels.len(),
            want.len()
        ));
    }
    for (k, ((tic, map, kind, caps, map_sha), (l, points))) in
        run.levels.iter().zip(&want).enumerate()
    {
        let all: Vec<u8> = caps
            .iter()
            .flat_map(|c| (0..32).map(|i| u8::from_str_radix(&c.0[2 * i..2 * i + 2], 16).unwrap()))
            .collect();
        let head = [
            tic.to_string(),
            map.to_string(),
            kind.clone(),
            caps.len().to_string(),
            map_sha.clone().unwrap_or_default(),
            sha256(&all),
        ];
        if head[..] != l[3..9]
            || points
                .iter()
                .any(|(lt, (t, dg))| caps[lt - 1] != (dg.clone(), t.clone()))
        {
            return Err(format!(
                "{} load {k}: the state differs from the genuine DSIM44 rows",
                item.name
            ));
        }
    }
    drain(s.sound().sound(), &mut writer);
    let got = writer.finish();
    if let Some(i) = (0..g.mixer.len().max(got.len())).find(|i| g.mixer.get(*i) != got.get(*i)) {
        return Err(format!("{} sb{sb} mixer row {i}", item.name));
    }
    eprintln!(
        "{} sb{sb}: {} tics, {} sound rows, {} mixer rows, {} melts of {} steps, {} levels of state",
        item.name,
        cmds.len(),
        run.sound_rows,
        got.len(),
        run.melts,
        run.steps,
        want.len()
    );
    Ok(cmds.len())
}

/// The G, C, I, J and F rows of a tic.
fn tic_rows(s: &Session, g: &Golden, run: &Run, tic: i64, state: i64) -> Res {
    let at = |k: &str| g.session.at(k, tic);
    let (lv, d) = (s.level(), &run.draws);
    check(
        at("G"),
        "G",
        &cols![
            "gamestate" => state, "gameaction" => i64::from(s.gameaction()), "episode" => 1, "map" => i64::from(s.map()),
            "leveltime" => lv.tic() as i64, "gametic" => s.gametic() as i64, "levelstarttic" => s.levelstarttic() as i64,
            "prndindex" => i64::from(lv.rng().indices()[0]), "rndindex" => i64::from(s.display().random_index()),
            "snd_draws" => d[0], "st_draws" => d[1], "wipe_draws" => d[2], "wi_draws" => d[3], "fin_draws" => 0,
            "other_draws" => 0, "wipegamestate" => state, "secretexit" => i64::from(s.secretexit()),
            "loads" => run.loads + 1, "wipes" => run.wipes,
        ],
    )?;
    if at("C").is_some() {
        check(at("C"), "C", &player(s))?;
    }
    if let Some(v) = s.finale() {
        let got = [
            ("finalestage", v.stage.into()),
            ("finalecount", v.count.into()),
        ];
        check(at("F"), "F", &got)?;
    } else if at("F").is_some() {
        return Err("a genuine F row outside the finale".into());
    }
    if at("I").is_none() {
        return if state == 1 {
            Err("no genuine I row in the intermission".into())
        } else {
            Ok(())
        };
    }
    let (w, wi, p) = (s.wminfo(), s.intermission(), &s.wminfo().plyr);
    check(
        at("I"),
        "I",
        &cols![
            "epsd" => i64::from(w.epsd), "didsecret" => i64::from(w.didsecret), "last" => i64::from(w.last),
            "next" => i64::from(w.next), "maxkills" => i64::from(w.maxkills), "maxitems" => i64::from(w.maxitems),
            "maxsecret" => i64::from(w.maxsecret), "maxfrags" => 0, "partime" => i64::from(w.partime), "pnum" => 0,
            "in" => 1, "skills" => i64::from(p.skills), "sitems" => i64::from(p.sitems), "ssecret" => i64::from(p.ssecret),
            "stime" => i64::from(p.stime), "score" => 0,
        ],
    )?;
    check(
        at("J"),
        "J",
        &cols![
            "state" => i64::from(wi.state), "acceleratestage" => i64::from(wi.acceleratestage), "me" => 0,
            "cnt" => i64::from(wi.cnt), "bcnt" => i64::from(wi.bcnt), "firstrefresh" => i64::from(wi.firstrefresh),
            "sp_state" => i64::from(wi.sp_state), "cnt_kills" => i64::from(wi.cnt_kills), "cnt_items" => i64::from(wi.cnt_items),
            "cnt_secret" => i64::from(wi.cnt_secret), "cnt_time" => i64::from(wi.cnt_time), "cnt_par" => i64::from(wi.cnt_par),
            "cnt_pause" => i64::from(wi.cnt_pause), "snl_pointeron" => i64::from(wi.snl_pointeron), "nanims" => 10,
            "wbs_is_wminfo" => 1,
        ],
    )?;
    let ctr: Vec<String> = wi.anims.iter().map(|a| a.ctr.to_string()).collect();
    let next: Vec<String> = wi.anims.iter().map(|a| a.nexttic.to_string()).collect();
    let j = at("J").unwrap();
    if j["anim_ctr"] != ctr.join(" ") || j["anim_nexttic"] != next.join(" ") {
        return Err("animations differ".into());
    }
    Ok(())
}

/// The tic's frame against the genuine F, M, P and Q rows (or the G row outside a level).
fn frame(s: &Session, r: &Report, g: &Golden, run: &Run, tic: i64, state: i64) -> Res {
    let gf = g.frames.get(&tic).ok_or("no genuine frame")?;
    let px = s.frame().pixels();
    let index = i64::from(s.display().random_index());
    let Some(f) = &r.level_frame else {
        let status = vec![
            i64::from(r.palette),
            r.palette_set.map_or(-1, i64::from),
            index,
        ];
        let same =
            (gf.gamestate, gf.sha256.as_str(), &gf.status) == (state, sha256(px).as_str(), &status);
        return if same && state != 0 {
            Ok(())
        } else {
            Err(format!("game state {state} frame or status differs"))
        };
    };
    let (q, c) = (gf.q, f.view.counters);
    let mut nf = px.to_vec();
    for y in q[3] as usize..(q[5] as usize).min(SCREENHEIGHT) {
        nf[y * SCREENWIDTH + q[2] as usize..y * SCREENWIDTH + (q[4] as usize).min(SCREENWIDTH)]
            .fill(0);
    }
    let counters = cols![
        "fuzzpos" => i64::from(f.view.fuzzpos), "mapped" => i64::from(c.mapped), "visplanes" => i64::from(c.visplanes),
        "drawsegs" => i64::from(c.drawsegs), "vissprites" => i64::from(c.vissprites), "openings" => i64::from(c.openings),
        "solidsegs" => i64::from(c.solidsegs_peak), "drops" => i64::from(c.drawseg_drops),
        "vissprite_drop" => i64::from(c.vissprite_drop),
    ];
    let status = [
        i64::from(f.palette),
        f.palette_set.map_or(-1, i64::from),
        i64::from(f.face),
        i64::from(f.message_on),
        index,
    ];
    let ok = gf.gamestate == 0
        && sha256(px) == gf.sha256
        && sha256(&nf) == gf.noface
        && [q[0], q[1]] == [run.draws[0], run.draws[2]]
        && counters.iter().map(|(_, v)| *v).eq(gf.f.iter().copied())
        && f.view.newly_mapped == gf.mapped
        && !f.view.switch_parity_unknown
        && f.view.flagged_columns.is_empty()
        && status[..] == gf.status[..];
    if ok {
        Ok(())
    } else {
        Err("level frame differs".into())
    }
}

/// The tic's sound rows and its mixer starts (and its buffer when it did not melt).
#[allow(clippy::too_many_arguments, reason = "the run's parts, as in m6a2_run")]
fn sound(
    s: &Session,
    r: &Report,
    g: &Golden,
    run: &mut Run,
    w: &mut GoldenWriter,
    tic: i64,
    old: Option<ActorId>,
    new_level: bool,
) -> Res {
    let level = r.level_frame.is_some();
    let starts = r.wi_starts.iter().map(|x| SoundRecord::Start {
        sfx: x.sfx(),
        origin: SoundOrigin::None,
        at: None,
    });
    let records: Vec<SoundRecord> = if level {
        s.level().sound_records().to_vec()
    } else {
        starts.collect()
    };
    let (first_seq, listener, t) = (
        run.carry.seq,
        s.level().sound_listener().map(|l| l.actor),
        &r.sound.sound,
    );
    let mut rows = stream_rows_with(
        t,
        &records,
        s.sound().sound(),
        listener,
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
    let genuine = g.sound.by_tic.get(&tic).cloned().unwrap_or_default();
    for i in 0..rows.len().max(genuine.len()) {
        let (a, b) = (
            rows.get(i).map_or("<none>", String::as_str),
            genuine.get(i).map_or("<none>", String::as_str),
        );
        run.row(a, b)
            .map_err(|e| format!("sound row {i} ({e}):\n  genuine: {b}\n  rust:    {a}"))?;
        run.sound_rows += 1;
    }
    w.starts(t, first_seq);
    if let Some(b) = &t.buffer {
        w.buffer(t.tic, b, s.sound().sound().mixer().mask());
    }
    Ok(())
}

/// The melt the tic's display started, if any, against the genuine Y and V rows.
fn melt(
    s: &mut Session,
    r: &Report,
    g: &Golden,
    run: &mut Run,
    w: &mut GoldenWriter,
    tic: i64,
) -> Res {
    let Some(gm) = g.melts.get(&tic).filter(|_| r.wiped) else {
        let differs = r.wiped || s.melt_in_progress() || g.melts.contains_key(&tic);
        return if differs {
            Err("melt presence differs".into())
        } else {
            Ok(())
        };
    };
    let (a, b) = s.display().melt_screens().ok_or("a wipe without a melt")?;
    let palette = s.display().melt_palette().unwrap();
    let got = (
        sha256(a),
        sha256(b),
        i64::from(palette),
        r.palette_set.map_or(-1, i64::from),
    );
    if got != (gm.start.clone(), gm.end.clone(), gm.palette, gm.palette_set) {
        return Err("melt screens or palette differ".into());
    }
    let (gt, shown, snd) = (
        s.gametic(),
        s.display().melt_steps_shown(),
        s.sound().clone(),
    );
    let refused = matches!(s.tic_finale_and_melt(TicCommand::default()), Err(e) if e.witness == SessionWitness::MeltInProgress);
    if !refused || (s.gametic(), s.display().melt_steps_shown(), s.sound()) != (gt, shown, &snd) {
        return Err("a tic during the melt was not refused with nothing changed".into());
    }
    let (mut picture, mut steps) = (Frame::new(), Vec::new());
    loop {
        let before = s.sound().clone();
        let m = s
            .melt_step(&mut picture)
            .map_err(|e| format!("melt step refused: {e}"))?;
        steps.push((sha256(picture.pixels()), m.done));
        if usize::from(m.step) != steps.len() || m.palette != palette || m.tic != tic as u64 {
            return Err(format!("melt step report {m:?}"));
        }
        if m.done {
            if let Some(b) = &m.buffer {
                w.buffer(tic as u64, b, s.sound().sound().mixer().mask());
            }
            break;
        }
        if s.sound() != &before || m.buffer.is_some() {
            return Err(format!("melt step {} changed the sound", m.step));
        }
    }
    if steps != gm.steps {
        return Err(format!(
            "melt steps differ ({} genuine {})",
            steps.len(),
            gm.steps.len()
        ));
    }
    run.melts += 1;
    run.steps += steps.len();
    Ok(())
}

/// SKILL0 and SKILL1 through the wire 28 session at screenblocks 10 and 11 against every genuine
/// stream; the self-test drives SKILL1 (no `sk_baby` rules) against SKILL0's goldens.
#[test]
fn skills_0_and_1_against_every_genuine_stream() {
    let mut tics = 0;
    for name in ["SKILL0", "SKILL1"] {
        for sb in [10, 11] {
            tics += gate(&m6a::item(name), name, sb).unwrap_or_else(|e| panic!("{e}"));
        }
    }
    assert_eq!(tics, 4 * 364);
    let caught =
        gate(&m6a::item("SKILL1"), "SKILL0", 10).expect_err("skill 1 against skill 0's goldens");
    eprintln!("self-test: {caught}");
    assert!(caught.starts_with("SKILL1 sb10 tic "), "{caught}");
}

/// FIN0 or FIN1: FINRET's item with the demo, skill, end and tics of `wire28_fin/results.json`.
fn fin_item(name: &str) -> Item {
    let doc: serde_json::Value = serde_json::from_str(&read("wire28_fin/results.json")).unwrap();
    let r = |k: &str| doc["items"][name][k].clone();
    Item {
        name: name.into(),
        lmp: format!("tests/reference/wire28_fin/{name}.lmp"),
        skill: r("skill").as_u64().unwrap() as u8,
        end: r("end").as_str().unwrap().into(),
        session_tics: r("session_tics").as_u64().unwrap() as usize,
        ..m6a::item("FINRET")
    }
}

/// FIN0 and FIN1 through the wire 28 session at screenblocks 10 and 11 against every genuine
/// stream (the victory on the genuine tic); the self-test drives FIN1 against FIN0's goldens.
#[test]
fn victory_on_skills_0_and_1_against_every_genuine_stream() {
    for (name, sb) in [("FIN0", 10), ("FIN0", 11), ("FIN1", 10), ("FIN1", 11)] {
        let item = fin_item(name);
        assert_eq!(gate(&item, name, sb), Ok(item.session_tics));
    }
    let caught = gate(&fin_item("FIN1"), "FIN0", 10).expect_err("FIN1 against FIN0's goldens");
    eprintln!("self-test: {caught}");
    assert!(caught.starts_with("FIN1 sb10 tic "), "{caught}");
}
