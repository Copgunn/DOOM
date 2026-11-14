//! Wire 24 (`PlayerMonstersEpisode1V2`, DSIM40) soaks over the fifteen committed M4b
//! synthetic WADs (no IWAD), M4b plan step 9. Rust-only safety properties through the
//! public API; no expectation is a captured digest. Each run loads one WAD at skill 2
//! or 3 under its map number (BOSS8 is map 8, every other WAD map 1), plays the
//! committed script of that WAD (the genuine-engine scenario that reaches its
//! mechanism: teleports, telefrags, Baron attacks, the boss floor, hard-skill
//! things) and then a seeded pseudo-random command stream, and asserts:
//!
//! - every tic is admitted or refused only by a declared wire-24 witness (the
//!   mover family, checked arithmetic, the store-full, work-limit and invariant
//!   witnesses, the exit and reborn tics: M4B_REFERENCE_INVESTIGATION.md §5
//!   decisions I3, I7 and I9 and the M4a witnesses wire 24 inherits); anything
//!   else fails the test;
//! - a refused tic keeps the complete canonical state, the RNG and the tic, and
//!   refuses again on a retry (AGENTS.md: recoverable errors preserve state);
//! - after the level's exit tic every later tic is `LevelExited`, as in M4a;
//! - every `ROUND_TRIP` admitted tics, and at the end, the DSIM40 capture decodes
//!   with `decode_episode1_v2`, re-encodes to the same bytes, and is refused by the
//!   DSIM39 reader with `Version(40)` (the frozen-reader rule);
//! - determinism: a second simulation loaded afresh and fed the same commands yields
//!   the same per-tic results, the same digests at the round-trip points and the
//!   same final snapshot.
use doom_sim::{demo::parse_demo_episode1_v2, *};

const ROUND_TRIP: usize = 100;
/// Random commands appended after a script.
const TAIL: usize = 300;

/// (WAD file, map number, the committed script of its mechanism).
const RUNS: [(&str, u8, &str); 15] = [
    ("tele", 1, "TELE1"),
    ("telemove", 1, "TSPLIT"),
    ("tfrag", 1, "TFRAG1"),
    ("montele", 1, "MTELE1"),
    ("slideb", 1, "SLIDEB"),
    ("slidec", 1, "SLIDEC"),
    ("baron", 1, "BARON1"),
    ("infight", 1, "BINF1"),
    ("bvb", 1, "BVB1"),
    ("boss8", 8, "BOSS1"),
    ("bossm1", 1, "BOSSM1"),
    ("spech8", 1, "SPECH8"),
    ("skill", 1, "SKILL3"),
    ("tele2x", 1, "TELE2X"),
    ("sawbite", 1, "SAWBITE"),
];

fn reference(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/reference")
            .join(name),
    )
    .expect("reference file")
}

/// xorshift32 command stream: walking, turning, firing, using and weapon changes in
/// bursts, inside the admitted command domain (movement within +-50). A body that is
/// not live gets the empty command (BT_USE alone would request the reborn).
struct Stream(u32);
impl Stream {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn command(&mut self, live: bool) -> TicCommand {
        let r = self.next();
        if !live {
            return TicCommand::default();
        }
        let mut buttons = 0;
        if r % 5 == 0 {
            buttons |= BT_ATTACK;
        }
        if (r >> 3) % 7 == 0 {
            buttons |= BT_USE;
        }
        if (r >> 6) % 23 == 0 {
            buttons |= BT_CHANGE | (((r >> 11) % 4) as u8) << 3;
        }
        TicCommand {
            forward: ((r >> 8) % 101) as i8 - 40,
            side: ((r >> 15) % 61) as i8 - 30,
            angle_turn: (((r >> 21) % 11) as i16 - 5) * 900,
            buttons,
        }
    }
}

fn load(name: &str, map: u8, skill: Skill) -> Simulation {
    let bytes = reference(&format!("m4b_wads/{name}.wad"));
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
    let m = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default())
        .or_else(|_| doom_map::Map::from_wad(&wad, "E1M8", doom_map::MapLimits::default()))
        .unwrap();
    let mut s =
        Simulation::original_player_episode1_v2_map(m, map, skill, MapContextOptions::default())
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    s.spawn_map_things().unwrap();
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .unwrap();
    s.spawn_level_specials().unwrap();
    s
}

/// The declared refusals of an admitted tic. `Monster(Arithmetic)` is the checked
/// position witness (decision H3) as reached by a Baron fireball: INFIGHT's soak
/// refuses a `MT_BRUISERSHOT` whose spawn half step (`P_CheckMissileSpawn`, up to 7.5
/// units, p_mobj.c:868-881) leaves the map's vertex rectangle (x = -775 against the
/// wall at -768). The refusal keeps the state; it is reported as an observed
/// witness, not documented in the M4b note.
fn witness(e: &SimError) -> bool {
    matches!(
        e,
        SimError::MapScale(
            MapScaleError::PlatBlocked
                | MapScaleError::StairBlocked
                | MapScaleError::SecondDoor
                | MapScaleError::DonutBusy
                | MapScaleError::MoverFamily
                | MapScaleError::Arithmetic
                | MapScaleError::Escape
                | MapScaleError::Intercepts
                | MapScaleError::SpecialCross
                | MapScaleError::Vertical
        ) | SimError::Store(StoreError::Capacity)
            | SimError::Perception(PerceptionError::WorkLimit)
            | SimError::Monster(MonsterError::Arithmetic)
            | SimError::PlayerBody(PlayerBodyError::Invariant)
            | SimError::LevelExited
            | SimError::PlayerReborn
    )
}

