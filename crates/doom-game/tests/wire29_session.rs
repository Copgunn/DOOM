//! Wire 29 in the session (M6b step 6): `Configuration::FinaleAndMeltWire29`
//! (`Session::new_finale_and_melt_wire29`) against the genuine linuxdoom-1.10 session streams of
//! the 19 M6b items (`tests/reference/m6b_session/`, frozen, and `tests/reference/wire29_session/`,
//! the other items' streams pinned by `m6b_genuine.txt`'s digests, from
//! `tools/reference/wire29_pause.py`) and of the three new items DEADSPEC, LOADP and TURBOP; the level
//! captures against the DSIM45 goldens of the independent encoder (`wire29_state.txt`,
//! `wire29_session/<item>_state.txt`). No Rust output feeds an expectation.
//!
//! Every tic: the G row (game state, `gameaction`, map, `leveltime`, `gametic`, `levelstarttic`,
//! both random indices, the cumulative draws, `secretexit`, `paused`, loads and wipes), the C row
//! (the player; outside a level the finished one), the I and J rows (the intermission) and the F
//! row (the finale). After every tic in the first level, its DSIM45 capture's digest. Frames,
//! sound and mixer are M6b step 7 (the `M_PAUSE` picture, `DDSP` 5, `DSND` 4); TURBOP's HUD
//! message state is compared here from its frame stream's P and X rows.
//!
//! The paused schedule is the session's own (`G_Ticker`'s toggle, `Session::paused`); it is
//! checked against the genuine G rows' `paused` column every tic. A skill byte above 4 (CLAMP5)
//! is played as 4 (g_game.c:1379-1380): the clamp is the demo reader's (M6b step 8).
//!
//! Planted faults (the session's `PlantedFault`, each making the session behave like an altered
//! genuine engine of `tools/reference/m6b_goldens.py`): `pause_mask` and `pause_run` first differ
//! at their `V` rows' DSIM45 captures with the variants' digests; `pause_st` at the session-stream
//! line its `V` row names (the G row's `rndindex`); `pause_keep` at the session tic of the
//! second level's first dump (its `V` row's capture 127); `pause_wi` at the intermission tic the
//! variant's fire press is lost, before that capture (its variant never loads the second level,
//! `V` row digest `-`). `pause_hu` differs in frames only (step 7).
mod m6a;

use doom_game::session::finale_melt::{Configuration, FinaleMeltTicKind as Kind};
use doom_game::session::state::DsesError;
use doom_game::session::state_v3::SessionStateV3;
use doom_game::session::state_v4::SessionStateV4;
use doom_game::session::{PlantedFault, Session, SessionSettings, SessionWitness, StartError};
use doom_render::{Frame, GameState};
use doom_sim::{GameParameters, PlayerBodyState, PlayerSlot, Snapshot, StateField, TicCommand};
use doom_test_evidence::sha256;
use m6a::{Rows, int};
use std::collections::BTreeMap;

const SHARED: [&str; 5] = [
    "tests/reference/m5b_gfx.wad",
    "tests/reference/m5c_sfx.wad",
    "tests/reference/m6a_wads/m6a_gfx.wad",
    "tests/reference/m6a_wads/m6a_music.wad",
    "tests/reference/m6b_wads/m6b_gfx.wad",
];
/// The 19 M6b items (`tools/reference/m6b_wad.py` ITEMS).
const ITEMS: [&str; 19] = [
    "AMMO4", "BITS4", "BITS3", "CLAMP5", "REACT4", "REACT3", "REACT3F", "FAST4", "FAST3", "FAST3F",
    "RESP4", "RESP3", "RESP3R", "BLOCK4", "GIB4", "PAUSE", "PAUSE0", "PAUSEWI", "PAUSEFIN",
];

fn read(path: &str) -> String {
    std::fs::read_to_string(m6a::root(&format!("tests/reference/{path}")))
        .expect(path)
        .replace("\r\n", "\n")
}

/// Where an item's files are: the frozen M6b goldens, or `wire29_session/`.
fn new_item(name: &str) -> bool {
    matches!(name, "DEADSPEC" | "LOADP" | "TURBOP")
}

