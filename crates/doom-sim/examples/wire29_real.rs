//! Wire 29 at the simulation level on one demo of a real map, for
//! `tools/local/wire29_real.py` (local only: it reads the IWAD; M6b phase 2 step 5).
//!
//! `wire29_real --wad <iwad> --demo <lmp> --schedule <file> [--twin28]` plays the
//! demo under wire 29 at the header's skill (byte 1, above 4 played as 4,
//! g_game.c:1379-1380), `respawnparm` (byte 5) and `fastparm` (byte 6), from a new
//! game on the header's map (byte 3). Each session tic either runs a level tic or,
//! after an exit, is an intermission tic. The session's choices are taken as the
//! session makes them (`crates/doom-game/src/session/mod.rs`): a reborn first, then
//! an exit; a reborn reloads the map with the `P_Random` index; an exit carries the
//! player (`level_finish`) with the latches the last intermission tic left
//! (`WI_checkForAccelerate`, wi_stuff.c:1470-1498) into the next map (g_game.c:1085-
//! 1101: the secret exit to 9, 9 to 4), a dead player as a reborn. The one input
//! from the genuine engine is the schedule: one line `T,map` per genuine load after
//! the first, whose `T` names the session tic at which an intermission ends (the
//! intermission belongs to the session, not to the simulation); an exit on map 8 is
//! the victory, where the comparison ends. Every load starts at `gametic` = its
//! session tic.
//!
//! Prints `L,level,tic,map,kind` per level, `C,level,leveltime,digest` per capture
//! after a level tic (`tests/reference/m6a_state.txt`'s state digest over DSIM45,
//! every capture also read by `Diagnostic::decode_episode1_v7`) and one end line:
//! `E,marker,tic`, `E,victory,tic`, `E,refused,tic,error` (state kept), or
//! `E,failed,tic,why`. `--twin28` (no parameters, skill 0 to 3) also plays wire 28
//! alongside and requires every capture to be the DSIM44 one with `profile`,
//! `context.schema` and the two DSIM45 fields as the only differences, and the same
//! refusals. `--fields LEVEL,LEVELTIME` (diagnosis) also prints `F,path,hex` for
//! every field of that capture but `map.*`. Without `--wad` it prints its usage and
//! exits successfully.

use doom_sim::{
    CarriedStart, Diagnostic, DiagnosticLimits, GameParameters, LevelExit, LevelStart,
    MapContextOptions, Simulation, Skill, Snapshot, TicCommand,
};
use doom_test_evidence::sha256;
use std::{error::Error, path::PathBuf};

const USAGE: &str =
    "usage: wire29_real --wad <iwad> --demo <lmp> --schedule <file> [--twin28] [--fields K,T]";
const SKILLS: [Skill; 5] = [
    Skill::Baby,
    Skill::Easy,
    Skill::Medium,
    Skill::Hard,
    Skill::Nightmare,
];

fn framed(out: &mut Vec<u8>, path: &str, value: &[u8]) {
    out.extend((path.len() as u32).to_le_bytes());
    out.extend(path.as_bytes());
    out.extend((value.len() as u64).to_le_bytes());
    out.extend(value);
}

