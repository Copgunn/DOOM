#![no_main]
//! `DSES` version 1 (M6a, the saved session state with its embedded `DSIM` 42, `DDSP` version 3
//! and `DSND` version 2): `SessionState::decode` on arbitrary bytes. Never a panic; an accepted
//! state passes `check`, and its encoding decodes to an equal state and encodes to itself.
//! Rust-only safety and consistency; restore is not driven here (it needs a level). Run with
//! `-max_len=140000` on seeds written by `m6a_dses_reader`'s `write_fuzz_seeds_local`.
use doom_game::session::state::SessionState;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(state) = SessionState::decode(data) else {
        return;
    };
    state.check().expect("a decoded state is checked");
    let bytes = state.encode();
    let again = SessionState::decode(&bytes).expect("an accepted state re-encodes to one");
    assert_eq!(again, state);
    assert_eq!(again.encode(), bytes);
});
