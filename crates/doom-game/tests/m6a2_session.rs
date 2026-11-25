//! M6a part 2 steps 3 and 4, the session: the finale-and-melt configuration of the session
//! (`Session::new_finale_and_melt`) against the genuine linuxdoom-1.10 goldens, end to end, on
//! every drivable item of both golden sets at screenblocks 10 and 11 (the gate is
//! `tests/m6a2_run/mod.rs`; no Rust output feeds an expectation), the frozen session beside it,
//! the planted faults, and the session demo across the finale and the melts.
//!
//! **Coverage** (the owner's coverage gate before the hand test): every tic, sound row, mixer
//! buffer and melt step of every driven item is compared, and the totals are asserted against the
//! goldens' own totals: the 29 part 1 items hold 16,925 tics, 22,548 sound rows, 10,425 mixer
//! buffers and, at screenblocks 10, 125 melts of 4,946 steps; the screenblocks-11 melt goldens of
//! the part 2 set hold 131 melts of 5,182 steps over 31 items (the 29 and FULLREG and FULLRET).
//! **Not compared**, each named by the test: SKILL0 and SKILL1, whose demos the session refuses
//! (the easy skills come with a later pull request, wire 28); NOE1M2AT runs no tic (refused at
//! its load, as the genuine engine stops there), so it has nothing but its mixer golden's header
//! to compare.
mod m6a;
mod m6a2_run;
#[path = "../../doom-sound/tests/common/rows.rs"]
mod rows;

use doom_game::session::demo::{SessionDemo, replay_finale_and_melt};
use doom_game::session::finale_melt::FinaleMeltTicKind;
use doom_game::session::{NotYet, PlantedFault, Session, SessionWitness};
use doom_render::Frame;
use doom_test_evidence::sha256;
use m6a2_run::{
    Counts, DriverFault, Golden, Options, Source, commands, new_session, run, spec, specs,
};

/// The items whose demo the session refuses (skill bytes 0 and 1: the next pull request).
const NOT_DRIVEN: [&str; 2] = ["SKILL0", "SKILL1"];

/// The goldens' own totals, read from the files, and what the runs compared.
#[derive(Debug, Default)]
struct Totals {
    golden_tics: usize,
    golden_sound_rows: usize,
    golden_buffers: u64,
    golden_melts: usize,
    golden_steps: usize,
    uncompared_tics: usize,
    uncompared_sound_rows: usize,
    uncompared_buffers: u64,
    uncompared_melts: usize,
    uncompared_steps: usize,
    compared: Counts,
}

/// Runs every item of `source` at `sb`; every run must equal the goldens.
fn gate(sb: u8, source: Source) -> Totals {
    let mut t = Totals::default();
    let mut not_driven = Vec::new();
    let mut failures = Vec::new();
    for spec in specs().into_iter().filter(|s| s.source == source) {
        let g = Golden::of(&spec, sb);
        let steps: usize = g.melts().values().map(|m| m.steps.len()).sum();
        t.golden_tics += spec.item.session_tics;
        t.golden_sound_rows += g.sound_rows();
        t.golden_buffers += g.buffers();
        t.golden_melts += g.melts().len();
        t.golden_steps += steps;
        let o = run(&spec, Options::at(sb));
        eprintln!(
            "{} sb{sb}: {:?} end {:?}{}",
            spec.item.name,
            o.counts,
            o.end.as_ref().map(ToString::to_string),
            o.not_driven
                .as_deref()
                .map_or(String::new(), |w| format!(" NOT DRIVEN: {w}"))
        );
        if let Some(d) = o.first_difference {
            failures.push(d);
        }
        if o.not_driven.is_some() {
            not_driven.push(spec.item.name.clone());
            t.uncompared_tics += spec.item.session_tics;
            t.uncompared_sound_rows += g.sound_rows();
            t.uncompared_buffers += g.buffers();
            t.uncompared_melts += g.melts().len();
            t.uncompared_steps += steps;
        }
        t.compared.add(&o.counts);
    }
    assert!(
        failures.is_empty(),
        "{} differences:\n{}",
        failures.len(),
        failures.join("\n")
    );
    let want: &[&str] = if source == Source::Part1 {
        &NOT_DRIVEN
    } else {
        &[]
    };
    assert_eq!(not_driven, want, "the items not driven");
    eprintln!("sb{sb} {source:?}: {t:?}");
    // Every genuine tic, sound row, buffer, melt and melt step of a driven item was compared.
    let c = &t.compared;
    assert_eq!(c.tics + t.uncompared_tics, t.golden_tics, "tics");
    assert_eq!(c.g_rows, c.tics, "a G row per tic");
    assert_eq!(
        c.level_frames + c.intermission_frames + c.finale_frames,
        c.tics,
        "a frame per tic"
    );
    assert_eq!(
        c.sound_rows + t.uncompared_sound_rows,
        t.golden_sound_rows,
        "sound rows"
    );
    assert_eq!(
        c.buffers + t.uncompared_buffers,
        t.golden_buffers,
        "buffers"
    );
    assert_eq!(c.melts + t.uncompared_melts, t.golden_melts, "melts");
    assert_eq!(c.melt_steps + t.uncompared_steps, t.golden_steps, "steps");
    assert_eq!(
        c.tics_refused_in_melt, c.melts,
        "a tic refused in every melt"
    );
    t
}

