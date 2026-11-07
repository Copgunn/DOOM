#![no_main]
//! Bounded public command streams under `PlayerMonstersEpisode1V1` (wire 23,
//! DSIM39) on the nine committed M4a synthetic mechanism WADs
//! (`tests/reference/m4a_wads/*.wad`, map E1M1; no IWAD): doors, floors, plats,
//! stairs and the donut, the exits, the weapons room, items and powers, the Spectre
//! room and the scale map. Each input loads one WAD with
//! `Simulation::original_player_episode1_map` (then `spawn_map_things`,
//! `initialize_player_weapons` with the default inventory and
//! `spawn_level_specials`, as the `player_level_run` example does) and drives
//! `advance` with a byte-derived slot-One command stream.
//!
//! Input: a four-byte header, then four-byte command records. Byte 0 picks the WAD
//! (`data[0] % 9`, order of [`WADS`]); byte 1 the start (0: the default context, as
//! the committed scripts run; any other value: gameplay RNG index `data[1]`);
//! byte 2 the low byte of every command's `angleturn`; byte 3 the capture stride
//! (`16 << (data[3] % 4)` admitted tics between DSIM39 round trips). A record is a
//! version-110 demo ticcmd (forward, side, `angleturn` high byte, buttons), so the
//! command bytes of the committed `.lmp` scripts are records as they stand. Buttons
//! bits 0x40/0x80 are never sent: they are the record's repeat count (none 1, 0x40
//! 4, 0x80 16, both 64 tics). Forward and side −128 are sent as −127. The other
//! button bits are sent as they are (`BT_ATTACK`, `BT_USE`, `BT_CHANGE`, weapon
//! 0–7). At most 600 tics run per input; `-max_len=2404` (4 + 4 × 600).
//!
//! Properties: no panic; every refused tic leaves `snapshot().canonical_bytes()`
//! and the RNG of the state before it; a command inside the admitted command domain
//! of this step (movement −50..50, weapon 0–3 with `BT_CHANGE`) is either executed
//! or refused by a declared witness, which ends the run (each later tic must
//! repeat a sticky `LevelExited` / `PlayerReborn`); a command outside it (movement
//! beyond ±50, weapon 4–7, weapon bits without `BT_CHANGE`) may also be refused by
//! the command preflight, the state kept, and the run continues; the DSIM39 capture
//! round-trips through `Diagnostic::decode_episode1` every stride-th admitted tic
//! and at the end while `decode_level` rejects it with `Version(39)`, and a mutated
//! copy never panics; a second simulation loaded afresh and fed the same bytes
//! yields the same per-tic results and the same digest sequence.
use doom_sim::*;
use doom_types::DoomRng;
use libfuzzer_sys::fuzz_target;
use std::sync::OnceLock;

/// The nine M4a synthetic mechanism WADs (`tests/reference/m4a_manifest.json`).
const WADS: [&[u8]; 9] = [
    include_bytes!("../../tests/reference/m4a_wads/doors.wad"),
    include_bytes!("../../tests/reference/m4a_wads/floors.wad"),
    include_bytes!("../../tests/reference/m4a_wads/plats.wad"),
    include_bytes!("../../tests/reference/m4a_wads/stairs.wad"),
    include_bytes!("../../tests/reference/m4a_wads/exits.wad"),
    include_bytes!("../../tests/reference/m4a_wads/weapons.wad"),
    include_bytes!("../../tests/reference/m4a_wads/items.wad"),
    include_bytes!("../../tests/reference/m4a_wads/spectre.wad"),
    include_bytes!("../../tests/reference/m4a_wads/scale.wad"),
];
const HEADER: usize = 4;
const RECORD: usize = 4;
const TIC_BUDGET: usize = 600;
/// The command bound of the admitted domain at this step (`MAX_COMMAND_MOVE`).
const MAX_MOVE: i8 = 50;

fn maps() -> &'static [doom_map::Map] {
    static MAPS: OnceLock<Vec<doom_map::Map>> = OnceLock::new();
    MAPS.get_or_init(|| {
        WADS.iter()
            .map(|bytes| {
                let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
                doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).unwrap()
            })
            .collect()
    })
}

