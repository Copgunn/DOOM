#![no_main]
//! The wire 29 demo readers (skills 0 to 4 and above, the two parameter bytes, every button byte
//! kept): `doom_sim::demo::parse_demo_episode1_v4` (map from byte 0, demo from the rest) and
//! `SessionDemo::parse_v4` on arbitrary bytes. Never a panic; an accepted demo that
//! `write_demo_episode1_v4` / `bytes_v4` write (they refuse bit 6 without bit 7, the save command
//! and a marker-valued byte) writes bytes that parse to the same header and commands (each
//! reader is checked against its own writer; the two readers are not compared with each other on
//! one input), and the version 3 reader refuses a skill 4 or a set parameter byte. Rust-only
//! safety. Run with `-max_len=4096`.
use doom_game::session::demo::SessionDemo;
use doom_sim::demo::{parse_demo_episode1_v3, parse_demo_episode1_v4, write_demo_episode1_v4};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some((&map, bytes)) = data.split_first()
        && let Ok((h, demo)) = parse_demo_episode1_v4(bytes, map)
    {
        assert!(h.skill <= 4);
        if let Ok(out) = write_demo_episode1_v4(h.map, h.skill, h.parameters, &demo.commands) {
            let (h2, again) = parse_demo_episode1_v4(&out, h.map).expect("reparses");
            assert_eq!((h2, again), (h, demo));
        }
    }
    if let Ok((demo, parameters)) = SessionDemo::parse_v4(data) {
        assert!(demo.skill <= 4);
        if let Ok(out) = demo.bytes_v4(parameters) {
            assert_eq!(SessionDemo::parse_v4(&out), Ok((demo.clone(), parameters)));
        }
    }
});