/// The 29 part 1 items at screenblocks 10: every G, C, I, J and F row, every frame, every melt
/// and melt step (the part 1 melt goldens), every sound row and mixer buffer, and every level tic's
/// whole state where `m6a_state.txt` has the item. Totals: the goldens hold 16,925 tics, 22,548
/// sound rows, 10,425 buffers and 125 melts of 4,946 steps; SKILL0 and SKILL1 (728 tics, 1,370
/// sound rows, 6 melts of 238 steps and their buffers) are not driven.
#[test]
fn part1_items_at_screenblocks_10_against_the_genuine_goldens() {
    let t = gate(10, Source::Part1);
    assert_eq!(
        (
            t.golden_tics,
            t.golden_sound_rows,
            t.golden_buffers,
            t.golden_melts,
            t.golden_steps
        ),
        (16925, 22548, 10425, 125, 4946)
    );
    assert_eq!(
        (
            t.uncompared_tics,
            t.uncompared_sound_rows,
            t.uncompared_melts,
            t.uncompared_steps
        ),
        (728, 1370, 6, 238)
    );
    let c = &t.compared;
    assert_eq!(
        (c.tics, c.sound_rows, c.melts, c.melt_steps),
        (16197, 21178, 119, 4708)
    );
    // The whole-state comparison: the 20 items of the state golden, every level tic.
    assert_eq!(c.state_levels, 63);
    eprintln!(
        "part 1 sb10: {} tics, {} sound rows, {} buffers, {} melts, {} steps, {} finale tics, \
         {} state captures",
        c.tics, c.sound_rows, c.buffers, c.melts, c.melt_steps, c.finale_tics, c.state_captures
    );
}

/// The same 29 items at screenblocks 11 (melts from `m6a2_melt/<ITEM>.sb11.melt`): every row,
/// frame, sound row, buffer, melt and step.
#[test]
fn part1_items_at_screenblocks_11_against_the_genuine_goldens() {
    let t = gate(11, Source::Part1);
    let c = &t.compared;
    assert_eq!(
        (c.tics, c.sound_rows, c.melts, c.melt_steps),
        (16197, 21178, 119, 4708)
    );
    assert_eq!((t.golden_melts, t.golden_steps), (125, 4946));
}

/// FULLREG and FULLRET (the whole crawl, the part 2 set) at both sizes: every row (the F rows
/// through the last character on finale tic 1,327 and the art screen from tic 1,571), frame, melt,
/// step and sound row; their buffers against FINREG's and FINRET's genuine mixer goldens (the
/// genuine sound streams are byte-identical, asserted in `Golden::of`).
#[test]
fn full_crawl_items_at_both_sizes_against_the_genuine_goldens() {
    let mut steps = [0usize; 2];
    for (i, sb) in [10u8, 11].into_iter().enumerate() {
        let t = gate(sb, Source::Full);
        let c = &t.compared;
        assert_eq!((c.tics, c.melts), (3902, 6));
        assert_eq!(c.f_rows, c.finale_tics);
        steps[i] = c.melt_steps;
    }
    // 12 melts and 472 steps over both sizes.
    assert_eq!(steps, [236, 236]);
}

