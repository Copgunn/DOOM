#![no_main]
//! `DSIM` 45 (wire 29: `DSIM` 44 field for field under policy 29, with `actors.parameters` and
//! `level.gametic_lead`, skill 4 admitted): `Diagnostic::decode_episode1_v7` on arbitrary bytes.
//! Never a panic; an accepted capture is version 45 and the DSIM44 reader refuses it. Rust-only
//! safety. Run with `-max_len=131072`.
use doom_sim::{Diagnostic, DiagnosticLimits};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let limits = DiagnosticLimits::default();
    let Ok(capture) = Diagnostic::decode_episode1_v7(data, limits) else {
        return;
    };
    assert_eq!(capture.version(), 45);
    assert!(Diagnostic::decode_episode1_v6(data, limits).is_err());
});
