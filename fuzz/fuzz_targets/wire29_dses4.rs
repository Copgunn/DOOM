#![no_main]
//! `DSES` version 4 (wire 29, the saved state of a wire 29 finale-and-melt session with its
//! embedded `DSIM` 45, and the session's `respawnparm`, `fastparm` and `paused` bytes):
//! `SessionStateV4::decode` on arbitrary bytes. Never a panic; an accepted state passes `check`,
//! its skill byte is its embedded level's skill, its parameters are the level's
//! `actors.parameters`, its encoding decodes to an equal state and encodes to itself, and the
//! version 2 and 3 readers refuse it. Rust-only safety; restore is not driven here (it needs a
//! level). A synthetic state saved in mid-melt is about 206 KB, so a campaign needs `-max_len`
//! above that.
use doom_game::session::state_v2::SessionStateV2;
use doom_game::session::state_v3::SessionStateV3;
use doom_game::session::state_v4::SessionStateV4;
use doom_sim::{Diagnostic, DiagnosticLimits};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(state) = SessionStateV4::decode(data) else {
        return;
    };
    state.check().expect("a decoded state is checked");
    let level = Diagnostic::decode_episode1_v7(&state.base.level, DiagnosticLimits::default())
        .expect("an accepted state holds a DSIM45 capture");
    assert_eq!(
        level.field("actors.options").and_then(|f| f.first()),
        Some(&state.base.settings.skill)
    );
    assert_eq!(
        level.field("actors.parameters"),
        Some(
            &[
                u8::from(state.v4.parameters.respawn),
                u8::from(state.v4.parameters.fast)
            ][..]
        )
    );
    let bytes = state.encode();
    let again = SessionStateV4::decode(&bytes).expect("an accepted state re-encodes to one");
    assert_eq!(again, state);
    assert_eq!(again.encode(), bytes);
    assert!(SessionStateV3::decode(&bytes).is_err());
    assert!(SessionStateV2::decode(&bytes).is_err());
});