fn options(rng: u8) -> MapContextOptions {
    if rng == 0 {
        MapContextOptions::default()
    } else {
        MapContextOptions {
            initial_rng: DoomRng::from_indices(rng, 0),
            respawn_monsters: false,
        }
    }
}

/// The level start of WAD `index`, or `None` when this step's policy refuses the
/// WAD at load or spawn (a recoverable refusal; every committed WAD is admitted
/// once the Episode 1 part-1 roster is complete).
fn start(index: usize, rng: u8) -> Option<Simulation> {
    let map = maps()[index].clone();
    let bounds = Episode1Bounds::from_map(&map);
    let mut s = Simulation::original_player_episode1_map(map, options(rng)).ok()?;
    assert_eq!(s.episode1_bounds(), Some(bounds));
    s.spawn_map_things().ok()?;
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .ok()?;
    s.spawn_level_specials().ok()?;
    Some(s)
}

/// The DSIM39 capture of an admitted state: `decode_episode1` round-trips it, the
/// DSIM38 reader rejects it with `Version(39)`, and a mutated copy never panics
/// (a mutated copy that decodes re-encodes to itself).
fn capture(s: &Simulation, flip: [u8; 3]) {
    let limits = DiagnosticLimits::default();
    let snapshot = s.snapshot();
    assert_eq!(snapshot.version(), 39);
    let mut bytes = snapshot.diagnostic_bytes(limits).unwrap();
    let view = Diagnostic::decode_episode1(&bytes, limits).unwrap();
    assert_eq!(view.encode(limits).unwrap(), bytes);
    assert_eq!(
        Diagnostic::decode_level(&bytes, limits).err(),
        Some(DiagnosticError::Version(39))
    );
    let at = usize::from(u16::from_le_bytes([flip[0], flip[1]])) % bytes.len();
    bytes[at] ^= flip[2] | 1;
    if let Ok(v) = Diagnostic::decode_episode1(&bytes, limits) {
        assert_eq!(v.encode(limits).unwrap(), bytes);
    }
}

/// A command the step's command preflight admits: movement inside ±50, weapon
/// bits only with `BT_CHANGE` and then a weapon 0–3.
fn admitted(c: &TicCommand) -> bool {
    let weapon = (c.buttons & BT_WEAPONMASK) >> 3;
    (-MAX_MOVE..=MAX_MOVE).contains(&c.forward)
        && (-MAX_MOVE..=MAX_MOVE).contains(&c.side)
        && if c.buttons & BT_CHANGE != 0 {
            weapon <= 3
        } else {
            weapon == 0
        }
}

/// The preflight refusals of a command outside this step's admitted domain but
/// inside the wire-23 domain (until the command-domain lift of plan step 7).
fn domain_refusal(e: &SimError) -> bool {
    matches!(
        e,
        SimError::PlayerBody(PlayerBodyError::Unsupported)
            | SimError::Weapon(WeaponError::UnsupportedWeapon)
            | SimError::UnsupportedButtons
    )
}

/// The witnesses an admitted tic may be refused with, state preserved, ending the
/// run: the Episode 1 mover witnesses (H7, H8, D1) and the per-tic mover family,
/// checked arithmetic (H3), the store-full witness (H4), the barrels/level
/// policy's witnesses (the 129th intercept, the corner jump, `SpecialCross`, the
/// vertical witness, the per-explosion sight budget, the unresolved dead-player
/// attacker) and the exit and reborn tics.
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
            | SimError::PlayerBody(PlayerBodyError::Invariant)
            | SimError::LevelExited
            | SimError::PlayerReborn
    )
}