fn lmp_path(name: &str) -> String {
    if new_item(name) {
        format!("tests/reference/wire29_session/{name}.lmp")
    } else {
        format!("tests/reference/m6b_scripts/{name}.lmp")
    }
}

fn stream(name: &str) -> Rows {
    let frozen = m6a::root(&format!("tests/reference/m6b_session/{name}.session"));
    Rows::parse(&if frozen.exists() {
        read(&format!("m6b_session/{name}.session"))
    } else {
        read(&format!("wire29_session/{name}.session"))
    })
}

/// The item as `m6a::data` takes it (WADs, demo, map and skill from the demo's header).
fn item(name: &str) -> (m6a::Item, GameParameters, Vec<TicCommand>) {
    let episode = matches!(name, "PAUSEWI" | "PAUSEFIN" | "DEADSPEC" | "LOADP");
    let first = if episode {
        "tests/reference/m6a_wads/episode.wad"
    } else {
        "tests/reference/m6b_wads/m6b.wad"
    };
    let lmp = std::fs::read(m6a::root(&lmp_path(name))).expect("demo");
    let body = &lmp[13..lmp.len() - 1];
    let commands = body
        .chunks_exact(4)
        .map(|c| TicCommand {
            forward: c[0] as i8,
            side: c[1] as i8,
            angle_turn: (u16::from(c[2]) << 8) as i16,
            buttons: c[3],
        })
        .collect::<Vec<_>>();
    let item = m6a::Item {
        name: name.into(),
        mode: "registered".into(),
        wads: std::iter::once(first)
            .chain(SHARED)
            .map(String::from)
            .collect(),
        lmp: lmp_path(name),
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

/// The DSIM45 state goldens of an item's first level: capture digests and `V` rows
/// (variant, first differing capture, the variant's digest).
fn state_golden(name: &str) -> (Vec<String>, Vec<(String, usize, String)>) {
    let text = if new_item(name) {
        read(&format!("wire29_session/{}_state.txt", name.to_lowercase()))
    } else {
        read("wire29_state.txt")
    };
    let (mut caps, mut variants) = (Vec::new(), Vec::new());
    for l in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = l.split(',').collect();
        match f[0] {
            "C" if f[1] == name => caps.push(f[3].to_owned()),
            "V" if f[2] == name => {
                if let Ok(k) = f[4].parse() {
                    variants.push((f[1].to_owned(), k, f[5].to_owned()));
                }
            }
            _ => {}
        }
    }
    (caps, variants)
}

/// `m6a_state.txt`'s state digest over DSIM45 (as doom-sim's `nightmare_genuine_tests`).
fn digest(s: &Snapshot) -> String {
    assert_eq!(s.version(), 45, "a wire 29 capture");
    let framed = |out: &mut Vec<u8>, p: &str, v: &[u8]| {
        out.extend((p.len() as u32).to_le_bytes());
        out.extend(p.as_bytes());
        out.extend((v.len() as u64).to_le_bytes());
        out.extend(v);
    };
    let kept = |f: &&StateField| !f.path().starts_with("map.") && !f.path().starts_with("events.");
    let mut body = b"DSIM".to_vec();
    body.extend(45_u32.to_le_bytes());
    body.extend((s.fields().filter(kept).count() as u64).to_le_bytes());
    for f in s.fields().filter(kept) {
        framed(&mut body, f.path(), f.value());
    }
    for f in s
        .fields()
        .filter(|f| f.path().starts_with("events.sounds."))
    {
        framed(&mut body, f.path(), f.value());
    }
    sha256(&body)
}

/// Compares `got` with the genuine row's columns of the same names.
fn check(
    row: Option<&BTreeMap<String, String>>,
    kind: &str,
    got: &[(&str, i64)],
) -> Result<(), String> {
    let row = row.ok_or(format!("no genuine {kind} row"))?;
    match got.iter().find(|(n, v)| int(row, n) != *v) {
        Some((n, v)) => Err(format!("{kind} {n} {v} genuine {}", row[*n])),
        None => Ok(()),
    }
}

/// The C row's columns (as `wire28_genuine.rs`'s `player`).
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
    let pwr = |k: usize| i64::from(pw.powers[k]) * keep;
    let mut m = vec![
        ("playerstate", state),
        ("health", i64::from(p.health)),
        ("armorpoints", i64::from(armor.points)),
        ("armortype", i64::from(armor.kind.wire_id())),
        ("readyweapon", w.ready as i64),
        ("pendingweapon", w.pending.map_or(10, |x| x as i64)),
        ("weaponowned", i64::from(w.owned)),
        ("ammo0", i64::from(w.bullets)),
        ("ammo1", i64::from(w.shells)),
        ("ammo2", i64::from(w.cells)),
        ("ammo3", i64::from(w.rockets)),
        ("maxammo0", max(200)),
        ("maxammo1", max(50)),
        ("maxammo2", max(300)),
        ("maxammo3", max(50)),
        ("backpack", i64::from(w.backpack)),
        ("cards", cards * keep),
        ("pw0", pwr(0)),
        ("pw1", pwr(1)),
        ("pw2", pwr(2)),
        ("pw3", pwr(3)),
        ("pw4", pwr(4)),
        ("pw5", pwr(5)),
        ("cheats", 0),
        (
            "killcount",
            i64::from(l.level_statistics().unwrap().single_player_kill_count),
        ),
        ("itemcount", i64::from(pick.item_count)),
        (
            "secretcount",
            i64::from(l.original_player_secret_count().unwrap()),
        ),
        ("refire", i64::from(w.refire)),
        ("damagecount", i64::from(p.damage_count) * keep),
        ("bonuscount", i64::from(pick.bonus_count) * keep),
        ("didsecret", i64::from(s.didsecret())),
    ];
    if outside {
        let c = s.carried().unwrap();
        m.extend([
            ("attackdown", i64::from(c.latches.attackdown)),
            ("usedown", i64::from(c.latches.usedown)),
            ("viewheight", i64::from(c.viewheight)),
            ("deltaviewheight", i64::from(c.deltaviewheight)),
        ]);
    } else {
        let snap = || {
            l.snapshot()
                .fields()
                .any(|f| f.path() == "original_player.0.use_down" && f.value() == [1])
        };
        let v = s.display().level_view().unwrap();
        m.extend([
            ("attackdown", i64::from(w.attack_down)),
            (
                "usedown",
                i64::from(finish.carried.map_or_else(snap, |c| c.use_down)),
            ),
            ("viewheight", i64::from(v.viewheight)),
            ("deltaviewheight", i64::from(v.deltaviewheight)),
        ]);
    }
    m
}

