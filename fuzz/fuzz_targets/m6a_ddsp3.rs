#![no_main]
//! `DDSP` version 3 (M6a, the session display state): `SessionDisplayState::decode` on arbitrary
//! bytes. Never a panic, in `decode` or in `check`; an accepted state's encoding decodes to an
//! equal state and encodes to itself (run counts need not be canonical in the input). Rust-only
//! safety and consistency; restore is not driven here. Run with `-max_len=140000`.
use doom_render::SessionDisplayState;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(state) = SessionDisplayState::decode(data) else {
        return;
    };
    let _ = state.check();
    let bytes = state.encode();
    let again = SessionDisplayState::decode(&bytes).expect("an accepted state re-encodes to one");
    assert_eq!(again, state);
    assert_eq!(again.encode(), bytes);
});
