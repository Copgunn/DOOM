//! Wire 29 through the whole session on one demo, for `tools/local/wire29_real.py` (local only: it
//! reads the IWAD; M6b phase 2 step 8): the real maps' pause and skill 4 kinds and the owner's
//! doom-play recordings (ruling N8).
//!
//! `wire29_session_real --wad <iwad> [--wad <pwad> ..] --demo <lmp> --mode retail|registered` reads
//! the demo with the wire 29 reader (`SessionDemo::parse_v4`) and plays it through
//! `Session::new_finale_and_melt_wire29` at screenblocks 10, as doom-play does, every melt step
//! between tics. After each session tic it prints `G,tic,gamestate,map,leveltime,gametic,
//! levelstarttic,prndindex,rndindex,paused` (the genuine session stream's G columns of those names)
//! and, after a tic that ends in a level (paused tics and paused load tics included),
//! `C,tic,digest,shift`: `tests/reference/m6a_state.txt`'s state digest over DSIM45 and the same
//! digest without `level.gametic_lead`, `level.start.gametic` (a later load's session tic) and
//! `player.0.buttons` (what inserted pause commands or a special byte in place of buttons 0 change
//! on a level tic, p_user.c:280-281, 227), nor, in a demo with a `forward` above 50, the pickup
//! message, which the turbo test sets by `gametic` (g_game.c:672-679).
//! Ends with `E,marker,tics` or `E,refused,tic,witness`.
//!
//! `--record <lmp> --skill N --tics T [--respawn] [--fast]` (with the `--wad`s and `--mode`) instead
//! makes a recording headlessly through doom-play's app (`GameData::load_wire29_session`, the app's
//! recorder and Pause key): the up arrow held, the right arrow every other second, fire and use
//! pressed now and then, the Pause key at tics 200 and 260 and at 700 and 701, for the
//! self-test of the hand-test check.

use doom_game::app::{App, GameData, Host, Mode, SessionConfig, Timestamp};
use doom_game::input::keys::{KEY_PAUSE, KEY_RCTRL, KEY_RIGHTARROW, KEY_UPARROW};
use doom_game::session::demo::SessionDemo;
use doom_game::session::finale_melt::FinaleMeltTicKind as Kind;
use doom_game::session::{ModeSetting, Session, SessionData, SessionMode, SessionSettings};
use doom_render::{Frame, GameState};
use doom_sim::{GameParameters, Snapshot};
use doom_test_evidence::sha256;
use std::{error::Error, sync::Arc};

/// The state digest over DSIM45 of the fields `keep` passes (`nightmare_genuine_tests.rs`).
fn digest(s: &Snapshot, keep: impl Fn(&str) -> bool) -> String {
    let framed = |out: &mut Vec<u8>, p: &str, v: &[u8]| {
        out.extend((p.len() as u32).to_le_bytes());
        out.extend(p.as_bytes());
        out.extend((v.len() as u64).to_le_bytes());
        out.extend(v);
    };
    let kept: Vec<_> = (s.fields())
        .filter(|f| !f.path().starts_with("map.") && !f.path().starts_with("events."))
        .filter(|f| keep(f.path()))
        .collect();
    let mut body = b"DSIM".to_vec();
    body.extend(45_u32.to_le_bytes());
    body.extend((kept.len() as u64).to_le_bytes());
    kept.iter()
        .for_each(|f| framed(&mut body, f.path(), f.value()));
    for f in s
        .fields()
        .filter(|f| f.path().starts_with("events.sounds."))
    {
        framed(&mut body, f.path(), f.value());
    }
    sha256(&body)
}

/// Keeps the last demo the app saved.
struct Keep(Vec<u8>);
impl Host for Keep {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, _: &str) -> bool {
        false
    }
    fn save(&mut self, _: &str, bytes: &[u8]) -> Result<(), String> {
        self.0 = bytes.to_vec();
        Ok(())
    }
}

