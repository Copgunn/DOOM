#![no_main]
//! `DSIM` 42 (M6a, wire 26: `DSIM` 41 plus the level-start group):
//! `Diagnostic::decode_episode1_v4` on arbitrary bytes. Never a panic; an accepted capture is
//! version 42 and every earlier explicit reader refuses it. Rust-only safety; `diagnostic_format`
//! covers the shared container. Run with `-max_len=131072`.
use doom_sim::{Diagnostic, DiagnosticLimits};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let limits = DiagnosticLimits::default();
    let Ok(capture) = Diagnostic::decode_episode1_v4(data, limits) else {
        return;
    };
    assert_eq!(capture.version(), 42);
    assert!(capture.field("level.start.kind").is_some());
    assert!(Diagnostic::decode_episode1_v3(data, limits).is_err());
});