/// The frozen session (part 1, `Session::tic`) and the new one give the same reports, frames,
/// `M_Random` index, sound (the new session's buffer of a melting tic taken from the melt's last
/// step) and saved contents (`DSES` version 1 against version 2, field for field, the embedded
/// level capture field for field under wire 26's names (DSIM42 against DSIM43), `DDSP` version 3 against version 4's version-3 part and `DSND`
/// version 2 against version 3's contents), on every tic both run: every drivable part 1 item up
/// to the frozen session's first refusal (the victory, `NotYet::Victory`), at screenblocks 10.
#[test]
fn the_frozen_and_new_sessions_agree_wherever_both_run() {
    let (mut tics, mut saves, mut items) = (0usize, 0usize, 0usize);
    for spec in specs().into_iter().filter(|s| s.source == Source::Part1) {
        let item = &spec.item;
        let Ok(cmds) = commands(item) else {
            assert!(NOT_DRIVEN.contains(&item.name.as_str()));
            continue;
        };
        let data = m6a::data(item, None);
        let settings = m6a2_run::settings(item, 10);
        let (Ok(mut frozen), Ok(mut new)) = (
            Session::new(data.clone(), settings),
            Session::new_finale_and_melt(data, settings),
        ) else {
            assert_eq!(item.name, "NOE1M2AT");
            continue;
        };
        items += 1;
        let mut picture = Frame::new();
        for (t, &cmd) in cmds.iter().enumerate() {
            let a = match frozen.tic(cmd) {
                Ok(a) => a,
                Err(e) => {
                    let b = new.tic_finale_and_melt(cmd);
                    match (&e.witness, &b) {
                        (SessionWitness::NotYet(NotYet::Victory), Ok(b)) => {
                            assert_eq!(b.kind, FinaleMeltTicKind::Victory, "{} {t}", item.name);
                        }
                        (SessionWitness::NotYet(NotYet::Victory), Err(b)) => {
                            // NOVICTOR: the new session refuses its missing D_VICTOR here.
                            assert!(matches!(b.witness, SessionWitness::Sound(_)), "{b}");
                        }
                        (_, Err(b)) => assert_eq!(&e, b, "{} tic {t}", item.name),
                        (_, Ok(_)) => panic!("{} tic {t}: only the frozen refused: {e}", item.name),
                    }
                    break;
                }
            };
            let mut b = new
                .tic_finale_and_melt(cmd)
                .expect("the new session runs it");
            while new.melt_in_progress() {
                let m = new.melt_step(&mut picture).expect("melt step");
                if m.done {
                    b.sound.sound.buffer = m.buffer;
                }
            }
            let w = |what: &str| format!("{} tic {t}: {what}", item.name);
            assert_eq!(b.kind.frozen(), Some(a.kind), "{}", w("kind"));
            assert_eq!(
                (b.tic, b.palette, b.palette_set, b.wiped, b.wi_draws),
                (a.tic, a.palette, a.palette_set, a.wiped, a.wi_draws),
                "{}",
                w("report")
            );
            assert_eq!(b.wi_starts, a.wi_starts, "{}", w("intermission starts"));
            assert_eq!(b.level_frame, a.level_frame, "{}", w("level frame"));
            assert_eq!(b.sound, a.sound, "{}", w("sound"));
            assert_eq!(b.finale, None);
            assert_eq!(
                frozen.frame().pixels(),
                new.frame().pixels(),
                "{}",
                w("frame")
            );
            assert_eq!(
                frozen.display().random_index(),
                new.display().random_index(),
                "{}",
                w("index")
            );
            tics += 1;
            if t % 35 == 0 || !matches!(b.kind, FinaleMeltTicKind::Level) {
                let v1 = frozen.save().expect("version 1");
                let v2 = new.save_v2().expect("version 2");
                assert_eq!(
                    (
                        v1.mode,
                        v1.settings,
                        v1.gametic,
                        v1.gamestate,
                        v1.map,
                        v1.gameaction,
                        v1.secretexit,
                        v1.didsecret
                    ),
                    (
                        v2.mode,
                        v2.settings,
                        v2.gametic,
                        v2.gamestate,
                        v2.map,
                        v2.gameaction,
                        v2.secretexit,
                        v2.didsecret
                    ),
                    "{}",
                    w("saved fields")
                );
                assert_eq!(
                    (v1.wminfo, v1.wi, v1.carried),
                    (v2.wminfo, v2.wi, v2.carried),
                    "{}",
                    w("saved session group")
                );
                // Wire 26 (DSIM42) under the frozen session, wire 27 (DSIM43) under the new one:
                // equal field for field once the three values that name the wire (the version,
                // `profile`, `context.schema`) are read under wire 26's names (`wire26_fields`).
                let (l1, l2) = (frozen.level().snapshot(), new.level().snapshot());
                assert_eq!((l1.version(), l2.version()), (42, 43), "{}", w("wires"));
                assert_eq!(v1.level, l1.canonical_bytes(), "{}", w("saved level 1"));
                assert_eq!(v2.level, l2.canonical_bytes(), "{}", w("saved level 2"));
                assert_eq!(
                    m6a2_run::wire26_fields(&l1),
                    m6a2_run::wire26_fields(&l2),
                    "{}",
                    w("saved level")
                );
                assert!(v2.display.melt.is_none() && !v2.sound.state.melt);
                assert_eq!(v1.display, v2.display.session, "{}", w("saved display"));
                assert_eq!(v1.sound, v2.sound.state, "{}", w("saved sound"));
                saves += 1;
            }
        }
    }
    eprintln!("frozen and new: {items} items, {tics} tics equal, {saves} saved states equal");
    assert_eq!(items, 26);
    assert!(tics > 10_000, "{tics}");
}