/// The scripted recording of `--record` (module documentation).
fn record(data: GameData, tics: u64) -> Vec<u8> {
    let mut host = Keep(Vec::new());
    let mut app = App::new(data, SessionConfig::default(), &mut host);
    app.key(KEY_UPARROW, true, &mut host);
    let mut k = 0;
    while app.tic() < tics && app.mode() == &Mode::Playing {
        let t = app.tic();
        app.key(KEY_RIGHTARROW, t % 70 < 35, &mut host);
        app.key(KEY_RCTRL, t % 35 < 5, &mut host);
        app.key(b' ', t % 100 == 50, &mut host);
        if [200, 260, 700, 701].contains(&t) {
            app.key(KEY_PAUSE, true, &mut host);
            app.key(KEY_PAUSE, false, &mut host);
        }
        while app.tic() == t && app.mode() == &Mode::Playing {
            k += 1;
            app.update(k * 28_572, &mut host); // one tic per update below 50,000 tics
        }
    }
    app.quit(&mut host);
    host.0
}

fn main() -> Result<(), Box<dyn Error>> {
    let (mut wads, mut demo, mut mode) = (Vec::new(), None, SessionMode::Retail);
    let (mut out, mut skill, mut tics, mut p) = (None, 2, 0, GameParameters::default());
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--wad" => wads.push(std::fs::read(value()?)?),
            "--demo" => demo = Some(std::fs::read(value()?)?),
            "--mode" if value()? == "registered" => mode = SessionMode::Registered,
            "--mode" => {}
            "--record" => out = Some(value()?),
            "--skill" => skill = value()?.parse()?,
            "--tics" => tics = value()?.parse()?,
            "--respawn" => p.respawn = true,
            "--fast" => p.fast = true,
            _ => return Err(format!("unknown option {a}").into()),
        }
    }
    let refs: Vec<&[u8]> = wads.iter().map(Vec::as_slice).collect();
    if let Some(out) = out {
        let data = GameData::load_wire29_session(&refs, ModeSetting::Force(mode), 1, skill, p)?;
        std::fs::write(out, record(data, tics))?;
        return Ok(());
    }
    let Some(lmp) = demo else {
        println!("usage: wire29_session_real --wad <iwad> [--wad <pwad>] --demo <lmp> --mode M");
        return Ok(());
    };
    let data = SessionData::load(&refs, ModeSetting::Force(mode)).map_err(|e| format!("{e:?}"))?;
    let (d, parameters) = SessionDemo::parse_v4(&lmp).map_err(|e| format!("{e:?}"))?;
    let settings = SessionSettings {
        map: d.map,
        skill: d.skill,
        screenblocks: 10,
    };
    let mut s = Session::new_finale_and_melt_wire29(Arc::new(data), settings, parameters)
        .map_err(|e| format!("{e:?}"))?;
    let mut unseen = Frame::new();
    let turbo = d.commands.iter().any(|c| c.forward > 50); // TURBOTHRESHOLD 0x32
    for (tic, &cmd) in d.commands.iter().enumerate() {
        let r = match s.tic_finale_and_melt(cmd) {
            Ok(r) => r,
            Err(e) => {
                println!("E,refused,{tic},{:?}", e.witness);
                return Ok(());
            }
        };
        let state = [GameState::Level, GameState::Intermission, GameState::Finale]
            .iter()
            .position(|x| *x == s.gamestate())
            .unwrap_or(9);
        let l = s.level();
        println!(
            "G,{tic},{state},{},{},{},{},{},{},{}",
            s.map(),
            l.tic(),
            s.gametic(),
            s.levelstarttic(),
            l.rng().indices()[0],
            s.display().random_index(),
            u8::from(s.paused())
        );
        if matches!(r.kind, Kind::Level | Kind::Load { .. }) {
            let snap = l.snapshot();
            let moved = [
                "level.gametic_lead",
                "level.start.gametic",
                "player.0.buttons",
            ];
            let message = "original_player.0.pickups.message";
            let shift = |p: &str| !moved.contains(&p) && !(turbo && p == message);
            println!(
                "C,{tic},{},{}",
                digest(&snap, |_| true),
                digest(&snap, shift)
            );
        }
        while s.melt_in_progress() {
            s.melt_step(&mut unseen).map_err(|e| format!("{e:?}"))?;
        }
    }
    println!("E,marker,{}", d.commands.len());
    Ok(())
}