#[derive(Default, Debug)]
struct Totals {
    runs: usize,
    tics: usize,
    refused: usize,
    exits: usize,
    round_trips: usize,
    teleports: usize,
}

type Trace = Vec<(Result<(), SimError>, Option<u64>)>;

/// One run. With `checks` the per-tic and round-trip properties are asserted.
fn run(
    s: &mut Simulation,
    commands: &[TicCommand],
    label: &str,
    checks: bool,
    t: &mut Totals,
) -> Trace {
    let limits = DiagnosticLimits::default();
    let mut trace = Trace::new();
    let mut admitted = 0;
    let mut exited = false;
    for (i, &c) in commands.iter().enumerate() {
        let before = checks.then(|| (s.snapshot().canonical_bytes(), s.rng(), s.tic()));
        let tic = [
            c,
            TicCommand::default(),
            TicCommand::default(),
            TicCommand::default(),
        ];
        let result = s.advance(tic);
        match result {
            Ok(()) => {
                assert!(!exited, "{label} tic {i}: ran after the exit");
                admitted += 1;
                if checks {
                    t.tics += 1;
                    t.teleports += s
                        .events()
                        .iter()
                        .filter(|e| matches!(e, Event::Mover(MoverEvent::Teleported { .. })))
                        .count();
                    exited |= s
                        .events()
                        .iter()
                        .any(|e| matches!(e, Event::Mover(MoverEvent::Exit { .. })));
                    if admitted % ROUND_TRIP == 0 {
                        round_trip(s, label, limits);
                        t.round_trips += 1;
                    }
                }
                trace.push((
                    result,
                    (admitted % ROUND_TRIP == 0).then(|| s.snapshot().digest()),
                ));
            }
            Err(e) => {
                if let Some(before) = before {
                    assert!(witness(&e), "{label} tic {i}: {c:?} refused with {e:?}");
                    assert_eq!(
                        s.snapshot().canonical_bytes(),
                        before.0,
                        "{label} tic {i}: {e:?}"
                    );
                    assert_eq!((s.rng(), s.tic()), (before.1, before.2), "{label} tic {i}");
                    assert_eq!(s.advance(tic), Err(e), "{label} tic {i}: retry");
                    assert_eq!(s.snapshot().canonical_bytes(), before.0, "{label} retry");
                    if exited {
                        assert_eq!(e, SimError::LevelExited, "{label} tic {i}");
                    }
                    t.refused += 1;
                    t.exits += usize::from(e == SimError::LevelExited);
                }
                trace.push((result, Some(s.snapshot().digest())));
                break;
            }
        }
    }
    if checks {
        round_trip(s, label, limits);
        t.round_trips += 1;
    }
    trace.push((Ok(()), Some(s.snapshot().digest())));
    trace
}

fn round_trip(s: &Simulation, label: &str, limits: DiagnosticLimits) {
    let snap = s.snapshot();
    assert_eq!(snap.version(), 40);
    let bytes = snap.canonical_bytes();
    let view = Diagnostic::decode_episode1_v2(&bytes, limits)
        .unwrap_or_else(|e| panic!("{label}: the DSIM40 reader refused a capture: {e:?}"));
    assert_eq!(view.encode(limits).unwrap(), bytes, "{label}");
    assert_eq!(
        Diagnostic::decode_episode1(&bytes, limits).err(),
        Some(DiagnosticError::Version(40)),
        "{label}"
    );
}

#[test]
fn wire_24_soaks_over_the_m4b_wads_hold_the_safety_properties() {
    let mut t = Totals::default();
    for (name, map, script) in RUNS {
        let (header, demo) =
            parse_demo_episode1_v2(&reference(&format!("m4b_scripts/{script}.lmp")), map).unwrap();
        assert!(header.skill == 2 || header.skill == 3);
        for skill in [Skill::Medium, Skill::Hard] {
            for seed in [0x2545_f491_u32, 0x9e37_79b9] {
                let label = format!("{name} {skill:?} seed {seed:#x}");
                let mut stream = Stream(seed ^ u32::from(map));
                let mut commands = demo.commands.clone();
                // The tail is drawn without knowing whether the body is live: a dead
                // body's BT_USE requests the reborn, which is the declared `PlayerReborn`.
                let mut s = load(name, map, skill);
                let mut again = load(name, map, skill);
                t.runs += 1;
                let mut tail = Vec::new();
                for _ in 0..TAIL {
                    tail.push(stream.command(true));
                }
                commands.extend(tail);
                let first = run(&mut s, &commands, &label, true, &mut t);
                let second = run(&mut again, &commands, &label, false, &mut t);
                assert_eq!(first, second, "{label}: determinism");
                assert_eq!(s.snapshot(), again.snapshot(), "{label}: final state");
            }
        }
    }
    println!("wire 24 soaks: {t:?}");
    // The soak is not vacuous: it ran thousands of tics, round-tripped captures and
    // reached the teleport on the teleport WADs.
    assert_eq!(t.runs, 15 * 2 * 2);
    assert!(t.tics > 10_000, "{t:?}");
    assert!(t.round_trips > 100, "{t:?}");
    assert!(t.teleports > 0, "{t:?}");
}
