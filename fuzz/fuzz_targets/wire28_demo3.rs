#![no_main]
//! The wire 28 demo readers (skills 0 to 3): `doom_sim::demo::parse_demo_episode1_v3` (map from
//! byte 0, demo from the rest) and `SessionDemo::parse_v3` on arbitrary bytes. Never a panic; an
//! accepted demo that `write_demo_episode1_v3` / `bytes_v3` write (they refuse the commands the
//! readers pass on to the session: a record's two top button bits, a marker-valued byte) writes
//! bytes that parse to the same commands, the two readers agree, and the version 2 reader agrees
//! on skills 2 and 3 and refuses skills 0 and 1. Rust-only safety. Run with `-max_len=4096`.
use doom_game::session::demo::SessionDemo;
use doom_sim::demo::{parse_demo_episode1_v2, parse_demo_episode1_v3, write_demo_episode1_v3};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some((&map, bytes)) = data.split_first()
        && let Ok((h, demo)) = parse_demo_episode1_v3(bytes, map)
    {
        if let Ok(out) = write_demo_episode1_v3(h.map, h.skill, &demo.commands) {
            let (h2, again) = parse_demo_episode1_v3(&out, h.map).expect("reparses");
            assert_eq!((h2, again), (h, demo));
        }
        assert_eq!(parse_demo_episode1_v2(bytes, map).is_ok(), h.skill >= 2);
    }
    if let Ok(demo) = SessionDemo::parse_v3(data) {
        if let Ok(out) = demo.bytes_v3() {
            assert_eq!(SessionDemo::parse_v3(&out), Ok(demo.clone()));
        }
        assert_eq!(SessionDemo::parse(data).is_ok(), demo.skill >= 2);
    }
});