/// The G, C, I, J and F rows of tic `tic` (`draws`: cumulative sound, status bar, wipe and
/// intermission draws; `counts`: loads and wipes).
fn tic_rows(
    s: &Session,
    g: &Rows,
    tic: i64,
    draws: &[i64; 4],
    counts: [i64; 2],
) -> Result<(), String> {
    let at = |k: &str| g.at(k, tic);
    let state = [GameState::Level, GameState::Intermission, GameState::Finale]
        .iter()
        .position(|x| *x == s.gamestate())
        .unwrap() as i64;
    let lv = s.level();
    check(
        at("G"),
        "G",
        &[
            ("gamestate", state),
            ("gameaction", i64::from(s.gameaction())),
            ("episode", 1),
            ("map", i64::from(s.map())),
            ("leveltime", lv.tic() as i64),
            ("gametic", s.gametic() as i64),
            ("levelstarttic", s.levelstarttic() as i64),
            ("prndindex", i64::from(lv.rng().indices()[0])),
            ("rndindex", i64::from(s.display().random_index())),
            ("snd_draws", draws[0]),
            ("st_draws", draws[1]),
            ("wipe_draws", draws[2]),
            ("wi_draws", draws[3]),
            ("fin_draws", 0),
            ("other_draws", 0),
            ("wipegamestate", state),
            ("secretexit", i64::from(s.secretexit())),
            ("paused", i64::from(s.paused())),
            ("loads", counts[0]),
            ("wipes", counts[1]),
        ],
    )?;
    if at("C").is_some() {
        check(at("C"), "C", &player(s))?;
    }
    match (s.finale(), at("F")) {
        (Some(v), f) => check(
            f,
            "F",
            &[
                ("finalestage", v.stage.into()),
                ("finalecount", v.count.into()),
            ],
        )?,
        (None, Some(_)) => return Err("a genuine F row outside the finale".into()),
        (None, None) => {}
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
        &[
            ("last", i64::from(w.last)),
            ("next", i64::from(w.next)),
            ("maxkills", i64::from(w.maxkills)),
            ("maxitems", i64::from(w.maxitems)),
            ("maxsecret", i64::from(w.maxsecret)),
            ("partime", i64::from(w.partime)),
            ("didsecret", i64::from(w.didsecret)),
            ("skills", i64::from(p.skills)),
            ("sitems", i64::from(p.sitems)),
            ("ssecret", i64::from(p.ssecret)),
            ("stime", i64::from(p.stime)),
        ],
    )?;
    check(
        at("J"),
        "J",
        &[
            ("state", i64::from(wi.state)),
            ("acceleratestage", i64::from(wi.acceleratestage)),
            ("cnt", i64::from(wi.cnt)),
            ("bcnt", i64::from(wi.bcnt)),
            ("sp_state", i64::from(wi.sp_state)),
            ("cnt_kills", i64::from(wi.cnt_kills)),
            ("cnt_items", i64::from(wi.cnt_items)),
            ("cnt_secret", i64::from(wi.cnt_secret)),
            ("cnt_time", i64::from(wi.cnt_time)),
            ("cnt_par", i64::from(wi.cnt_par)),
            ("cnt_pause", i64::from(wi.cnt_pause)),
        ],
    )
}

