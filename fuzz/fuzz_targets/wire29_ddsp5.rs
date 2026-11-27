#![no_main]
//! `DDSP` version 5 (wire 29, the pause session display state: `DDSP` 4 plus `paused` and the
//! level's paused tics): `SessionDisplayStateV5::decode` on arbitrary bytes. Never a panic, in
//! `decode` or in `check` (which replays a melt in progress); an accepted state's encoding
//! decodes to an equal state and encodes to itself (run counts need not be canonical in the
//! input), and the version 4 reader refuses it. Rust-only safety; restore is not driven here.
//! Run with `-max_len=200000` (a melt in progress holds three 64,000-byte screens).
use doom_render::{SessionDisplayStateV4, SessionDisplayStateV5};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(state) = SessionDisplayStateV5::decode(data) else {
        return;
    };
    let _ = state.check();
    let bytes = state.encode();
    let again = SessionDisplayStateV5::decode(&bytes).expect("an accepted state re-encodes to one");
    assert_eq!(again, state);
    assert_eq!(again.encode(), bytes);
    assert!(SessionDisplayStateV4::decode(&bytes).is_err());
});