/// Each planted fault is caught by the comparison with the genuine goldens, named: the finale
/// started one tic late (FINRET: the victory tic's F row), a game tic run during a melt (the
/// session's guard removed and the driver asking for a tic after one step: the melt's pictures
/// stop short), a demo command consumed during a melt step (the commands slip), the stage-1 wipe
/// missing (FINRET tic 1,891: no melt where the genuine run melts), and a buffer mixed before the
/// melt ends (the first melt step changes the sound; the mixer rows differ as well).
#[test]
fn the_planted_faults_are_caught() {
    let cases: [(&str, Option<PlantedFault>, Option<DriverFault>, &str); 6] = [
        (
            "FINRET",
            Some(PlantedFault::FinaleLate),
            None,
            "FINRET sb10 tic 321: F stage and count (0, 0) genuine (0, 1)",
        ),
        (
            "CHAIN",
            Some(PlantedFault::TicInMelt),
            Some(DriverFault::TicAfterOneMeltStep),
            "CHAIN sb10 tic 0: melt has 1 steps",
        ),
        (
            "CHAIN",
            None,
            Some(DriverFault::CommandInMelt),
            "CHAIN sb10 tic ",
        ),
        (
            "FINRET",
            Some(PlantedFault::NoArtWipe),
            None,
            "FINRET sb10 tic 1891: ",
        ),
        (
            "FINRET",
            Some(PlantedFault::MixInMelt),
            None,
            "FINRET sb10 tic 0: melt step 1 changed the sound",
        ),
        (
            "FULLRET",
            Some(PlantedFault::NoArtWipe),
            None,
            "FULLRET sb10 tic 1891: ",
        ),
    ];
    for (name, sf, df, want) in cases {
        let o = run(
            &spec(name),
            Options {
                sb: 10,
                session_fault: sf,
                driver_fault: df,
            },
        );
        let d = o
            .first_difference
            .unwrap_or_else(|| panic!("{name} {sf:?} {df:?}: not caught"));
        eprintln!("{name} {sf:?} {df:?}: {}", d.lines().next().unwrap_or(""));
        assert!(
            d.contains(want),
            "{name} {sf:?} {df:?}: caught elsewhere: {d}"
        );
    }
    // Without the session fault, the early tic of the driver is the typed refusal.
    let chain = spec("CHAIN").item;
    let mut s = new_session(&chain, 10).expect("session");
    let cmds = commands(&chain).expect("demo");
    s.tic_finale_and_melt(cmds[0]).expect("tic 0");
    s.melt_step(&mut Frame::new()).expect("one step");
    let before = s.save_v2().expect("save").encode();
    let end = s.tic_finale_and_melt(cmds[1]).expect_err("refused");
    assert_eq!((end.tic, end.witness), (1, SessionWitness::MeltInProgress));
    assert_eq!(s.save_v2().expect("save").encode(), before, "unchanged");
    // A buffer mixed in a melt is also caught by the mixer comparison alone: the session with the
    // fault, driven without the per-step check, loses the buffer of the first melting tic.
    let item = spec("FINRET").item;
    let mut s = new_session(&item, 10).expect("session");
    s.plant_fault(PlantedFault::MixInMelt);
    let cmds = commands(&item).expect("demo");
    let r = s.tic_finale_and_melt(cmds[0]).expect("tic 0");
    assert!(r.wiped && r.sound.sound.buffer.is_none());
    let mut f = Frame::new();
    let mut last = None;
    while s.melt_in_progress() {
        last = Some(s.melt_step(&mut f).expect("step"));
    }
    let last = last.expect("a melt at tic 0");
    // The genuine mixer golden has buffer 0 after tic 0 (512 * 0 / 315 = 0); the faulty session
    // hands out none with the melt's end.
    assert!(last.done && last.buffer.is_none(), "the buffer was lost");
}

