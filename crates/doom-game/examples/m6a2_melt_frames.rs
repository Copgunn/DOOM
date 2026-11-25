//! Frames of an M6a part 2 item for the README animations (`tools/media/m6a2_readme_visual.py`).
//!
//! `--wad FILE`... `--demo LMP --mode registered|retail --screenblocks N --out DIR` drives the finale-and-melt
//! session through the demo as the committed gate does (a tic per command, then every step of the melt the
//! tic's display started) and writes, in the layout of the genuine runner's tap (`tools/media/m6a2_pixel_tap.c`):
//! `d_TIC.raw` (the tic's frame, 320x200 palette indices), `ms_TIC.raw` and `me_TIC.raw` (a melt's start and end
//! screens), `m_TIC_STEP.raw` (each melt step) and `index.txt` (`D,tic,palette` and `M,tic,step,palette,done`).

use doom_game::session::demo::SessionDemo;
use doom_game::session::{ModeSetting, Session, SessionData, SessionMode, SessionSettings};
use doom_render::Frame;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

fn main() -> Res<()> {
    let (mut wads, mut out, mut demo, mut sb, mut mode) =
        (Vec::new(), None, None, 10u8, SessionMode::Retail);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut v = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--wad" => wads.push(std::fs::read(v()?)?),
            "--demo" => demo = Some(std::fs::read(v()?)?),
            "--out" => out = Some(PathBuf::from(v()?)),
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
    let demo = SessionDemo::parse(&lmp).map_err(|e| format!("{e:?}"))?;
    let refs: Vec<&[u8]> = wads.iter().map(Vec::as_slice).collect();
    let data = SessionData::load_with(&refs, &[("DEMO", &lmp)], ModeSetting::Force(mode))
        .map_err(|e| format!("{e:?}"))?;
    let settings = SessionSettings {
        map: demo.map,
        skill: demo.skill,
        screenblocks: sb,
    };
    let mut s =
        Session::new_finale_and_melt(Arc::new(data), settings).map_err(|e| format!("{e:?}"))?;
    std::fs::create_dir_all(&out)?;
    let (mut index, mut picture) = (String::new(), Frame::new());
    for &cmd in &demo.commands {
        let Ok(r) = s.tic_finale_and_melt(cmd) else {
            break;
        };
        writeln!(index, "D,{},{}", r.tic, r.palette)?;
        if r.wiped {
            let (start, end) = s.display().melt_screens().ok_or("no melt")?;
            std::fs::write(out.join(format!("ms_{}.raw", r.tic)), start)?;
            std::fs::write(out.join(format!("me_{}.raw", r.tic)), end)?;
            loop {
                let m = s.melt_step(&mut picture).map_err(|e| e.to_string())?;
                std::fs::write(
                    out.join(format!("m_{}_{}.raw", r.tic, m.step)),
                    picture.pixels(),
                )?;
                writeln!(
                    index,
                    "M,{},{},{},{}",
                    r.tic,
                    m.step,
                    m.palette,
                    u8::from(m.done)
                )?;
                if m.done {
                    break;
                }
            }
        }
        std::fs::write(out.join(format!("d_{}.raw", r.tic)), s.frame().pixels())?;
    }
    std::fs::write(out.join("index.txt"), index)?;
    Ok(())
}
