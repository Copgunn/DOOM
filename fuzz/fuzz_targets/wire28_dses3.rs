#![no_main]
//! `DSES` version 3 (wire 28, the saved state of a wire 28 finale-and-melt session with its
//! embedded `DSIM` 44): `SessionStateV3::decode` on arbitrary bytes. Never a panic; an accepted
//! state passes `check`, its skill byte is its embedded level's skill (owner's ruling 2 of
//! 2026-11-26; seed `skill_and_level` is a state refused for that alone), its encoding decodes to
//! an equal state and encodes to itself, and the version 2 reader refuses it. Rust-only safety; restore is not driven here (it needs a level).
//! A synthetic state saved in mid-melt is about 206 KB, so a campaign needs `-max_len` above that.
use doom_game::session::state_v2::SessionStateV2;
use doom_game::session::state_v3::SessionStateV3;
use doom_sim::{Diagnostic, DiagnosticLimits};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(state) = SessionStateV3::decode(data) else {
        return;
    };
    state.check().expect("a decoded state is checked");
    let level = Diagnostic::decode_episode1_v6(&state.0.level, DiagnosticLimits::default());
    let skill = level
        .as_ref()
        .ok()
        .and_then(|l| l.field("actors.options")?.first());
    assert_eq!(skill, Some(&state.0.settings.skill));
    let bytes = state.encode();
    let again = SessionStateV3::decode(&bytes).expect("an accepted state re-encodes to one");
    assert_eq!(again, state);
    assert_eq!(again.encode(), bytes);
    assert!(SessionStateV2::decode(&bytes).is_err());
});