/// The slot-One commands of an input, expanded and bounded by [`TIC_BUDGET`].
fn commands(data: &[u8]) -> Vec<TicCommand> {
    let low = data[2];
    let mut out = Vec::new();
    for r in data[HEADER..].as_chunks::<RECORD>().0 {
        let repeat = [1, 4, 16, 64][usize::from(r[3] >> 6)];
        let c = TicCommand {
            forward: i8::from_le_bytes([r[0]]).max(-127),
            side: i8::from_le_bytes([r[1]]).max(-127),
            angle_turn: i16::from_le_bytes([low, r[2]]),
            buttons: r[3] & 0x3f,
        };
        for _ in 0..repeat {
            if out.len() == TIC_BUDGET {
                return out;
            }
            out.push(c);
        }
    }
    out
}

/// One tic's observable outcome: its result and, at a check point, the digest.
type Trace = Vec<(Result<(), SimError>, Option<u64>)>;

/// Drives `s` with `commands`. With `checks`, asserts the properties of the
/// module documentation; the digest is recorded at every stride-th admitted tic,
/// at every refusal and at the end, in both runs.
fn run(s: &mut Simulation, commands: &[TicCommand], stride: usize, checks: bool) -> Trace {
    let mut trace = Trace::new();
    let mut ended: Option<SimError> = None;
    let mut admitted_tics = 0;
    for (t, &c) in commands.iter().enumerate() {
        let before = checks.then(|| s.clone());
        let dead =
            checks && s.original_player(PlayerSlot::One).unwrap().state != PlayerBodyState::Live;
        let mut tic = [TicCommand::default(); 4];
        tic[0] = c;
        let result = s.advance(tic);
        if let Some(before) = &before {
            if let Err(e) = result {
                // Preflight before mutation: the refused tic keeps every byte.
                assert_eq!(
                    s.snapshot().canonical_bytes(),
                    before.snapshot().canonical_bytes(),
                    "tic {t}: {e:?} changed the state"
                );
                assert_eq!(s.rng(), before.rng());
                assert_eq!(s.tic(), before.tic());
                if let Some(sticky) = ended {
                    assert_eq!(e, sticky, "tic {t}");
                } else if admitted(&c) {
                    assert!(witness(&e), "tic {t}: admitted {c:?} refused: {e:?}");
                } else {
                    assert!(
                        witness(&e) || domain_refusal(&e),
                        "tic {t}: {c:?} refused: {e:?}"
                    );
                }
            } else {
                assert_eq!(ended, None, "tic {t}: ran after the run ended");
                assert_eq!(s.tic(), before.tic() + 1);
            }
        }
        match result {
            Ok(()) => {
                admitted_tics += 1;
                let point = admitted_tics % stride == 0;
                if checks {
                    if s.events()
                        .iter()
                        .any(|e| matches!(e, Event::Mover(MoverEvent::Exit { .. })))
                    {
                        ended = Some(SimError::LevelExited);
                    }
                    // P_DeathThink reads BT_USE on an unpaused tic that starts dead.
                    if dead && c.buttons & BT_USE != 0 && ended.is_none() {
                        ended = Some(SimError::PlayerReborn);
                    }
                    if point {
                        capture(s, [c.forward as u8, c.side as u8, c.buttons]);
                    }
                }
                trace.push((result, point.then(|| s.snapshot().digest())));
            }
            Err(e) => {
                trace.push((result, Some(s.snapshot().digest())));
                if witness(&e) {
                    break;
                }
            }
        }
    }
    trace.push((Ok(()), Some(s.snapshot().digest())));
    trace
}

fuzz_target!(|data: &[u8]| {
    if data.len() < HEADER {
        return;
    }
    let index = usize::from(data[0]) % WADS.len();
    let Some(mut s) = start(index, data[1]) else {
        return;
    };
    let stride = 16 << (data[3] % 4);
    let commands = commands(data);
    capture(&s, [data[1], data[2], data[3]]);
    let trace = run(&mut s, &commands, stride, true);
    capture(&s, [data[0], data[3], 0x80]);
    // Determinism: a fresh load fed the same bytes repeats every result and digest.
    let mut again = start(index, data[1]).expect("the same WAD loads twice");
    assert_eq!(run(&mut again, &commands, stride, false), trace);
    assert_eq!(again.snapshot(), s.snapshot());
});
