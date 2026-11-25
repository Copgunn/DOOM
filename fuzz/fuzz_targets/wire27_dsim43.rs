#![no_main]
//! `DSIM` 43 (wire 27: `DSIM` 42 field for field under policy 27):
//! `Diagnostic::decode_episode1_v5` on arbitrary bytes. Never a panic; an accepted capture is
//! version 43 and the DSIM42 reader refuses it. Rust-only safety. Run with `-max_len=131072`.
use doom_sim::{Diagnostic, DiagnosticLimits};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let limits = DiagnosticLimits::default();
    let Ok(capture) = Diagnostic::decode_episode1_v5(data, limits) else {
        return;
    };
    assert_eq!(capture.version(), 43);
    assert!(Diagnostic::decode_episode1_v4(data, limits).is_err());
});