/// A demo recorded from the new session across the finale and the melts (one command per session
/// tic in every game state; a melt step reads none) is written by the recorder's writer, read
/// back, and replayed through a fresh session by `replay_finale_and_melt`: the replay runs the
/// same number of tics and melt steps, and its `DSES` version 2 bytes equal the recording run's
/// after every tic and after the last. The recording is FINRET's demo, so the file also equals the
/// committed demo byte for byte.
#[test]
fn a_demo_recorded_across_the_finale_and_the_melts_replays_to_the_same_state() {
    for name in ["FINRET", "CHAIN", "REBHELD"] {
        let item = spec(name).item;
        let cmds = commands(&item).expect("demo");
        let mut s = new_session(&item, 10).expect("session");
        let mut recorded = Vec::new();
        let mut digests = Vec::new();
        let (mut steps, mut f) = (0usize, Frame::new());
        for &cmd in &cmds {
            s.tic_finale_and_melt(cmd).expect("tic");
            recorded.push(cmd);
            while s.melt_in_progress() {
                s.melt_step(&mut f).expect("step");
                steps += 1;
            }
            digests.push(sha256(&s.save_v2().expect("save").encode()));
        }
        let demo = SessionDemo {
            map: item.map,
            skill: item.skill,
            commands: recorded,
        };
        let bytes = demo.bytes().expect("written");
        assert_eq!(
            bytes,
            std::fs::read(m6a::root(&item.lmp)).expect("demo"),
            "{name}: the recording is the committed demo"
        );
        let back = SessionDemo::parse(&bytes).expect("read back");
        let mut r = new_session(&item, 10).expect("session");
        let (mut tics, mut replay_steps, mut k) = (0usize, 0usize, 0usize);
        let replay_digests = std::cell::RefCell::new(Vec::new());
        let end = replay_finale_and_melt(
            &mut r,
            &back.commands,
            |_, _| tics += 1,
            |_, m, _| {
                replay_steps += 1;
                if m.done {
                    k += 1;
                }
            },
        );
        assert_eq!(end, None, "{name}: replayed to the end");
        // The digests after each tic of the replay, taken again by a second replay that saves.
        let mut r2 = new_session(&item, 10).expect("session");
        replay_finale_and_melt(
            &mut r2,
            &back.commands,
            |sess, rep| {
                if !rep.wiped {
                    let d = sha256(&sess.save_v2().expect("save").encode());
                    replay_digests.borrow_mut().push(d);
                }
            },
            |sess, m, _| {
                if m.done {
                    let d = sha256(&sess.save_v2().expect("save").encode());
                    replay_digests.borrow_mut().push(d);
                }
            },
        );
        let replay_digests = replay_digests.into_inner();
        assert_eq!((tics, replay_steps), (cmds.len(), steps), "{name}");
        assert_eq!(replay_digests, digests, "{name}: the state after every tic");
        assert_eq!(
            r.save_v2().expect("save").encode(),
            s.save_v2().expect("save").encode(),
            "{name}: the final state"
        );
        eprintln!(
            "{name}: {tics} tics, {replay_steps} melt steps over {k} melts, every state equal"
        );
    }
}

