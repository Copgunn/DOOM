//! M6a part 2 step 2: the Episode 1 finale (`doom_game::session::finale`, `F_StartFinale` and
//! `F_Ticker`; `doom_render::f_finale::draw`, `F_Drawer`) and the melt step
//! (`doom_render::f_wipe::Melt`, `wipe_initMelt` and `wipe_doMelt` with `ticks = 1`) against the
//! committed goldens of the genuine linuxdoom-1.10 engine (`tools/reference/m6a_goldens.py`; no
//! Rust output feeds an expectation). These functions are reached by no configuration: the
//! session still refuses the victory tic and the display the melt pictures.
//!
//! Each item runs from its committed demo and WADs: the session layer (wire 26, the
//! intermission, the session display) produces every level and intermission frame up to the
//! session's victory refusal; from that tic the new finale logic and drawer take over, the
//! drawer starting from the session's last frame (`screens[0]` persists). Nothing is projected
//! from a golden into a frame. One input is read from the goldens: each melt's `M_Random` index
//! before its 320 draws, the session row's `rndindex` minus 320 (M6a note §1.16.1, as
//! `tools/reference/m6a_melt_steps.py` reads it); where the session ran that tic, the Rust
//! display's index is required to equal the golden's too.
//!
//! Compared: the finale state (`finalestage`, `finalecount`, the flat and the text: session F
//! rows) on every finale tic; every finale frame's digest at screenblocks 10 and 11 (frames G
//! rows); `NOHELP2`'s stop (the frames E row); and every melt step's picture digest, `done` flag
//! and step count (`tests/reference/m6a_melt`, V rows) from start and end screens whose digests
//! equal the golden's (Y rows).
mod m6a;

use doom_game::session::demo::SessionDemo;
use doom_game::session::finale::{self, FinaleView};
use doom_game::session::{NotYet, Session, SessionSettings, SessionWitness};
use doom_render::f_finale;
use doom_render::f_wipe::{MELT_COLUMNS, Melt, melt_offsets};
use doom_render::{GameMode, SCREENHEIGHT, SCREENWIDTH};
use doom_test_evidence::sha256;
use m6a::{Item, Rows, int};
use std::collections::{BTreeMap, BTreeSet};

const PIXELS: usize = SCREENWIDTH * SCREENHEIGHT;
/// The `M_Random` draws of one melt (f_wipe.c:159-166).
const MELT_DRAWS: i64 = 320;

/// A planted fault of the finale (self-tests only; the functions under test have none).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FinaleFault {
    /// `F_Ticker`'s stage test `>=` instead of `>` (f_finale.c:241): stage 1 a tic early.
    Threshold,
    /// One character too many in the crawl (`(finalecount - 10) / 3 + 1`, f_finale.c:298).
    CharacterCount,
    /// The other game mode's art lump (f_finale.c:714-720).
    ArtLump,
}

/// What one item's run produced.
#[derive(Default)]
struct Run {
    /// The composed frame (`screens[0]`) after the wanted tics.
    frames: BTreeMap<i64, Vec<u8>>,
    /// The Rust display's `M_Random` index after each session tic.
    index: BTreeMap<i64, u8>,
    /// The finale state and frame digest after each finale tic.
    finale: BTreeMap<i64, (FinaleView, String)>,
    /// A refused tic and its message.
    stop: Option<(i64, String)>,
    /// Why the Rust side produced no frame at all.
    uncovered: Option<String>,
}