/// One drive of an item: the first tic whose rows differ (and how), the first differing
/// first-level capture (and the port's digest there), the tics and captures run, the HUD's
/// `message_on` per level tic, the highest intermission kill count and the refusal.
#[derive(Debug, Default)]
struct Drive {
    row: Option<(i64, String)>,
    capture: Option<(usize, String)>,
    tics: usize,
    captures: usize,
    paused_tics: usize,
    message_on: BTreeMap<i64, bool>,
    kills: i32,
    refusal: Option<String>,
}

fn drive(name: &str, fault: Option<PlantedFault>) -> Drive {
    let (item, parameters, cmds) = item(name);
    let g = stream(name);
    let (want, _) = state_golden(name);
    let settings = SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: 10,
    };
    let mut s = Session::new_finale_and_melt_wire29(m6a::data(&item, None), settings, parameters)
        .expect("starts");
    if let Some(f) = fault {
        s.plant_fault(f);
    }
    let mut d = Drive::default();
    let (mut draws, mut counts, mut first_level) = ([0i64; 4], [1i64, 0], true);
    let mut caps = vec![digest(&s.level().snapshot())];
    let mut unseen = Frame::new();
    for (t, &cmd) in cmds.iter().enumerate() {
        let tic = t as i64;
        let r = match s.tic_finale_and_melt(cmd) {
            Ok(r) => r,
            Err(e) => {
                d.refusal = Some(format!("{e}"));
                break;
            }
        };
        let level = matches!(r.kind, Kind::Level | Kind::Load { .. });
        first_level &= matches!(r.kind, Kind::Level);
        counts[0] += i64::from(matches!(r.kind, Kind::Load { .. }));
        counts[1] += i64::from(r.wiped);
        let add = [
            i64::from(r.sound.sound.draws),
            i64::from(level),
            320 * i64::from(r.wiped),
            i64::from(r.wi_draws),
        ];
        (0..4).for_each(|k| draws[k] += add[k]);
        d.paused_tics += usize::from(r.paused);
        if let Some(f) = &r.level_frame {
            d.message_on.insert(tic, f.message_on);
        }
        d.kills = d.kills.max(s.intermission().cnt_kills);
        if let (None, Err(e)) = (&d.row, tic_rows(&s, &g, tic, &draws, counts)) {
            d.row = Some((tic, e));
        }
        if first_level && caps.len() < want.len() {
            caps.push(digest(&s.level().snapshot()));
        }
        while s.melt_in_progress() {
            s.melt_step(&mut unseen).expect("a melt step");
        }
        d.tics += 1;
    }
    d.captures = caps.len();
    d.capture = caps
        .iter()
        .zip(&want)
        .position(|(a, b)| a != b)
        .map(|k| (k, caps[k].clone()));
    d
}