/// Local only (never in CI: it reads the owner's IWAD): the sessions `doom-play` recorded and
/// our idle-extended copies of them, driven through the finale-and-melt session against the
/// goldens `tools/local/m6a2_play_check.py` wrote from the genuine engine, at both screen sizes:
/// everything [`run`] compares, the finale and every melt included. Prints, per item, the session
/// tics compared and how the run ended. Run with `M6A_REFERENCE_DIR=<its --out> cargo test
/// --release -p doom-game --test m6a2_session -- --ignored real_maps --nocapture`; without the
/// variable it does nothing.
#[test]
#[ignore = "local only: needs M6A_REFERENCE_DIR from tools/local/m6a2_play_check.py"]
fn real_maps_finale_and_melt_local() {
    if !m6a::real_maps() {
        eprintln!("skip: M6A_REFERENCE_DIR is not set");
        return;
    }
    let mut failures = Vec::new();
    for item in m6a::items() {
        let spec = m6a2_run::Spec {
            item,
            source: Source::Part1,
        };
        for sb in [10, 11] {
            let o = run(&spec, Options::at(sb));
            eprintln!(
                "{} sb{sb}: genuine {} tics, end {}; compared {:?}; Rust end {:?}",
                spec.item.name,
                spec.item.session_tics,
                spec.item.end,
                o.counts,
                o.end.as_ref().map(ToString::to_string),
            );
            assert!(o.not_driven.is_none(), "{}", spec.item.name);
            failures.extend(o.first_difference);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The local gate can fail: on every item of `M6A_REFERENCE_DIR`, a buffer mixed during a melt
/// is reported, on every item that reaches the finale so is a finale that starts a tic late, and
/// on every item that reaches the art screen so is a missing wipe before it (an item has a wipe
/// at its start, at each load and at the victory; one more is the art screen's). Prints each
/// first difference.
#[test]
#[ignore = "local only: needs M6A_REFERENCE_DIR from tools/local/m6a2_play_check.py"]
fn real_maps_finale_and_melt_local_catches_planted_faults() {
    if !m6a::real_maps() {
        eprintln!("skip: M6A_REFERENCE_DIR is not set");
        return;
    }
    for item in m6a::items() {
        let spec = m6a2_run::Spec {
            item,
            source: Source::Part1,
        };
        let clean = run(&spec, Options::at(10)).counts;
        let mut faults = vec![PlantedFault::MixInMelt];
        if clean.finale_tics > 0 {
            faults.push(PlantedFault::FinaleLate);
        }
        if clean.finale_tics > 0 && clean.wipes > clean.loads + 2 {
            faults.push(PlantedFault::NoArtWipe);
        }
        for fault in faults {
            let options = Options {
                session_fault: Some(fault),
                ..Options::at(10)
            };
            let d = run(&spec, options).first_difference;
            eprintln!("{} {fault:?}: {d:?}", spec.item.name);
            assert!(d.is_some(), "{} {fault:?} not caught", spec.item.name);
        }
    }
}
