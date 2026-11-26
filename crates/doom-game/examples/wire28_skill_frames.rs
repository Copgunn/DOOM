//! Level frames of a wire 28 session item for `tools/media/wire28_readme_visual.py`.
//!
//! `--wad FILE`... `--demo LMP --mode registered|retail --screenblocks N --out DIR` plays the demo through the wire 28
//! session as the gate `skills_0_and_1_against_every_genuine_stream` does (melt steps taken and dropped) and writes
//! `t_TIC.raw` (320x200 palette indices) for every tic with a level frame, and `index.txt` lines `tic,palette`.

use doom_game::session::demo::SessionDemo;
use doom_game::session::{ModeSetting, Session, SessionData, SessionMode, SessionSettings};
use doom_render::Frame;
use std::fmt::Write as _;
use std::sync::Arc;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

fn main() -> Res<()> {
    let (mut wads, mut out, mut demo, mut sb) = (Vec::new(), None, None, 10u8);
    let mut mode = SessionMode::Registered;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut v = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--wad" => wads.push(std::fs::read(v()?)?),
            "--demo" => demo = Some(std::fs::read(v()?)?),
            "--out" => out = Some(v()?),
            "--screenblocks" => sb = v()?.parse()?,
            "--mode" => {
                mode = match v()?.as_str() {
                    "registered" => SessionMode::Registered,
                    "retail" => SessionMode::Retail,
                    m => return Err(format!("mode {m}").into()),
                }
            }
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    let out = out.ok_or("--out DIR is required")?;
    let lmp = demo.ok_or("--demo LMP is required")?;
    let demo = SessionDemo::parse_v3(&lmp).map_err(|e| format!("{e:?}"))?;
    let refs: Vec<&[u8]> = wads.iter().map(Vec::as_slice).collect();
    let data = SessionData::load_with(&refs, &[("DEMO", &lmp)], ModeSetting::Force(mode))
        .map_err(|e| format!("{e:?}"))?;
    let settings = SessionSettings {
        map: demo.map,
        skill: demo.skill,
        screenblocks: sb,
    };
    let mut s = Session::new_finale_and_melt_wire28(Arc::new(data), settings)
        .map_err(|e| format!("{e:?}"))?;
    std::fs::create_dir_all(&out)?;
    let (mut index, mut picture) = (String::new(), Frame::new());
    for &cmd in &demo.commands {
        let r = s.tic_finale_and_melt(cmd).map_err(|e| format!("{e:?}"))?;
        if r.wiped {
            while !s.melt_step(&mut picture).map_err(|e| e.to_string())?.done {}
        }
        if r.level_frame.is_some() {
            std::fs::write(format!("{out}/t_{}.raw", r.tic), s.frame().pixels())?;
            writeln!(index, "{},{}", r.tic, r.palette)?;
        }
    }
    std::fs::write(format!("{out}/index.txt"), index)?;
    Ok(())
}
