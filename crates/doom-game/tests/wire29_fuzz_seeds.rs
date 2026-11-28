//! Seeds for the five wire 29 fuzz targets (`wire29_dsim45`, `wire29_dses4`, `wire29_ddsp5`,
//! `wire29_dsnd4`, `wire29_demo4`), written outside the repository:
//! `WIRE29_FUZZ_SEEDS=DIR cargo test --release -p doom-game --test wire29_fuzz_seeds -- --ignored
//! write_fuzz_seeds` (docs/TESTING.md). The states are saved from the committed synthetic items
//! (PAUSE paused, RESP4 inside the respawn's fogs, LOADP between a paused load and its tic 0, and
//! the first melt of PAUSEWI); the demos are the committed `.lmp` files, each as it is and with
//! its map number in front (the simulation reader's input). Nothing here is an expectation.
mod m6a;

use doom_game::session::demo::SessionDemo;
use doom_game::session::{Session, SessionSettings};
use doom_render::{Frame, SessionDisplayStateV5};
use doom_sim::{GameParameters, TicCommand};
use doom_sound::chain_pause::ChainSoundStateV4;
use std::path::{Path, PathBuf};

fn put(dir: &Path, target: &str, name: &str, bytes: &[u8]) {
    let d = dir.join(target);
    std::fs::create_dir_all(&d).expect("seed directory");
    std::fs::write(d.join(name), bytes).expect("seed");
}

fn demo(path: PathBuf) -> (m6a::Item, GameParameters, Vec<TicCommand>, Vec<u8>) {
    let bytes = std::fs::read(&path).expect("demo");
    let (d, parameters) = SessionDemo::parse_v4(&bytes).expect("reads");
    let item = m6a::Item {
        name: String::new(),
        mode: "registered".into(),
        wads: Vec::new(),
        lmp: String::new(),
        map: d.map,
        skill: d.skill,
        end: "marker".into(),
        session_tics: d.commands.len(),
    };
    (item, parameters, d.commands, bytes)
}

#[test]
#[ignore = "writes seeds under WIRE29_FUZZ_SEEDS"]
fn write_fuzz_seeds() {
    let dir = PathBuf::from(std::env::var("WIRE29_FUZZ_SEEDS").expect("WIRE29_FUZZ_SEEDS=DIR"));
    let shared = [
        "tests/reference/m5b_gfx.wad",
        "tests/reference/m5c_sfx.wad",
        "tests/reference/m6a_wads/m6a_gfx.wad",
        "tests/reference/m6a_wads/m6a_music.wad",
        "tests/reference/m6b_wads/m6b_gfx.wad",
    ];
    let mut lmps: Vec<PathBuf> = ["m6b_scripts", "wire29_session"]
        .iter()
        .flat_map(|d| std::fs::read_dir(m6a::root(&format!("tests/reference/{d}"))).unwrap())
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "lmp"))
        .collect();
    lmps.sort();
    for p in &lmps {
        let (item, _, _, bytes) = demo(p.clone());
        let name = p.file_stem().unwrap().to_string_lossy().into_owned();
        put(&dir, "wire29_demo4", &name, &bytes);
        let mut with_map = vec![item.map];
        with_map.extend(&bytes);
        put(&dir, "wire29_demo4", &format!("{name}.map"), &with_map);
    }
    // (item, first WAD, session tics at which to save, saved after a tic that began a melt)
    let items = [
        ("PAUSE", "m6b_wads/m6b.wad", vec![0, 73], false),
        ("RESP4", "m6b_wads/m6b.wad", vec![740], false),
        ("LOADP", "m6a_wads/episode.wad", vec![143], false),
        ("PAUSEWI", "m6a_wads/episode.wad", vec![], true),
    ];
    let mut states = 0;
    for (name, first, at, melt) in items {
        let lmp = if name == "LOADP" {
            format!("tests/reference/wire29_session/{name}.lmp")
        } else {
            format!("tests/reference/m6b_scripts/{name}.lmp")
        };
        let (mut item, parameters, cmds, _) = demo(m6a::root(&lmp));
        item.lmp = lmp;
        item.wads = std::iter::once(format!("tests/reference/{first}"))
            .chain(shared.map(String::from))
            .collect();
        let settings = SessionSettings {
            map: item.map,
            skill: item.skill,
            screenblocks: 10,
        };
        let mut s =
            Session::new_finale_and_melt_wire29(m6a::data(&item, None), settings, parameters)
                .expect("starts");
        let mut unseen = Frame::new();
        let save = |s: &Session, tag: String| {
            let state = s.save_v4().expect("saveable");
            put(&dir, "wire29_dses4", &tag, &state.encode());
            put(&dir, "wire29_dsim45", &tag, &state.base.level);
            let d = SessionDisplayStateV5 {
                display: state.base.display.clone(),
                pause: state.v4.display,
            };
            put(&dir, "wire29_ddsp5", &tag, &d.encode());
            let n = ChainSoundStateV4 {
                state: state.base.sound.state.clone(),
                pause: state.v4.sound,
            };
            put(&dir, "wire29_dsnd4", &tag, &n.encode());
            1
        };
        let mut melt_saved = false;
        for (t, &c) in cmds.iter().enumerate() {
            if at.contains(&t) {
                states += save(&s, format!("{name}_{t}"));
            }
            s.tic_finale_and_melt(c).expect("a tic");
            if melt && !melt_saved && s.melt_in_progress() {
                states += save(&s, format!("{name}_melt_{t}"));
                melt_saved = true;
            }
            while s.melt_in_progress() {
                s.melt_step(&mut unseen).expect("a melt step");
            }
        }
        assert!(!melt || melt_saved, "{name} has a melt");
    }
    assert_eq!(states, 5);
    eprintln!("{} demos, {states} saved states", lmps.len());
}
