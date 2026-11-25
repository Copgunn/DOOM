#![no_main]
//! `DSES` version 2 (M6a part 2, the saved state of a finale-and-melt session with its embedded
//! `DSIM` 43 (wire 27), `DDSP` version 4 and `DSND` version 3): `SessionStateV2::decode` on arbitrary
//! bytes. Never a panic; an accepted state passes `check`, and its encoding decodes to an equal
//! state and encodes to itself. Rust-only safety and consistency; restore is not driven here (it
//! needs a level). A synthetic state saved in mid-melt is about 206 KB (three screens and the
//! level capture), so a campaign needs `-max_len` above that.
use doom_game::session::state_v2::SessionStateV2;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(state) = SessionStateV2::decode(data) else {
        return;
    };
    state.check().expect("a decoded state is checked");
    let bytes = state.encode();
    let again = SessionStateV2::decode(&bytes).expect("an accepted state re-encodes to one");
    assert_eq!(again, state);
    assert_eq!(again.encode(), bytes);
});
