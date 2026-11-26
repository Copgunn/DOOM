#![no_main]
//! `DSIM` 44 (wire 28: `DSIM` 43 field for field under policy 28, skills 0 and 1 admitted):
//! `Diagnostic::decode_episode1_v6` on arbitrary bytes. Never a panic; an accepted capture is
//! version 44 and the DSIM43 reader refuses it. Rust-only safety. Run with `-max_len=131072`.
use doom_sim::{Diagnostic, DiagnosticLimits};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let limits = DiagnosticLimits::default();
    let Ok(capture) = Diagnostic::decode_episode1_v6(data, limits) else {
        return;
    };
    assert_eq!(capture.version(), 44);
    assert!(Diagnostic::decode_episode1_v5(data, limits).is_err());
});