/// Runs `item` at screenblocks 10 from its demo, keeping the frames of the tics in `wanted`.
fn run(item: &Item, wanted: &BTreeSet<i64>, fault: Option<FinaleFault>) -> Run {
    let mut out = Run::default();
    let lmp = std::fs::read(m6a::root(&item.lmp)).expect("demo");
    let demo = match SessionDemo::parse(&lmp) {
        Ok(d) => d,
        Err(e) => {
            out.uncovered = Some(format!("the session demo is refused: {e:?}"));
            return out;
        }
    };
    let data = m6a::data(item, None);
    let settings = SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: 10,
    };
    let mut session = match Session::new(data.clone(), settings) {
        Ok(s) => s,
        Err(e) => {
            out.uncovered = Some(format!("the session does not start: {e:?}"));
            return out;
        }
    };
    let res = data.resources().clone();
    let font = doom_render::hu_stuff::load_font(&res).expect("font");
    let mode = match (data.mode().mode.game_mode(), fault) {
        (GameMode::Retail, Some(FinaleFault::ArtLump)) => GameMode::Registered,
        (_, Some(FinaleFault::ArtLump)) => GameMode::Retail,
        (m, _) => m,
    };
    // In the finale: the view, `screens[0]` and its taint.
    let mut fin: Option<(FinaleView, Vec<u8>, Vec<u32>)> = None;
    for (t, &cmd) in demo.commands.iter().enumerate() {
        let tic = t as i64;
        if fin.is_none() {
            match session.tic(cmd) {
                Ok(_) => {
                    out.index.insert(tic, session.display().random_index());
                    if wanted.contains(&tic) {
                        out.frames.insert(tic, session.frame().pixels().to_vec());
                    }
                    continue;
                }
                Err(e)
                    if e.witness == SessionWitness::NotYet(NotYet::Victory)
                        && tic < item.session_tics as i64 =>
                {
                    let view = finale::start(1).expect("episode 1");
                    let screen = session.frame().pixels().to_vec();
                    fin = Some((view, screen, vec![0; PIXELS]));
                }
                Err(e) => {
                    out.stop = Some((tic, e.to_string()));
                    break;
                }
            }
        }
        let Some((view, screen, taint)) = fin.as_mut() else {
            unreachable!("in the finale")
        };
        finale::tick(view).expect("finale tic");
        if fault == Some(FinaleFault::Threshold)
            && *view
                == (FinaleView {
                    stage: 0,
                    count: finale::STAGE_THRESHOLD,
                })
        {
            *view = FinaleView { stage: 1, count: 0 };
        }
        let mut drawn = *view;
        if fault == Some(FinaleFault::CharacterCount) && drawn.stage == 0 {
            drawn.count += f_finale::TEXTSPEED;
        }
        if let Err(e) = f_finale::draw(&res, &font, mode, &drawn, screen, taint) {
            out.stop = Some((tic, e.to_string()));
            break;
        }
        out.finale.insert(tic, (*view, sha256(screen)));
        if wanted.contains(&tic) {
            out.frames.insert(tic, screen.clone());
        }
    }
    out
}

/// The finale rows of an item against a run: the session F rows, the frames G rows at
/// screenblocks 10 and 11 and the frames E row. Returns the number of frames compared, or the
/// first difference.
fn compare_finale(item: &Item, r: &Run) -> Result<usize, String> {
    let d = |tic: i64, what: String| format!("{} tic {tic}: {what}", item.name);
    let session = Rows::read(&format!("m6a_session/{}.session", item.name));
    let f_rows = session.of("F");
    if f_rows.is_empty() || f_rows.len() != r.finale.len() {
        return Err(format!(
            "{}: {} finale tics, the genuine run {}",
            item.name,
            r.finale.len(),
            f_rows.len()
        ));
    }
    for f in f_rows {
        let tic = int(f, "tic");
        let Some((v, _)) = r.finale.get(&tic) else {
            return Err(d(tic, "no Rust finale tic".into()));
        };
        let got = [i64::from(v.stage), i64::from(v.count)];
        let want = [int(f, "finalestage"), int(f, "finalecount")];
        if got != want || f["flat"] != f_finale::E1FLAT || f["text"] != "1" {
            return Err(d(tic, format!("F {got:?} genuine {want:?}")));
        }
    }
    let mut frames = 0;
    for sb in [10, 11] {
        let (golden, error) = m6a::read_frames(&item.name, sb);
        let finale: Vec<_> = golden.iter().filter(|(_, g)| g.gamestate == 2).collect();
        if finale.len() != r.finale.len() {
            return Err(format!(
                "{} sb{sb}: {} finale frames, the genuine run {}",
                item.name,
                r.finale.len(),
                finale.len()
            ));
        }
        for (tic, g) in finale {
            match r.finale.get(tic) {
                Some((_, sha)) if *sha == g.sha256 => frames += 1,
                _ => return Err(d(*tic, format!("sb{sb} finale frame differs"))),
            }
        }
        let want_stop = error.map(|e| (item.session_tics as i64, e));
        if r.stop != want_stop {
            return Err(format!(
                "{} sb{sb}: stop {:?}, genuine {want_stop:?}",
                item.name, r.stop
            ));
        }
    }
    Ok(frames)
}

/// The finale's three items: every finale tic and frame equal, and `NOHELP2`'s stop.
#[test]
fn the_finale_against_the_genuine_goldens() {
    let mut total = 0;
    for name in ["FINRET", "FINREG", "NOHELP2"] {
        let item = m6a::item(name);
        let r = run(&item, &BTreeSet::new(), None);
        let n = compare_finale(&item, &r).unwrap_or_else(|e| panic!("{e}"));
        eprintln!(
            "{name}: {} finale tics, {n} finale frames (sb10 and sb11) equal; stop {:?}",
            r.finale.len(),
            r.stop
        );
        total += n;
    }
    eprintln!("finale frames compared equal: {total}");
}

