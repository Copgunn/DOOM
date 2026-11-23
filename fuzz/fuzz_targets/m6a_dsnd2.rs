#![no_main]
//! `DSND` version 2 (M6a, the session's sound state): `ChainSoundState::decode` on arbitrary
//! bytes. Never a panic; the encoding is canonical, so an accepted state encodes to the input and
//! decodes to itself. Rust-only safety and consistency; restore is not driven here.
use doom_sound::chain_state::{ChainSoundState, MAX_BYTES};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(state) = ChainSoundState::decode(data) else {
        return;
    };
    assert!(data.len() <= MAX_BYTES);
    assert_eq!(state.encode(), data, "an accepted state is canonical");
    assert_eq!(ChainSoundState::decode(data).as_ref(), Ok(&state));
});