/// The state digest of a DSIM45 capture (`nightmare_genuine_tests.rs`).
fn digest(s: &Snapshot) -> String {
    let kept = |f: &&doom_sim::StateField| {
        !f.path().starts_with("map.") && !f.path().starts_with("events.")
    };
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

/// `Err(why)` unless the DSIM45 capture is the DSIM44 one but for the profile, the
/// context schema and the two new fields.
fn twin(f45: &Snapshot, f44: &Snapshot) -> Result<(), String> {
    let new = ["actors.parameters", "level.gametic_lead"];
    let renamed = ["profile", "context.schema"];
    let a: Vec<_> = f45.fields().filter(|f| !new.contains(&f.path())).collect();
    let b: Vec<_> = f44.fields().collect();
    if a.len() != b.len() {
        return Err(format!("{} fields against {}", a.len(), b.len()));
    }
    for (x, y) in a.iter().zip(&b) {
        if x.path() != y.path() || (x.value() != y.value() && !renamed.contains(&x.path())) {
            return Err(format!(
                "field {} differs from DSIM44's {}",
                x.path(),
                y.path()
            ));
        }
    }
    Ok(())
}

struct Level {
    sim: Simulation,
    old: Option<Simulation>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let (mut wad, mut demo, mut schedule, mut twin28) = (None::<PathBuf>, None, None, false);
    let mut fields = String::new();
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--wad" => wad = Some(PathBuf::from(value()?)),
            "--demo" => demo = Some(PathBuf::from(value()?)),
            "--schedule" => schedule = Some(PathBuf::from(value()?)),
            "--twin28" => twin28 = true,
            "--fields" => fields = value()?,
            _ => return Err(format!("unknown option {a}\n{USAGE}").into()),
        }
    }
    let Some(wad) = wad else {
        println!("{USAGE}");
        return Ok(());
    };
    let bytes = std::fs::read(wad)?;
    let wad =
        doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).map_err(|e| format!("{e:?}"))?;
    let lmp = std::fs::read(demo.ok_or("--demo is required")?)?;
    if lmp.len() < 14 || (lmp.len() - 14) % 4 != 0 || lmp.last() != Some(&0x80) {
        return Err("not a demo with an end marker".into());
    }
    let commands: Vec<TicCommand> = lmp[13..lmp.len() - 1]
        .chunks_exact(4)
        .map(|c| TicCommand {
            forward: c[0] as i8,
            side: c[1] as i8,
            angle_turn: (u16::from(c[2]) << 8) as i16,
            buttons: c[3],
        })
        .collect();
    let skill = SKILLS[usize::from(lmp[1].min(4))];
    let parameters = GameParameters {
        respawn: lmp[5] != 0,
        fast: lmp[6] != 0,
    };
    if twin28 && (lmp[1] > 3 || parameters != GameParameters::default()) {
        return Err("--twin28 needs skill 0 to 3 and no parameters".into());
    }
    let loads: Vec<(usize, u8)> =
        std::fs::read_to_string(schedule.ok_or("--schedule is required")?)?
            .lines()
            .map(|l| {
                let (t, m) = l.split_once(',').ok_or("schedule line")?;
                Ok((t.parse()?, m.parse()?))
            })
            .collect::<Result<_, Box<dyn Error>>>()?;
    let load = |map: u8, start: LevelStart, gametic: usize| -> Result<Level, String> {
        let m = doom_map::Map::from_wad(&wad, &format!("E1M{map}"), doom_map::MapLimits::default())
            .map_err(|e| format!("map {e:?}"))?;
        let ctx = MapContextOptions::default();
        let g = gametic as u64;
        let sim = Simulation::original_player_episode1_v7_level_at(
            m.clone(),
            map,
            skill,
            parameters,
            ctx,
            start,
            g,
        )
        .map_err(|e| format!("load {e:?}"))?;
        let old = twin28
            .then(|| Simulation::original_player_episode1_v6_level_at(m, map, skill, ctx, start, g))
            .transpose()
            .map_err(|e| format!("wire 28 load {e:?}"))?;
        Ok(Level { sim, old })
    };
    let mut map = lmp[3];
    let mut level = load(map, LevelStart::NewGame, 0)?;
    let mut before = level.sim.snapshot().canonical_bytes();
    let (mut k, mut intermission) = (0, None::<(Option<CarriedStart>, u8)>);
    println!("L,0,0,{map},new");
    let none = TicCommand::default();
    let end = 'run: {
        for (tic, c) in commands.iter().enumerate() {
            let fail = |why: String| format!("failed,{tic},{why}");
            let mut start = None;
            if let Some((carried, next)) = intermission {
                let Some(&(_, m)) = loads.iter().find(|l| l.0 == tic) else {
                    continue;
                };
                if m != next {
                    break 'run fail(format!("the genuine load is of map {m}, not {next}"));
                }
                let p_random = level.sim.rng().indices()[0];
                let b = commands[tic - 1].buttons;
                start = Some((
                    next,
                    match carried {
                        Some(c) => LevelStart::Carried(CarriedStart {
                            attack_down: b & 1 != 0,
                            use_down: b & 2 != 0,
                            p_random,
                            ..c
                        }),
                        None => LevelStart::Reborn { p_random },
                    },
                ));
                intermission = None;
            } else if level.sim.reborn_requested() {
                let p_random = level.sim.rng().indices()[0];
                start = Some((map, LevelStart::Reborn { p_random }));
            } else if let Some(exit) = level.sim.level_exit_request() {
                if map == 8 {
                    break 'run format!("victory,{tic}");
                }
                let finish = match level.sim.level_finish() {
                    Ok(f) => f,
                    Err(e) => break 'run fail(format!("level_finish {e:?}")),
                };
                let next = match (map, exit) {
                    (9, _) => 4,
                    (_, LevelExit::Secret) => 9,
                    _ => map + 1,
                };
                intermission = Some((finish.carried, next));
                continue;
            }
            if let Some((m, s)) = start {
                map = m;
                level = match load(m, s, tic) {
                    Ok(l) => l,
                    Err(why) => break 'run fail(why),
                };
                before = level.sim.snapshot().canonical_bytes();
                k += 1;
                let kind = match s {
                    LevelStart::Carried(_) => "carried",
                    _ => "reborn",
                };
                println!("L,{k},{tic},{m},{kind}");
            }
            let r = level.sim.advance([*c, none, none, none]);
            let r28 = level
                .old
                .as_mut()
                .map(|o| o.advance([*c, none, none, none]));
            if let Some(r28) = &r28 {
                if r28.as_ref().err() != r.as_ref().err() {
                    break 'run fail(format!("wire 28 {r28:?}, wire 29 {r:?}"));
                }
            }
            if let Err(e) = r {
                if level.sim.snapshot().canonical_bytes() != before {
                    break 'run fail("the refused tic changed the state".into());
                }
                break 'run format!("refused,{tic},{e:?}");
            }
            let snap = level.sim.snapshot();
            before = snap.canonical_bytes();
            if let Err(e) = Diagnostic::decode_episode1_v7(&before, DiagnosticLimits::default()) {
                break 'run fail(format!("the DSIM45 reader refuses: {e:?}"));
            }
            if let Some(o) = &level.old {
                if let Err(why) = twin(&snap, &o.snapshot()) {
                    break 'run fail(format!("wire 28 twin: {why}"));
                }
            }
            println!("C,{k},{},{}", level.sim.tic(), digest(&snap));
            if fields == format!("{k},{}", level.sim.tic()) {
                for f in snap.fields().filter(|f| !f.path().starts_with("map.")) {
                    let hex: String = f.value().iter().map(|b| format!("{b:02x}")).collect();
                    println!("F,{},{hex}", f.path());
                }
            }
        }
        format!("marker,{}", commands.len())
    };
    println!("E,{end}");
    Ok(())
}
