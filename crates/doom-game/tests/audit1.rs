//! Certificate audit 1 (docs/validation/AUDIT1.md, item 8; 2026-11-26): pins of current behaviour
//! that the wire 28 audit list named. Rust-only: no genuine engine ran these, and nothing here
//! changes what a wire does. Each test says what the linuxdoom-1.10 source does at that place.
#![allow(clippy::unwrap_used, reason = "test code")]
mod m6a;

use doom_game::session::demo::SessionDemo;
use doom_game::session::{Session, SessionSettings, SessionWitness};
use doom_render::Frame;
use doom_sim::demo::{DEMO_HEADER, DemoError, parse_demo_episode1_v3, write_demo_episode1_v3};
use doom_sim::{Diagnostic, DiagnosticError, DiagnosticLimits, TicCommand};

fn settings(item: &m6a::Item) -> SessionSettings {
    SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: 10,
    }
}

/// One session tic with its melt steps drained.
fn tic(s: &mut Session, cmd: TicCommand) -> Result<(), SessionWitness> {
    s.tic_finale_and_melt(cmd).map_err(|e| e.witness)?;
    let mut picture = Frame::new();
    while s.melt_in_progress() {
        s.melt_step(&mut picture).map_err(|e| e.witness)?;
    }
    Ok(())
}

/// The demo version-3 readers keep a command's button byte whatever its top two bits are, as
/// `G_ReadDemoTiccmd` does (g_game.c:1491-1503 reads the byte as it is); the writers refuse to
/// write such a command (`DemoError::Buttons`), and the wire 28 session refuses it at its tic
/// with the state kept: `BT_SPECIAL` (0x80: the original's pause and save, g_game.c:698-724) as
/// `Special`, bit 0x40 alone (which nothing in the original reads) on a level tic as `Command`.
/// So a file with such a byte is read and then stops at that tic; nothing is silently changed.
#[test]
fn the_version_3_demo_readers_accept_a_button_byte_the_writers_refuse() {
    let item = m6a::item("SKILL0");
    let lmp = std::fs::read(m6a::root(&item.lmp)).unwrap();
    let clean = SessionDemo::parse_v3(&lmp).unwrap();
    let at = 20_usize;
    for bits in [0x40_u8, 0x80, 0xc0] {
        let mut bad = lmp.clone();
        bad[DEMO_HEADER.len() + 4 * at + 3] |= bits;
        let (_, sim) = parse_demo_episode1_v3(&bad, item.map).unwrap();
        let demo = SessionDemo::parse_v3(&bad).unwrap();
        assert_eq!(sim.commands, demo.commands, "the two readers");
        assert_eq!(demo.commands[at].buttons, clean.commands[at].buttons | bits);
        assert_eq!(demo.bytes_v3(), Err(DemoError::Buttons { tic: at }));
        assert_eq!(
            write_demo_episode1_v3(item.map, item.skill, &demo.commands),
            Err(DemoError::Buttons { tic: at })
        );
        let mut s =
            Session::new_finale_and_melt_wire28(m6a::data(&item, None), settings(&item)).unwrap();
        for cmd in &demo.commands[..at] {
            tic(&mut s, *cmd).unwrap();
        }
        let before = (s.gametic(), s.level().snapshot().canonical_bytes());
        let want = if bits & 0x80 != 0 {
            SessionWitness::Special
        } else {
            SessionWitness::Command
        };
        assert_eq!(tic(&mut s, demo.commands[at]), Err(want), "{bits:#x}");
        assert_eq!(
            before,
            (s.gametic(), s.level().snapshot().canonical_bytes()),
            "{bits:#x}: the state is kept"
        );
    }
}

/// `Snapshot::diagnostic_bytes` writes DSIM versions 20 to 40 and refuses the rest, so it
/// refuses the capture of a wire 27 session (DSIM43) and of a wire 28 session (DSIM44) with
/// `DiagnosticError::Version`; their bytes come from `canonical_bytes`, which the DSIM44 reader
/// accepts. The original has no such format: this is the port's own API, not reached in play.
#[test]
fn diagnostic_bytes_refuses_the_captures_of_wires_27_and_28() {
    let limits = DiagnosticLimits::default();
    let item = m6a::item("SKILL2");
    let s27 = Session::new_finale_and_melt(m6a::data(&item, None), settings(&item)).unwrap();
    let snap = s27.level().snapshot();
    assert_eq!(snap.version(), 43);
    assert_eq!(
        snap.diagnostic_bytes(limits),
        Err(DiagnosticError::Version(43))
    );
    let item = m6a::item("SKILL0");
    let s28 = Session::new_finale_and_melt_wire28(m6a::data(&item, None), settings(&item)).unwrap();
    let snap = s28.level().snapshot();
    assert_eq!(snap.version(), 44);
    assert_eq!(
        snap.diagnostic_bytes(limits),
        Err(DiagnosticError::Version(44))
    );
    let bytes = snap.canonical_bytes();
    assert!(Diagnostic::decode_episode1_v6(&bytes, limits).is_ok());
}