/// The planted faults of the finale are caught, each at its first place.
#[test]
fn the_finale_faults_are_caught() {
    let cases = [
        ("FINRET", FinaleFault::Threshold, "tic 1890: F"),
        (
            "FINRET",
            FinaleFault::CharacterCount,
            "finale frame differs",
        ),
        (
            "FINREG",
            FinaleFault::ArtLump,
            "tic 1891: sb10 finale frame differs",
        ),
    ];
    for (name, fault, want) in cases {
        let item = m6a::item(name);
        let e = compare_finale(&item, &run(&item, &BTreeSet::new(), Some(fault)))
            .expect_err("a planted fault must be caught");
        eprintln!("{name} {fault:?}: {e}");
        assert!(e.contains(want), "{name} {fault:?}: {e}");
    }
}

/// One melt of a melt golden: the Y row and its V rows (digest, `done`).
struct GoldenMelt {
    tic: i64,
    start: String,
    end: String,
    steps: Vec<(String, bool)>,
}

fn golden_melts(name: &str) -> Vec<GoldenMelt> {
    let rows = Rows::read(&format!("m6a_melt/{name}.melt"));
    let mut melts: Vec<GoldenMelt> = rows
        .of("Y")
        .iter()
        .map(|y| GoldenMelt {
            tic: int(y, "tic"),
            start: y["start_sha256"].clone(),
            end: y["end_sha256"].clone(),
            steps: Vec::new(),
        })
        .collect();
    for v in rows.of("V") {
        let m = melts
            .iter_mut()
            .find(|m| m.tic == int(v, "tic"))
            .expect("a V row of a Y row's melt");
        assert_eq!(
            int(v, "step"),
            m.steps.len() as i64 + 1,
            "{name}: step order"
        );
        assert_eq!(int(v, "tics"), 1, "{name}: --melt-clock unit");
        m.steps.push((v["sha256"].clone(), int(v, "done") == 1));
    }
    melts
}

/// A planted fault of the melt (self-tests only).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MeltFault {
    /// Column pair 80's initial offset one tic shorter or longer.
    Column,
    /// Step 5 not shown (two steps before its comparison).
    DroppedStep,
    /// The 320 draws start one index later.
    Index,
}

/// One melt from `start` to `end` with the draws after `index`, step for step against `g`.
/// Returns the number of steps compared, or the first difference.
fn compare_melt(
    start: &[u8],
    end: &[u8],
    index: u8,
    g: &GoldenMelt,
    fault: Option<MeltFault>,
) -> Result<usize, String> {
    let index = if fault == Some(MeltFault::Index) {
        index.wrapping_add(1)
    } else {
        index
    };
    let mut m = if fault == Some(MeltFault::Column) {
        let (mut o, _) = melt_offsets(index);
        let c = MELT_COLUMNS / 2;
        o[c] += if o[c] == 0 { -1 } else { 1 };
        Melt::from_offsets(start, end, o).expect("melt")
    } else {
        let (m, after) = Melt::new(start, end, index).expect("melt");
        assert_eq!(i64::from(after), (i64::from(index) + MELT_DRAWS) & 0xff);
        m
    };
    let mut pictures = Vec::new();
    loop {
        let k = pictures.len();
        if fault == Some(MeltFault::DroppedStep) && k == 4 {
            m.step();
        }
        let done = m.step();
        let sha = sha256(m.picture());
        match g.steps.get(k) {
            Some((want, want_done)) if *want == sha && *want_done == done => {}
            Some(_) => return Err(format!("tic {} step {}: picture differs", g.tic, k + 1)),
            None => return Err(format!("tic {}: more than {} steps", g.tic, k)),
        }
        pictures.push(sha);
        if done {
            break;
        }
    }
    let n = pictures.len();
    if n != g.steps.len() {
        return Err(format!(
            "tic {}: {n} steps, genuine {}",
            g.tic,
            g.steps.len()
        ));
    }
    if n < 2 || pictures[n - 1] != g.end || pictures[n - 2] != g.end {
        return Err(format!(
            "tic {}: the last two pictures are not the end screen",
            g.tic
        ));
    }
    Ok(n)
}

