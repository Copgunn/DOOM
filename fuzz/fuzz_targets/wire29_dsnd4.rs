#![no_main]
//! `DSND` version 4 (wire 29, the pause sound session's state: `DSND` 2's contents plus
//! `mus_paused` and the level's paused tics): `ChainSoundStateV4::decode` on arbitrary bytes.
//! Never a panic; the encoding is canonical, so an accepted state encodes to the input and
//! decodes to itself; version 3's reader refuses every accepted input. Rust-only safety and
//! consistency; restore is not driven here.
use doom_sound::chain_melt::ChainSoundStateV3;
use doom_sound::chain_pause::ChainSoundStateV4;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(state) = ChainSoundStateV4::decode(data) else {
        return;
    };
    assert_eq!(state.encode(), data, "an accepted state is canonical");
    assert_eq!(ChainSoundStateV4::decode(data).as_ref(), Ok(&state));
    assert!(
        ChainSoundStateV3::decode(data).is_err(),
        "version 3 refuses version 4"
    );
});