/// Every item through the wire 29 session equals the genuine session stream at every tic and
/// the DSIM45 goldens at every capture of its first level; the pause items' paused tics are
/// counted, and the intermission's kill counter runs past 100 % on RESP4 (200) and RESP3R (150)
/// (N10; wi_stuff.c:1338).
#[test]
fn wire_29_session_equals_every_genuine_session_stream() {
    let (mut tics, mut captures) = (0, 0);
    for name in ITEMS {
        let d = drive(name, None);
        let (want, _) = state_golden(name);
        assert_eq!((&d.row, &d.refusal), (&None, &None), "{name}");
        assert_eq!(d.tics, item(name).2.len(), "{name}: every tic");
        assert_eq!(
            (d.capture.clone(), d.captures),
            (None, want.len()),
            "{name}: captures"
        );
        let paused = stream(name)
            .of("G")
            .iter()
            .filter(|r| int(r, "paused") == 1)
            .count();
        assert_eq!(d.paused_tics, paused, "{name}: paused tics");
        let kills = match name {
            "RESP4" => 200,
            "RESP3R" => 150,
            "RESP3" => 100,
            _ => d.kills,
        };
        assert_eq!(d.kills, kills, "{name}: the intermission's kill count");
        eprintln!(
            "{name}: {} tics, {} paused, {} captures, kills {}",
            d.tics, d.paused_tics, d.captures, d.kills
        );
        (tics, captures) = (tics + d.tics, captures + d.captures);
    }
    eprintln!(
        "wire 29 session: {} items, {tics} tics, {captures} captures",
        ITEMS.len()
    );
}

/// The session's planted faults are caught at their variants' places (module documentation).
#[test]
fn the_sessions_planted_faults_are_caught_at_the_variants_places() {
    let at_capture = |name: &str, fault, variant: &str| {
        let (_, variants) = state_golden(name);
        let (_, k, dg) = variants
            .iter()
            .find(|v| v.0 == variant)
            .expect(variant)
            .clone();
        let d = drive(name, Some(fault));
        assert_eq!(d.capture, Some((k, dg)), "{variant} on {name}");
        eprintln!("planted {variant}: {name} caught at capture {k}");
    };
    at_capture("PAUSE", PlantedFault::PauseMask, "pause_mask");
    at_capture("PAUSE", PlantedFault::PauseRun, "pause_run");
    at_capture("PAUSE0", PlantedFault::PauseRun, "pause_run");
    // pause_st: m6b_genuine.txt's place is line 154 of the session stream without its #config
    // lines; the row there names the tic.
    let body: Vec<String> = read("m6b_session/PAUSE.session")
        .lines()
        .filter(|l| !l.starts_with("#config"))
        .map(String::from)
        .collect();
    let line = &body[154];
    let tic: i64 = line.split(',').nth(1).unwrap().parse().unwrap();
    let d = drive("PAUSE", Some(PlantedFault::PauseSt));
    let (t, e) = d.row.expect("pause_st differs");
    assert_eq!(t, tic, "pause_st: {e} (genuine line {line})");
    assert!(e.contains("rndindex") || e.contains("st_draws"), "{e}");
    eprintln!("planted pause_st: PAUSE caught at tic {t} ({e})");
    // pause_keep and pause_wi: PAUSEWI's capture 127 is the second level's first dump, after the
    // load tic (its L row).
    let load = stream("PAUSEWI")
        .of("L")
        .iter()
        .map(|r| int(r, "tic"))
        .max()
        .unwrap();
    let d = drive("PAUSEWI", Some(PlantedFault::PauseKeep));
    let (t, e) = d
        .row
        .clone()
        .unwrap_or_else(|| panic!("pause_keep differs: {d:?}"));
    assert_eq!(t, load, "pause_keep: {e}");
    eprintln!("planted pause_keep: PAUSEWI caught at tic {t} ({e})");
    let d = drive("PAUSEWI", Some(PlantedFault::PauseWi));
    let (t, e) = d.row.expect("pause_wi differs");
    assert!(t < load, "pause_wi at {t}: {e}");
    eprintln!("planted pause_wi: PAUSEWI caught at tic {t}, before the load at {load} ({e})");
}