/// The start and end screens and the starting index of a melt, checked against the goldens.
fn melt_inputs<'a>(
    r: &'a Run,
    session: &Rows,
    g: &GoldenMelt,
    zero: &'a [u8],
) -> Result<(&'a [u8], &'a [u8], u8), String> {
    let start = if g.tic == 0 {
        zero
    } else {
        r.frames
            .get(&(g.tic - 1))
            .ok_or_else(|| format!("tic {}: no Rust start screen", g.tic))?
    };
    let end = r
        .frames
        .get(&g.tic)
        .ok_or_else(|| format!("tic {}: no Rust end screen", g.tic))?;
    if sha256(start) != g.start || sha256(end) != g.end {
        return Err(format!(
            "tic {}: start or end screen differs from the Y row",
            g.tic
        ));
    }
    let row = session
        .at("G", g.tic)
        .ok_or_else(|| format!("tic {}: no session row", g.tic))?;
    let after = int(row, "rndindex");
    if let Some(&rust) = r.index.get(&g.tic) {
        if i64::from(rust) != after {
            return Err(format!("tic {}: Rust index {rust}, genuine {after}", g.tic));
        }
    }
    let before = u8::try_from((after - MELT_DRAWS) & 0xff).expect("an index");
    Ok((start, end, before))
}

fn wanted(melts: &[GoldenMelt]) -> BTreeSet<i64> {
    melts.iter().flat_map(|m| [m.tic - 1, m.tic]).collect()
}

/// Every melt of every item whose start and end screens the Rust side produces, step for step.
#[test]
fn every_melt_against_the_genuine_pictures() {
    let zero = vec![0u8; PIXELS];
    let (mut melts, mut melts_ok, mut steps, mut steps_ok) = (0, 0, 0, 0);
    let mut uncovered = BTreeMap::new();
    let mut failures = Vec::new();
    for item in m6a::items() {
        let golden = golden_melts(&item.name);
        let n_steps: usize = golden.iter().map(|m| m.steps.len()).sum();
        melts += golden.len();
        steps += n_steps;
        if golden.is_empty() {
            eprintln!("{}: no melt", item.name);
            continue;
        }
        let r = run(&item, &wanted(&golden), None);
        if let Some(why) = &r.uncovered {
            eprintln!(
                "{}: {} melts, {n_steps} steps not covered: {why}",
                item.name,
                golden.len()
            );
            uncovered.insert(item.name.clone(), (golden.len(), n_steps));
            continue;
        }
        let session = Rows::read(&format!("m6a_session/{}.session", item.name));
        let (mut ok, mut ok_steps) = (0, 0);
        for g in &golden {
            let res = melt_inputs(&r, &session, g, &zero)
                .and_then(|(s, e, i)| compare_melt(s, e, i, g, None));
            match res {
                Ok(n) => {
                    ok += 1;
                    ok_steps += n;
                }
                Err(e) => failures.push(format!("{}: {e}", item.name)),
            }
        }
        eprintln!(
            "{}: {ok} of {} melts, {ok_steps} of {n_steps} steps equal",
            item.name,
            golden.len()
        );
        melts_ok += ok;
        steps_ok += ok_steps;
    }
    eprintln!(
        "melts equal: {melts_ok} of {melts}; steps equal: {steps_ok} of {steps}; not covered: \
         {uncovered:?}"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    // Only the easy-skill items are out of reach: the session refuses skills 0 and 1 (M6a step
    // 10, after the hand test), so it produces none of their frames.
    let names: Vec<&str> = uncovered.keys().map(String::as_str).collect();
    assert_eq!(names, ["SKILL0", "SKILL1"]);
    let (um, us) = uncovered
        .values()
        .fold((0, 0), |(a, b), (m, s)| (a + m, b + s));
    assert_eq!((melts_ok + um, steps_ok + us), (melts, steps));
}

/// The planted faults of the melt are caught: on CHAIN's level-to-intermission melt (tic 126)
/// and on FINRET's text-to-art melt (tic 1891, drawn by the new finale drawer).
#[test]
fn the_melt_faults_are_caught() {
    let zero = vec![0u8; PIXELS];
    for (name, tic) in [("CHAIN", 126), ("FINRET", 1891)] {
        let item = m6a::item(name);
        let golden = golden_melts(name);
        let g = golden.iter().find(|m| m.tic == tic).expect("the melt");
        let r = run(&item, &wanted(&golden), None);
        let session = Rows::read(&format!("m6a_session/{name}.session"));
        let (s, e, i) = melt_inputs(&r, &session, g, &zero).expect("inputs");
        assert_eq!(compare_melt(s, e, i, g, None), Ok(g.steps.len()));
        for fault in [MeltFault::Column, MeltFault::DroppedStep, MeltFault::Index] {
            let err = compare_melt(s, e, i, g, Some(fault)).expect_err("caught");
            eprintln!("{name} tic {tic} {fault:?}: {err}");
            if fault == MeltFault::DroppedStep {
                assert!(err.contains("step 5: picture differs"), "{err}");
            }
        }
    }
}
