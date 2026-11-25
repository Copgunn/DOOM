#![no_main]
//! `DDSP` version 4 (M6a part 2, the finale-and-melt session display state):
//! `SessionDisplayStateV4::decode` on arbitrary bytes. Never a panic, in `decode` or in `check`
//! (which replays a melt in progress); an accepted state's encoding decodes to an equal state and
//! encodes to itself (run counts need not be canonical in the input). Rust-only safety and
//! consistency; restore is not driven here. Run with `-max_len=200000` (a melt in progress holds
//! three 64,000-byte screens).
use doom_render::SessionDisplayStateV4;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(state) = SessionDisplayStateV4::decode(data) else {
        return;
    };
    let _ = state.check();
    let bytes = state.encode();
    let again = SessionDisplayStateV4::decode(&bytes).expect("an accepted state re-encodes to one");
    assert_eq!(again, state);
    assert_eq!(again.encode(), bytes);
});
