#![no_main]
//! `DSND` version 3 (M6a part 2, the finale-and-melt sound session's state):
//! `ChainSoundStateV3::decode` on arbitrary bytes. Never a panic; the encoding is canonical, so an
//! accepted state encodes to the input and decodes to itself; version 2's reader refuses every
//! accepted input (the version field differs). Rust-only safety and consistency; restore is not
//! driven here.
use doom_sound::chain_melt::ChainSoundStateV3;
use doom_sound::chain_state::{ChainSoundState, MAX_BYTES};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(state) = ChainSoundStateV3::decode(data) else {
        return;
    };
    assert!(data.len() <= MAX_BYTES);
    assert_eq!(state.encode(), data, "an accepted state is canonical");
    assert_eq!(ChainSoundStateV3::decode(data).as_ref(), Ok(&state));
    assert!(
        ChainSoundState::decode(data).is_err(),
        "version 2 refuses version 3"
    );
});