/// Admission and `DSES` version 4 (Rust-only safety): skill 4 starts under wire 29 alone; a
/// state round-trips, restores and continues as the uninterrupted session; version 3 refuses
/// version 4's bytes and version 4 refuses version 3's; a session that has shown a paused frame
/// is not saved (temporary, M6b step 7).
#[test]
fn dses_version_4_and_the_wire_29_admission() {
    let (resp4, parameters, cmds) = item("RESP4");
    let data = m6a::data(&resp4, None);
    let settings = SessionSettings {
        map: resp4.map,
        skill: 4,
        screenblocks: 10,
    };
    assert!(matches!(
        Session::new_finale_and_melt_wire28(data.clone(), settings),
        Err(StartError::Domain)
    ));
    let mut s =
        Session::new_finale_and_melt_wire29(data.clone(), settings, parameters).expect("skill 4");
    assert_eq!(s.configuration(), Configuration::FinaleAndMeltWire29);
    let mut points = 0;
    for (t, &cmd) in cmds.iter().enumerate() {
        if t % 300 == 150 {
            let state = s.save_v4().expect("saveable");
            let bytes = state.encode();
            assert!(matches!(
                SessionStateV3::decode(&bytes),
                Err(DsesError::Version(4))
            ));
            let decoded = SessionStateV4::decode(&bytes).expect("decodes");
            assert_eq!(decoded, state);
            let mut r =
                Session::restore_v4(data.clone(), &decoded, s.level().clone()).expect("restores");
            assert_eq!(r.save_v4().expect("saveable").encode(), bytes);
            let mut u = s.clone();
            let mut unseen = Frame::new();
            for &c in &cmds[t..(t + 40).min(cmds.len())] {
                let (a, b) = (
                    r.tic_finale_and_melt(c).map(|x| x.kind),
                    u.tic_finale_and_melt(c).map(|x| x.kind),
                );
                assert_eq!(a, b, "tic {t}");
                while u.melt_in_progress() {
                    let (x, y) = (
                        r.melt_step(&mut unseen).unwrap(),
                        u.melt_step(&mut unseen).unwrap(),
                    );
                    assert_eq!(x, y);
                }
            }
            assert_eq!(r.save_v4().unwrap().encode(), u.save_v4().unwrap().encode());
            points += 1;
        }
        s.tic_finale_and_melt(cmd).expect("a tic");
        while s.melt_in_progress() {
            s.melt_step(&mut Frame::new()).expect("a melt step");
        }
    }
    assert_eq!(points, 5);
    let (pause, parameters, cmds) = item("PAUSE");
    let data = m6a::data(&pause, None);
    let settings = SessionSettings {
        map: pause.map,
        skill: pause.skill,
        screenblocks: 10,
    };
    let mut s = Session::new_finale_and_melt_wire29(data, settings, parameters).expect("starts");
    let v4 = s.save_v4().expect("saveable before the pause").encode();
    assert!(matches!(
        doom_game::session::state_v2::SessionStateV2::decode(&v4),
        Err(DsesError::Version(4))
    ));
    for &c in &cmds[..73] {
        s.tic_finale_and_melt(c).expect("a tic");
        while s.melt_in_progress() {
            s.melt_step(&mut Frame::new()).expect("a melt step");
        }
    }
    assert!(s.paused());
    assert!(matches!(s.save_v4(), Err(DsesError::PausedNotYet)));
    assert!(matches!(
        s.save_v3(),
        Err(DsesError::Configuration(Configuration::FinaleAndMeltWire29))
    ));
    // The save command stays refused, with nothing changed.
    let save = TicCommand {
        buttons: 0x82,
        ..TicCommand::default()
    };
    let r = s.tic_finale_and_melt(save).map(|_| ()).unwrap_err();
    assert_eq!(r.witness, SessionWitness::Special);
    assert!(s.paused());
}
